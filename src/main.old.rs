mod driver;
mod network;
mod utils;

use std::env;
use std::sync::Arc;
use std::thread;

use driver::lora_driver::LoRaDriver;
use driver::udp_driver::UDPDriver;
use network::network_interface::GenericNetworkInterface;
use network::network_interface::NetworkInterface;
use utils::mavlink_utils::create_mavlink_header;
use utils::mavlink_utils::create_mavlink_heartbeat_frame;
use utils::mavlink_utils::request_parameters;
use utils::mavlink_utils::request_stream;
use utils::types::MavFramePacket;
use utils::types::NodeType;

fn main() {
    let args: Vec<String> = env::args().collect();
    let node_type = NodeType::from_str(&args[1]).unwrap();
    std::env::set_var("NODE_TYPE", &args[1]);
    match node_type {
        NodeType::Drone => {
            let udp_network = GenericNetworkInterface::<UDPDriver, MavFramePacket>::new();
            let udp_network = Arc::new(udp_network);
            let lora_network = GenericNetworkInterface::<LoRaDriver, MavFramePacket>::new();
            let udp = udp_network.clone();
            udp.push_to_send_queue(create_mavlink_heartbeat_frame());
            udp.send_all();
            thread::spawn(move || loop {
                udp.receive();
                thread::sleep(std::time::Duration::from_millis(1000));
            });

            loop {
                // udp_network.push_to_send_queue(MavFramePacket {
                //     header: create_mavlink_header(),
                //     msg: request_parameters(),
                //     protocol_version: mavlink::MavlinkVersion::V2,
                // });
                // udp_network.push_to_send_queue(MavFramePacket {
                //     header: create_mavlink_header(),
                //     msg: request_stream(),
                //     protocol_version: mavlink::MavlinkVersion::V2,
                // });
                // udp_network.push_to_send_queue(create_mavlink_heartbeat_frame());
                // udp_network.send_all();

                lora_network.receive();
                let mavlink_frame = lora_network.pop_received_queue();
                if let Some(mavlink_frame) = mavlink_frame {
                    udp_network.push_to_send_queue(mavlink_frame);
                    udp_network.send_all();
                }

                let mavlink_frame = udp_network.pop_received_queue();
                if let Some(mavlink_frame) = mavlink_frame {
                    lora_network.push_to_send_queue(mavlink_frame);
                    lora_network.send_all();
                }
                thread::sleep(std::time::Duration::from_millis(1000));
            }
        }
        NodeType::Gateway => {
            let udp_network = GenericNetworkInterface::<UDPDriver, MavFramePacket>::new();
            let udp_network = Arc::new(udp_network);
            let lora_network = GenericNetworkInterface::<LoRaDriver, MavFramePacket>::new();
            // spawn new thread that received from udp network
            let udp = udp_network.clone();

            thread::spawn(move || loop {
                udp.receive();
                thread::sleep(std::time::Duration::from_millis(1000));
            });

            udp_network.push_to_send_queue(create_mavlink_heartbeat_frame());
            udp_network.send_all();
            loop {
                lora_network.receive();
                let mavlink_frame = lora_network.pop_received_queue();
                if let Some(mavlink_frame) = mavlink_frame {
                    udp_network.push_to_send_queue(mavlink_frame);
                    udp_network.send_all();
                }

                let mavlink_frame = udp_network.pop_received_queue();
                if let Some(mavlink_frame) = mavlink_frame {
                    lora_network.push_to_send_queue(mavlink_frame);
                    lora_network.send_all();
                }
                thread::sleep(std::time::Duration::from_millis(1000));
            }
        }
    }
}
