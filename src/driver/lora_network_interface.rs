use std::time::Duration;

use lora_phy::mod_traits::TargetIrqState;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tracing::Instrument;

use crate::driver::lora_driver::LORA_DRIVER;
use crate::utils::logging_utils::log_debug_receive_packet;
use crate::utils::lora_utils::{
    create_lora, create_modulation_params, create_rx_packet_params, create_spi, create_tx_packet_params, lora_trans,
    prepare_for_tx,
};
use crate::utils::mavlink_utils::deserialize_frame;
use crate::utils::types::MavFramePacket;

pub trait NetworkInterface {
    async fn send(&mut self, packet: MavFramePacket);
    async fn receive(&mut self) -> Option<MavFramePacket>;
    fn new(to_send: Receiver<MavFramePacket>, received: Sender<MavFramePacket>) -> Self;
    async fn run(&mut self);
}

pub struct LoRaNetworkInterface {
    send_channel: Sender<MavFramePacket>,
    recv_channel: Receiver<MavFramePacket>,
}

impl LoRaNetworkInterface {
    pub fn new(buffer_size: usize) -> (Self, Sender<MavFramePacket>, Receiver<MavFramePacket>) {
        let (tx_send, rx_send) = mpsc::channel(buffer_size);
        let (tx_recv, rx_recv) = mpsc::channel(buffer_size);

        (
            LoRaNetworkInterface {
                send_channel: tx_send,
                recv_channel: rx_recv,
            },
            tx_recv,
            rx_send,
        )
    }

    pub fn new_barebone(tx_send: Sender<MavFramePacket>, rx_recv: Receiver<MavFramePacket>) -> Self {
        LoRaNetworkInterface {
            send_channel: tx_send,
            recv_channel: rx_recv,
        }
    }

    pub async fn run(mut self) -> tokio::task::JoinHandle<()> {
        let lora_task = tokio::task::Builder::new()
        .name("lora")
        .spawn(async move {
            let spi = create_spi().unwrap();
            let mut lora = create_lora(spi).await.expect("Failed to create LoRa instance");
            let mdltn_params = create_modulation_params(&mut lora).unwrap();
            let rx_pkt_params = create_rx_packet_params(&mut lora, &mdltn_params).unwrap();
            let mut tx_pkt_params = create_tx_packet_params(&mut lora, &mdltn_params);
            const CONTINOUS_TRANSMISSION_PACKET_LIMIT: u8 = 3;

            loop {
                lora.prepare_for_rx(lora_phy::RxMode::Continuous, &mdltn_params, &rx_pkt_params, true).await.unwrap();
                tokio::select! {
                    // Transmit packets received through channel
                    Some(packet) = self.recv_channel.recv() => {
                        prepare_for_tx(&mut lora, &mdltn_params).await;
                        lora_trans(&mut lora, &packet, &mdltn_params, &mut tx_pkt_params).await;
                        let mut continous_transmission_packet_count: u8 = 0;
                        while let Ok(Some(packet)) = tokio::time::timeout(Duration::from_millis(2), self.recv_channel.recv()).await
                        {
                            if continous_transmission_packet_count >= CONTINOUS_TRANSMISSION_PACKET_LIMIT {
                                break;
                            }
                            lora_trans(&mut lora, &packet, &mdltn_params, &mut tx_pkt_params).await;
                            continous_transmission_packet_count += 1;
                        }
                    }
                    // Receive packets from LoRa
                    Ok(_) = lora.wait_for_irq() => {
                        if let Ok(Some(TargetIrqState::Done)) = lora.process_irq_event(TargetIrqState::Done).await {
                            let mut receiving_buffer = [00u8; 255];
                            let (received_len, rx_pkt_status) = lora.rx(&rx_pkt_params, &mut receiving_buffer).instrument(tracing::span!(tracing::Level::DEBUG, "Receiving", driver = LORA_DRIVER, target= "network")).await.unwrap();
                            let received_data = Vec::from(&receiving_buffer[..received_len as usize]);
                            if let Some(mavlink_frame) = deserialize_frame(&received_data[..]) {
                                log_debug_receive_packet(LORA_DRIVER, &mavlink_frame, Some(rx_pkt_status.rssi));
                                self.send_channel.send(mavlink_frame).await.unwrap();
                            }
                        }
                    }
                }
            }
        })
        .unwrap();
        lora_task
    }
}