use std::sync::Arc;

use mavlink_network_node::full_duplex_network::FullDuplexNetwork;
use mavlink_network_node::logging_utils::init_logging;
use mavlink_network_node::types::NodeType;
use mavlink_network_node::udp_driver::{UDPConfig, UDPDriver};
use mavlink_network_node::{NetworkInterface, RunHandle};

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let node_type = NodeType::from_str(&args[1]).unwrap();
    std::env::set_var("NODE_TYPE", &args[1]);
    let _guard = init_logging();

    match node_type {
        NodeType::Drone => {
            drone().await;
        }
        NodeType::Gateway => {
            gateway().await;
        }
    }
}

async fn drone() {}

async fn gateway() {
    let config = UDPConfig {
        addr: "0.0.0.0:14550".to_string(),            // Listening address
        dest_addr: "192.168.1.255:14550".to_string(), // Destination address for sending
        broadcast: true,
    };

    let driver = Arc::new(UDPDriver::new(config).await);
    let channel_size = 100;
    let (udp_network, _tx, _rx) = FullDuplexNetwork::new(driver, channel_size);

    let run_handle = udp_network.run().await;

    if let RunHandle::Dual(recv_handle, send_handle) = run_handle {
        let _ = tokio::join!(recv_handle, send_handle);
    }
}
