mod driver;
mod network;
mod utils;

use std::env;

use driver::udp_driver::UDP_DRIVER;
use network::lora_network_interface::LoRaNetworkInterface;
use network::udp_network_interface::UDPNetworkInterface;
use tokio::sync::mpsc;
use utils::logging_utils::{init_logging, log_debug_send_to_network};
use utils::mavlink_utils::MavlinkHeaderGenerator;
use utils::types::{MavFramePacket, NodeType};

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let node_type = NodeType::from_str(&args[1]).unwrap();
    std::env::set_var("NODE_TYPE", &args[1]);
    let _guard = init_logging();

    match node_type {
        NodeType::Drone => {
            // Set up udp network with channels
            let addr = "0.0.0.0:0"; // Bind to all interfaces for receiving
            let discovery_addr = "192.168.0.255:14540";
            let (udp_network_interface, udp_tx, udp_rx) = UDPNetworkInterface::new(100);
            let (udp_recv_task, udp_send_task) = udp_network_interface.run(addr, discovery_addr, true).await;
            let lora_to_udp_tx = udp_tx.clone();

            let lora_network_interface = LoRaNetworkInterface::new_barebone(lora_to_udp_tx, udp_rx);
            let lora_task = lora_network_interface.run().await;

            let udp_heartbeat = tokio::task::Builder::new()
                .name("udp heartbeat")
                .spawn(send_heartbeat_to_network(udp_tx, UDP_DRIVER, 1000))
                .unwrap();

            let _ = tokio::try_join!(lora_task, udp_heartbeat, udp_recv_task, udp_send_task);
        }
        NodeType::Gateway => {
            // Set up udp network with channels
            let addr = "0.0.0.0:0"; // Bind to all interfaces for receiving
            let discovery_addr = "192.168.1.255:14550";
            let (udp_network_interface, udp_tx, udp_rx) = UDPNetworkInterface::new(100);
            let (udp_recv_task, udp_send_task) = udp_network_interface.run(addr, discovery_addr, true).await;
            let lora_to_udp_tx = udp_tx.clone();

            let lora_network_interface = LoRaNetworkInterface::new_barebone(lora_to_udp_tx, udp_rx);
            let lora_task = lora_network_interface.run().await;

            let udp_heartbeat = tokio::task::Builder::new()
                .name("udp heartbeat")
                .spawn(send_heartbeat_to_network(udp_tx, UDP_DRIVER, 1000))
                .unwrap();

            let _ = tokio::try_join!(lora_task, udp_heartbeat, udp_recv_task, udp_send_task);
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
