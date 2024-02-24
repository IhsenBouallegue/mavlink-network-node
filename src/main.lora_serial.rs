mod driver;
mod network;
mod utils;

use std::env;
use std::time::Duration;

use tokio::time::sleep;
use utils::logging_utils::init_logging;
use utils::lora_serial::Sx1262UartE22;
use utils::types::NodeType;

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let node_type = NodeType::from_str(&args[1]).unwrap();
    std::env::set_var("NODE_TYPE", &args[1]);
    let _guard = init_logging();

    match node_type {
        NodeType::Drone => {
            let mut sx126x = Sx1262UartE22::new("/dev/ttyS0").unwrap();
            loop {
                println!("Sending message");
                sx126x.send(0, 868, &"Hello World".as_bytes().to_vec()).unwrap();
                sleep(Duration::from_secs(1)).await;
            }
        }
        NodeType::Gateway => {
            let mut sx126x = Sx1262UartE22::new("/dev/ttyS0").unwrap();
            loop {
                sx126x.receive().unwrap();
            }
        }
    }
}
