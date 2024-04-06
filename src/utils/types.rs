use mavlink::ardupilotmega::MavMessage;
use mavlink::MavFrame;

pub type MavFramePacket = MavFrame<MavMessage>;

#[derive(Debug)]
pub enum NodeType {
    Uav,
    Gateway,
}

impl NodeType {
    pub fn from_str(s: &str) -> Result<NodeType, ()> {
        match s {
            "Uav" => Ok(NodeType::Uav),
            "Gateway" => Ok(NodeType::Gateway),
            _ => Err(println!("Invalid node type")),
        }
    }
}
