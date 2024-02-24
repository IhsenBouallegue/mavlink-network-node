use embedded_hal_bus::spi::ExclusiveDevice;
use lora_phy::sx126x::Sx126x;
use lora_phy::sx127x::Sx127x;
use lora_phy::LoRa;
use mavlink::ardupilotmega::MavMessage;
use mavlink::{MavConnection, MavFrame};
use rppal::gpio::{InputPin, OutputPin};
use rppal::hal::Delay;
use rppal::spi::Spi;

use super::adapter::BlockingAsync;
use super::delay_adapter::WithDelayNs;
use super::iv::{GenericSx126xInterfaceVariant, GenericSx127xInterfaceVariant};

pub type SpiDevice = ExclusiveDevice<BlockingAsync<Spi>, OutputPin, WithDelayNs<Delay>>;

type RadioType = Sx127x<SpiDevice, GenericSx127xInterfaceVariant<OutputPin, InputPin>>;
pub type LoRaDevice = LoRa<RadioType, WithDelayNs<Delay>>;

type RadioTypeSx126x = Sx126x<SpiDevice, GenericSx126xInterfaceVariant<OutputPin, InputPin>>;
pub type LoRaDeviceSx126x = LoRa<RadioTypeSx126x, WithDelayNs<Delay>>;

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
