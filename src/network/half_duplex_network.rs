use std::sync::Arc;
use std::time::Duration;

use tokio::spawn;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tracing::error;

use super::NetworkInterface;
use crate::driver::Driver;
use crate::utils::logging_utils::log_debug_send_to_main;

const CONTINOUS_TRANSMISSION_PACKET_LIMIT: u8 = 5;
const CONTINOUS_TRANSMISSION_WAIT_MS: u64 = 2;

pub struct HalfDuplexNetwork<P> {
    driver: Arc<dyn Driver<P> + Send + Sync>,
    send_channel: Sender<P>,
    recv_channel: Receiver<P>,
}

impl<P: Send + 'static> NetworkInterface<P> for HalfDuplexNetwork<P> {
    fn new(driver: Arc<dyn Driver<P> + Send + Sync>, buffer_size: usize) -> (Self, Sender<P>, Receiver<P>) {
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

    fn new_barebone(driver: Arc<dyn Driver<P> + Send + Sync>, tx_send: Sender<P>, rx_recv: Receiver<P>) -> Self {
        HalfDuplexNetwork {
            driver,
            send_channel: tx_send,
            recv_channel: rx_recv,
        }
    }

    async fn run(mut self) -> Vec<JoinHandle<()>> {
        let task = spawn(async move {
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
        });

        vec![task]
    }
}
