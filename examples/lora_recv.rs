use std::sync::Arc;
use std::time::Duration;

use mavlink_network_node::discover::DiscoveryService;
use mavlink_network_node::logging_utils::init_logging;
use mavlink_network_node::lora_sx1262_spi::LoRaSx1262SpiDriver;
use mavlink_network_node::mavlink_utils::MavlinkHeaderGenerator;
use mavlink_network_node::types::NodeType;
use mavlink_network_node::Driver;
use tokio::time::sleep;

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let node_type = NodeType::from_str(&args[1]).unwrap();
    std::env::set_var("NODE_TYPE", &args[1]);
    let (_discovery_service, discovery_notifier) = DiscoveryService::new();
    let _guard = init_logging(discovery_notifier);

    match node_type {
        NodeType::Uav => {
            uav().await;
        }
        NodeType::Gateway => {
            gateway().await;
        }
    }
}

async fn uav() {
    let driver = Arc::new(LoRaSx1262SpiDriver::new(None).await);
    let mavlink_generator = MavlinkHeaderGenerator::new();
    driver.prepare_to_send().await.unwrap();
    loop {
        driver.send(&mavlink_generator.create_mavlink_heartbeat_frame()).await;
        sleep(Duration::from_millis(1000)).await;
    }
}

async fn gateway() {
    let driver = Arc::new(LoRaSx1262SpiDriver::new(None).await);
    driver.prepare_to_receive().await.unwrap();
    loop {
        driver.ready_to_receive().await.unwrap();
        driver.receive().await;
    }
}
