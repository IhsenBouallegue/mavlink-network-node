extern crate linux_embedded_hal as hal;
extern crate sx127x_lora;

use hal::spidev::{SpiModeFlags, SpidevOptions};
use hal::sysfs_gpio::Direction;
use hal::{Delay, Pin, Spidev, SysfsPin};
use std::io;
use sx127x_lora::LoRa;

const LORA_CS_PIN: u64 = 25;
const LORA_RESET_PIN: u64 = 17;
const FREQUENCY: i64 = 868;

pub fn create_spi() -> io::Result<Spidev> {
    let mut spi = Spidev::open("/dev/spidev0.0")?;
    let options: SpidevOptions = SpidevOptions::new()
        .bits_per_word(8)
        .max_speed_hz(20_000)
        .mode(SpiModeFlags::SPI_MODE_0)
        .build();
    spi.configure(&options)?;
    Ok(spi)
}

pub fn create_lora(spi: Spidev) -> io::Result<LoRa<Spidev, SysfsPin, SysfsPin, Delay>> {
    let cs = Pin::new(LORA_CS_PIN);
    cs.export().unwrap();
    cs.set_direction(Direction::Out).unwrap();

    let reset = Pin::new(LORA_RESET_PIN);
    reset.export().unwrap();
    reset.set_direction(Direction::Out).unwrap();

    let mut lora = sx127x_lora::LoRa::new(spi, cs, reset, FREQUENCY, Delay).unwrap();
    let _ = lora.set_tx_power(14, 1);
    let _ = lora.set_crc(true);
    Ok(lora)
}
