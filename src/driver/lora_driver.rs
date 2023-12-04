use super::abstract_driver::Driver;
use crate::utils::lora_utils::{create_lora, create_spi, lora_receive};
use crate::utils::mavlink_utils::deserialize_frame;
use crate::utils::types::MavFramePacket;
use crate::utils::{lora_utils::transmit, types::LoRaDevice};
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
            transmit(&mut lora, &data);
        }
    }

    fn receive(&self, on_receive: Arc<Mutex<impl Fn(MavFramePacket)>>) {
        let lora = self.driver_instance.clone();
        let mut lora = lora.write().unwrap();
        let buffer = lora_receive(&mut lora);
        let mavlink_frame: MavFramePacket = deserialize_frame(&buffer);
        let on_receive = on_receive.lock().unwrap();
        on_receive(mavlink_frame);
    }

    fn create_instance() -> Self {
        let spi = create_spi().unwrap();
        let lora = create_lora(spi).unwrap();
        Self {
            driver_instance: Arc::new(RwLock::new(lora)),
        }
    }
}
