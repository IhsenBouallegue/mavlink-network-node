use std::fmt::Display;
use std::sync::Arc;

use serde::Serialize;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::driver::abstract_driver::Driver;
use crate::utils::logging_utils::{
    log_debug_send_to_main, log_listen_initiated, log_network_interface_creation, log_network_interface_running,
    log_transmit_initiated,
};

pub trait NetworkInterface<DriverType: Driver<PacketType>, PacketType: Send> {
    async fn transmit(&mut self, packet: PacketType);
    async fn listen(&mut self) -> Option<PacketType>;
    fn new(to_send: Receiver<PacketType>, received: Sender<PacketType>) -> Self;
    async fn run(&mut self);
}

pub struct HalfDuplexNetworkInterface<DriverType: Driver<PacketType>, PacketType: Send> {
    driver: Arc<DriverType>,
    to_send: Receiver<PacketType>,
    received: Sender<PacketType>,
}

impl<DriverType: Driver<PacketType>, PacketType: Send> NetworkInterface<DriverType, PacketType>
    for HalfDuplexNetworkInterface<DriverType, PacketType>
where
    DriverType: Driver<PacketType> + Display,
    PacketType: std::fmt::Debug + Send + 'static + Serialize,
{
    fn new(to_send: Receiver<PacketType>, received: Sender<PacketType>) -> Self {
        let driver_instance = Arc::new(DriverType::create_instance());
        log_network_interface_creation(&driver_instance.to_string());
        Self {
            driver: driver_instance,
            to_send,
            received,
        }
    }

    async fn transmit(&mut self, packet: PacketType) {
        log_transmit_initiated(&self.driver.to_string());
        self.driver.send(packet).await;
    }

    async fn listen(&mut self) -> Option<PacketType> {
        log_listen_initiated(&self.driver.to_string());
        self.driver.receive().await
    }

    async fn run(&mut self) {
        log_network_interface_running(&self.driver.to_string());
        loop {
            tokio::select! {
                Some(packet) = self.driver.receive() => {
                    log_listen_initiated(&self.driver.to_string());
                    self.received.send(packet).await.unwrap();
                    log_debug_send_to_main(&self.driver.to_string());
                }
                Some(packet) = self.to_send.recv() => {
                    self.transmit(packet).await;
                }
            }
        }
    }
}
