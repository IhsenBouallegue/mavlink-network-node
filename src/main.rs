mod driver;
mod network;
mod utils;

use std::sync::{Arc, Mutex};
use std::{env, thread};

use driver::lora_driver::{LoRaDriver, LORA_DRIVER};
use driver::udp_driver::{UDPDriver, UDP_DRIVER};
use futures::executor::block_on;
use mavlink::ardupilotmega::{MavCmd, MavMessage, COMMAND_LONG_DATA};
use mavlink::{MavFrame, MavHeader, Message};
use network::network_interface::{HalfDuplexNetworkInterface, NetworkInterface};
use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use tracing::info;
use utils::logging_utils::{init_logging, log_debug_receive_packet, log_debug_send_packet, log_debug_send_to_network};
use utils::lora_utils::{create_lora, create_spi};
use utils::mavlink_utils::MavlinkHeaderGenerator;
use utils::types::{MavFramePacket, NodeType};

use crate::utils::lora_utils::{
    create_modulation_params, create_tx_packet_params, lora_receive, lora_recv, lora_transmit, prepare_for_tx,
};
use crate::utils::mavlink_utils::deserialize_frame;

#[tokio::main(flavor = "multi_thread", worker_threads = 3)]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let node_type = NodeType::from_str(&args[1]).unwrap();
    std::env::set_var("NODE_TYPE", &args[1]);
    let _guard = init_logging();

    match node_type {
        NodeType::Drone => {
            // Set up udp network with channels
            // let (transmit_udp_tx, transmit_udp_rx) = mpsc::channel(128);
            // let (received_udp_tx, received_udp_rx) = mpsc::channel(128);

            // let udp_thread_handle = tokio::spawn(async move {
            //     let mut udp_network =
            //         HalfDuplexNetworkInterface::<UDPDriver, MavFramePacket>::new(transmit_udp_rx, received_udp_tx);
            //     udp_network.run().await;
            // });

            // Set up lora network with channels
            let (transmit_lora_tx, transmit_lora_rx) = mpsc::channel(128);
            let (received_lora_tx, mut received_lora_rx) = mpsc::channel(128);
            let looa_thread_handle = thread::spawn(move || {
                let runtime = Runtime::new().expect("Failed to create a runtime");
                runtime.block_on(async {
                    let mut lora_network = HalfDuplexNetworkInterface::<LoRaDriver, MavFramePacket>::new(
                        transmit_lora_rx,
                        received_lora_tx,
                    )
                    .await;
                    lora_network.run().await;
                });
            });

            // let transmit_udp_tx_clone = transmit_udp_tx.clone();
            // tokio::spawn(async move {
            //     send_heartbeat_to_network(transmit_udp_tx_clone, UDP_DRIVER, 1000).await;
            // });

            let mavconn = utils::mavlink_utils::create_mavlink();
            let mavconn = Arc::new(mavconn);
            let mavconn_clone = mavconn.clone();
            // Forward packets from lora to udp
            tokio::spawn(async move {
                while let Some(received) = received_lora_rx.recv().await {
                    log_debug_send_packet(UDP_DRIVER, &received);
                    mavconn_clone.send_frame(&received).unwrap();
                }
                // forward_packets(received_lora_rx, transmit_udp_tx, UDP_DRIVER).await;
            });
            let mavconn_clone2 = mavconn.clone();
            // Forward packets from udp to lora
            tokio::spawn(async move {
                let generator = MavlinkHeaderGenerator::new();
                mavconn_clone2
                    .send_frame(&generator.create_mavlink_heartbeat_frame())
                    .unwrap();

                // forward_packets(received_udp_rx, transmit_lora_tx, LORA_DRIVER).await;
                loop {
                    let mavframe = mavconn_clone2.recv_frame().unwrap();
                    log_debug_receive_packet(UDP_DRIVER, &mavframe, None);
                    mavconn_clone2
                        .send_frame(&generator.create_mavlink_heartbeat_frame())
                        .unwrap();
                    if mavframe.msg.message_id() == 30
                        || mavframe.msg.message_id() == 141
                        || mavframe.msg.message_id() == 74
                        || mavframe.msg.message_id() == 410
                    {
                        info!("Message ignored");
                        continue;
                    }
                    log_debug_send_to_network(LORA_DRIVER);
                    transmit_lora_tx.send(mavframe).await.unwrap();
                }
            });

            // udp_thread_handle.await.unwrap();
            looa_thread_handle.join().unwrap();
        }
        NodeType::Gateway => {
            // Set up lora network with channels
            let (transmit_lora_tx, mut transmit_lora_rx) = mpsc::channel::<MavFramePacket>(128);
            let (received_lora_tx, mut received_lora_rx) = mpsc::channel(128);

            let lora_task = tokio::task::Builder::new().name("lora").spawn(async move {
                let spi = create_spi().unwrap();
                let mut lora = block_on(create_lora(spi)).expect("Failed to create LoRa instance");

                loop {
                    tokio::select! {
                        Some(received_recv_result) = lora_recv(&mut lora) => {
                            if let Some(mavlink_frame) = deserialize_frame(&received_recv_result.buffer[..]) {
                                log_debug_receive_packet(LORA_DRIVER, &mavlink_frame, Some(received_recv_result.rssi));
                                received_lora_tx.send(mavlink_frame).await.unwrap();
                            }
                        }
                        Some(packet) = transmit_lora_rx.recv() => {
                            let mdltn_params = create_modulation_params(&mut lora).unwrap();

                            let buffer: &mut [u8; 255] = &mut [0; 255];
                            let length = packet.ser(buffer);
                            let sliced_buffer = &buffer[..length];

                            let mut tx_pkt_params = create_tx_packet_params(&mut lora, &mdltn_params);
                            prepare_for_tx(&mut lora, &mdltn_params).await;

                            match lora
                                .tx(&mdltn_params, &mut tx_pkt_params, sliced_buffer, 0xffffff)
                                .await
                            {
                                Ok(()) => {
                                    log_debug_send_packet(LORA_DRIVER, &packet);
                                }
                                Err(err) => {
                                    println!("Radio error = {:?}", err);
                                    return;
                                }
                            };
                        }
                    }
                }
            }).unwrap();

            let mavconn = utils::mavlink_utils::create_groundstation_mavlink();
            let mavconn = Arc::new(mavconn);
            let mavconn_clone = mavconn.clone();
            // Forward packets from lora to udp
            let lora_to_udp = tokio::task::Builder::new()
                .name("lora to udp")
                .spawn(async move {
                    while let Some(received) = received_lora_rx.recv().await {
                        log_debug_send_packet(UDP_DRIVER, &received);
                        mavconn_clone.send_frame(&received).unwrap();
                    }
                    // forward_packets(received_lora_rx, transmit_udp_tx, UDP_DRIVER).await;
                })
                .unwrap();

            let mavconn_clone2 = mavconn.clone();
            // Forward packets from udp to lora
            let udp_to_lora = tokio::task::Builder::new()
                .name("udp to lora")
                .spawn(async move {
                    let generator = MavlinkHeaderGenerator::new();
                    mavconn_clone2
                        .send_frame(&generator.create_mavlink_heartbeat_frame())
                        .unwrap();

                    // forward_packets(received_udp_rx, transmit_lora_tx, LORA_DRIVER).await;
                    loop {
                        let mavframe = mavconn_clone2.recv_frame().unwrap();
                        log_debug_receive_packet(UDP_DRIVER, &mavframe, None);
                        // mavconn_clone2
                        //     .send_frame(&generator.create_mavlink_heartbeat_frame())
                        //     .unwrap();
                        log_debug_send_to_network(LORA_DRIVER);
                        transmit_lora_tx.send(mavframe).await.unwrap();
                    }
                })
                .unwrap();

            let _ = tokio::try_join!(lora_task, lora_to_udp, udp_to_lora);
            // udp_thread_handle.await.unwrap();
            // looa_thread_handle.join().unwrap();
        }
    }
}

async fn forward_packets(
    mut received_rx: mpsc::Receiver<MavFramePacket>,
    transmit_tx: mpsc::Sender<MavFramePacket>,
    log_driver: &str,
) {
    loop {
        let received = received_rx.recv().await;
        if let Some(received) = received {
            log_debug_send_to_network(log_driver);
            transmit_tx.send(received).await.unwrap();
        }
    }
}

async fn send_heartbeat_to_network(transmit_tx: mpsc::Sender<MavFramePacket>, driver: &str, interval_ms: u64) {
    let generator = MavlinkHeaderGenerator::new();

    loop {
        log_debug_send_to_network(driver);
        transmit_tx
            .send(generator.create_mavlink_heartbeat_frame())
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(interval_ms)).await;
    }
}
