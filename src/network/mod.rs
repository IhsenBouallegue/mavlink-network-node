pub mod full_duplex_network;
pub mod half_duplex_network;

use std::sync::Arc;

use tokio::sync::mpsc::{Receiver, Sender};
use tokio::task::JoinHandle;

use crate::driver::Driver;
use crate::utils::types::MavFramePacket;

#[allow(async_fn_in_trait)]
pub trait NetworkInterface {
    /// Creates a new instance of the network interface along with channels for sending and receiving packets.
    fn new(driver: Arc<dyn Driver>, buffer_size: usize) -> (Self, Sender<MavFramePacket>, Receiver<MavFramePacket>)
    where
        Self: Sized;

    /// Creates a barebone instance of the network interface without creating channels for sending and receiving.
    fn new_barebone(
        driver: Arc<dyn Driver + Send + Sync>,
        tx_send: Sender<MavFramePacket>,
        rx_recv: Receiver<MavFramePacket>,
    ) -> Self
    where
        Self: Sized;

    /// Starts the network interface's asynchronous operation, returning a handle on 1 or 2 tasks depending on whether it is half or full duplex.
    async fn run(self) -> Vec<JoinHandle<()>>;
}
