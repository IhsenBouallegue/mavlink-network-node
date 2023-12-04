use crate::driver::abstract_driver::Driver;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

pub trait NetworkInterface<DriverType: Driver<PacketType> + Send + Sync, PacketType: Send> {
    fn new() -> Self;
    fn send(&self);
    fn prepare_to_send(&self, data: PacketType);
    fn receive(&self);
    fn get_received(&self) -> PacketType;
}

pub struct GenericNetworkInterface<DriverType: Driver<PacketType> + Send + Sync, PacketType: Send> {
    driver: Arc<DriverType>,
    to_send: Arc<Mutex<VecDeque<PacketType>>>,
    received: Arc<Mutex<VecDeque<PacketType>>>,
}

impl<DriverType: Driver<PacketType> + Send + Sync, PacketType: Send>
    NetworkInterface<DriverType, PacketType> for GenericNetworkInterface<DriverType, PacketType>
{
    fn new() -> Self {
        let driver = Arc::new(DriverType::create_instance());
        let to_send = Arc::new(Mutex::new(VecDeque::new()));
        let received = Arc::new(Mutex::new(VecDeque::new()));

        GenericNetworkInterface {
            driver,
            to_send,
            received,
        }
    }

    fn send(&self) {
        let to_send = self.to_send.clone();
        let driver = self.driver.clone();
        let get_packet_to_send = Arc::new(Mutex::new(move || to_send.lock().unwrap().pop_front()));
        driver.send(get_packet_to_send);
    }

    fn prepare_to_send(&self, data: PacketType) {
        let mut to_send = self.to_send.lock().unwrap();
        to_send.push_back(data);
    }

    fn receive(&self) {
        let received = self.received.clone();
        let driver = self.driver.clone();
        let on_receive = Arc::new(Mutex::new(move |data| {
            received.lock().unwrap().push_back(data)
        }));
        driver.receive(on_receive);
    }

    fn get_received(&self) -> PacketType {
        self.received.lock().unwrap().pop_front().unwrap()
    }
}
