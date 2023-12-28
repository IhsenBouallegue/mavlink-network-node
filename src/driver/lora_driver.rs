use std::sync::{Arc, RwLock};

use futures::executor::block_on;

use super::abstract_driver::Driver;
use crate::utils::logging_utils;
use crate::utils::lora_utils::{create_lora, create_spi, lora_receive, transmit};
use crate::utils::types::{LoRaDevice, MavFramePacket};
pub struct LoRaDriver {
    pub driver_instance: Arc<RwLock<LoRaDevice>>,
}

impl Driver<MavFramePacket> for LoRaDriver {
    async fn send(&self, packet_to_send: MavFramePacket) {
        let lora = self.driver_instance.clone();
        let mut lora = lora.write().unwrap();
        transmit(&mut lora, &packet_to_send).await;
    }

    async fn receive(&self) -> Option<MavFramePacket> {
        let lora = self.driver_instance.clone();
        let mut lora = lora.write().unwrap();
        let mavlink_frame = lora_receive(&mut lora).await;
        mavlink_frame
    }

    fn create_instance() -> Self {
        let spi = create_spi().unwrap();
        let lora = block_on(create_lora(spi)).expect("Failed to create LoRa instance");
        logging_utils::log_driver_creation("LoRaDriver");

        Self {
            driver_instance: Arc::new(RwLock::new(lora)),
        }
    }
}
