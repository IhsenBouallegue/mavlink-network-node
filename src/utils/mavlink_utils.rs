use std::io::Cursor;
use std::sync::atomic::{AtomicUsize, Ordering};

use mavlink::ardupilotmega::MavMessage;
use mavlink::{read_versioned_msg, MAVLinkV2MessageRaw, MavHeader};
use tracing::error;

use super::types::{MavFramePacket, NodeType};

pub fn deserialize_frame(buffer: &[u8]) -> Option<MavFramePacket> {
    let buffer_reader = &mut Cursor::new(buffer);
    let buffer_reader2 = &mut Cursor::new(buffer);
    match read_versioned_msg(buffer_reader, mavlink::MavlinkVersion::V2) {
        Ok(packet) => Some(MavFramePacket {
            header: packet.0,
            msg: packet.1,
            protocol_version: mavlink::MavlinkVersion::V2,
        }),
        Err(_) => match read_versioned_msg(buffer_reader2, mavlink::MavlinkVersion::V1) {
            Ok(packet) => Some(MavFramePacket {
                header: packet.0,
                msg: packet.1,
                protocol_version: mavlink::MavlinkVersion::V2,
            }),
            Err(_) => {
                error!("Failed to deserialize mavlink frame: {:?}", buffer);
                None
            }
        },
    }
}

pub fn serialize_frame(packet: MavFramePacket) -> Vec<u8> {
    let mut message_raw = MAVLinkV2MessageRaw::new();
    message_raw.serialize_message(packet.header, &packet.msg);
    message_raw.raw_bytes().to_vec()
}

/// Create a heartbeat message using 'ardupilotmega' dialect
pub fn heartbeat_message() -> MavMessage {
    MavMessage::HEARTBEAT(mavlink::ardupilotmega::HEARTBEAT_DATA {
        custom_mode: 0,
        mavtype: mavlink::ardupilotmega::MavType::MAV_TYPE_GCS,
        autopilot: mavlink::ardupilotmega::MavAutopilot::MAV_AUTOPILOT_INVALID,
        base_mode: mavlink::ardupilotmega::MavModeFlag::empty(),
        system_status: mavlink::ardupilotmega::MavState::MAV_STATE_UNINIT,
        mavlink_version: 0x3,
    })
}

pub struct MavlinkHeaderGenerator {
    sequence: AtomicUsize,
}

impl MavlinkHeaderGenerator {
    pub fn new() -> MavlinkHeaderGenerator {
        MavlinkHeaderGenerator {
            sequence: AtomicUsize::new(0),
        }
    }

    fn create_mavlink_header(&self) -> MavHeader {
        let node_type = NodeType::from_str(&std::env::var("NODE_TYPE").unwrap()).unwrap();
        let system_id = match node_type {
            NodeType::Drone => 201,
            NodeType::Gateway => 101,
        };

        let sequence = self.sequence.fetch_add(1, Ordering::SeqCst);

        MavHeader {
            sequence: sequence as u8,
            system_id: system_id,
            component_id: 1,
        }
    }

    pub fn create_mavlink_heartbeat_frame(&self) -> MavFramePacket {
        MavFramePacket {
            header: self.create_mavlink_header(),
            msg: heartbeat_message(),
            protocol_version: mavlink::MavlinkVersion::V2,
        }
    }
}
