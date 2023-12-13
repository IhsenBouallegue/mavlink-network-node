use super::abstract_driver::Driver;
use crate::utils::lora_utils::transmit;
use crate::utils::lora_utils::{create_lora, create_spi, lora_receive};
use crate::utils::types::LoRaDevice;
use crate::utils::types::MavFramePacket;
use futures::executor::block_on;
use std::sync::{Arc, Mutex, RwLock};
pub struct LoRaDriver {
    pub driver_instance: Arc<RwLock<LoRaDevice>>,
}

impl Driver<MavFramePacket> for LoRaDriver {
    fn send(&self, get_packet_to_send: Arc<Mutex<impl Fn() -> Option<MavFramePacket>>>) {
        let get_packet_to_send = get_packet_to_send.lock().unwrap();
        let lora = self.driver_instance.clone();
        let mut lora = lora.write().unwrap();
        if let Some(data) = get_packet_to_send() {
            block_on(transmit(&mut lora, &data));
        }
    }

    async fn receive(&self, on_receive: Arc<Mutex<impl Fn(MavFramePacket)>>) {
        let lora = self.driver_instance.clone();
        let mut lora = lora.write().unwrap();
        let mavlink_frame = lora_receive(&mut lora).await;
        let mavlink_frame = match mavlink_frame {
            Some(mavlink_frame) => mavlink_frame,
            None => return,
        };
        println!("Received: {:?}", mavlink_frame);
        on_receive.lock().unwrap()(mavlink_frame);
    }

    fn create_instance() -> Self {
        let spi = create_spi().unwrap();
        let lora = block_on(create_lora(spi)).expect("Failed to create LoRa instance");
        Self {
            driver_instance: Arc::new(RwLock::new(lora)),
        }
    }
}
