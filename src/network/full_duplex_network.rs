use std::sync::Arc;

use tokio::sync::mpsc::{self, Receiver, Sender};
use tracing::error;

use super::NetworkInterface;
use crate::driver::Driver;
use crate::utils::logging_utils::log_debug_send_to_main;
use crate::utils::types::MavFramePacket;

pub struct FullDuplexNetwork {
    driver: Arc<dyn Driver>,
    send_channel: Sender<MavFramePacket>,
    recv_channel: Receiver<MavFramePacket>,
}

impl NetworkInterface for FullDuplexNetwork {
    fn new(driver: Arc<dyn Driver>, buffer_size: usize) -> (Self, Sender<MavFramePacket>, Receiver<MavFramePacket>) {
        let (tx_send, rx_send) = mpsc::channel(buffer_size);
        let (tx_recv, rx_recv) = mpsc::channel(buffer_size);

        (
            FullDuplexNetwork {
                driver,
                send_channel: tx_send,
                recv_channel: rx_recv,
            },
            tx_recv,
            rx_send,
        )
    }

    fn new_barebone(
        driver: Arc<dyn Driver + Send + Sync>,
        tx_send: Sender<MavFramePacket>,
        rx_recv: Receiver<MavFramePacket>,
    ) -> Self {
        FullDuplexNetwork {
            driver,
            send_channel: tx_send,
            recv_channel: rx_recv,
        }
    }

    async fn run(self) -> Vec<tokio::task::JoinHandle<()>> {
        // Receiving task
        let send_channel = self.send_channel;
        let receive_driver = self.driver.clone();

        let recv_task = tokio::task::Builder::new()
            .name(&format!("{} recv", &receive_driver.to_string()))
            .spawn(async move {
                loop {
                    if let Some(mavlink_frame) = receive_driver.receive().await {
                        match send_channel.try_send(mavlink_frame) {
                            Err(mpsc::error::TrySendError::Full(_)) => {
                                error!("Send channel is full, dropping packet.");
                            }
                            Ok(_) => {
                                log_debug_send_to_main(&receive_driver.to_string());
                            }
                            _ => {}
                        }
                    }
                }
            })
            .unwrap();

        // Sending task
        let sending_driver = self.driver.clone();
        let mut recv_channel = self.recv_channel;
        let send_task = tokio::task::Builder::new()
            .name(&format!("{} send", &sending_driver.to_string()))
            .spawn(async move {
                while let Some(packet) = recv_channel.recv().await {
                    sending_driver.send(&packet).await;
                }
            })
            .unwrap();

        vec![recv_task, send_task]
    }
}
