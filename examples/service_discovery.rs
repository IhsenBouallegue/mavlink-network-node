use mavlink_network_node::discover::DiscoveryService;
use mavlink_network_node::logging_utils::init_logging;
use mavlink_network_node::types::NodeType;

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let node_type = NodeType::from_str(&args[1]).unwrap();
    std::env::set_var("NODE_TYPE", &args[1]);
    let (discovery_service, discovery_notifier) = DiscoveryService::new();
    let _guard = init_logging(discovery_notifier);

    match node_type {
        NodeType::Drone => {
            let handle = discovery_service.discover().await;
            handle.await.unwrap();
        }
        NodeType::Gateway => {}
    }
}
