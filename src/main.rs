mod driver;
mod network;
mod utils;

use std::thread;

use driver::lora_driver::LoRaDriver;
// use driver::udp_driver::UDPDriver;
use network::network_interface::GenericNetworkInterface;
use network::network_interface::NetworkInterface;
use utils::types::MavFramePacket;

use crate::utils::mavlink_utils::create_mavlink_heartbeat_frame;

fn main() {
    // let udp_network = GenericNetworkInterface::<UDPDriver, MavFramePacket>::new();
    // udp_network.push_to_send_queue(create_mavlink_heartbeat_frame());
    // udp_network.send_all();
    let lora_network = GenericNetworkInterface::<LoRaDriver, MavFramePacket>::new();
    // lora_network.push_to_send_queue(create_mavlink_heartbeat_frame());
    // lora_network.send_all();
    loop {
        // udp_network.receive();
        lora_network.push_to_send_queue(create_mavlink_heartbeat_frame());
        lora_network.send_all();
        // let received = udp_network.pop_received_queue();
        // println!("{:#?}", received);
        // let received = lora_network.pop_received_queue();
        // println!("{:#?}", received);
        thread::sleep(std::time::Duration::from_millis(3000));
    }
}
