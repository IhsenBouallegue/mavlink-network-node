use mavlink::ardupilotmega::MavMessage;
use mavlink::MavConnection;
use mavlink::MavFrame;
use rppal::gpio::OutputPin;
use rppal::hal::Delay;
use rppal::spi::Spi;
use sx127x_lora::LoRa;

pub type LoRaDevice = LoRa<Spi, OutputPin, OutputPin, Delay>;

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
