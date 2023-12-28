use std::sync::Arc;

use super::abstract_driver::Driver;
use crate::utils::logging_utils;
use crate::utils::mavlink_utils::{
    create_groundstation_mavlink, create_mavlink, mavlink_receive_blcoking, mavlink_send,
};
use crate::utils::types::{MavDevice, MavFramePacket, NodeType};

pub struct UDPDriver {
    pub driver_instance: Arc<MavDevice>,
}

impl Driver<MavFramePacket> for UDPDriver {
    async fn send(&self, packet_to_send: MavFramePacket) {
        let mavlink = self.driver_instance.clone();
        mavlink_send(&mavlink, &packet_to_send)
    }

    async fn receive(&self) -> Option<MavFramePacket> {
        let mavlink = self.driver_instance.clone();
        let mavlink_frame: MavFramePacket = mavlink_receive_blcoking(&mavlink);
        Some(mavlink_frame)
    }

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

        logging_utils::log_driver_creation("UDPDriver");

        Self {
            driver_instance: Arc::new(mavlink),
        }
    }
}
