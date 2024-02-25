use std::fmt::Display;
use std::sync::Arc;

use lora_phy::mod_params::{ModulationParams, PacketParams};
use lora_phy::mod_traits::{IrqState, TargetIrqState};
use tokio::sync::Mutex;

use super::Driver;
use crate::mavlink_utils::deserialize_frame;
use crate::utils::logging_utils::{log_debug_receive_packet, log_debug_send_packet, log_driver_creation};
use crate::utils::lora_utils::{create_lora_sx1262_spi, create_spi_sx1262, LORA_FREQUENCY_IN_HZ};
use crate::utils::types::{LoRaDeviceSx126x, MavFramePacket};

pub const LORA_SX1262_SPI_DRIVER: &str = "lora_sx1262_spi_driver";

#[allow(dead_code)]
pub struct LoRaSx1262SpiConfig {
    mdltn_params: ModulationParams,
    rx_pkt_params: PacketParams,
    tx_pkt_params: PacketParams,
}

pub struct LoRaSx1262SpiDriver {
    pub device: Arc<Mutex<LoRaDeviceSx126x>>,
    config: LoRaSx1262SpiConfig,
}

impl Display for LoRaSx1262SpiDriver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", LORA_SX1262_SPI_DRIVER)
    }
}

#[allow(dead_code)]
impl LoRaSx1262SpiDriver {
    pub async fn new(_config: Option<LoRaSx1262SpiConfig>) -> Self {
        let spi = create_spi_sx1262().unwrap();
        let mut lora = create_lora_sx1262_spi(spi)
            .await
            .expect("Failed to create LoRa instance");
        let mdltn_params = lora
            .create_modulation_params(
                lora_phy::mod_params::SpreadingFactor::_7,
                lora_phy::mod_params::Bandwidth::_250KHz,
                lora_phy::mod_params::CodingRate::_4_5,
                LORA_FREQUENCY_IN_HZ,
            )
            .unwrap();
        let rx_pkt_params = lora
            .create_rx_packet_params(4, false, 255 as u8, true, false, &mdltn_params)
            .unwrap();
        let tx_pkt_params = lora
            .create_tx_packet_params(4, false, true, false, &mdltn_params)
            .unwrap();

        log_driver_creation(LORA_SX1262_SPI_DRIVER);

        Self {
            device: Arc::new(Mutex::new(lora)),
            config: LoRaSx1262SpiConfig {
                mdltn_params,
                rx_pkt_params,
                tx_pkt_params,
            },
        }
    }
}

#[async_trait::async_trait]
impl Driver for LoRaSx1262SpiDriver {
    async fn send(&self, packet: &MavFramePacket) {
        let mut lora = self.device.lock().await;
        let buffer: &mut [u8; 255] = &mut [0; 255];
        let length = packet.ser(buffer);
        let sliced_buffer = &buffer[..length];

        match lora
            .tx(
                &self.config.mdltn_params,
                &mut self.config.tx_pkt_params.clone(),
                sliced_buffer,
                0xffffff,
            )
            .await
        {
            Ok(()) => {
                log_debug_send_packet(&self.to_string(), &packet);
            }
            Err(err) => {
                println!("Radio error = {:?}", err);
                return;
            }
        };
    }

    async fn receive(&self) -> Option<MavFramePacket> {
        let mut lora = self.device.lock().await;

        let target_irq_state = lora.process_irq_event(TargetIrqState::Done).await.unwrap();
        if let Some(TargetIrqState::Done) = target_irq_state {
            let mut receiving_buffer = [00u8; 255];
            match lora
                .process_rx_irq(&self.config.rx_pkt_params, &mut receiving_buffer, TargetIrqState::Done)
                .await
            {
                Ok(IrqState::RxDone(received_len, rx_pkt_status)) => {
                    let received_data = Vec::from(&receiving_buffer[..received_len as usize]);
                    if let Some(mavlink_frame) = deserialize_frame(&received_data[..]) {
                        // log_packet_received(received_len as usize, None, &mavlink_frame, LORA_DRIVER);
                        log_debug_receive_packet(&self.to_string(), &mavlink_frame, Some(rx_pkt_status.rssi));
                        return Some(mavlink_frame);
                    }
                }
                // PreambleReceived is not expected here as we passed target_rx_state = TargetIrqState::Done
                Ok(IrqState::PreambleReceived) => unreachable!(),
                _ => return None,
            }
        }
        None
    }

    async fn ready_to_receive(&self) -> Result<(), &str> {
        // DANGER not cancellation safe
        let mut lora = self.device.lock().await;
        match lora.wait_for_irq().await {
            Ok(_) => Ok(()),
            Err(_err) => {
                return Err("Failed to wait for IRQ");
            }
        }
    }

    async fn prepare_to_receive(&self) -> Result<(), &str> {
        // DANGER not cancellation safe
        let mut lora = self.device.lock().await;
        match lora
            .prepare_for_rx(
                lora_phy::RxMode::Continuous,
                &self.config.mdltn_params,
                &self.config.rx_pkt_params,
                true,
            )
            .await
        {
            Ok(()) => Ok(()),
            Err(err) => {
                println!("Radio error = {:?}", err);
                Err("Failed to prepare for RX")
            }
        }
    }

    async fn prepare_to_send(&self) -> Result<(), &str> {
        // DANGER not cancellation safe
        let mut lora = self.device.lock().await;
        match lora.prepare_for_tx(&self.config.mdltn_params, 12, true).await {
            Ok(()) => Ok(()),
            Err(err) => {
                println!("Radio error = {:?}", err);
                Err("Failed to prepare for RX")
            }
        }
    }
}
