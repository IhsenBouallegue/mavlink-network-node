use std::fmt::Debug;

use chrono::Utc;
use serde::Serialize;
use serde_json::to_value;
use tracing::{debug, error, field, info};
use tracing_appender::rolling::{self, RollingFileAppender};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{fmt, EnvFilter, Registry};

// Initialization of the logging system
pub fn init_logging() -> tracing_appender::non_blocking::WorkerGuard {
    let file_name = format!("logs_{}.json", Utc::now().format("%Y-%m-%d_%H-%M-%S%.3f"));
    let file_appender = RollingFileAppender::new(rolling::Rotation::NEVER, "./logs", &file_name);
    let (non_blocking_file_writer, _guard) = tracing_appender::non_blocking(file_appender);

    let filter_layer = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let file_layer = fmt::layer().json().with_writer(non_blocking_file_writer);
    let stdout_layer = fmt::layer().with_writer(std::io::stdout);

    let subscriber = Registry::default()
        .with(filter_layer)
        .with(file_layer)
        .with(stdout_layer);

    tracing::subscriber::set_global_default(subscriber).expect("Unable to set global subscriber");

    _guard
}

// Log an error during packet sending with ERROR level
pub fn log_packet_send_error<Packet: Debug>(driver: &str, packet: &Packet, error: &str) {
    error!(target: "network", driver, ?packet, error, "Packet send failed");
}

// Log an error during packet receiving with ERROR level
pub fn log_packet_receive_error(driver: &str, error: &str) {
    error!(target: "network", driver, %error, "Packet receive failed");
}

// Log the contents of a packet being sent with DEBUG level
pub fn log_debug_send_packet<Packet: Debug + Serialize>(driver: &str, packet: &Packet) {
    match to_value(packet) {
        Ok(json_packet) => {
            debug!(target: "network", driver, %json_packet, "Sending packet with contents");
        }
        Err(e) => {
            debug!(target: "network", driver, "Failed to serialize packet for logging: {:?}", e);
        }
    }
}

// Log the contents of a packet being received with DEBUG level
pub fn log_debug_receive_packet<Packet: Debug + Serialize>(driver: &str, packet: &Packet) {
    match to_value(packet) {
        Ok(json_packet) => {
            debug!(target: "network", driver, %json_packet, "Received packet with contents");
        }
        Err(e) => {
            debug!(target: "network", driver, "Failed to serialize packet for logging: {:?}", e);
        }
    }
}

// Log the creation of a driver instance with DEBUG level
pub fn log_driver_creation(driver: &str) {
    debug!(target: "network", driver, "Driver instance created");
}

// Log the start of the transmission process with INFO level
pub fn log_transmit_initiated(driver: &str) {
    info!(target: "network", driver, "Transmit initiated");
}

// Log the start of the listening process with INFO level
pub fn log_listen_initiated(driver: &str) {
    info!(target: "network", driver, "Listen initiated");
}

// Log an error during transmission with ERROR level
pub fn log_transmit_error(driver: &str, error: &str) {
    error!(target: "network", driver, %error, "Transmit error");
}

// Log an error during listening with ERROR level
pub fn log_listen_error(driver: &str, error: &str) {
    error!(target: "network", driver, %error, "Listen error");
}

// Log the creation of a network interface with DEBUG level
pub fn log_network_interface_creation(driver: &'static str) {
    debug!(target: "network", driver, "Network interface created with channels");
}

// Log the running of the network interface with INFO level
pub fn log_network_interface_running(driver: &str) {
    info!(target: "network", driver, "Running network interface");
}
