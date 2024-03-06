use std::fmt::Display;
use std::sync::Arc;

use lora_phy::mod_params::{Bandwidth, CodingRate, ModulationParams, PacketParams, SpreadingFactor};
use lora_phy::mod_traits::{IrqState, TargetIrqState};
use tokio::sync::Mutex;

use super::Driver;
use crate::define_struct_with_defaults;
use crate::mavlink_utils::{deserialize_frame, serialize_frame};
use crate::utils::logging_utils::{log_debug_receive_packet, log_debug_send_packet, log_driver_creation};
use crate::utils::lora_utils::{create_lora_sx1276_spi, create_spi};
use crate::utils::types::{LoRaDevice, MavFramePacket};

pub const LORA_SX1276_SPI_DRIVER: &str = "lora_sx1276_spi_driver";

define_struct_with_defaults! {
    LoRaSx1276SpiOptionalInitConfig, LoRaSx1276SpiInitConfig {
        spreading_factor: SpreadingFactor = SpreadingFactor::_7,
        bandwidth: Bandwidth = Bandwidth::_250KHz,
        coding_rate: CodingRate = CodingRate::_4_5,
        frequency: u32 = 868_100_000,
        max_payload_length: u8 = 255,
        tx_power: i32 = 12,
        tx_boost: bool = true,
        preamble_length: u16 = 4,
        implicit_header: bool = false,
        crc_enabled: bool = true,
        iq_inverted: bool = false,
    }
}

#[allow(dead_code)]
pub struct LoRaSx1276SpiConfig {
    modulation_params: ModulationParams,
    rx_pkt_params: PacketParams,
    tx_pkt_params: PacketParams,
    tx_power: i32,
    tx_boost: bool,
}

pub struct LoRaSx1276SpiDriver {
    pub device: Arc<Mutex<LoRaDevice>>,
    config: LoRaSx1276SpiConfig,
}

impl Display for LoRaSx1276SpiDriver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", LORA_SX1276_SPI_DRIVER)
    }
}

#[allow(dead_code)]
impl LoRaSx1276SpiDriver {
    pub async fn new(init_config: Option<LoRaSx1276SpiOptionalInitConfig>) -> Self {
        let init_config = init_config.unwrap_or_default().build();

        let spi = create_spi().expect("Failed to create SPI");
        let mut lora = create_lora_sx1276_spi(spi)
            .await
            .expect("Failed to create LoRa instance");

        let modulation_params = lora
            .create_modulation_params(
                init_config.spreading_factor,
                init_config.bandwidth,
                init_config.coding_rate,
                init_config.frequency,
            )
            .expect("Failed to create modulation params");

        let rx_pkt_params = lora
            .create_rx_packet_params(
                init_config.preamble_length,
                init_config.implicit_header,
                init_config.max_payload_length,
                init_config.crc_enabled,
                init_config.iq_inverted,
                &modulation_params,
            )
            .expect("Failed to create RX packet params");

        let tx_pkt_params = lora
            .create_tx_packet_params(
                init_config.preamble_length,
                init_config.implicit_header,
                init_config.crc_enabled,
                init_config.iq_inverted,
                &modulation_params,
            )
            .expect("Failed to create TX packet params");

        log_driver_creation(LORA_SX1276_SPI_DRIVER);

        Self {
            device: Arc::new(Mutex::new(lora)),
            config: LoRaSx1276SpiConfig {
                modulation_params,
                rx_pkt_params,
                tx_pkt_params,
                tx_power: init_config.tx_power,
                tx_boost: init_config.tx_boost,
            },
        }
    }
}

#[async_trait::async_trait]
impl Driver for LoRaSx1276SpiDriver {
    async fn send(&self, packet: &MavFramePacket) {
        let mut lora = self.device.lock().await;
        let serialised_packet = serialize_frame(packet.clone());

        match lora
            .tx(
                &self.config.modulation_params,
                &mut self.config.tx_pkt_params.clone(),
                &serialised_packet,
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

        let target_irq_state = lora.process_irq_event().await.unwrap();
        if let Some(TargetIrqState::Done) = target_irq_state {
            let mut receiving_buffer = [00u8; 255];
            match lora
                .process_rx_irq(&self.config.rx_pkt_params, &mut receiving_buffer)
                .await
            {
                Ok(IrqState::RxDone(received_len, rx_pkt_status)) => {
                    let received_data = Vec::from(&receiving_buffer[..received_len as usize]);
                    if let Some(mavlink_frame) = deserialize_frame(&received_data[..]) {
                        // log_packet_received(received_len as usize, None, &mavlink_frame, LORA_DRIVER);
                        log_debug_receive_packet(
                            &self.to_string(),
                            &mavlink_frame,
                            Some(rx_pkt_status.rssi),
                            Some(rx_pkt_status.snr),
                        );
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
                &self.config.modulation_params,
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
        match lora
            .prepare_for_tx(
                &self.config.modulation_params,
                self.config.tx_power,
                self.config.tx_boost,
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
}
