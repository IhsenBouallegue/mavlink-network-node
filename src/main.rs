mod driver;
mod network;
mod utils;

use driver::udp_driver::UDPDriver;
use mavlink::ardupilotmega::MavMessage;
use mavlink::MavFrame;
use network::network_interface::GenericNetworkInterface;
use network::network_interface::NetworkInterface;
use utils::mavlink_utils::create_mavlink_heartbeat_frame;
use utils::mavlink_utils::heartbeat_message;
use utils::mavlink_utils::request_parameters;
use utils::mavlink_utils::request_stream;
use utils::types::MavFramePacket;

fn main() {
    let udp_network = GenericNetworkInterface::<UDPDriver, MavFramePacket>::new();
    udp_network.prepare_to_send(create_mavlink_heartbeat_frame());
    udp_network.send();
    loop {
        udp_network.receive();
        let received = udp_network.get_received();
        println!("{:#?}", received);
    }
    // let lora_network = GenericNetworkInterface::<LoRaDriver, MavFramePacket>::new();
    // lora_network.prepare_to_send(3.14);
    // lora_network.receive();
}
