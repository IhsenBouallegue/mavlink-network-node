use std::fmt::Display;
use std::sync::Arc;

use super::abstract_driver::Driver;
use crate::utils::logging_utils::{log_debug_receive_packet, log_debug_send_packet, log_driver_creation};
use crate::utils::mavlink_utils::{create_groundstation_mavlink, create_mavlink, mavlink_receive_async, mavlink_send};
use crate::utils::types::{MavDevice, MavFramePacket, NodeType};

pub const UDP_DRIVER: &str = "udp_driver";

pub struct UDPDriver {
    pub driver_instance: Arc<MavDevice>,
}

impl Display for UDPDriver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", UDP_DRIVER)
    }
}

impl Driver<MavFramePacket> for UDPDriver {
    fn create_instance() -> Self {
        let node_type = NodeType::from_str(std::env::var("NODE_TYPE").unwrap().as_str()).unwrap();
        let mavlink;
        match node_type {
            NodeType::Drone => {
                mavlink = create_mavlink();
            }
            NodeType::Gateway => {
                mavlink = create_groundstation_mavlink();
            }
        }
        log_driver_creation(UDP_DRIVER);

        Self {
            driver_instance: Arc::new(mavlink),
        }
    }

    #[tracing::instrument(
        skip(self),
        level = "debug",
        target = "network",
        name = "Transmitting",
        fields(packet_to_send, driver = UDP_DRIVER)
    )]
    async fn send(&self, packet_to_send: MavFramePacket) {
        let mavlink = self.driver_instance.clone();
        log_debug_send_packet(&self.to_string(), &packet_to_send);
        mavlink_send(&mavlink, &packet_to_send)
    }

    #[tracing::instrument(
        skip(self),
        level = "debug",
        target = "network",
        name = "Receiving",
        fields(packet_to_send, driver = UDP_DRIVER)
    )]
    async fn receive(&self) -> Option<MavFramePacket> {
        let mavlink = self.driver_instance.clone();
        if let Some(mavlink_frame) = mavlink_receive_async(mavlink).await {
            log_debug_receive_packet(UDP_DRIVER, &mavlink_frame, None);
            return Some(mavlink_frame);
        }
        None
    }
}
