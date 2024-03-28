use std::sync::Arc;

use futures::future::join_all;
use mavlink_network_node::discover::DiscoveryService;
use mavlink_network_node::full_duplex_network::FullDuplexNetwork;
use mavlink_network_node::half_duplex_network::HalfDuplexNetwork;
use mavlink_network_node::logging_utils::{init_logging, log_debug_send_to_network};
use mavlink_network_node::lora_sx1276_spi::LoRaSx1276SpiDriver;
use mavlink_network_node::mavlink_utils::MavlinkHeaderGenerator;
use mavlink_network_node::types::{MavFramePacket, NodeType};
use mavlink_network_node::udp_driver::{UDPConfig, UDPDriver, UDP_DRIVER};
use mavlink_network_node::NetworkInterface;
use tokio::sync::mpsc;

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let node_type = NodeType::from_str(&args[1]).unwrap();
    std::env::set_var("NODE_TYPE", &args[1]);
    let (discovery_service, discovery_notifier) = DiscoveryService::new();
    let _handle = discovery_service.discover().await;
    let _guard = init_logging(discovery_notifier);

    match node_type {
        NodeType::Drone => {
            drone().await;
        }
        NodeType::Gateway => {
            gateway().await;
        }
    }
}

async fn drone() {
    let config = UDPConfig {
        addr: "0.0.0.0:0".to_string(),                // Bind to all interfaces for receiving
        dest_addr: "192.168.0.255:14540".to_string(), // Destination address for sending
        broadcast: true,
    };

    let udp_driver = Arc::new(UDPDriver::new(config).await);
    let channel_size = 100;
    let (udp_network, udp_tx, udp_rx) = FullDuplexNetwork::new(udp_driver, channel_size);
    let udp_run_handle = udp_network.run().await;

    let lora_to_udp_tx = udp_tx.clone();
    let lora_driver = Arc::new(LoRaSx1276SpiDriver::new(None).await);
    let lora_network = HalfDuplexNetwork::new_barebone(lora_driver, lora_to_udp_tx, udp_rx);
    let lora_run_handle = lora_network.run().await;

    let udp_heartbeat = tokio::spawn(send_heartbeat_to_network(udp_tx, UDP_DRIVER, 1000));

    // get udp_run_handle and lora_run_handle and join them
    join_all(
        udp_run_handle
            .into_iter()
            .chain(lora_run_handle.into_iter())
            .chain(std::iter::once(udp_heartbeat)),
    )
    .await;
}

async fn gateway() {
    let config = UDPConfig {
        addr: "0.0.0.0:0".to_string(),                // Bind to all interfaces for receiving
        dest_addr: "192.168.1.255:14550".to_string(), // Destination address for sending
        broadcast: true,
    };

    let udp_driver = Arc::new(UDPDriver::new(config).await);
    let channel_size = 100;
    let (udp_network, udp_tx, udp_rx) = FullDuplexNetwork::new(udp_driver, channel_size);
    let udp_run_handle = udp_network.run().await;

    let lora_to_udp_tx = udp_tx.clone();
    let lora_driver = Arc::new(LoRaSx1276SpiDriver::new(None).await);
    let lora_network = HalfDuplexNetwork::new_barebone(lora_driver, lora_to_udp_tx, udp_rx);
    let lora_run_handle = lora_network.run().await;

    let udp_heartbeat = tokio::spawn(send_heartbeat_to_network(udp_tx, UDP_DRIVER, 1000));

    // get udp_run_handle and lora_run_handle and join them
    join_all(
        udp_run_handle
            .into_iter()
            .chain(lora_run_handle.into_iter())
            .chain(std::iter::once(udp_heartbeat)),
    )
    .await;
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
