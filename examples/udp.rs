use std::sync::Arc;

use futures::future::join_all;
use mavlink_network_node::discover::DiscoveryService;
use mavlink_network_node::full_duplex_network::FullDuplexNetwork;
use mavlink_network_node::logging_utils::init_logging;
use mavlink_network_node::types::NodeType;
use mavlink_network_node::udp_driver::{UDPConfig, UDPDriver};
use mavlink_network_node::NetworkInterface;

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

async fn uav() {}

async fn gateway() {
    let config = UDPConfig {
        addr: "0.0.0.0:14550".to_string(),            // Listening address
        dest_addr: "192.168.1.255:14550".to_string(), // Destination address for sending
        broadcast: true,
    };

    let driver = Arc::new(UDPDriver::new(config).await);
    let channel_size = 100;
    let (udp_network, _tx, _rx) = FullDuplexNetwork::new(driver, channel_size);

    let run_handles = udp_network.run().await;

    join_all(run_handles).await;
}
