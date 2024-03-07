use std::io::Write;

use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc::{self, Receiver, Sender};
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
    pub fn new(discovery_notifier: Receiver<String>) -> Self {
        let (sender, mut receiver) = mpsc::channel(3); // Adjust buffer size as needed
        tokio::spawn(async move {
            // Udp discovery of the websocket server ip
            let ws_url = discover_websocket_server(discovery_notifier).await;

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

async fn discover_websocket_server(mut discovery_notifier: Receiver<String>) -> Uri {
    loop {
        if let Some(discovered_node) = discovery_notifier.recv().await {
            // Attempt to establish a WebSocket connection to the discovered node
            let ws_url = format!("ws://{}", discovered_node);
            return Uri::try_from(ws_url).unwrap();
        }
    }
}
