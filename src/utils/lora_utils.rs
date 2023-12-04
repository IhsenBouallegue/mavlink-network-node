extern crate sx127x_lora;

use super::types::{LoRaDevice, MavFramePacket};
use ansi_term::Color;
use rppal::{
    gpio::Gpio,
    hal::Delay,
    spi::{Bus, Mode, SlaveSelect, Spi},
};
use std::error::Error;

const LORA_CS_PIN: u8 = 25;
const LORA_RESET_PIN: u8 = 17;
const LORA_DIO0_PIN: u8 = 4;
const LORA_BUSY_PIN: u8 = 11;
const FREQUENCY: i64 = 868;

pub fn create_spi() -> Result<Spi, Box<dyn Error>> {
    let spi = Spi::new(Bus::Spi0, SlaveSelect::Ss0, 16_000_000, Mode::Mode0)?;
    Ok(spi)
}

pub fn create_lora(spi: Spi) -> Result<LoRaDevice, Box<dyn Error>> {
    let gpio = Gpio::new()?;
    let mut nss = gpio.get(LORA_CS_PIN)?.into_output();
    nss.set_high();
    let mut reset = gpio.get(LORA_RESET_PIN)?.into_output();
    reset.set_high();
    // let mut dio1 = Gpio::new()?.get(LORA_DIO0_PIN)?.into_input();
    // let mut busy = Gpio::new()?.get(LORA_BUSY_PIN)?.into_input();
    let mut lora = sx127x_lora::LoRa::new(spi, nss, reset, FREQUENCY, Delay).unwrap();
    let _ = lora.set_tx_power(14, 1);
    let _ = lora.set_crc(true);
    Ok(lora)
}

pub fn transmit(lora: &mut LoRaDevice, mavlink_frame: &MavFramePacket) {
    let buffer: &mut [u8; 255] = &mut [0; 255];
    let length = mavlink_frame.ser(buffer);
    let transmit = lora.transmit_payload(*buffer, length);
    match transmit {
        Ok(_) => println!(
            "{}",
            Color::White
                .italic()
                .bold()
                .paint("Sending over long link..."),
        ),
        Err(error) => println!("{:?}", error),
    }
}

pub fn lora_receive(lora: &mut LoRaDevice) -> [u8; 255] {
    println!("{}", Color::Yellow.paint("Receiving started..."));
    let poll = lora.poll_irq(Some(30)).unwrap();
    let buffer = lora.read_packet().unwrap();
    buffer
}
