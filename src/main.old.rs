mod driver;
mod network;
mod utils;

use std::{env, thread};

use driver::lora_driver::{LoRaDriver, LORA_DRIVER};
use driver::udp_driver::{UDPDriver, UDP_DRIVER};
use network::network_interface::{HalfDuplexNetworkInterface, NetworkInterface};
use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use utils::logging_utils::{init_logging, log_debug_send_to_network};
use utils::mavlink_utils::create_mavlink_heartbeat_frame;
use utils::types::{MavFramePacket, NodeType};

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let node_type = NodeType::from_str(&args[1]).unwrap();
    std::env::set_var("NODE_TYPE", &args[1]);
    let _guard = init_logging();

    match node_type {
        NodeType::Drone => {
            // Set up udp network with channels
            let (transmit_udp_tx, transmit_udp_rx) = mpsc::channel(32);
            let (received_udp_tx, mut received_udp_rx) = mpsc::channel(32);

            let udp_thread_handle = tokio::spawn(async move {
                let mut udp_network =
                    HalfDuplexNetworkInterface::<UDPDriver, MavFramePacket>::new(transmit_udp_rx, received_udp_tx);
                udp_network.run().await;
            });

            // Set up lora network with channels
            let (transmit_lora_tx, transmit_lora_rx) = mpsc::channel(32);
            let (received_lora_tx, mut received_lora_rx) = mpsc::channel(32);

            let looa_thread_handle = thread::spawn(move || {
                let runtime = Runtime::new().expect("Failed to create a runtime");
                let mut lora_network =
                    HalfDuplexNetworkInterface::<LoRaDriver, MavFramePacket>::new(transmit_lora_rx, received_lora_tx);
                runtime.block_on(async {
                    lora_network.run().await;
                });
            });

            // Periodically send a heartbeat to udp network
            let transmit_udp_tx_clone = transmit_udp_tx.clone();
            tokio::spawn(async move {
                loop {
                    log_debug_send_to_network(UDP_DRIVER);
                    transmit_udp_tx_clone
                        .send(create_mavlink_heartbeat_frame())
                        .await
                        .unwrap();
                    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                }
            });

            // Periodically send a heartbeat to lora network
            let transmit_lora_tx_clone = transmit_lora_tx.clone();
            tokio::spawn(async move {
                loop {
                    log_debug_send_to_network(LORA_DRIVER);
                    transmit_lora_tx_clone
                        .send(create_mavlink_heartbeat_frame())
                        .await
                        .unwrap();
                    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                }
            });

            // Forward packets from lora to udp
            tokio::spawn(async move {
                loop {
                    let received = received_lora_rx.recv().await;
                    if let Some(received) = received {
                        log_debug_send_to_network(UDP_DRIVER);
                        transmit_udp_tx.send(received).await.unwrap();
                    }
                }
            });

            // Forward packets from udp to lora
            tokio::spawn(async move {
                loop {
                    let received = received_udp_rx.recv().await;
                    if let Some(received) = received {
                        log_debug_send_to_network(LORA_DRIVER);
                        transmit_lora_tx.send(received).await.unwrap();
                    }
                }
            });

            udp_thread_handle.await.unwrap();
            looa_thread_handle.join().unwrap();
        }
        NodeType::Gateway => {
            // Set up udp network with channels
            let (transmit_udp_tx, transmit_udp_rx) = mpsc::channel(32);
            let (received_udp_tx, mut received_udp_rx) = mpsc::channel(32);

            let udp_thread_handle = tokio::spawn(async move {
                let mut udp_network =
                    HalfDuplexNetworkInterface::<UDPDriver, MavFramePacket>::new(transmit_udp_rx, received_udp_tx);
                udp_network.run().await;
            });

            // Set up lora network with channels
            let (transmit_lora_tx, transmit_lora_rx) = mpsc::channel(32);
            let (received_lora_tx, mut received_lora_rx) = mpsc::channel(32);

            let looa_thread_handle = thread::spawn(move || {
                let runtime = Runtime::new().expect("Failed to create a runtime");
                let mut lora_network =
                    HalfDuplexNetworkInterface::<LoRaDriver, MavFramePacket>::new(transmit_lora_rx, received_lora_tx);
                runtime.block_on(async {
                    lora_network.run().await;
                });
            });

            // Periodically send a heartbeat to udp network
            let transmit_udp_tx_clone = transmit_udp_tx.clone();
            tokio::spawn(async move {
                loop {
                    log_debug_send_to_network(UDP_DRIVER);
                    transmit_udp_tx_clone
                        .send(create_mavlink_heartbeat_frame())
                        .await
                        .unwrap();
                    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                }
            });

            // Periodically send a heartbeat to lora network
            let transmit_lora_tx_clone = transmit_lora_tx.clone();
            tokio::spawn(async move {
                loop {
                    log_debug_send_to_network(LORA_DRIVER);
                    transmit_lora_tx_clone
                        .send(create_mavlink_heartbeat_frame())
                        .await
                        .unwrap();
                    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                }
            });

            // Forward packets from lora to udp
            tokio::spawn(async move {
                loop {
                    let received = received_lora_rx.recv().await;
                    if let Some(received) = received {
                        log_debug_send_to_network(UDP_DRIVER);
                        transmit_udp_tx.send(received).await.unwrap();
                    }
                }
            });

            // Forward packets from udp to lora
            tokio::spawn(async move {
                loop {
                    let received = received_udp_rx.recv().await;
                    if let Some(received) = received {
                        log_debug_send_to_network(LORA_DRIVER);
                        transmit_lora_tx.send(received).await.unwrap();
                    }
                }
            });

            udp_thread_handle.await.unwrap();
            looa_thread_handle.join().unwrap();
        }
    }
}
