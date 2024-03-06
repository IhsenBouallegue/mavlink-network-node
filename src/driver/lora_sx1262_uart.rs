use std::fmt::Display;
use std::sync::Arc;

use tokio::sync::Mutex;

use super::Driver;
use crate::lora_serial::{AirSpeed, PackageSize, PowerLevel};
use crate::mavlink_utils::{deserialize_frame, serialize_frame};
use crate::utils::logging_utils::{log_debug_receive_packet, log_debug_send_packet, log_driver_creation};
use crate::utils::lora_serial::Sx1262UartE22;
use crate::utils::types::MavFramePacket;

pub const LORA_SX1262_UART_DRIVER: &str = "lora_sx1262_uart_driver";

#[allow(dead_code)]
pub struct LoRaSx1262UartConfig {}

#[allow(dead_code)]
pub struct LoRaSx1262UartDriver {
    pub device: Arc<Mutex<Sx1262UartE22>>,
    config: LoRaSx1262UartConfig,
}

impl Display for LoRaSx1262UartDriver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", LORA_SX1262_UART_DRIVER)
    }
}

#[allow(dead_code)]
impl LoRaSx1262UartDriver {
    pub async fn new(_config: LoRaSx1262UartConfig) -> Self {
        let mut lora = Sx1262UartE22::new("/dev/ttyS0").unwrap();
        lora.set(
            868,
            0,
            0xFFFF,
            PowerLevel::Power22dBm,
            true,
            AirSpeed::Speed2400,
            PackageSize::Size240Byte,
            0,
        )
        .unwrap();
        log_driver_creation(LORA_SX1262_UART_DRIVER);

        Self {
            device: Arc::new(Mutex::new(lora)),
            config: LoRaSx1262UartConfig {},
        }
    }
}

#[async_trait::async_trait]
impl Driver<MavFramePacket> for LoRaSx1262UartDriver {
    async fn send(&self, packet: &MavFramePacket) {
        let mut lora = self.device.lock().await;
        let serialised_frame = serialize_frame(packet.clone());
        lora.send(0, 868, &serialised_frame).unwrap();
        log_debug_send_packet(&self.to_string(), packet);
    }

    async fn receive(&self) -> Option<MavFramePacket> {
        let mut lora = self.device.lock().await;
        if let Some(receive_result) = lora.receive() {
            if let Some(mavlink_frame) = deserialize_frame(&receive_result.data[..]) {
                log_debug_receive_packet(
                    &self.to_string(),
                    &mavlink_frame,
                    Some(receive_result.rssi as i16),
                    Some(receive_result.snr as i16),
                );
                return Some(mavlink_frame);
            }
        }
        None
    }
}
