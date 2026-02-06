//! Structured logging for cloud-gateway-client.
//!
//! This module provides structured logging with correlation identifiers
//! for end-to-end tracing of commands, telemetry, and system events.

use chrono::{DateTime, Utc};
use serde::Serialize;
use tracing::{debug, error, info, warn, Level};
use uuid::Uuid;

/// Log entry with structured fields for JSON output.
#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    /// Timestamp of the log entry
    pub timestamp: DateTime<Utc>,
    /// Log level
    pub level: LogLevel,
    /// Command ID if this log relates to a specific command
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command_id: Option<String>,
    /// Correlation ID for end-to-end tracing
    pub correlation_id: String,
    /// Type of event being logged
    pub event_type: EventType,
    /// Additional details as JSON
    pub details: serde_json::Value,
}

impl LogEntry {
    /// Create a new log entry with auto-generated correlation ID.
    pub fn new(level: LogLevel, event_type: EventType) -> Self {
        Self {
            timestamp: Utc::now(),
            level,
            command_id: None,
            correlation_id: Uuid::new_v4().to_string(),
            event_type,
            details: serde_json::Value::Null,
        }
    }

    /// Create a new log entry with a specific correlation ID.
    pub fn with_correlation_id(
        level: LogLevel,
        event_type: EventType,
        correlation_id: String,
    ) -> Self {
        Self {
            timestamp: Utc::now(),
            level,
            command_id: None,
            correlation_id,
            event_type,
            details: serde_json::Value::Null,
        }
    }

    /// Set the command ID for this log entry.
    pub fn with_command_id(mut self, command_id: String) -> Self {
        self.command_id = Some(command_id);
        self
    }

    /// Set additional details for this log entry.
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = details;
        self
    }

    /// Emit this log entry using tracing.
    pub fn emit(&self) {
        let json = serde_json::to_string(self).unwrap_or_else(|_| format!("{:?}", self));

        match self.level {
            LogLevel::Trace => debug!(target: "cloud_gateway_client", "{}", json),
            LogLevel::Debug => debug!(target: "cloud_gateway_client", "{}", json),
            LogLevel::Info => info!(target: "cloud_gateway_client", "{}", json),
            LogLevel::Warn => warn!(target: "cloud_gateway_client", "{}", json),
            LogLevel::Error => error!(target: "cloud_gateway_client", "{}", json),
        }
    }
}

/// Log level for structured logging.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// Trace level (most verbose)
    Trace,
    /// Debug level
    Debug,
    /// Info level
    Info,
    /// Warning level
    Warn,
    /// Error level
    Error,
}

impl From<Level> for LogLevel {
    fn from(level: Level) -> Self {
        match level {
            Level::TRACE => LogLevel::Trace,
            Level::DEBUG => LogLevel::Debug,
            Level::INFO => LogLevel::Info,
            Level::WARN => LogLevel::Warn,
            Level::ERROR => LogLevel::Error,
        }
    }
}

/// Event types for structured logging.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    // MQTT connection events
    /// MQTT connection established
    MqttConnected,
    /// MQTT connection lost
    MqttDisconnected,
    /// MQTT reconnection attempt
    MqttReconnecting,

    // Command processing events
    /// Command received from MQTT
    CommandReceived,
    /// Command validation completed
    CommandValidated,
    /// Command forwarded to LOCKING_SERVICE
    CommandForwarded,
    /// Command processing completed
    CommandCompleted,

    // Response events
    /// Response published to MQTT
    ResponsePublished,

    // Telemetry events
    /// Telemetry published to MQTT
    TelemetryPublished,
    /// Telemetry buffered (MQTT offline)
    TelemetryBuffered,
    /// Telemetry buffer drained
    TelemetryBufferDrained,

    // DATA_BROKER events
    /// DATA_BROKER connection established
    DataBrokerConnected,
    /// DATA_BROKER connection lost
    DataBrokerDisconnected,
    /// Signal received from DATA_BROKER
    SignalReceived,

    // Certificate events
    /// Certificate reload succeeded
    CertReloadSuccess,
    /// Certificate reload failed
    CertReloadFailed,

    // Shutdown events
    /// Graceful shutdown initiated
    ShutdownInitiated,
    /// Graceful shutdown completed
    ShutdownCompleted,
}

/// Logger for the cloud-gateway-client service.
#[derive(Debug, Clone)]
pub struct Logger {
    /// Service name for log context
    service_name: String,
    /// Vehicle Identification Number
    vin: String,
}

impl Logger {
    /// Create a new logger for the service.
    pub fn new(service_name: String, vin: String) -> Self {
        Self { service_name, vin }
    }

    /// Generate a new correlation ID.
    pub fn new_correlation_id(&self) -> String {
        Uuid::new_v4().to_string()
    }

    /// Log an MQTT connected event.
    pub fn log_mqtt_connected(&self, broker_url: &str) {
        LogEntry::new(LogLevel::Info, EventType::MqttConnected)
            .with_details(serde_json::json!({
                "service": self.service_name,
                "vin": self.vin,
                "broker_url": broker_url
            }))
            .emit();
    }

    /// Log an MQTT disconnected event.
    pub fn log_mqtt_disconnected(&self, reason: &str) {
        LogEntry::new(LogLevel::Warn, EventType::MqttDisconnected)
            .with_details(serde_json::json!({
                "service": self.service_name,
                "vin": self.vin,
                "reason": reason
            }))
            .emit();
    }

    /// Log an MQTT reconnecting event.
    pub fn log_mqtt_reconnecting(&self, attempt: u32, delay_ms: u64) {
        LogEntry::new(LogLevel::Info, EventType::MqttReconnecting)
            .with_details(serde_json::json!({
                "service": self.service_name,
                "vin": self.vin,
                "attempt": attempt,
                "delay_ms": delay_ms
            }))
            .emit();
    }

    /// Log a command received event.
    pub fn log_command_received(
        &self,
        correlation_id: &str,
        command_id: &str,
        command_type: &str,
        topic: &str,
    ) {
        LogEntry::with_correlation_id(
            LogLevel::Info,
            EventType::CommandReceived,
            correlation_id.to_string(),
        )
        .with_command_id(command_id.to_string())
        .with_details(serde_json::json!({
            "service": self.service_name,
            "vin": self.vin,
            "command_type": command_type,
            "topic": topic
        }))
        .emit();
    }

    /// Log a command validated event.
    pub fn log_command_validated(&self, correlation_id: &str, command_id: &str, valid: bool) {
        let level = if valid {
            LogLevel::Info
        } else {
            LogLevel::Warn
        };

        LogEntry::with_correlation_id(
            level,
            EventType::CommandValidated,
            correlation_id.to_string(),
        )
        .with_command_id(command_id.to_string())
        .with_details(serde_json::json!({
            "service": self.service_name,
            "valid": valid
        }))
        .emit();
    }

    /// Log a command forwarded event.
    pub fn log_command_forwarded(&self, correlation_id: &str, command_id: &str, target: &str) {
        LogEntry::with_correlation_id(
            LogLevel::Info,
            EventType::CommandForwarded,
            correlation_id.to_string(),
        )
        .with_command_id(command_id.to_string())
        .with_details(serde_json::json!({
            "service": self.service_name,
            "target": target
        }))
        .emit();
    }

    /// Log a command completed event.
    pub fn log_command_completed(
        &self,
        correlation_id: &str,
        command_id: &str,
        success: bool,
        duration_ms: u64,
    ) {
        let level = if success {
            LogLevel::Info
        } else {
            LogLevel::Warn
        };

        LogEntry::with_correlation_id(
            level,
            EventType::CommandCompleted,
            correlation_id.to_string(),
        )
        .with_command_id(command_id.to_string())
        .with_details(serde_json::json!({
            "service": self.service_name,
            "success": success,
            "duration_ms": duration_ms
        }))
        .emit();
    }

    /// Log a response published event.
    pub fn log_response_published(
        &self,
        correlation_id: &str,
        command_id: &str,
        status: &str,
        topic: &str,
    ) {
        LogEntry::with_correlation_id(
            LogLevel::Info,
            EventType::ResponsePublished,
            correlation_id.to_string(),
        )
        .with_command_id(command_id.to_string())
        .with_details(serde_json::json!({
            "service": self.service_name,
            "status": status,
            "topic": topic
        }))
        .emit();
    }

    /// Log a telemetry published event.
    pub fn log_telemetry_published(&self, topic: &str) {
        LogEntry::new(LogLevel::Debug, EventType::TelemetryPublished)
            .with_details(serde_json::json!({
                "service": self.service_name,
                "vin": self.vin,
                "topic": topic
            }))
            .emit();
    }

    /// Log a telemetry buffered event.
    pub fn log_telemetry_buffered(&self, buffer_size: usize) {
        LogEntry::new(LogLevel::Debug, EventType::TelemetryBuffered)
            .with_details(serde_json::json!({
                "service": self.service_name,
                "vin": self.vin,
                "buffer_size": buffer_size
            }))
            .emit();
    }

    /// Log a telemetry buffer drained event.
    pub fn log_telemetry_buffer_drained(&self, count: usize) {
        LogEntry::new(LogLevel::Info, EventType::TelemetryBufferDrained)
            .with_details(serde_json::json!({
                "service": self.service_name,
                "vin": self.vin,
                "messages_drained": count
            }))
            .emit();
    }

    /// Log a DATA_BROKER connected event.
    pub fn log_data_broker_connected(&self, socket_path: &str) {
        LogEntry::new(LogLevel::Info, EventType::DataBrokerConnected)
            .with_details(serde_json::json!({
                "service": self.service_name,
                "socket_path": socket_path
            }))
            .emit();
    }

    /// Log a DATA_BROKER disconnected event.
    pub fn log_data_broker_disconnected(&self, reason: &str) {
        LogEntry::new(LogLevel::Warn, EventType::DataBrokerDisconnected)
            .with_details(serde_json::json!({
                "service": self.service_name,
                "reason": reason
            }))
            .emit();
    }

    /// Log a signal received event.
    pub fn log_signal_received(&self, signal_path: &str) {
        LogEntry::new(LogLevel::Trace, EventType::SignalReceived)
            .with_details(serde_json::json!({
                "service": self.service_name,
                "signal_path": signal_path
            }))
            .emit();
    }

    /// Log a certificate reload success event.
    pub fn log_cert_reload_success(&self, cert_path: &str, expiry_date: Option<DateTime<Utc>>) {
        LogEntry::new(LogLevel::Info, EventType::CertReloadSuccess)
            .with_details(serde_json::json!({
                "service": self.service_name,
                "cert_path": cert_path,
                "expiry_date": expiry_date.map(|d| d.to_rfc3339())
            }))
            .emit();
    }

    /// Log a certificate reload failure event.
    pub fn log_cert_reload_failed(&self, cert_path: &str, error: &str) {
        LogEntry::new(LogLevel::Error, EventType::CertReloadFailed)
            .with_details(serde_json::json!({
                "service": self.service_name,
                "cert_path": cert_path,
                "error": error
            }))
            .emit();
    }

    /// Log a shutdown initiated event.
    pub fn log_shutdown_initiated(&self, in_flight_count: usize) {
        LogEntry::new(LogLevel::Info, EventType::ShutdownInitiated)
            .with_details(serde_json::json!({
                "service": self.service_name,
                "in_flight_operations": in_flight_count
            }))
            .emit();
    }

    /// Log a shutdown completed event.
    pub fn log_shutdown_completed(&self, duration_ms: u64, forced: bool) {
        let level = if forced {
            LogLevel::Warn
        } else {
            LogLevel::Info
        };

        LogEntry::new(level, EventType::ShutdownCompleted)
            .with_details(serde_json::json!({
                "service": self.service_name,
                "duration_ms": duration_ms,
                "forced": forced
            }))
            .emit();
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self::new("cloud-gateway-client".to_string(), "UNKNOWN".to_string())
    }
}

/// Initialize tracing subscriber with structured output.
///
/// Uses compact format by default. For production, JSON output can be
/// enabled by configuring tracing-subscriber with the json feature.
pub fn init_tracing() {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().compact())
        .init();
}

/// Initialize tracing subscriber with pretty output for development.
pub fn init_tracing_pretty() {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().pretty())
        .init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_entry_creation() {
        let entry = LogEntry::new(LogLevel::Info, EventType::MqttConnected);

        assert_eq!(entry.level, LogLevel::Info);
        assert_eq!(entry.event_type, EventType::MqttConnected);
        assert!(entry.command_id.is_none());
        assert!(!entry.correlation_id.is_empty());
    }

    #[test]
    fn test_log_entry_with_command_id() {
        let entry = LogEntry::new(LogLevel::Info, EventType::CommandReceived)
            .with_command_id("cmd-123".to_string());

        assert_eq!(entry.command_id, Some("cmd-123".to_string()));
    }

    #[test]
    fn test_log_entry_with_details() {
        let entry = LogEntry::new(LogLevel::Info, EventType::MqttConnected)
            .with_details(serde_json::json!({"broker": "localhost"}));

        assert_eq!(entry.details["broker"], "localhost");
    }

    #[test]
    fn test_log_entry_serialization() {
        let entry = LogEntry::new(LogLevel::Info, EventType::MqttConnected)
            .with_command_id("cmd-123".to_string())
            .with_details(serde_json::json!({"test": true}));

        let json = serde_json::to_string(&entry).unwrap();

        assert!(json.contains("\"level\":\"info\""));
        assert!(json.contains("\"event_type\":\"mqtt_connected\""));
        assert!(json.contains("\"command_id\":\"cmd-123\""));
        assert!(json.contains("\"test\":true"));
    }

    #[test]
    fn test_log_level_from_tracing() {
        assert_eq!(LogLevel::from(Level::TRACE), LogLevel::Trace);
        assert_eq!(LogLevel::from(Level::DEBUG), LogLevel::Debug);
        assert_eq!(LogLevel::from(Level::INFO), LogLevel::Info);
        assert_eq!(LogLevel::from(Level::WARN), LogLevel::Warn);
        assert_eq!(LogLevel::from(Level::ERROR), LogLevel::Error);
    }

    #[test]
    fn test_event_type_serialization() {
        let types = vec![
            (EventType::MqttConnected, "mqtt_connected"),
            (EventType::MqttDisconnected, "mqtt_disconnected"),
            (EventType::MqttReconnecting, "mqtt_reconnecting"),
            (EventType::CommandReceived, "command_received"),
            (EventType::CommandValidated, "command_validated"),
            (EventType::CommandForwarded, "command_forwarded"),
            (EventType::CommandCompleted, "command_completed"),
            (EventType::ResponsePublished, "response_published"),
            (EventType::TelemetryPublished, "telemetry_published"),
            (EventType::TelemetryBuffered, "telemetry_buffered"),
            (
                EventType::TelemetryBufferDrained,
                "telemetry_buffer_drained",
            ),
            (EventType::DataBrokerConnected, "data_broker_connected"),
            (
                EventType::DataBrokerDisconnected,
                "data_broker_disconnected",
            ),
            (EventType::SignalReceived, "signal_received"),
            (EventType::CertReloadSuccess, "cert_reload_success"),
            (EventType::CertReloadFailed, "cert_reload_failed"),
            (EventType::ShutdownInitiated, "shutdown_initiated"),
            (EventType::ShutdownCompleted, "shutdown_completed"),
        ];

        for (event_type, expected_str) in types {
            let json = serde_json::to_string(&event_type).unwrap();
            assert_eq!(json, format!("\"{}\"", expected_str));
        }
    }

    #[test]
    fn test_logger_creation() {
        let logger = Logger::new("test-service".to_string(), "VIN123".to_string());
        assert!(!logger.new_correlation_id().is_empty());
    }

    #[test]
    fn test_logger_default() {
        let logger = Logger::default();
        assert!(!logger.new_correlation_id().is_empty());
    }
}
