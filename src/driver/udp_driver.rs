use super::abstract_driver::Driver;
use crate::utils::mavlink_utils::{
    create_groundstation_mavlink, create_incoming_groundstation_mavlink, create_mavlink,
    mavlink_receive_blcoking, mavlink_send,
};
use crate::utils::types::{MavDevice, MavFramePacket};
use std::sync::{Arc, Mutex, RwLock};

pub struct UDPDriver {
    pub driver_instance: Arc<RwLock<MavDevice>>,
}

impl Driver<MavFramePacket> for UDPDriver {
    fn send(&self, get_packet_to_send: Arc<Mutex<impl Fn() -> Option<MavFramePacket>>>) {
        let get_packet_to_send = get_packet_to_send.lock().unwrap();
        let mavlink = self.driver_instance.clone();
        let mavlink = mavlink.write().unwrap();
        if let Some(data) = get_packet_to_send() {
            mavlink_send(&mavlink, &data)
        }
    }

    fn receive(&self, on_receive: Arc<Mutex<impl Fn(MavFramePacket)>>) {
        let mavlink = self.driver_instance.clone();
        let mavlink = mavlink.write().unwrap();
        let mavlink_frame: MavFramePacket = mavlink_receive_blcoking(&mavlink);
        let on_receive = on_receive.lock().unwrap();
        on_receive(mavlink_frame);
    }

    fn create_instance() -> Self {
        // let mavlink = create_groundstation_mavlink();
        // let mavlink = create_mavlink();
        let mavlink = create_incoming_groundstation_mavlink();
        Self {
            driver_instance: Arc::new(RwLock::new(mavlink)),
        }
    }
}
