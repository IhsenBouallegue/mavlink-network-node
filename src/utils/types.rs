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
