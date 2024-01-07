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
use utils::mavlink_utils::MavlinkHeaderGenerator;
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
            // Set up udp network with channels
            let (transmit_udp_tx, transmit_udp_rx) = mpsc::channel(32);
            let (received_udp_tx, mut received_udp_rx) = mpsc::channel(32);

            let udp_thread_handle = tokio::spawn(async move {
                let mut udp_network =
                    HalfDuplexNetworkInterface::<UDPDriver, MavFramePacket>::new(transmit_udp_rx, received_udp_tx);
                udp_network.run().await;
            });

            // Periodically send a heartbeat to udp network
            let transmit_udp_tx_clone = transmit_udp_tx.clone();
            tokio::spawn(async move {
                let generator = MavlinkHeaderGenerator::new();
                loop {
                    log_debug_send_to_network(UDP_DRIVER);
                    transmit_udp_tx_clone
                        .send(generator.create_mavlink_heartbeat_frame())
                        .await
                        .unwrap();
                    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                }
            });

            udp_thread_handle.await.unwrap();
        }
    }
}
