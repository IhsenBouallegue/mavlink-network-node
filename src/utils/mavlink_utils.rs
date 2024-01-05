use std::sync::atomic::{AtomicUsize, Ordering};

use mavlink::ardupilotmega::MavMessage;
use mavlink::MavHeader;

use super::logging_utils::{log_packet_receive_error, log_packet_transmit_error};
use super::types::{MavDevice, MavFramePacket, NodeType};
use crate::driver::udp_driver::UDP_DRIVER;

const GROUNDSATION_IP: &str = "192.168.1.150";
const QGROUNDCONTROL_PORT: &str = "14550";

const DRONE_IP: &str = "192.168.0.4";
const DRONE_PORT: &str = "14540";

/// Create mavlink connection from flight computer to compagnion computer
pub fn create_mavlink() -> MavDevice {
    let mavconn = mavlink::connect::<MavMessage>(format!("udpout:{}:{}", DRONE_IP, DRONE_PORT).as_str()).unwrap();
    mavconn
}

/// Create mavlink connection from groundstation
pub fn create_groundstation_mavlink() -> MavDevice {
    let mut mavconn =
        mavlink::connect::<MavMessage>(format!("udpout:{}:{}", GROUNDSATION_IP, QGROUNDCONTROL_PORT).as_str()).unwrap();
    mavconn.set_protocol_version(mavlink::MavlinkVersion::V2);
    mavconn
}

/// Create a heartbeat message using 'ardupilotmega' dialect
pub fn heartbeat_message() -> MavMessage {
    MavMessage::HEARTBEAT(mavlink::ardupilotmega::HEARTBEAT_DATA {
        custom_mode: 0,
        mavtype: mavlink::ardupilotmega::MavType::MAV_TYPE_QUADROTOR,
        autopilot: mavlink::ardupilotmega::MavAutopilot::MAV_AUTOPILOT_ARDUPILOTMEGA,
        base_mode: mavlink::ardupilotmega::MavModeFlag::empty(),
        system_status: mavlink::ardupilotmega::MavState::MAV_STATE_STANDBY,
        mavlink_version: 0x3,
    })
}

// /// Create a message requesting the parameters list
// pub fn request_parameters() -> MavMessage {
//     MavMessage::PARAM_REQUEST_LIST(mavlink::ardupilotmega::PARAM_REQUEST_LIST_DATA {
//         target_system: 1,
//         target_component: 1,
//     })
// }

// /// Create a message enabling data streaming
// pub fn request_stream() -> MavMessage {
//     mavlink::ardupilotmega::MavMessage::REQUEST_DATA_STREAM(mavlink::ardupilotmega::REQUEST_DATA_STREAM_DATA {
//         target_system: 0,
//         target_component: 0,
//         req_stream_id: 0,
//         req_message_rate: 10,
//         start_stop: 1,
//     })
// }

pub fn deserialize_frame(buffer: &[u8]) -> Option<MavFramePacket> {
    let mavlink_frame_result = MavFramePacket::deser(mavlink::MavlinkVersion::V2, buffer);
    match mavlink_frame_result {
        Ok(mavlink_frame) => Some(mavlink_frame),
        Err(_) => {
            println!("Failed to deserialize mavlink frame: {:?}", buffer);
            None
        }
    }
}

pub fn mavlink_receive_blcoking(mavlink_device: &MavDevice) -> Option<MavFramePacket> {
    let mavlink_frame = mavlink_device.recv_frame();
    match mavlink_frame {
        Ok(mavlink_frame) => Some(mavlink_frame),
        Err(err) => {
            log_packet_receive_error(UDP_DRIVER, &err.to_string());
            None
        }
    }
}

pub fn mavlink_send(mavlink_device: &MavDevice, mavlink_frame: &MavFramePacket) {
    match mavlink_device.send_frame(&mavlink_frame) {
        Ok(_size) => {}
        Err(err) => log_packet_transmit_error(UDP_DRIVER, mavlink_frame, &err.to_string()),
    }
}

static SEQUENCE: AtomicUsize = AtomicUsize::new(0);

pub fn create_mavlink_header() -> MavHeader {
    let node_type = NodeType::from_str(std::env::var("NODE_TYPE").unwrap().as_str()).unwrap();
    let system_id = match node_type {
        NodeType::Drone => 201,
        NodeType::Gateway => 101,
    };

    let sequence = SEQUENCE.fetch_add(1, Ordering::SeqCst);

    mavlink::MavHeader {
        sequence: sequence as u8,
        system_id: system_id,
        component_id: 0,
    }
}

pub fn create_mavlink_heartbeat_frame() -> MavFramePacket {
    MavFramePacket {
        header: create_mavlink_header(),
        msg: heartbeat_message(),
        protocol_version: mavlink::MavlinkVersion::V2,
    }
}
