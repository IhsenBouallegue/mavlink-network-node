use embedded_hal_bus::spi::ExclusiveDevice;
use lora_phy::sx1276_7_8_9::SX1276_7_8_9;
use lora_phy::LoRa;
use mavlink::ardupilotmega::MavMessage;
use mavlink::MavConnection;
use mavlink::MavFrame;
use rppal::gpio::InputPin;
use rppal::gpio::OutputPin;
use rppal::hal::Delay;
use rppal::spi::Spi;

use super::adapter::BlockingAsync;
use super::delay_adapter::WithDelayNs;
use super::iv::GenericSx127xInterfaceVariant;

pub type SpiDevice = ExclusiveDevice<BlockingAsync<Spi>, OutputPin, WithDelayNs<Delay>>;

pub type LoRaDevice =
    LoRa<SX1276_7_8_9<SpiDevice, GenericSx127xInterfaceVariant<OutputPin, InputPin>>, WithDelayNs<Delay>>;

pub type MavDevice = Box<dyn MavConnection<MavMessage> + Send + Sync>;

pub type MavFramePacket = MavFrame<MavMessage>;

#[derive(Debug)]
pub enum NodeType {
    Drone,
    Gateway,
}

impl NodeType {
    pub fn from_str(s: &str) -> Result<NodeType, ()> {
        match s {
            "Drone" => Ok(NodeType::Drone),
            "Gateway" => Ok(NodeType::Gateway),
            _ => Err(println!("Invalid node type")),
        }
    }
}
