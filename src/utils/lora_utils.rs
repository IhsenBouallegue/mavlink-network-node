extern crate sx127x_lora;

use std::error::Error;

use embedded_hal_bus::spi::ExclusiveDevice;
use lora_phy::mod_params::{Bandwidth, CodingRate, ModulationParams, RadioError, SpreadingFactor};
use lora_phy::sx1276_7_8_9::{self, SX1276_7_8_9};
use lora_phy::LoRa;
use rppal::gpio::Gpio;
use rppal::hal::Delay;
use rppal::spi::{Bus, Mode, SlaveSelect, Spi};

use super::adapter::BlockingAsync;
use super::delay_adapter::WithDelayNs;
use super::iv::GenericSx127xInterfaceVariant;
use super::logging_utils::log_debug_send_packet;
use super::types::{LoRaDevice, MavFramePacket, SpiDevice};
use crate::driver::lora_driver::LORA_DRIVER;

const LORA_CS_PIN: u8 = 25;
const LORA_RESET_PIN: u8 = 17;
const LORA_DIO0_PIN: u8 = 4;
// const LORA_BUSY_PIN: u8 = 11;
const LORA_FREQUENCY_IN_HZ: u32 = 868_000_000;

pub fn create_spi() -> Result<SpiDevice, Box<dyn Error>> {
    let gpio = Gpio::new().unwrap();
    let nss = gpio.get(LORA_CS_PIN).unwrap().into_output();
    let spi_bus = BlockingAsync::new(Spi::new(Bus::Spi0, SlaveSelect::Ss0, 20_000, Mode::Mode0).unwrap());
    let spi = ExclusiveDevice::new(spi_bus, nss, WithDelayNs::new(Delay));
    Ok(spi)
}

pub async fn create_lora(spi: SpiDevice) -> Result<LoRaDevice, Box<dyn Error>> {
    let gpio = Gpio::new().unwrap();
    let mut reset = gpio.get(LORA_RESET_PIN).unwrap().into_output();
    reset.set_high();
    let dio0 = gpio.get(LORA_DIO0_PIN).unwrap().into_input_pullup();

    let config = sx1276_7_8_9::Config {
        chip: sx1276_7_8_9::Sx127xVariant::Sx1276,
        tcxo_used: false,
    };
    let iv = GenericSx127xInterfaceVariant::new(reset, dio0, None, None).unwrap();

    let lora = LoRa::new(SX1276_7_8_9::new(spi, iv, config), false, WithDelayNs::new(Delay))
        .await
        .unwrap();

    Ok(lora)
}

pub async fn lora_transmit(lora: &mut LoRaDevice, mavlink_frame: &MavFramePacket) {
    let mdltn_params = create_modulation_params(lora).unwrap();

    let buffer: &mut [u8; 255] = &mut [0; 255];
    let length = mavlink_frame.ser(buffer);
    let sliced_buffer = &buffer[..length];

    let mut tx_pkt_params = {
        match lora.create_tx_packet_params(4, false, true, false, &mdltn_params) {
            Ok(pp) => pp,
            Err(err) => {
                println!("Radio error = {:?}", err);
                return;
            }
        }
    };

    match lora.prepare_for_tx(&mdltn_params, 17, true).await {
        Ok(()) => {}
        Err(err) => {
            println!("Radio error = {:?}", err);
            return;
        }
    };

    match lora
        .tx(&mdltn_params, &mut tx_pkt_params, sliced_buffer, 0xffffff)
        .await
    {
        Ok(()) => {
            log_debug_send_packet(LORA_DRIVER, &mavlink_frame);
        }
        Err(err) => {
            println!("Radio error = {:?}", err);
            return;
        }
    };
}

pub struct LoRaReceiveResult {
    pub buffer: Vec<u8>,
    pub rssi: i16,
}

pub async fn lora_receive(lora: &mut LoRaDevice) -> Option<LoRaReceiveResult> {
    let mdltn_params = create_modulation_params(lora).unwrap();
    let mut receiving_buffer = [00u8; 255];
    let rx_pkt_params = {
        match lora.create_tx_packet_params(4, false, true, false, &mdltn_params) {
            Ok(pp) => pp,
            Err(err) => {
                println!("Radio error = {:?}", err);
                return None;
            }
        }
    };
    match lora
        .prepare_for_rx(&mdltn_params, &rx_pkt_params, None, None, false)
        .await
    {
        Ok(()) => {}
        Err(err) => {
            println!("Radio error = {:?}", err);
            return None;
        }
    };
    match lora.rx(&rx_pkt_params, &mut receiving_buffer).await {
        Ok((received_len, rx_pkt_status)) => {
            let received_data = Vec::from(&receiving_buffer[..received_len as usize]);
            Some(LoRaReceiveResult {
                buffer: received_data,
                rssi: rx_pkt_status.rssi,
            })
        }
        Err(err) => {
            println!("rx unsuccessful = {:?}", err);
            None
        }
    }
}

pub fn create_modulation_params(lora: &mut LoRaDevice) -> Result<ModulationParams, RadioError> {
    lora.create_modulation_params(
        SpreadingFactor::_10,
        Bandwidth::_250KHz,
        CodingRate::_4_8,
        LORA_FREQUENCY_IN_HZ,
    )
}
