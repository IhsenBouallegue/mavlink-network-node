mod driver;
mod network;
mod utils;

use std::env;
use std::thread;

use crate::utils::mavlink_utils::create_mavlink_heartbeat_frame;
use driver::lora_driver::LoRaDriver;
use driver::udp_driver::UDPDriver;
use network::network_interface::GenericNetworkInterface;
use network::network_interface::NetworkInterface;
use utils::types::MavFramePacket;
use utils::types::NodeType;

fn main() {
    let args: Vec<String> = env::args().collect();
    let node_type = NodeType::from_str(&args[1]).unwrap();
    std::env::set_var("NODE_TYPE", &args[1]);
    match node_type {
        NodeType::Drone => {
            let udp_network = GenericNetworkInterface::<UDPDriver, MavFramePacket>::new();
            let lora_network = GenericNetworkInterface::<LoRaDriver, MavFramePacket>::new();
            loop {
                udp_network.push_to_send_queue(create_mavlink_heartbeat_frame());
                udp_network.send_all();
                udp_network.receive();
                let mavlink_frame = udp_network.pop_received_queue();
                if let Some(mavlink_frame) = mavlink_frame {
                    lora_network.push_to_send_queue(mavlink_frame);
                    lora_network.send_all();
                }
                lora_network.receive();
                let mavlink_frame = lora_network.pop_received_queue();
                if let Some(mavlink_frame) = mavlink_frame {
                    udp_network.push_to_send_queue(mavlink_frame);
                    udp_network.send_all();
                }
                thread::sleep(std::time::Duration::from_millis(3000));
            }
        }
        NodeType::Gateway => {
            let udp_network = GenericNetworkInterface::<UDPDriver, MavFramePacket>::new();
            let lora_network = GenericNetworkInterface::<LoRaDriver, MavFramePacket>::new();
            loop {
                udp_network.push_to_send_queue(create_mavlink_heartbeat_frame());
                udp_network.send_all();

                lora_network.receive();
                let mavlink_frame = lora_network.pop_received_queue();
                if let Some(mavlink_frame) = mavlink_frame {
                    println!("Received frame: {:#?}", mavlink_frame);
                    udp_network.push_to_send_queue(mavlink_frame);
                    udp_network.send_all();
                }
                print!("heelo");
                udp_network.receive();
                let mavlink_frame = udp_network.pop_received_queue();
                if let Some(mavlink_frame) = mavlink_frame {
                    lora_network.push_to_send_queue(mavlink_frame);
                    lora_network.send_all();
                }
                thread::sleep(std::time::Duration::from_millis(3000));
            }
        }
    }
}
