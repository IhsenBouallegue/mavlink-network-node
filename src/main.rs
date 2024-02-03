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
use utils::udp_comm::UdpComm;

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
            // Set up udp network with channels
            let addr = "0.0.0.0:0"; // Bind to all interfaces for receiving
            let discovery_addr = "192.168.0.255:14540";
            let (udp_comm, transmit_udp_tx, mut received_udp_rx) = UdpComm::new(100);
            udp_comm.run(addr, discovery_addr, true).await;
            let lora_transmit_udp_tx = transmit_udp_tx.clone();

            let udp_to_lora = tokio::task::Builder::new()
                .name("udp to lora")
                .spawn(async move {
                    let generator = MavlinkHeaderGenerator::new();
                    loop {
                        transmit_udp_tx
                            .send(generator.create_mavlink_heartbeat_frame())
                            .await
                            .unwrap();
                        sleep(std::time::Duration::from_millis(1000)).await;
                    }
                })
                .unwrap();

            // Set up lora network with channels
            let lora_task = tokio::task::Builder::new().name("lora").spawn(async move {
                let spi = create_spi().unwrap();
                let mut lora = create_lora(spi).await.expect("Failed to create LoRa instance");
                let mdltn_params = create_modulation_params(&mut lora).unwrap();
                let rx_pkt_params = create_rx_packet_params(&mut lora, &mdltn_params).unwrap();
                let mut tx_pkt_params = create_tx_packet_params(&mut lora, &mdltn_params);

                loop {
                    lora.prepare_for_rx(lora_phy::RxMode::Continuous, &mdltn_params, &rx_pkt_params, true).await.unwrap();
                    tokio::select! {
                        Some(packet) = received_udp_rx.recv() => {
                            prepare_for_tx(&mut lora, &mdltn_params).await;
                            lora_trans(&mut lora, &packet, &mdltn_params, &mut tx_pkt_params).await;
                            while let Ok(Some(packet)) = tokio::time::timeout(Duration::from_millis(1), received_udp_rx.recv()).await {
                                lora_trans(&mut lora, &packet, &mdltn_params, &mut tx_pkt_params).await;
                            }
                        }
                        Ok(_) = lora.wait_for_irq() => {
                            if let Ok(Some(TargetIrqState::Done)) = lora.process_irq_event(TargetIrqState::Done).await {
                                let mut receiving_buffer = [00u8; 255];

                                let (received_len, rx_pkt_status) = lora.rx(&rx_pkt_params, &mut receiving_buffer).instrument(tracing::span!(tracing::Level::DEBUG, "Receiving", driver = LORA_DRIVER, target= "network")).await.unwrap();
                                let received_data = Vec::from(&receiving_buffer[..received_len as usize]);

                                if let Some(mavlink_frame) = deserialize_frame(&received_data[..]) {
                                    log_debug_receive_packet(LORA_DRIVER, &mavlink_frame, Some(rx_pkt_status.rssi));
                                    lora_transmit_udp_tx.send(mavlink_frame).await.unwrap();
                                }
                            }
                        }
                    }
                }
            }).unwrap();

            let _ = tokio::try_join!(lora_task, udp_to_lora);
        }
        NodeType::Gateway => {
            // Set up udp network with channels
            let addr = "0.0.0.0:0"; // Bind to all interfaces for receiving
            let discovery_addr = "192.168.1.255:14550";
            let (udp_comm, transmit_udp_tx, mut received_udp_rx) = UdpComm::new(100);
            udp_comm.run(addr, discovery_addr, true).await;
            let lora_transmit_udp_tx = transmit_udp_tx.clone();

            let udp_to_lora = tokio::task::Builder::new()
                .name("udp to lora")
                .spawn(async move {
                    let generator = MavlinkHeaderGenerator::new();
                    loop {
                        transmit_udp_tx
                            .send(generator.create_mavlink_heartbeat_frame())
                            .await
                            .unwrap();
                        sleep(std::time::Duration::from_millis(1000)).await;
                    }
                })
                .unwrap();

            // Set up lora network with channels
            let lora_task = tokio::task::Builder::new()
                .name("lora")
                .spawn(async move {
                    let spi = create_spi().unwrap();
                    let mut lora = create_lora(spi).await.expect("Failed to create LoRa instance");
                    let mdltn_params = create_modulation_params(&mut lora).unwrap();
                    let rx_pkt_params = create_rx_packet_params(&mut lora, &mdltn_params).unwrap();
                    let mut tx_pkt_params = create_tx_packet_params(&mut lora, &mdltn_params);

                    loop {
                        lora.prepare_for_rx(lora_phy::RxMode::Continuous, &mdltn_params, &rx_pkt_params, true).await.unwrap();
                        tokio::select! {
                            Some(packet) = received_udp_rx.recv() => {
                                lora_trans(&mut lora, &packet, &mdltn_params, &mut tx_pkt_params).await;
                            }
                            Ok(_) = lora.wait_for_irq() => {
                                if let Ok(Some(TargetIrqState::Done)) = lora.process_irq_event(TargetIrqState::Done).await {
                                    let mut receiving_buffer = [00u8; 255];
                                    let (received_len, rx_pkt_status) = lora.rx(&rx_pkt_params, &mut receiving_buffer).instrument(tracing::span!(tracing::Level::DEBUG, "Receiving", driver = LORA_DRIVER, target= "network")).await.unwrap();
                                    let received_data = Vec::from(&receiving_buffer[..received_len as usize]);
                                    if let Some(mavlink_frame) = deserialize_frame(&received_data[..]) {
                                        log_debug_receive_packet(LORA_DRIVER, &mavlink_frame, Some(rx_pkt_status.rssi));
                                        lora_transmit_udp_tx.send(mavlink_frame).await.unwrap();
                                    }
                                }
                            }
                        }
                    }
                })
                .unwrap();

            let _ = tokio::try_join!(lora_task, udp_to_lora);
        }
    }
}

async fn forward_packets(
    mut received_rx: mpsc::Receiver<MavFramePacket>,
    transmit_tx: mpsc::Sender<MavFramePacket>,
    log_driver: &str,
) {
    loop {
        let received = received_rx.recv().await;
        if let Some(received) = received {
            log_debug_send_to_network(log_driver);
            transmit_tx.send(received).await.unwrap();
        }
    }
}

async fn send_heartbeat_to_network(transmit_tx: mpsc::Sender<MavFramePacket>, driver: &str, interval_ms: u64) {
    let generator = MavlinkHeaderGenerator::new();

    loop {
        log_debug_send_to_network(driver);
        transmit_tx
            .send(generator.create_mavlink_heartbeat_frame())
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(interval_ms)).await;
    }
}
