use mavlink::{ardupilotmega::MavMessage, MavFrame};
use std::sync::Arc;

const GROUNDSATION_IP: &str = "192.168.1.150";
const QGROUNDCONTROL_PORT: &str = "14550";

const DRONE_IP: &str = "192.168.0.4";
const DRONE_PORT: &str = "14540";

/// Create mavlink connection from flight computer to compagnion computer
pub fn create_mavlink() -> Arc<Box<dyn mavlink::MavConnection<MavMessage> + Send + Sync>> {
    let mavconn =
        mavlink::connect::<MavMessage>(format!("udpout:{}:{}", DRONE_IP, DRONE_PORT).as_str())
            .unwrap();

    let vehicle = Arc::new(mavconn);
    vehicle
        .send_frame(&MavFrame {
            header: mavlink::MavHeader::default(),
            msg: request_parameters(),
            protocol_version: mavlink::MavlinkVersion::V2,
        })
        .unwrap();
    vehicle
        .send_frame(&MavFrame {
            header: mavlink::MavHeader::default(),
            msg: request_stream(),
            protocol_version: mavlink::MavlinkVersion::V2,
        })
        .unwrap();
    vehicle
}

/// Create mavlink connection from gateway to groundstation
pub fn create_groundstation_mavlink(
) -> Arc<Box<dyn mavlink::MavConnection<MavMessage> + Send + Sync>> {
    let mavconn = mavlink::connect::<MavMessage>(
        format!("udpout:{}:{}", GROUNDSATION_IP, QGROUNDCONTROL_PORT).as_str(),
    )
    .unwrap();
    let groundstation = Arc::new(mavconn);

    groundstation
}

/// Create mavlink connection from groundstation to gateway
pub fn create_incoming_groundstation_mavlink(
) -> Arc<Box<dyn mavlink::MavConnection<MavMessage> + Send + Sync>> {
    let mut mavconn =
        mavlink::connect::<MavMessage>(format!("udpin:{}:{}", "0.0.0.0", "14530").as_str())
            .unwrap();
    mavconn.set_protocol_version(mavlink::MavlinkVersion::V1);

    // let msg = mavconn.recv().unwrap();
    // println!("{:#?}", msg);
    let groundstation = Arc::new(mavconn);
    groundstation
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

/// Create a message requesting the parameters list
pub fn request_parameters() -> MavMessage {
    MavMessage::PARAM_REQUEST_LIST(mavlink::ardupilotmega::PARAM_REQUEST_LIST_DATA {
        target_system: 0,
        target_component: 0,
    })
}

/// Create a message enabling data streaming
pub fn request_stream() -> MavMessage {
    MavMessage::MESSAGE_INTERVAL(mavlink::ardupilotmega::MESSAGE_INTERVAL_DATA {
        message_id: 0,
        interval_us: 10000,
    })
}
