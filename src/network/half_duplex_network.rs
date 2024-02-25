use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::time::timeout;
use tracing::error;

use super::{NetworkInterface, RunHandle};
use crate::driver::Driver;
use crate::utils::logging_utils::log_debug_send_to_main;
use crate::utils::types::MavFramePacket;

const CONTINOUS_TRANSMISSION_PACKET_LIMIT: u8 = 5;
const CONTINOUS_TRANSMISSION_WAIT_MS: u64 = 2;

pub struct HalfDuplexNetwork {
    driver: Arc<dyn Driver>,
    send_channel: Sender<MavFramePacket>,
    recv_channel: Receiver<MavFramePacket>,
}

impl NetworkInterface for HalfDuplexNetwork {
    fn new(driver: Arc<dyn Driver>, buffer_size: usize) -> (Self, Sender<MavFramePacket>, Receiver<MavFramePacket>) {
        let (tx_send, rx_send) = mpsc::channel(buffer_size);
        let (tx_recv, rx_recv) = mpsc::channel(buffer_size);

        (
            HalfDuplexNetwork {
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
        HalfDuplexNetwork {
            driver,
            send_channel: tx_send,
            recv_channel: rx_recv,
        }
    }

    async fn run(mut self) -> RunHandle {
        let task = tokio::task::Builder::new()
        .name(&self.driver.to_string())
        .spawn(async move {
            loop {
                self.driver.prepare_to_receive().await.unwrap();
                tokio::select! {
                    // Transmit packets received through channel
                    Some(packet) = self.recv_channel.recv() => {
                        self.driver.prepare_to_send().await.unwrap();
                        self.driver.send(&packet).await;
                        let mut continous_transmission_packet_count: u8 = 0;
                        while let Ok(Some(packet)) = timeout(Duration::from_millis(CONTINOUS_TRANSMISSION_WAIT_MS), self.recv_channel.recv()).await
                        {
                            if continous_transmission_packet_count >= CONTINOUS_TRANSMISSION_PACKET_LIMIT {
                                break;
                            }
                            self.driver.send(&packet).await;
                            continous_transmission_packet_count += 1;
                        }
                    }
                    // Receive packets from LoRa
                    Ok(_) = self.driver.ready_to_receive() => {
                        if let Some(mavlink_frame) = self.driver.receive().await {
                            match self.send_channel.try_send(mavlink_frame) {
                                Err(mpsc::error::TrySendError::Full(_)) => {
                                    error!("Send channel is full, dropping packet.");
                                }
                                Ok(_) => {
                                    log_debug_send_to_main(&self.driver.to_string());
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        })
        .unwrap();

        RunHandle::Single(task)
    }
}
