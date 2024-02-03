mod driver;
mod network;
mod utils;

use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{env, thread};

use driver::lora_driver::{LoRaDriver, LORA_DRIVER};
use driver::udp_driver::{UDPDriver, UDP_DRIVER};
use futures::executor::block_on;
use lora_phy::mod_traits::TargetIrqState;
use lora_phy::RxMode;
use mavlink::ardupilotmega::{MavCmd, MavMessage, COMMAND_LONG_DATA};
use mavlink::{MavFrame, MavHeader, Message};
use network::network_interface::{HalfDuplexNetworkInterface, NetworkInterface};
use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tracing::{info, Instrument};
use utils::logging_utils::{init_logging, log_debug_receive_packet, log_debug_send_packet, log_debug_send_to_network};
use utils::lora_utils::{create_lora, create_spi};
use utils::mavlink_utils::MavlinkHeaderGenerator;
use utils::types::{MavFramePacket, NodeType};

use crate::utils::lora_utils::{
    create_modulation_params, create_rx_packet_params, create_tx_packet_params, lora_receive, lora_recv, lora_trans,
    lora_transmit, prepare_for_rx, prepare_for_tx,
};
use crate::utils::mavlink_utils::deserialize_frame;

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let node_type = NodeType::from_str(&args[1]).unwrap();
    std::env::set_var("NODE_TYPE", &args[1]);
    let _guard = init_logging();

    match node_type {
        NodeType::Drone => {
            let (transmit_lora_tx, mut transmit_lora_rx) = mpsc::channel::<MavFramePacket>(128);
            let (received_lora_tx, mut received_lora_rx) = mpsc::channel(128);

            let lora_task = tokio::task::Builder::new()
                .name("lora")
                .spawn(async move {
                    let spi = create_spi().unwrap();
                    let mut lora = create_lora(spi).await.expect("Failed to create LoRa instance");
                    let mdltn_params = create_modulation_params(&mut lora).unwrap();
                    let rx_pkt_params = create_rx_packet_params(&mut lora, &mdltn_params).unwrap();

                    loop {
                        lora.prepare_for_rx(lora_phy::RxMode::Continuous, &mdltn_params, &rx_pkt_params, true).await.unwrap();

                        tokio::select! {
                            Some(packet) = transmit_lora_rx.recv() => {
                                lora_trans(&mut lora, &packet).await;
                            }
                            Ok(_) = lora.wait_for_irq() => {
                                if let Ok(Some(TargetIrqState::Done)) = lora.process_irq_event(TargetIrqState::Done).await {
                                    let mut receiving_buffer = [00u8; 255];
                                    let (received_len, rx_pkt_status) = lora.rx(&rx_pkt_params, &mut receiving_buffer).instrument(tracing::span!(tracing::Level::DEBUG, "Receiving", driver = LORA_DRIVER, target= "network")).await.unwrap();
                                    let received_data = Vec::from(&receiving_buffer[..received_len as usize]);

                                    if let Some(mavlink_frame) = deserialize_frame(&received_data[..]) {
                                        log_debug_receive_packet(LORA_DRIVER, &mavlink_frame, Some(rx_pkt_status.rssi));
                                        received_lora_tx.send(mavlink_frame).await.unwrap();
                                    }
                                }
                            }
                        }
                        // // put in receive mode
                        // prepare_for_rx(&mut lora, &mdltn_params, &rx_pkt_params).await;
                        // if let Ok(Some(TargetIrqState::Done)) = lora.peek_irq(TargetIrqState::Done).await {
                        //     let received_recv_result = lora_recv(&mut lora).await.unwrap();
                        //     if let Some(mavlink_frame) = deserialize_frame(&received_recv_result.buffer[..]) {
                        //         log_debug_receive_packet(LORA_DRIVER, &mavlink_frame, Some(received_recv_result.rssi));
                        //         received_lora_tx.send(mavlink_frame).await.unwrap();
                        //     }
                        // }
                    }
                })
                .unwrap();

            let udp_to_lora = tokio::task::Builder::new()
                .name("udp to lora")
                .spawn(async move {
                    let generator = MavlinkHeaderGenerator::new();
                    loop {
                        log_debug_send_to_network(LORA_DRIVER);
                        transmit_lora_tx
                            .send(generator.create_mavlink_heartbeat_frame())
                            .await
                            .unwrap();
                        sleep(Duration::from_millis(500)).await;
                    }
                })
                .unwrap();

            let _ = tokio::try_join!(lora_task, udp_to_lora);
            // udp_thread_handle.await.unwrap();
        }
        NodeType::Gateway => {
            let (transmit_lora_tx, mut transmit_lora_rx) = mpsc::channel::<MavFramePacket>(128);
            let (received_lora_tx, mut received_lora_rx) = mpsc::channel::<MavFramePacket>(128);

            let lora_task = tokio::task::Builder::new()
                .name("lora")
                .spawn(async move {
                    let spi = create_spi().unwrap();
                    let mut lora = create_lora(spi).await.expect("Failed to create LoRa instance");
                    let mdltn_params = create_modulation_params(&mut lora).unwrap();
                    let rx_pkt_params = create_rx_packet_params(&mut lora, &mdltn_params).unwrap();

                    loop {
                        lora.prepare_for_rx(lora_phy::RxMode::Continuous, &mdltn_params, &rx_pkt_params, true).await.unwrap();

                        tokio::select! {
                            Some(packet) = transmit_lora_rx.recv() => {
                                lora_trans(&mut lora, &packet).await;
                            }
                            Ok(_) = lora.wait_for_irq() => {
                                if let Ok(Some(TargetIrqState::Done)) = lora.process_irq_event(TargetIrqState::Done).await {
                                    let mut receiving_buffer = [00u8; 255];

                                    let (received_len, rx_pkt_status) = lora.rx(&rx_pkt_params, &mut receiving_buffer).instrument(tracing::span!(tracing::Level::DEBUG, "Receiving", driver = LORA_DRIVER, target= "network")).await.unwrap();
                                    let received_data = Vec::from(&receiving_buffer[..received_len as usize]);

                                    if let Some(mavlink_frame) = deserialize_frame(&received_data[..]) {
                                        log_debug_receive_packet(LORA_DRIVER, &mavlink_frame, Some(rx_pkt_status.rssi));
                                        received_lora_tx.send(mavlink_frame).await.unwrap();
                                    }
                                }
                            }
                        }
                        // Wait for receive from transmit channel
                        // if let Some(packet) = transmit_lora_rx.recv().await {
                        //     lora_trans(&mut lora, &packet).await;
                        // }
                    }
                })
                .unwrap();

            let mavconn = utils::mavlink_utils::create_groundstation_mavlink();
            let mavconn = Arc::new(mavconn);
            // let mavconn_clone = mavconn.clone();
            // Forward packets from lora to udp
            let lora_to_udp = tokio::task::Builder::new()
                .name("lora to udp")
                .spawn(async move {
                    loop {
                        if let Some(received) = received_lora_rx
                            .recv()
                            .instrument(tracing::span!(tracing::Level::INFO, "lora_to_udp"))
                            .await
                        {
                            log_debug_send_packet(UDP_DRIVER, &received);
                            // mavconn_clone.send_frame(&received).unwrap();
                        }
                    }
                    // forward_packets(received_lora_rx, transmit_udp_tx, UDP_DRIVER).await;
                })
                .unwrap();

            let mavconn_clone2 = mavconn.clone();
            // Forward packets from udp to lora
            let udp_to_lora = tokio::task::Builder::new()
                .name("udp to lora")
                .spawn(async move {
                    let generator = MavlinkHeaderGenerator::new();
                    mavconn_clone2
                        .send_frame(&generator.create_mavlink_heartbeat_frame())
                        .unwrap();

                    // forward_packets(received_udp_rx, transmit_lora_tx, LORA_DRIVER).await;
                    loop {
                        let mavframe = mavconn_clone2.recv_frame().unwrap();
                        log_debug_receive_packet(UDP_DRIVER, &mavframe, None);
                        // mavconn_clone2
                        //     .send_frame(&generator.create_mavlink_heartbeat_frame())
                        //     .unwrap();
                        log_debug_send_to_network(LORA_DRIVER);
                        transmit_lora_tx.send(mavframe).await.unwrap();
                    }
                })
                .unwrap();

            let _ = tokio::try_join!(lora_task, lora_to_udp, udp_to_lora);
            // udp_thread_handle.await.unwrap();
        }
    }
}
