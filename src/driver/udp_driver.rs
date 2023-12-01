// driver/udp_driver.rs

use super::abstract_driver::Driver;
use std::sync::{Arc, Mutex, RwLock};

pub struct UDPDriver {
    pub driver_instance: Arc<RwLock<Option<i32>>>,
}

impl Driver<i32> for UDPDriver {
    fn send(&self, get_packet_to_send: Arc<Mutex<impl Fn() -> Option<i32>>>) {
        let get_packet_to_send = get_packet_to_send.lock().unwrap();
        if let Some(data) = get_packet_to_send() {
            println!("Sent data over UDP: {:?}", data);
        }
    }

    fn receive(&self, on_receive: Arc<Mutex<impl Fn(i32)>>) {
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
