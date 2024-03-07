#[cfg(feature = "embedded")]
pub mod lora_sx1262_spi;
#[cfg(feature = "embedded")]
pub mod lora_sx1262_uart;
#[cfg(feature = "embedded")]
pub mod lora_sx1276_spi;
pub mod udp_driver;
pub mod websocket_driver;

use std::fmt::Display;

#[async_trait::async_trait]
pub trait Driver<P>: Display + Send + Sync {
    async fn send(&self, packet_to_send: &P);
    async fn receive(&self) -> Option<P>;
    // Only relevant for drivers that work in half-duplex mode
    async fn prepare_to_receive(&self) -> Result<(), &str> {
        Ok(())
    }
    async fn prepare_to_send(&self) -> Result<(), &str> {
        Ok(())
    }
    async fn ready_to_receive(&self) -> Result<(), &str> {
        Ok(())
    }
}
