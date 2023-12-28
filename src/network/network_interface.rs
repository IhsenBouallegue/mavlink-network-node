use std::sync::Arc;

use tokio::sync::mpsc::{Receiver, Sender};

use crate::driver::abstract_driver::Driver;
use crate::utils::logging_utils::{
    log_listen_initiated, log_packet_receive, log_transmit_error, log_transmit_initiated,
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
    DriverType: Driver<PacketType>,
    PacketType: std::fmt::Debug + Send + 'static,
{
    fn new(to_send: Receiver<PacketType>, received: Sender<PacketType>) -> Self {
        Self {
            driver: Arc::new(DriverType::create_instance()),
            to_send,
            received,
        }
    }

    async fn transmit(&mut self) {
        log_transmit_initiated();
        match self.to_send.try_recv() {
            Ok(packet) => {
                log_packet_receive(&packet);
                self.driver.send(packet).await;
                // tokio::time::sleep(Duration::from_millis(200)).await;
            }
            Err(err) => log_transmit_error(err.to_string().as_str()),
        }
    }

    async fn listen(&self) {
        log_listen_initiated();
        let packet = self.driver.receive().await;
        if let Some(packet) = packet {
            log_packet_receive(&packet);
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
