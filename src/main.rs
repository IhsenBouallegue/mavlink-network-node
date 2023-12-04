mod driver;
mod network;
mod utils;

use driver::lora_driver::LoRaDriver;
use driver::udp_driver::UDPDriver;
use network::network_interface::GenericNetworkInterface;
use network::network_interface::NetworkInterface;
use utils::types::MavFramePacket;

fn main() {
    let udp_network = GenericNetworkInterface::<UDPDriver, MavFramePacket>::new();

    let lora_network = GenericNetworkInterface::<LoRaDriver, MavFramePacket>::new();
    // lora_network.prepare_to_send(3.14);
    // lora_network.receive();
}
