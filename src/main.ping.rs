mod driver;
mod network;
mod utils;

use std::{env, thread};

use driver::lora_driver::LoRaDriver;
use network::network_interface::{HalfDuplexNetworkInterface, NetworkInterface};
use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use utils::logging_utils::init_logging;
use utils::mavlink_utils::create_mavlink_heartbeat_frame;
use utils::types::{MavFramePacket, NodeType};

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let node_type = NodeType::from_str(&args[1]).unwrap();
    std::env::set_var("NODE_TYPE", &args[1]);
    let _guard = init_logging();

    match node_type {
        NodeType::Drone => {
            let (to_send_tx, to_send_rx) = mpsc::channel(32);
            let (received_tx, mut received_rx) = mpsc::channel(32);
            let to_send_clone = to_send_tx.clone();

            let handle = thread::spawn(move || {
                let runtime = Runtime::new().expect("Failed to create a runtime");
                let mut lora_network =
                    HalfDuplexNetworkInterface::<LoRaDriver, MavFramePacket>::new(to_send_rx, received_tx);
                runtime.block_on(async {
                    lora_network.run().await;
                });
            });
            tokio::spawn(async move {
                loop {
                    to_send_clone.send(create_mavlink_heartbeat_frame()).await.unwrap();
                    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                }
            });

            handle.join().unwrap();
        }
        NodeType::Gateway => {
            // handle.await.unwrap();
            let (to_send_tx, to_send_rx) = mpsc::channel(32);
            let (received_tx, mut received_rx) = mpsc::channel(32);
            let to_send_clone = to_send_tx.clone();

            let handle = thread::spawn(move || {
                let runtime = Runtime::new().expect("Failed to create a runtime");
                let mut lora_network =
                    HalfDuplexNetworkInterface::<LoRaDriver, MavFramePacket>::new(to_send_rx, received_tx);
                runtime.block_on(async {
                    lora_network.run().await;
                });
            });
            // tokio::spawn(async move {
            //     loop {
            //         to_send_clone.send(create_mavlink_heartbeat_frame()).await.unwrap();
            //         tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
            //     }
            // });
            // let handler = tokio::spawn(async move {
            //     loop {
            //         let received = received_rx.recv().await;
            //         if let Some(received) = received {
            //             println!("Received: {:?}", received);
            //         }
            //     }
            // });

            // handler.await.unwrap();
            handle.join().unwrap();
        }
    }
}
