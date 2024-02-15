use std::fmt::Debug;

use chrono::Utc;
use mavlink::Message;
use serde::Serialize;
use serde_json::to_value;
use tracing::{debug, error, info};
use tracing_appender::rolling::{self, RollingFileAppender};
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{fmt, EnvFilter, Registry};

use super::types::MavFramePacket;
use super::websocket_layer::WebSocketMakeWriter;

// Constants for log messages
const PACKET_TRANSMIT_ERROR_MSG: &str = "Packet transmit failed";
const PACKET_RECEIVE_ERROR_MSG: &str = "Packet receive failed";
const SEND_PACKET_MSG: &str = "Sending packet";
const RECEIVE_PACKET_MSG: &str = "Received packet";
const DRIVER_CREATION_MSG: &str = "Driver instance created";
const TRANSMIT_INITIATED_MSG: &str = "Transmit initiated";
const LISTEN_INITIATED_MSG: &str = "Listen initiated";
const TRANSMIT_ERROR_MSG: &str = "Transmit error";
const SEND_TO_MAIN_MSG: &str = "Send to main";
const SEND_TO_NETWORK_MSG: &str = "Send to network";
const NETWORK_INTERFACE_CREATION_MSG: &str = "Network interface created";
const NETWORK_INTERFACE_RUNNING_MSG: &str = "Running network interface";

// Initialization of the logging system
pub fn init_logging() -> tracing_appender::non_blocking::WorkerGuard {
    let file_name = format!("logs_{}.json", Utc::now().format("%Y-%m-%d_%H-%M-%S%.3f"));
    let file_appender: RollingFileAppender = RollingFileAppender::new(rolling::Rotation::NEVER, "./logs", &file_name);
    let (non_blocking_file_writer, _guard) = tracing_appender::non_blocking(file_appender);

    // let console_layer = console_subscriber::ConsoleLayer::builder()
    //     .server_addr(([127, 0, 0, 1], 5555))
    //     .spawn();
    let filter_layer = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"))
        .add_directive("lora_phy=error".parse().unwrap());
    // .add_directive("tokio=trace".parse().unwrap())
    // .add_directive("runtime=trace".parse().unwrap());
    let file_layer = fmt::layer()
        .json()
        .with_writer(non_blocking_file_writer)
        .with_span_events(FmtSpan::CLOSE);
    let stdout_layer = fmt::layer().pretty().with_writer(std::io::stdout);
    let websocket_layer = fmt::layer()
        .json()
        .with_writer(WebSocketMakeWriter::new())
        .with_span_events(FmtSpan::CLOSE);

    let subscriber = Registry::default()
        .with(filter_layer)
        .with(file_layer)
        .with(stdout_layer)
        .with(websocket_layer);
    // .with(console_layer);

    tracing::subscriber::set_global_default(subscriber).expect("Unable to set global subscriber");

    _guard
}

// Log an error during packet sending with ERROR level
pub fn log_packet_transmit_error<Packet: Debug>(driver: &str, packet: &Packet, error: &str) {
    error!(target: "network", driver, ?packet, error, "{}", PACKET_TRANSMIT_ERROR_MSG);
}

// Log an error during packet receiving with ERROR level
pub fn log_packet_receive_error(driver: &str, error: &str) {
    error!(target: "network", driver, %error, "{}", PACKET_RECEIVE_ERROR_MSG);
}

pub fn log_packet_received(
    size: usize,
    src_addr: Option<std::net::SocketAddr>,
    mavlink_frame: &MavFramePacket,
    driver: &str,
) {
    info!(
        target: "network",
        driver,
        size = size,
        src_addr = match src_addr {
            Some(addr) => Some(addr.to_string()),
            None => None,
        },
        message_type = &mavlink_frame.msg.message_name(),
        message_seq = mavlink_frame.header.sequence,
        "{}",
        RECEIVE_PACKET_MSG,
    );
}
pub fn log_packet_sent(size: usize, dest_addr: Option<&str>, packet: &MavFramePacket, driver: &str) {
    info!(
        target: "network",
        driver,
        size,
        dest_addr = match dest_addr {
            Some(addr) => Some(addr.to_string()),
            None => None,
        },
        message_type = &packet.msg.message_name(),
        message_seq = packet.header.sequence,
        "Sending packet",
    );
}

// Log the contents of a packet being sent with DEBUG level
pub fn log_debug_send_packet<Packet: Debug + Serialize>(driver: &str, packet: &Packet) {
    match to_value(packet) {
        Ok(json_packet) => {
            debug!(target: "network", driver, %json_packet, "{}", SEND_PACKET_MSG);
        }
        Err(e) => {
            debug!(target: "network", driver, "Failed to serialize packet for logging: {:?}", e);
        }
    }
}

// Log the contents of a packet being received with DEBUG level
pub fn log_debug_receive_packet<Packet: Debug + Serialize>(driver: &str, packet: &Packet, rssi: Option<i16>) {
    match to_value(packet) {
        Ok(json_packet) => {
            if let Some(rssi_value) = rssi {
                debug!(target: "network", driver, rssi = rssi_value, %json_packet, "{}", RECEIVE_PACKET_MSG);
            } else {
                debug!(target: "network", driver, %json_packet, "{}", RECEIVE_PACKET_MSG);
            }
        }
        Err(e) => {
            debug!(target: "network", driver, "Failed to serialize packet for logging: {:?}", e);
        }
    }
}

// Log the creation of a driver instance with DEBUG level
pub fn log_driver_creation(driver: &str) {
    debug!(target: "network", driver, "{}", DRIVER_CREATION_MSG);
}

// Log the start of the transmission process with INFO level
pub fn log_transmit_initiated(driver: &str) {
    info!(target: "network", driver, "{}", TRANSMIT_INITIATED_MSG);
}

// Log the start of the listening process with INFO level
pub fn log_listen_initiated(driver: &str) {
    info!(target: "network", driver, "{}", LISTEN_INITIATED_MSG);
}

// Log an error during transmission with ERROR level
pub fn log_transmit_error(driver: &str, error: &str) {
    error!(target: "network", driver, %error, "{}", TRANSMIT_ERROR_MSG);
}

// Log send to main with DEBUG level
pub fn log_debug_send_to_main(driver: &str) {
    debug!(target: "network", driver, "{}",SEND_TO_MAIN_MSG);
}

// Log send to network with DEBUG level
pub fn log_debug_send_to_network(driver: &str) {
    debug!(target: "main", driver, "{}", SEND_TO_NETWORK_MSG);
}

// Log the creation of a network interface with DEBUG level
pub fn log_network_interface_creation(driver: &str) {
    debug!(target: "network", driver, "{}", NETWORK_INTERFACE_CREATION_MSG);
}

// Log the running of the network interface with INFO level
pub fn log_network_interface_running(driver: &str) {
    info!(target: "network", driver, "{}", NETWORK_INTERFACE_RUNNING_MSG);
}
