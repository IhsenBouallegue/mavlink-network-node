use tokio::time::sleep;
use utils::logging_utils::init_logging;
use utils::mavlink_utils::MavlinkHeaderGenerator;
use utils::types::NodeType;
use utils::udp_comm::UdpComm;

mod driver;
mod network;
mod utils;

#[tokio::main(flavor = "multi_thread", worker_threads = 1)]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let node_type = NodeType::from_str(&args[1]).unwrap();
    std::env::set_var("NODE_TYPE", &args[1]);
    let _guard = init_logging();

    match node_type {
        NodeType::Drone => {
            let addr = "0.0.0.0:0"; // Bind to all interfaces for receiving
            let discovery_addr = "192.168.0.255:14540";
            let (udp_comm, sender, receiver) = UdpComm::new(100);
            udp_comm.run(addr, discovery_addr, true).await;
            let generator = MavlinkHeaderGenerator::new();

            sender.send(generator.create_mavlink_heartbeat_frame()).await.unwrap();

            let udp_to_lora = tokio::task::Builder::new()
                .name("udp to lora")
                .spawn(async move {
                    let generator = MavlinkHeaderGenerator::new();
                    loop {
                        sender.send(generator.create_mavlink_heartbeat_frame()).await.unwrap();
                        sleep(std::time::Duration::from_millis(1000)).await;
                    }
                })
                .unwrap();

            let _ = tokio::try_join!(udp_to_lora);
        }
        NodeType::Gateway => {
            let addr = "0.0.0.0:0"; // Bind to all interfaces for receiving
            let discovery_addr = "192.168.1.255:14550";
            let (udp_comm, sender, receiver) = UdpComm::new(100);
            udp_comm.run(addr, discovery_addr, true).await;

            let udp_to_lora = tokio::task::Builder::new()
                .name("udp to lora")
                .spawn(async move {
                    let generator = MavlinkHeaderGenerator::new();
                    loop {
                        sender.send(generator.create_mavlink_heartbeat_frame()).await.unwrap();
                        sleep(std::time::Duration::from_millis(1000)).await;
                    }
                })
                .unwrap();

            let _ = tokio::try_join!(udp_to_lora);
        }
    }
}
