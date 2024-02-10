use std::io::Write;

use futures_util::{SinkExt, StreamExt};
use tokio::net::UdpSocket;
use tokio::sync::mpsc::{self, Sender};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::http::Uri;
use tokio_tungstenite::tungstenite::Message;
use tracing_subscriber::fmt::MakeWriter;

pub struct WebSocketWriter {
    sender: Sender<String>,
}

impl Write for WebSocketWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let message = String::from_utf8_lossy(buf).trim().to_string();
        let sender = self.sender.clone();
        tokio::spawn(async move {
            sender.send(message).await.unwrap();
        });

        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub struct WebSocketMakeWriter {
    sender: Option<Sender<String>>,
}

impl WebSocketMakeWriter {
    pub fn new() -> Self {
        let (sender, mut receiver) = mpsc::channel(3); // Adjust buffer size as needed
        tokio::spawn(async move {
            // Udp discovery of the websocket server ip
            let ws_url = discover_websocket_server().await.unwrap();

            let (ws_stream, _) = connect_async(ws_url).await.expect("Failed to connect");
            let (mut write, mut _read) = ws_stream.split();

            while let Some(message) = receiver.recv().await {
                let ws_message = Message::Text(message);
                if let Err(e) = write.send(ws_message).await {
                    eprintln!("WebSocket send error: {:?}", e);
                }
            }
        });
        Self { sender: Some(sender) }
    }
}

impl<'a> MakeWriter<'a> for WebSocketMakeWriter {
    type Writer = WebSocketWriter;

    fn make_writer(&'a self) -> Self::Writer {
        WebSocketWriter {
            sender: self.sender.clone().unwrap(),
        }
    }
}

const SERVER_DISCOVERY_MSG: &str = "WebSocketServer";
const CLIENT_DISCOVERY_MSG: &str = "Connected";

async fn discover_websocket_server() -> Result<Uri, Box<dyn std::error::Error>> {
    let socket = UdpSocket::bind("0.0.0.0:8080").await?;

    let mut buf = [0u8; 1024];
    loop {
        match socket.recv_from(&mut buf).await {
            Ok((size, src_addr)) => {
                let message = String::from_utf8_lossy(&buf[..size]);
                println!("Received message: {}", message);

                if message.trim() == SERVER_DISCOVERY_MSG {
                    socket.send_to(CLIENT_DISCOVERY_MSG.as_bytes(), src_addr).await.unwrap();
                    let ws_url = format!("ws://{}:8080", src_addr.ip());
                    return Ok(Uri::try_from(ws_url)?);
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // No message received yet, continue waiting
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
            Err(e) => return Err(Box::new(e)),
        }
    }
}
