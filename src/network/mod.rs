pub mod full_duplex_network;
pub mod half_duplex_network;

use std::sync::Arc;

use tokio::sync::mpsc::{Receiver, Sender};
use tokio::task::JoinHandle;

use crate::driver::Driver;
#[allow(async_fn_in_trait)]
pub trait NetworkInterface<P> {
    /// Creates a new instance of the network interface along with channels for sending and receiving packets of type `P`.
    fn new(driver: Arc<dyn Driver<P> + Send + Sync>, buffer_size: usize) -> (Self, Sender<P>, Receiver<P>)
    where
        Self: Sized;

    /// Creates a barebone instance of the network interface without creating channels for sending and receiving packets of type `P`.
    fn new_barebone(driver: Arc<dyn Driver<P> + Send + Sync>, tx_send: Sender<P>, rx_recv: Receiver<P>) -> Self
    where
        Self: Sized;

    /// Starts the network interface's asynchronous operation, returning a handle on 1 or 2 tasks depending on whether it is half or full duplex, for sending and receiving packets of type `P`.
    async fn run(self) -> Vec<JoinHandle<()>>;
}
