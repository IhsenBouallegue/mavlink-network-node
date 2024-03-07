// use std::fmt::Display;
// use std::sync::Arc;

// use futures::stream::{SplitSink, SplitStream};
// use futures::SinkExt;
// use futures_util::stream::StreamExt;
// use tokio::net::TcpStream;
// use tokio_tungstenite::tungstenite::protocol::Message;
// use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

// use crate::driver::Driver; // Make sure this path matches your project structure
// use crate::utils::types::MavFramePacket; // Assuming MavFramePacket is the data structure you want to send/receive

// pub const WEBSOCKET_DRIVER: &str = "websocket_driver";

// impl Display for WebSocketDriver {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "{}", WEBSOCKET_DRIVER)
//     }
// }

// pub struct WebSocketDriver {
//     write_half: Arc<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>,
//     read_half: Arc<SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>>,
// }

// impl WebSocketDriver {
//     async fn new(url: &str) -> Self {
//         let (ws_stream, _) = connect_async(url).await.expect("Failed to connect to WebSocket");

//         let (write_half, read_half) = ws_stream.split();

//         WebSocketDriver {
//             write_half: Arc::new(write_half),
//             read_half: Arc::new(read_half),
//         }
//     }
// }

// #[async_trait::async_trait]
// impl Driver<String> for WebSocketDriver {
//     async fn send(&self, packet: &String) {
//         // Assuming packet can be converted to a suitable format for WebSocket,
//         // such as JSON, binary, etc. This part depends on your implementation.
//         let message = Message::text(packet.to_string());
//         self.write_half
//             .clone()
//             .send(message)
//             .await
//             .map_err(|e| eprintln!("WebSocket send error: {}", e));
//     }

//     async fn receive(&self) -> Option<String> {
//         let mut lock = self.read_half.clone();
//         if let Some(message_result) = lock.next().await {
//             match message_result {
//                 Ok(message) => match message {
//                     Message::Text(text) => Some(text),
//                     // Handle other message types (binary, ping, close, etc.) as needed
//                     _ => None,
//                 },
//                 Err(e) => {
//                     eprintln!("WebSocket receive error: {}", e);
//                     None
//                 }
//             }
//         } else {
//             None
//         }
//     }
// }
