use super::abstract_driver::Driver;
use crate::utils::mavlink_utils::{
    create_groundstation_mavlink, create_mavlink, mavlink_receive_blcoking, mavlink_send,
};
use crate::utils::types::{MavDevice, MavFramePacket, NodeType};
use std::sync::{Arc, Mutex};

pub struct UDPDriver {
    pub driver_instance: Arc<MavDevice>,
}

impl Driver<MavFramePacket> for UDPDriver {
    fn send(&self, get_packet_to_send: Arc<Mutex<impl Fn() -> Option<MavFramePacket>>>) {
        let get_packet_to_send = get_packet_to_send.lock().unwrap();
        let mavlink = self.driver_instance.clone();
        if let Some(data) = get_packet_to_send() {
            mavlink_send(&mavlink, &data)
        }
    }

    async fn receive(&self, on_receive: Arc<Mutex<impl Fn(MavFramePacket)>>) {
        let mavlink = self.driver_instance.clone();
        let mavlink_frame: MavFramePacket = mavlink_receive_blcoking(&mavlink);
        let on_receive = on_receive.lock().unwrap();
        on_receive(mavlink_frame);
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

        Self {
            driver_instance: Arc::new(mavlink),
        }
    }
}
