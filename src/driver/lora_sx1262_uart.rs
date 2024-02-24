use std::fmt::Display;
use std::sync::Arc;

use tokio::sync::Mutex;

use super::Driver;
use crate::mavlink_utils::{deserialize_frame, serialize_frame};
use crate::utils::logging_utils::{log_debug_receive_packet, log_debug_send_packet, log_driver_creation};
use crate::utils::lora_serial::Sx1262UartE22;
use crate::utils::types::MavFramePacket;

pub const LORA_SX1262_UART_DRIVER: &str = "lora_sx1262_uart_driver";

#[allow(dead_code)]
pub struct LoRaSX1262UartConfig {}

#[allow(dead_code)]
pub struct LoRaSX1262UartDriver {
    pub device: Arc<Mutex<Sx1262UartE22>>,
    config: LoRaSX1262UartConfig,
}

impl Display for LoRaSX1262UartDriver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", LORA_SX1262_UART_DRIVER)
    }
}

#[allow(dead_code)]
impl LoRaSX1262UartDriver {
    async fn new(_config: LoRaSX1262UartConfig) -> Self {
        let lora = Sx1262UartE22::new("/dev/ttyS0").unwrap();
        log_driver_creation(LORA_SX1262_UART_DRIVER);

        Self {
            device: Arc::new(Mutex::new(lora)),
            config: LoRaSX1262UartConfig {},
        }
    }
}

#[async_trait::async_trait]
impl Driver for LoRaSX1262UartDriver {
    async fn send(&self, packet: &MavFramePacket) {
        let mut lora = self.device.lock().await;
        let serialised_frame = serialize_frame(packet.clone());
        lora.send(0, 868, &serialised_frame).unwrap();
        log_debug_send_packet(&self.to_string(), packet);
    }

    async fn receive(&self) -> Option<MavFramePacket> {
        let mut lora = self.device.lock().await;
        if let Some(serialised_frame) = lora.receive() {
            if let Some(mavlink_frame) = deserialize_frame(&serialised_frame[..]) {
                log_debug_receive_packet(&self.to_string(), &mavlink_frame, None);
                return Some(mavlink_frame);
            }
        }
        None
    }
}
