mod driver;
mod network;
mod utils;

use std::env;
use std::sync::Arc;
use std::thread;

use driver::lora_driver::LoRaDriver;
use network::network_interface::GenericNetworkInterface;
use network::network_interface::NetworkInterface;
use utils::mavlink_utils::create_mavlink_heartbeat_frame;
use utils::types::MavFramePacket;
use utils::types::NodeType;

fn main() {
    let args: Vec<String> = env::args().collect();
    let node_type = NodeType::from_str(&args[1]).unwrap();
    std::env::set_var("NODE_TYPE", &args[1]);
    match node_type {
        NodeType::Drone => {
            let lora_network = GenericNetworkInterface::<LoRaDriver, MavFramePacket>::new();
            let lora_network = Arc::new(lora_network);
            let lora: Arc<GenericNetworkInterface<LoRaDriver, mavlink::MavFrame<mavlink::ardupilotmega::MavMessage>>> =
                lora_network.clone();
            lora.push_to_send_queue(create_mavlink_heartbeat_frame());
            lora.send_all();
            loop {
                lora_network.receive();
                let mavlink_frame = lora_network.pop_received_queue();
                if let Some(mavlink_frame) = mavlink_frame {
                    lora_network.push_to_send_queue(create_mavlink_heartbeat_frame());
                    lora_network.push_to_send_queue(mavlink_frame);
                }
                thread::sleep(std::time::Duration::from_millis(1000));
                lora_network.send_all();
            }
        }
        NodeType::Gateway => {
            let lora_network = GenericNetworkInterface::<LoRaDriver, MavFramePacket>::new();
            let lora_network = Arc::new(lora_network);
            // let lora = lora_network.clone();
            // lora.push_to_send_queue(create_mavlink_heartbeat_frame());
            // lora.send_all();

            loop {
                println!("Start receiving");
                lora_network.receive();
                println!("Receiving done");
                thread::sleep(std::time::Duration::from_millis(100));
                let mavlink_frame = lora_network.pop_received_queue();
                if let Some(mavlink_frame) = mavlink_frame {
                    lora_network.push_to_send_queue(mavlink_frame);
                    lora_network.send_all();
                }
            }
        }
    }
}
