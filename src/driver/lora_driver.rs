use std::fmt::Display;
use std::sync::{Arc, Mutex};

use futures::executor::block_on;

use super::abstract_driver::Driver;
use crate::utils::logging_utils::{log_debug_receive_packet, log_driver_creation};
use crate::utils::lora_utils::{create_lora, create_spi, lora_receive, lora_transmit};
use crate::utils::mavlink_utils::deserialize_frame;
use crate::utils::types::{LoRaDevice, MavFramePacket};

pub const LORA_DRIVER: &str = "lora_driver";
pub struct LoRaDriver {
    pub driver_instance: Arc<Mutex<LoRaDevice>>,
}

impl Display for LoRaDriver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", LORA_DRIVER)
    }
}

impl Driver<MavFramePacket> for LoRaDriver {
    fn create_instance() -> Self {
        let spi = create_spi().unwrap();
        let lora = block_on(create_lora(spi)).expect("Failed to create LoRa instance");
        log_driver_creation(LORA_DRIVER);
        Self {
            driver_instance: Arc::new(Mutex::new(lora)),
        }
    }

    #[tracing::instrument(
        skip(self),
        level = "debug",
        target = "network",
        name = "Transmitting",
        fields(packet_to_send, driver = LORA_DRIVER)
    )]
    async fn send(&self, packet_to_send: MavFramePacket) {
        // log_debug_send_packet(&self.to_string(), &packet_to_send);
        lora_transmit(self.driver_instance.clone(), &packet_to_send).await;
    }

    #[tracing::instrument(
        skip(self),
        level = "debug",
        target = "network",
        name = "Receiving",
        fields(driver = LORA_DRIVER)
    )]
    async fn receive(&self) -> Option<MavFramePacket> {
        if let Some(received_recv_result) = lora_receive(self.driver_instance.clone()).await {
            if let Some(mavlink_frame) = deserialize_frame(&received_recv_result.buffer[..]) {
                log_debug_receive_packet(&self.to_string(), &mavlink_frame, Some(received_recv_result.rssi));
                return Some(mavlink_frame);
            }
            return None;
        }
        return None;
    }
}
