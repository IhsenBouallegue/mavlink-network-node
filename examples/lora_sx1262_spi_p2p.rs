use std::sync::Arc;
use std::time::Duration;

use mavlink_network_node::discover::DiscoveryService;
use mavlink_network_node::half_duplex_network::HalfDuplexNetwork;
use mavlink_network_node::logging_utils::init_logging;
use mavlink_network_node::lora_sx1262_spi::LoRaSx1262SpiDriver;
use mavlink_network_node::mavlink_utils::MavlinkHeaderGenerator;
use mavlink_network_node::types::NodeType;
use mavlink_network_node::NetworkInterface;
use tokio::time::sleep;

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let node_type = NodeType::from_str(&args[1]).unwrap();
    std::env::set_var("NODE_TYPE", &args[1]);
    let (_discovery_service, discovery_notifier) = DiscoveryService::new();
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
    let driver = Arc::new(LoRaSx1262SpiDriver::new(None).await);
    let (lora_network, tx, _rx) = HalfDuplexNetwork::new(driver, 100);
    let _run_handle = lora_network.run().await;
    let mavlink_generator = MavlinkHeaderGenerator::new();

    loop {
        let _ = tx.send(mavlink_generator.create_mavlink_heartbeat_frame()).await;
        sleep(Duration::from_millis(1000)).await;
    }
}

async fn gateway() {
    let driver = Arc::new(LoRaSx1262SpiDriver::new(None).await);
    let (lora_network, tx, mut rx) = HalfDuplexNetwork::new(driver, 100);
    let _run_handle = lora_network.run().await;
    let mavlink_generator = MavlinkHeaderGenerator::new();

    while let Some(_mavlink_frame) = rx.recv().await {
        sleep(Duration::from_millis(100)).await;
        tx.send(mavlink_generator.create_mavlink_heartbeat_frame())
            .await
            .unwrap();
    }
}
