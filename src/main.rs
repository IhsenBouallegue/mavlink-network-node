mod driver;
mod network;

use driver::{lora_driver::LoRaDriver, udp_driver::UDPDriver};
use network::network_interface::{GenericNetworkInterface, NetworkInterface};

fn main() {
    let udp_network = GenericNetworkInterface::<UDPDriver, i32>::new();
    udp_network.prepare_to_send(42);
    udp_network.send();
    udp_network.receive();
    udp_network.get_received();

    let lora_network = GenericNetworkInterface::<LoRaDriver, f64>::new();
    lora_network.prepare_to_send(3.14);
    lora_network.receive();
}
