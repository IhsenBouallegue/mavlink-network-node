mod driver;
mod network;
mod utils;

use driver::udp_driver::UDPDriver;
use network::network_interface::GenericNetworkInterface;
use network::network_interface::NetworkInterface;
use utils::mavlink_utils::create_mavlink_heartbeat_frame;
use utils::types::MavFramePacket;

fn main() {
    let udp_network = GenericNetworkInterface::<UDPDriver, MavFramePacket>::new();
    udp_network.push_to_send_queue(create_mavlink_heartbeat_frame());
    udp_network.send_all();
    loop {
        udp_network.receive();
        let received = udp_network.pop_received_queue();
        println!("{:#?}", received);
    }

    // let lora_network = GenericNetworkInterface::<LoRaDriver, MavFramePacket>::new();
    // lora_network.prepare_to_send(3.14);
    // lora_network.receive();
}
