//! CLOUD_GATEWAY_CLIENT library
//!
//! This library provides components for vehicle-to-cloud communication
//! in the RHIVOS safety partition (ASIL-B).
//!
//! # Overview
//!
//! The CLOUD_GATEWAY_CLIENT:
//! - Maintains persistent MQTT/TLS connection to CLOUD_GATEWAY
//! - Receives and validates lock/unlock commands
//! - Forwards commands to LOCKING_SERVICE via gRPC/UDS
//! - Publishes command responses back to cloud
//! - Subscribes to vehicle signals from DATA_BROKER
//! - Publishes telemetry to cloud with offline buffering

pub mod cert_watcher;
pub mod command;
pub mod config;
pub mod error;
pub mod forwarder;
pub mod handler;
pub mod logging;
pub mod mqtt;
pub mod offline_buffer;
pub mod response;
pub mod subscriber;
pub mod telemetry;
#[cfg(test)]
pub mod test_utils;
pub mod validator;

// Proto-generated code
#[allow(clippy::all)]
pub mod proto {
    pub mod locking {
        include!("proto/sdv.services.locking.rs");
    }
}

// Re-exports for convenience
pub use cert_watcher::{
    CertReloadEvent, CertReloadStatus, CertificatePaths, CertificateWatcher, LoadedCertificates,
};
pub use command::{Command, CommandResponse, CommandType, Door, ResponseStatus};
pub use config::{MqttConfig, ServiceConfig};
pub use error::{
    CertLoadError, CertWatcherError, CloudGatewayError, ForwardError, MqttError, TelemetryError,
    ValidationError,
};
pub use forwarder::{CommandForwarder, ForwardResult};
pub use handler::{CommandHandler, CommandProcessingResult};
pub use logging::{init_tracing, init_tracing_pretty, EventType, LogEntry, LogLevel, Logger};
pub use mqtt::{calculate_backoff_delay, ConnectionState, MqttClient, MqttMessage};
pub use offline_buffer::{BufferedTelemetry, OfflineTelemetryBuffer};
pub use response::ResponsePublisher;
pub use subscriber::{vss_paths, SignalSubscriber, SignalUpdate, SignalValue};
pub use telemetry::{TelemetryPublishStats, TelemetryPublisher};
pub use validator::CommandValidator;
