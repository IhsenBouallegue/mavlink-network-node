use std::error::Error;
use std::sync::Arc;

use embedded_hal_bus::spi::ExclusiveDevice;
use lora_phy::sx126x::{self, Sx126x, Sx126xVariant};
use lora_phy::sx127x::{self, Sx127x, Sx127xVariant};
use lora_phy::LoRa;
use rppal::gpio::{Gpio, Trigger};
use rppal::hal::Delay;
use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
use tokio::sync::{mpsc, Notify};

use super::adapter::BlockingAsync;
use super::delay_adapter::WithDelayNs;
use super::iv::{GenericSx126xInterfaceVariant, GenericSx127xInterfaceVariant};
use super::types::{LoRaDevice, LoRaDeviceSx126x, SpiDevice};

pub const LORA_FREQUENCY_IN_HZ: u32 = 869_525_000;

const LORA_SX1276_CS_PIN: u8 = 25;
const LORA_SX1276_RESET_PIN: u8 = 17;
const LORA_SX1276_DIO0_PIN: u8 = 4;
// const LORA_BUSY_PIN: u8 = 11;

const LORA_SX1262_CS_PIN: u8 = 21;
const LORA_SX1262_RESET_PIN: u8 = 18;
const LORA_SX1262_DIO1_PIN: u8 = 16;
const LORA_SX1262_DIO4_PIN: u8 = 6;
const LORA_SX1262_BUSY_PIN: u8 = 20;

pub fn create_spi() -> Result<SpiDevice, Box<dyn Error>> {
    let gpio = Gpio::new().unwrap();
    let nss = gpio.get(LORA_SX1276_CS_PIN).unwrap().into_output();
    let spi_bus = BlockingAsync::new(Spi::new(Bus::Spi0, SlaveSelect::Ss0, 20_000, Mode::Mode0).unwrap());
    let spi = ExclusiveDevice::new(spi_bus, nss, WithDelayNs::new(Delay));
    Ok(spi)
}
pub fn create_spi_sx1262() -> Result<SpiDevice, Box<dyn Error>> {
    let gpio = Gpio::new().unwrap();
    let nss = gpio.get(LORA_SX1262_CS_PIN).unwrap().into_output();
    let spi_bus = BlockingAsync::new(Spi::new(Bus::Spi0, SlaveSelect::Ss0, 20_000, Mode::Mode0).unwrap());
    let spi = ExclusiveDevice::new(spi_bus, nss, WithDelayNs::new(Delay));
    Ok(spi)
}

pub async fn create_lora_sx1276_spi(spi: SpiDevice) -> Result<LoRaDevice, Box<dyn Error>> {
    let gpio = Gpio::new().unwrap();
    let mut reset = gpio.get(LORA_SX1276_RESET_PIN).unwrap().into_output();
    let mut dio0: rppal::gpio::InputPin = gpio.get(LORA_SX1276_DIO0_PIN).unwrap().into_input_pullup();
    let (interrupt_tx, interrupt_rx) = mpsc::channel(3);

    let _ = dio0.set_async_interrupt(Trigger::RisingEdge, move |_| {
        interrupt_tx.try_send(()).unwrap();
    });

    reset.set_high();
    tokio::time::sleep(std::time::Duration::from_micros(100)).await;
    reset.set_low();
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;

    let config = sx127x::Config {
        chip: Sx127xVariant::Sx1276,
        tcxo_used: false,
    };
    let iv = GenericSx127xInterfaceVariant::new(reset, dio0, None, None, interrupt_rx).unwrap();

    let lora = LoRa::new(Sx127x::new(spi, iv, config), false, WithDelayNs::new(Delay))
        .await
        .unwrap();

    Ok(lora)
}

pub async fn create_lora_sx1262_spi(spi: SpiDevice) -> Result<LoRaDeviceSx126x, Box<dyn Error>> {
    let gpio = Gpio::new().unwrap();
    let reset = gpio.get(LORA_SX1262_RESET_PIN).unwrap().into_output();
    let mut dio1: rppal::gpio::InputPin = gpio.get(LORA_SX1262_DIO1_PIN).unwrap().into_input_pullup();
    let dio4: rppal::gpio::OutputPin = gpio.get(LORA_SX1262_DIO4_PIN).unwrap().into_output();
    let mut busy: rppal::gpio::InputPin = gpio.get(LORA_SX1262_BUSY_PIN).unwrap().into_input_pullup();
    // let (interrupt_tx, interrupt_rx) = mpsc::channel(3);
    let (interrupt_busy_tx, interrupt_busy_rx) = mpsc::channel(3);
    let notify = Arc::new(Notify::new());
    let notify_for_interrupt = notify.clone();
    let _ = dio1.set_async_interrupt(Trigger::RisingEdge, move |_| {
        notify_for_interrupt.notify_one();
    });

    // let _ = busy.set_async_interrupt(Trigger::FallingEdge, move |_| {
    //     interrupt_busy_tx.try_send(()).unwrap();
    // });

    let config = sx126x::Config {
        chip: Sx126xVariant::Sx1262,
        tcxo_ctrl: None,
        use_dcdc: false,
        use_dio2_as_rfswitch: true,
    };

    let iv =
        GenericSx126xInterfaceVariant::new(reset, dio1, busy, None, Some(dio4), notify, interrupt_busy_rx).unwrap();

    let lora = LoRa::new(Sx126x::new(spi, iv, config), false, WithDelayNs::new(Delay))
        .await
        .unwrap();

    Ok(lora)
}
