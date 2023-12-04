extern crate sx127x_lora;

use std::{error::Error, io};

use rppal::{
    gpio::{Gpio, OutputPin},
    hal::Delay,
    spi::{Bus, Mode, SlaveSelect, Spi},
};
use sx127x_lora::LoRa;

const LORA_CS_PIN: u8 = 25;
const LORA_RESET_PIN: u8 = 17;
const LORA_DIO0_PIN: u8 = 4;
const LORA_BUSY_PIN: u8 = 11;
const FREQUENCY: i64 = 868;

pub type LoRaDevice = LoRa<Spi, OutputPin, OutputPin, Delay>;

pub fn create_spi() -> Result<Spi, Box<dyn Error>> {
    let spi = Spi::new(Bus::Spi0, SlaveSelect::Ss0, 16_000_000, Mode::Mode0)?;
    Ok(spi)
}

pub fn create_lora(spi: Spi) -> Result<LoRaDevice, Box<dyn Error>> {
    let gpio = Gpio::new()?;
    let mut nss = Gpio::new()?.get(LORA_CS_PIN)?.into_output();
    nss.set_high();
    let mut reset = Gpio::new()?.get(LORA_RESET_PIN)?.into_output();
    reset.set_high();
    // let mut dio1 = Gpio::new()?.get(LORA_DIO0_PIN)?.into_input();
    // let mut busy = Gpio::new()?.get(LORA_BUSY_PIN)?.into_input();
    let mut lora = sx127x_lora::LoRa::new(spi, nss, reset, FREQUENCY, Delay).unwrap();
    let _ = lora.set_tx_power(14, 1);
    let _ = lora.set_crc(true);
    Ok(lora)
}
