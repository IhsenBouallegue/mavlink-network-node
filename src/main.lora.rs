mod driver;
mod network;
mod utils;

use std::env;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::time::sleep;
use tracing::info;
use utils::logging_utils::init_logging;
use utils::lora_utils::{create_lora, create_spi, lora_receive, lora_transmit};
use utils::mavlink_utils::MavlinkHeaderGenerator;
use utils::types::NodeType;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let node_type = NodeType::from_str(&args[1]).unwrap();
    std::env::set_var("NODE_TYPE", &args[1]);
    let _guard = init_logging();

    match node_type {
        NodeType::Drone => {
            let spi = create_spi().unwrap();
            let lora = &mut create_lora(spi).await.expect("Failed to create LoRa instance");

            let (tx, mut rx) = mpsc::channel(32);

            tokio::spawn(async move {
                let generator = MavlinkHeaderGenerator::new();
                loop {
                    tx.send(generator.create_mavlink_heartbeat_frame()).await.unwrap();
                    sleep(Duration::from_secs(3)).await;
                }
            });

            loop {
                tokio::select! {
                    Some(packet) = rx.recv() => {
                        lora_transmit(lora, &packet).await;
                    }
                    Some(val) = lora_receive(lora) => {
                        info!("Receiving over LoRa! mavlink size: {:?} with rssi: {}", val.buffer.len(), val.rssi);
                    }
                }
            }
        }
        NodeType::Gateway => {
            let spi = create_spi().unwrap();
            let lora = &mut create_lora(spi).await.expect("Failed to create LoRa instance");

            let (tx, mut rx) = mpsc::channel(32);

            tokio::spawn(async move {
                let generator = MavlinkHeaderGenerator::new();

                loop {
                    tx.send(generator.create_mavlink_heartbeat_frame()).await.unwrap();
                    sleep(Duration::from_secs(3)).await;
                }
            });

            loop {
                tokio::select! {
                    Some(packet) = rx.recv() => {
                        lora_transmit(lora, &packet).await;
                    }
                    Some(val) = lora_receive(lora) => {
                        info!("Receiving over LoRa! mavlink size: {:?} with rssi: {}", val.buffer.len(), val.rssi);
                    }
                }
            }
        }
    }
}
