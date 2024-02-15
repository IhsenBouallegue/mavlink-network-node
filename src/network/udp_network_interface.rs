use std::io::Cursor;
use std::sync::Arc;

use mavlink::{read_versioned_msg, MAVLinkV2MessageRaw, Message as MavMessage};
use tokio::net::UdpSocket;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::task::JoinHandle;
use tracing::error;

use crate::driver::udp_driver::UDP_DRIVER;
use crate::utils::logging_utils::{log_debug_receive_packet, log_debug_send_packet, log_debug_send_to_main};
use crate::utils::types::MavFramePacket;

pub struct UDPNetworkInterface {
    send_channel: Sender<MavFramePacket>,
    recv_channel: Receiver<MavFramePacket>,
}

impl UDPNetworkInterface {
    pub fn new(buffer_size: usize) -> (Self, Sender<MavFramePacket>, Receiver<MavFramePacket>) {
        let (tx_send, rx_send) = mpsc::channel(buffer_size);
        let (tx_recv, rx_recv) = mpsc::channel(buffer_size);

        (
            UDPNetworkInterface {
                send_channel: tx_send,
                recv_channel: rx_recv,
            },
            tx_recv,
            rx_send,
        )
    }

    pub async fn run(self, addr: &str, dest_addr: &str, broadcast: bool) -> (JoinHandle<()>, JoinHandle<()>) {
        let dest_addr = dest_addr.to_string();
        let socket = Arc::new(UdpSocket::bind(addr).await.unwrap());
        socket.set_broadcast(broadcast).expect("Failed to enable broadcast");

        let socket_recv = Arc::clone(&socket);
        let socket_send = Arc::clone(&socket);

        // Receiving task
        let send_channel = self.send_channel;
        let recv_task = tokio::task::Builder::new()
            .name("udp recv")
            .spawn(async move {
                loop {
                    let mut buf = [0; 256];

                    match socket_recv.recv_from(&mut buf).await {
                        Ok((size, src_addr)) => {
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
                                    continue;
                                }

                                match send_channel.try_send(mavlink_frame) {
                                    Err(mpsc::error::TrySendError::Full(_)) => {
                                        error!("Send channel is full, dropping packet.");
                                    }
                                    Ok(_) => {
                                        log_debug_send_to_main(UDP_DRIVER);
                                    }
                                    _ => {}
                                }
                            }
                        }
                        Err(_) => continue, // Ignoring errors for proof of concept
                    }
                }
            })
            .unwrap();

        // Sending task
        let mut recv_channel = self.recv_channel;
        let send_task = tokio::task::Builder::new()
            .name("udp send")
            .spawn(async move {
                while let Some(packet) = recv_channel.recv().await {
                    let cloned_packet = packet.clone();
                    let raw_frame = serialize_frame(cloned_packet);
                    // log_packet_sent(raw_frame.len(), Some(&dest_addr), &packet, UDP_DRIVER);
                    log_debug_send_packet(UDP_DRIVER, &packet);
                    let _ = socket_send.send_to(&raw_frame, &dest_addr).await;
                }
            })
            .unwrap();

        (recv_task, send_task)
    }
}

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
