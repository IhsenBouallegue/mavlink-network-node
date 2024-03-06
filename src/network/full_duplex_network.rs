use std::sync::Arc;

use tokio::sync::mpsc::{self, Receiver, Sender};
use tracing::error;

use super::NetworkInterface;
use crate::driver::Driver;
use crate::utils::logging_utils::log_debug_send_to_main;

pub struct FullDuplexNetwork<P> {
    driver: Arc<dyn Driver<P>>,
    send_channel: Sender<P>,
    recv_channel: Receiver<P>,
}

impl<P: Send + 'static> NetworkInterface<P> for FullDuplexNetwork<P> {
    fn new(driver: Arc<dyn Driver<P> + Send + Sync>, buffer_size: usize) -> (Self, Sender<P>, Receiver<P>) {
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

    fn new_barebone(driver: Arc<dyn Driver<P> + Send + Sync>, tx_send: Sender<P>, rx_recv: Receiver<P>) -> Self {
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
