pub mod lora_sx1262_spi;
pub mod lora_sx1262_uart;
pub mod lora_sx1276_spi;
pub mod udp_driver;

use std::fmt::Display;

use crate::utils::types::MavFramePacket;

#[async_trait::async_trait]
pub trait Driver: Display {
    async fn send(&self, packet_to_send: &MavFramePacket);
    async fn receive(&self) -> Option<MavFramePacket>;
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
