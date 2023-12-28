use std::fmt::Display;
use std::sync::Arc;

use serde::Serialize;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::driver::abstract_driver::Driver;
use crate::utils::logging_utils::{
    log_debug_receive_packet, log_debug_send_packet, log_driver_creation, log_listen_initiated, log_transmit_error,
    log_transmit_initiated,
};

pub trait NetworkInterface<DriverType: Driver<PacketType>, PacketType: Send> {
    async fn transmit(&mut self);
    async fn listen(&self);
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
        log_driver_creation(&driver_instance.to_string());

        Self {
            driver: driver_instance,
            to_send,
            received,
        }
    }

    async fn transmit(&mut self) {
        log_transmit_initiated(&self.driver.to_string());
        match self.to_send.try_recv() {
            Ok(packet) => {
                log_debug_send_packet(&self.driver.to_string(), &packet);
                self.driver.send(packet).await;
                // tokio::time::sleep(Duration::from_millis(200)).await;
            }
            Err(err) => log_transmit_error(&self.driver.to_string(), &err.to_string()),
        }
    }

    async fn listen(&self) {
        log_listen_initiated(&self.driver.to_string());
        let packet = self.driver.receive().await;
        if let Some(packet) = packet {
            log_debug_receive_packet(&self.driver.to_string(), &packet);
            self.received.send(packet).await.unwrap();
            // tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }

    async fn run(&mut self) {
        loop {
            self.transmit().await;
            self.listen().await;
        }
    }
}
