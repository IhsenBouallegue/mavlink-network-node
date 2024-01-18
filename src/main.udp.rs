mod driver;
mod network;
mod utils;

use std::{env, thread};

use driver::udp_driver::{UDPDriver, UDP_DRIVER};
use network::network_interface::{HalfDuplexNetworkInterface, NetworkInterface};
use tokio::sync::mpsc;
use utils::logging_utils::{init_logging, log_debug_send_to_network};
use utils::mavlink_utils::{create_groundstation_mavlink, MavlinkHeaderGenerator};
use utils::types::{MavFramePacket, NodeType};

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let node_type = NodeType::from_str(&args[1]).unwrap();
    std::env::set_var("NODE_TYPE", &args[1]);
    let _guard = init_logging();

    match node_type {
        NodeType::Drone => {}
        NodeType::Gateway => {
            let (transmit_udp_tx, transmit_udp_rx) = mpsc::channel(128);
            let (received_udp_tx, mut received_udp_rx) = mpsc::channel(128);

            let udp_thread_handle = tokio::spawn(async move {
                let mut udp_network =
                    HalfDuplexNetworkInterface::<UDPDriver, MavFramePacket>::new(transmit_udp_rx, received_udp_tx);
                udp_network.run().await;
            });

            let transmit_udp_tx_clone = transmit_udp_tx.clone();
            tokio::spawn(async move {
                send_heartbeat_to_network(transmit_udp_tx_clone, UDP_DRIVER, 1000).await;
            });

            tokio::spawn(async move {
                loop {
                    let received = received_udp_rx.recv().await;
                    if let Some(received) = received {
                        println!("Received packet: {:?}", received)
                    }
                }
            });

            // let mavconn = create_groundstation_mavlink();
            // let generator = MavlinkHeaderGenerator::new();
            // loop {
            //     println!("Waiting for mavlink frame...");
            //     mavconn.send_frame(&generator.create_mavlink_heartbeat_frame()).unwrap();
            //     let mav_frame = mavconn.recv_frame();
            //     println!("Received mavlink frame: {:?}", mav_frame);
            //     mavconn.send_frame(&generator.create_mavlink_heartbeat_frame()).unwrap();
            //     thread::sleep(std::time::Duration::from_millis(1000));
            // }
            udp_thread_handle.await.unwrap();
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
