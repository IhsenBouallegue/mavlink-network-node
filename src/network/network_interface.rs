use std::fmt::Display;
use std::sync::Arc;

use serde::Serialize;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::Mutex;

use crate::driver::abstract_driver::Driver;
use crate::utils::logging_utils::{
    log_debug_send_to_main, log_listen_initiated, log_network_interface_creation, log_network_interface_running,
    log_transmit_initiated,
};

pub trait NetworkInterface<DriverType: Driver<PacketType>, PacketType: Send> {
    async fn transmit(&mut self, packet: PacketType);
    async fn listen(&mut self) -> Option<PacketType>;
    async fn new(to_send: Receiver<PacketType>, received: Sender<PacketType>) -> Self;
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
    async fn new(to_send: Receiver<PacketType>, received: Sender<PacketType>) -> Self {
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
                Some(packet) = self.to_send.recv() => {
                    self.transmit(packet).await;
                }
                Some(packet) = self.driver.receive() => {
                    log_listen_initiated(&self.driver.to_string());
                    self.received.send(packet).await.unwrap();
                    log_debug_send_to_main(&self.driver.to_string());
                }
            }
        }
    }
}

// pub struct FullDuplexNetworkInterface<DriverType: Driver<PacketType>, PacketType: Send> {
//     driver: Arc<Mutex<DriverType>>,
//     to_send: Receiver<PacketType>,
//     received: Sender<PacketType>,
// }

// impl<DriverType: Driver<PacketType>, PacketType: Send> NetworkInterface<DriverType, PacketType>
//     for FullDuplexNetworkInterface<DriverType, PacketType>
// where
//     DriverType: Driver<PacketType> + Display + Send + Sync + 'static,
//     PacketType: std::fmt::Debug + Send + Sync + 'static + Serialize,
// {
//     async fn new(to_send: Receiver<PacketType>, received: Sender<PacketType>) -> Self {
//         let driver_instance = Arc::new(Mutex::new(DriverType::create_instance()));
//         log_network_interface_creation(&driver_instance.lock().await.to_string());
//         Self {
//             driver: driver_instance,
//             to_send,
//             received,
//         }
//     }

//     async fn transmit(&mut self, packet: PacketType) {
//         log_transmit_initiated(&self.driver.lock().await.to_string());
//         self.driver.lock().await.send(packet).await;
//     }

//     async fn listen(&mut self) -> Option<PacketType> {
//         log_listen_initiated(&self.driver.lock().await.to_string());
//         self.driver.lock().await.receive().await
//     }
//     async fn run(&mut self) {
//         log_network_interface_running(&self.driver.lock().await.to_string());

//         let listen_task = {
//             let received = self.received.clone();
//             let driver = self.driver.clone();
//             tokio::spawn(async move {
//                 loop {
//                     if let Some(packet) = driver.lock().await.receive().await {
//                         log_debug_send_to_main(&driver.lock().await.to_string());
//                         received.send(packet).await.unwrap();
//                     }
//                 }
//             })
//         };

//         while let Some(packet) = self.to_send.recv().await {
//             let driver = self.driver.lock().await;
//             log_transmit_initiated(&driver.to_string());
//             driver.send(packet).await;
//         }

//         listen_task.await.unwrap()
//     }
// }
