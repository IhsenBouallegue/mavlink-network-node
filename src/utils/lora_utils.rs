use std::error::Error;
use std::sync::{Arc, Mutex};

use embedded_hal_bus::spi::ExclusiveDevice;
use lora_phy::mod_params::{Bandwidth, CodingRate, ModulationParams, PacketParams, RadioError, SpreadingFactor};
use lora_phy::sx1276_7_8_9::{self, SX1276_7_8_9};
use lora_phy::LoRa;
use rppal::gpio::{Gpio, Trigger};
use rppal::hal::Delay;
use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
use tokio::sync::mpsc;

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
const LORA_FREQUENCY_IN_HZ: u32 = 869_525_000;

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
    let mut dio0: rppal::gpio::InputPin = gpio.get(LORA_DIO0_PIN).unwrap().into_input_pullup();
    let (interrupt_tx, interrupt_rx) = mpsc::channel(3);

    let _ = dio0.set_async_interrupt(Trigger::RisingEdge, move |_| {
        interrupt_tx.try_send(()).unwrap();
    });

    reset.set_high();
    tokio::time::sleep(std::time::Duration::from_micros(100)).await;
    reset.set_low();
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;

    let config = sx1276_7_8_9::Config {
        chip: sx1276_7_8_9::Sx127xVariant::Sx1276,
        tcxo_used: false,
    };
    let iv = GenericSx127xInterfaceVariant::new(reset, dio0, None, None, interrupt_rx).unwrap();

    let lora = LoRa::new(SX1276_7_8_9::new(spi, iv, config), false, WithDelayNs::new(Delay))
        .await
        .unwrap();

    Ok(lora)
}

#[tracing::instrument(
    skip(lora),
    level = "debug",
    target = "network",
    name = "Transmitting",
    fields(mavlink_frame, driver = LORA_DRIVER)
)]
pub async fn lora_transmit(lora: Arc<Mutex<LoRaDevice>>, mavlink_frame: &MavFramePacket) {
    let lora = &mut lora.lock().unwrap();
    let mdltn_params = create_modulation_params(lora).unwrap();

    let buffer: &mut [u8; 255] = &mut [0; 255];
    let length = mavlink_frame.ser(buffer);
    let sliced_buffer = &buffer[..length];

    let mut tx_pkt_params = create_tx_packet_params(lora, &mdltn_params);
    prepare_for_tx(lora, &mdltn_params).await;

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

#[tracing::instrument(
    skip(lora, mdltn_params, tx_pkt_params),
    level = "debug",
    target = "network",
    name = "Transmitting",
    fields(mavlink_frame, driver = LORA_DRIVER)
)]
pub async fn lora_trans(
    lora: &mut LoRaDevice,
    mavlink_frame: &MavFramePacket,
    mdltn_params: &ModulationParams,
    tx_pkt_params: &mut PacketParams,
) {
    let buffer: &mut [u8; 255] = &mut [0; 255];
    let length = mavlink_frame.ser(buffer);
    let sliced_buffer = &buffer[..length];

    match lora.tx(mdltn_params, tx_pkt_params, sliced_buffer, 0xffffff).await {
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

pub async fn lora_receive(lora: Arc<Mutex<LoRaDevice>>) -> Option<LoRaReceiveResult> {
    let lora = &mut lora.lock().unwrap();
    let mdltn_params = create_modulation_params(lora).unwrap();
    let rx_pkt_params = create_rx_packet_params(lora, &mdltn_params).unwrap();
    prepare_for_rx(lora, &mdltn_params, &rx_pkt_params).await;

    let mut receiving_buffer = [00u8; 255];
    loop {
        match lora.rx(&rx_pkt_params, &mut receiving_buffer).await {
            Ok((received_len, rx_pkt_status)) => {
                let received_data = Vec::from(&receiving_buffer[..received_len as usize]);
                return Some(LoRaReceiveResult {
                    buffer: received_data,
                    rssi: rx_pkt_status.rssi,
                });
            }
            Err(err) => {
                println!("rx unsuccessful = {:?}", err);
                return None;
            }
        }
    }
}

#[tracing::instrument(
    skip_all,
    level = "debug",
    target = "network",
    name = "Receiving",
    fields(driver = LORA_DRIVER)
)]
pub async fn lora_recv(lora: &mut LoRaDevice) -> Option<LoRaReceiveResult> {
    let mdltn_params = create_modulation_params(lora).unwrap();
    let rx_pkt_params = create_rx_packet_params(lora, &mdltn_params).unwrap();
    // prepare_for_rx(lora, &mdltn_params, &rx_pkt_params).await;

    let mut receiving_buffer = [00u8; 255];
    match lora.rx(&rx_pkt_params, &mut receiving_buffer).await {
        Ok((received_len, rx_pkt_status)) => {
            let received_data = Vec::from(&receiving_buffer[..received_len as usize]);
            return Some(LoRaReceiveResult {
                buffer: received_data,
                rssi: rx_pkt_status.rssi,
            });
        }
        Err(err) => {
            println!("rx unsuccessful = {:?}", err);
            return None;
        }
    }
}

#[tracing::instrument(
    skip(lora, mdltn_params),
    level = "debug",
    target = "network",
    name = "Prepare For TX",
    fields(driver = LORA_DRIVER)
)]
pub async fn prepare_for_tx(lora: &mut LoRaDevice, mdltn_params: &ModulationParams) {
    match lora.prepare_for_tx(mdltn_params, 12, true).await {
        Ok(()) => {}
        Err(err) => {
            println!("Radio error = {:?}", err);
            return;
        }
    };
}

pub fn create_tx_packet_params(lora: &mut LoRaDevice, mdltn_params: &ModulationParams) -> PacketParams {
    lora.create_tx_packet_params(4, false, true, false, mdltn_params)
        .unwrap()
}

#[tracing::instrument(
    skip(lora, mdltn_params, rx_pkt_params),
    level = "debug",
    target = "network",
    name = "Prepare For RX",
    fields(driver = LORA_DRIVER)
)]
pub async fn prepare_for_rx(lora: &mut LoRaDevice, mdltn_params: &ModulationParams, rx_pkt_params: &PacketParams) {
    match lora
        .prepare_for_rx(lora_phy::RxMode::Continuous, mdltn_params, rx_pkt_params, true)
        .await
    {
        Ok(()) => {}
        Err(err) => {
            println!("Radio error = {:?}", err);
            return;
        }
    };
}

#[tracing::instrument(
    skip(lora, mdltn_params, rx_pkt_params),
    level = "debug",
    target = "network",
    name = "Prepare For RX",
    fields(driver = LORA_DRIVER)
)]
pub async fn prepare_for_rx_2(
    lora: Arc<Mutex<LoRaDevice>>,
    mdltn_params: &ModulationParams,
    rx_pkt_params: &PacketParams,
) {
    let mut lora = lora.lock().unwrap();
    match lora
        .prepare_for_rx(lora_phy::RxMode::Continuous, mdltn_params, rx_pkt_params, true)
        .await
    {
        Ok(()) => {}
        Err(err) => {
            println!("Radio error = {:?}", err);
            return;
        }
    };
}

pub fn create_rx_packet_params(
    lora: &mut LoRaDevice,
    mdltn_params: &ModulationParams,
) -> Result<PacketParams, RadioError> {
    let rx_pkt_params = {
        match lora.create_rx_packet_params(4, false, 255 as u8, true, false, &mdltn_params) {
            Ok(pp) => pp,
            Err(err) => {
                println!("Radio error = {:?}", err);
                return Err(err);
            }
        }
    };
    Ok(rx_pkt_params)
}

pub fn create_modulation_params(lora: &mut LoRaDevice) -> Result<ModulationParams, RadioError> {
    lora.create_modulation_params(
        SpreadingFactor::_7,
        Bandwidth::_250KHz,
        CodingRate::_4_5,
        LORA_FREQUENCY_IN_HZ,
    )
}
