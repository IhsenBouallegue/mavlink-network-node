use super::abstract_driver::Driver;
use ansi_term::Color;
use std::sync::{Arc, Mutex, RwLock};

pub struct LoRaDriver {
    pub driver_instance: Arc<RwLock<Option<f64>>>,
}

impl Driver<f64> for LoRaDriver {
    fn send(&self, get_packet_to_send: Arc<Mutex<impl Fn() -> Option<f64>>>) {
        let get_packet_to_send = get_packet_to_send.lock().unwrap();
        if let Some(data) = get_packet_to_send() {}
    }

    fn receive(&self, on_receive: Arc<Mutex<impl Fn(f64)>>) {
        let data = Default::default();
        let on_receive = on_receive.lock().unwrap();
        on_receive(data);
    }

    fn create_instance() -> Self {
        Self {
            driver_instance: Arc::new(RwLock::new(None)),
        }
    }
}
