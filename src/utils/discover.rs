use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::net::UdpSocket;
use tokio::sync::mpsc;

const DISCOVER_RESPONSE: &str = "DISCOVER_RESPONSE";
const DISCOVER_REQUEST: &str = "DISCOVER_REQUEST";

pub struct DiscoveryService {
    discovered_nodes: Arc<Mutex<HashMap<String, Instant>>>, // Tracks discovered nodes and the last time they were seen
    discovery_notifier: mpsc::Sender<String>,               // Notifier for discovery events
}

impl DiscoveryService {
    pub fn new() -> (Self, mpsc::Receiver<String>) {
        let (discovery_notifier, discovery_receiver) = mpsc::channel(32); // Adjust buffer size as needed

        (
            DiscoveryService {
                discovered_nodes: Arc::new(Mutex::new(HashMap::new())),
                discovery_notifier,
            },
            discovery_receiver,
        )
    }

    pub async fn discover(&self) -> tokio::task::JoinHandle<()> {
        let socket = UdpSocket::bind("0.0.0.0:0").await.unwrap(); // Bind to an ephemeral port
        socket.set_broadcast(true).unwrap();
        let broadcast_address = "192.168.255.255:8080";
        let discovered_nodes = self.discovered_nodes.clone();
        let discover_notifier = self.discovery_notifier.clone();

        tokio::spawn(async move {
            loop {
                socket
                    .send_to(DISCOVER_REQUEST.as_bytes(), &broadcast_address)
                    .await
                    .unwrap();
                let mut buffer = [0; 1024];

                if let Ok((size, src)) = socket.recv_from(&mut buffer).await {
                    let message = std::str::from_utf8(&buffer[..size]).unwrap();
                    if message == DISCOVER_RESPONSE {
                        // If this is a known node, skip it
                        if discovered_nodes.lock().unwrap().contains_key(&src.to_string()) {
                            continue;
                        }
                        discovered_nodes.lock().unwrap().insert(src.to_string(), Instant::now());
                        println!("Discovered new node: {}", src.to_string());
                        match discover_notifier.send(src.to_string()).await {
                            Ok(_) => {}
                            Err(e) => {
                                println!("Error sending discovery notification: {}", e);
                            }
                        }
                    }
                }
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
        })
    }

    pub async fn listen(&self) -> tokio::task::JoinHandle<()> {
        let socket = UdpSocket::bind("0.0.0.0:8080").await.unwrap(); // Listen on port 8080
        let discovered_nodes = self.discovered_nodes.clone();
        let mut buffer = [0; 1024];
        tokio::spawn(async move {
            loop {
                if let Ok((size, src)) = socket.recv_from(&mut buffer).await {
                    let message = std::str::from_utf8(&buffer[..size]).unwrap();
                    if message == DISCOVER_REQUEST {
                        socket.send_to(DISCOVER_RESPONSE.as_bytes(), &src).await.unwrap();
                        let mut discovered_nodes = discovered_nodes.lock().unwrap();
                        discovered_nodes.insert(src.to_string(), Instant::now());
                    }
                }
            }
        })
    }

    #[allow(dead_code)]
    fn get_discovered_nodes(&self) -> Vec<String> {
        let mut nodes_to_remove = Vec::new();
        let mut active_nodes = Vec::new();
        let nodes = self.discovered_nodes.lock().unwrap();

        // Check for nodes that haven't been seen for more than 30 seconds
        let now = Instant::now();
        for (addr, &last_seen) in nodes.iter() {
            if now.duration_since(last_seen) > Duration::from_secs(30) {
                nodes_to_remove.push(addr.clone());
            } else {
                active_nodes.push(addr.clone());
            }
        }

        // Drop the lock before removing nodes to avoid potential deadlocks
        drop(nodes);

        // Remove inactive nodes
        if !nodes_to_remove.is_empty() {
            let mut nodes = self.discovered_nodes.lock().unwrap();
            for addr in nodes_to_remove {
                nodes.remove(&addr);
            }
        }

        active_nodes
    }
}
