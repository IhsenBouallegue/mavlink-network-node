use crate::driver::abstract_driver::Driver;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

pub trait NetworkInterface<DriverType: Driver<PacketType>, PacketType: Send> {
    fn new() -> Self;
    fn send_one(&self);
    fn send_all(&self);
    fn push_to_send_queue(&self, data: PacketType);
    fn receive(&self);
    fn pop_received_queue(&self) -> Option<PacketType>;
}

pub struct GenericNetworkInterface<DriverType: Driver<PacketType>, PacketType: Send> {
    driver: Arc<DriverType>,
    to_send: Arc<Mutex<VecDeque<PacketType>>>,
    received: Arc<Mutex<VecDeque<PacketType>>>,
}

impl<DriverType: Driver<PacketType>, PacketType: Send> NetworkInterface<DriverType, PacketType>
    for GenericNetworkInterface<DriverType, PacketType>
where
    PacketType: std::fmt::Debug,
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

    fn send_one(&self) {
        let to_send = self.to_send.clone();
        let driver = self.driver.clone();
        let get_packet_to_send = Arc::new(Mutex::new(move || to_send.lock().unwrap().pop_front()));
        driver.send(get_packet_to_send);
    }

    fn send_all(&self) {
        while !self.to_send.lock().unwrap().is_empty() {
            self.send_one();
        }
    }

    fn push_to_send_queue(&self, data: PacketType) {
        let mut to_send = self.to_send.lock().unwrap();
        to_send.push_back(data);
    }

    fn receive(&self) {
        let received = self.received.clone();
        let driver = self.driver.clone();
        let on_receive = Arc::new(Mutex::new(move |data| {
            received.lock().unwrap().push_back(data);
            println!("Received data packet!");
        }));
        driver.receive(on_receive);
    }

    fn pop_received_queue(&self) -> Option<PacketType> {
        let received_packet = self.received.lock().unwrap().pop_front();
        received_packet
    }
}
