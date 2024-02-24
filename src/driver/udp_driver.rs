use std::fmt::Display;
use std::sync::Arc;

use mavlink::Message as MavMessage;
use tokio::net::UdpSocket;

use super::Driver;
use crate::mavlink_utils::{deserialize_frame, serialize_frame};
use crate::utils::logging_utils::{log_debug_receive_packet, log_debug_send_packet, log_driver_creation};
use crate::utils::types::MavFramePacket;

pub const UDP_DRIVER: &str = "udp_driver";

#[allow(dead_code)]
pub struct UDPConfig {
    pub addr: String,
    pub dest_addr: String,
    pub broadcast: bool,
}

pub struct UDPDriver {
    pub device: Arc<UdpSocket>,
    config: UDPConfig,
}

impl Display for UDPDriver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", UDP_DRIVER)
    }
}

#[allow(dead_code)]
impl UDPDriver {
    pub async fn new(config: UDPConfig) -> Self {
        let socket = Arc::new(UdpSocket::bind(&config.addr).await.unwrap());
        socket
            .set_broadcast(config.broadcast)
            .expect("Failed to enable broadcast");

        log_driver_creation(UDP_DRIVER);

        Self { device: socket, config }
    }
}

#[async_trait::async_trait]
impl Driver for UDPDriver {
    async fn send(&self, packet: &MavFramePacket) {
        let socket_send = Arc::clone(&self.device);
        let serialised_frame = serialize_frame(packet.clone());
        // log_packet_sent(raw_frame.len(), Some(&dest_addr), &packet, UDP_DRIVER);
        log_debug_send_packet(&self.to_string(), packet);
        let _ = socket_send.send_to(&serialised_frame, &self.config.dest_addr).await;
    }

    async fn receive(&self) -> Option<MavFramePacket> {
        let mut buf = [0; 256];
        let socket_recv = Arc::clone(&self.device);

        match socket_recv.recv_from(&mut buf).await {
            Ok((size, _src_addr)) => {
                let received_data = Vec::from(&buf[..size]);
                if let Some(mavlink_frame) = deserialize_frame(&received_data[..]) {
                    // log_packet_received(size, Some(src_addr), &mavlink_frame, UDP_DRIVER);
                    log_debug_receive_packet(UDP_DRIVER, &mavlink_frame, None);
                    if mavlink_frame.msg.message_id() == 30
                        || mavlink_frame.msg.message_id() == 141
                        || mavlink_frame.msg.message_id() == 74
                        || mavlink_frame.msg.message_id() == 410
                    {
                        // info!("Message ignored");
                        None
                    } else {
                        Some(mavlink_frame)
                    }
                } else {
                    // info!("Message corrupted");
                    None
                }
            }
            Err(_) => None, // Ignoring errors for proof of concept
        }
    }
}
