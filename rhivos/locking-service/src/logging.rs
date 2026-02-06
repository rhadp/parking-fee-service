//! Structured logging for LOCKING_SERVICE.
//!
//! This module provides structured logging with correlation IDs for
//! end-to-end tracing of lock operations.

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

use crate::proto::Door;

/// Event types for structured logging.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    /// A command was received.
    CommandReceived,
    /// Authentication validation occurred.
    AuthValidation,
    /// Safety constraint validation occurred.
    SafetyValidation,
    /// Lock/unlock execution occurred.
    Execution,
    /// State publication to DATA_BROKER occurred.
    StatePublish,
    /// Command processing completed.
    CommandComplete,
}

/// Log levels for structured logging.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// A structured log entry.
#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    /// Timestamp of the log entry.
    #[serde(with = "system_time_serde")]
    pub timestamp: SystemTime,
    /// Log level.
    pub level: LogLevel,
    /// Command ID if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command_id: Option<String>,
    /// Correlation ID for tracing.
    pub correlation_id: String,
    /// Type of event being logged.
    pub event_type: EventType,
    /// Door if applicable (as string for serialization).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub door: Option<String>,
    /// Additional details.
    #[serde(skip_serializing_if = "serde_json::Value::is_null")]
    pub details: serde_json::Value,
}

impl LogEntry {
    /// Creates a new log entry with the current timestamp.
    pub fn new(level: LogLevel, correlation_id: impl Into<String>, event_type: EventType) -> Self {
        Self {
            timestamp: SystemTime::now(),
            level,
            command_id: None,
            correlation_id: correlation_id.into(),
            event_type,
            door: None,
            details: serde_json::Value::Null,
        }
    }

    /// Sets the command ID.
    pub fn with_command_id(mut self, command_id: impl Into<String>) -> Self {
        self.command_id = Some(command_id.into());
        self
    }

    /// Sets the door.
    pub fn with_door(mut self, door: impl std::fmt::Debug) -> Self {
        self.door = Some(format!("{:?}", door));
        self
    }

    /// Sets the details.
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = details;
        self
    }

    /// Serializes the log entry to JSON.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| format!("{:?}", self))
    }
}

/// Helper module for serializing SystemTime.
mod system_time_serde {
    use serde::{self, Serializer};
    use std::time::{SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let duration = time.duration_since(UNIX_EPOCH).unwrap_or_default();
        let millis = duration.as_millis() as u64;
        serializer.serialize_u64(millis)
    }
}

/// Logger for the locking service.
#[derive(Clone)]
pub struct Logger {
    service_name: String,
}

impl Default for Logger {
    fn default() -> Self {
        Self::new("locking-service")
    }
}

impl Logger {
    /// Creates a new logger with the given service name.
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
        }
    }

    /// Logs an entry at the specified level.
    pub fn log(&self, entry: &LogEntry) {
        let json = entry.to_json();
        match entry.level {
            LogLevel::Debug => tracing::debug!(service = %self.service_name, "{}", json),
            LogLevel::Info => tracing::info!(service = %self.service_name, "{}", json),
            LogLevel::Warn => tracing::warn!(service = %self.service_name, "{}", json),
            LogLevel::Error => tracing::error!(service = %self.service_name, "{}", json),
        }
    }

    /// Logs a command received event.
    pub fn log_command_received(
        &self,
        correlation_id: &str,
        command_id: &str,
        command_type: &str,
        door: Door,
    ) {
        let entry = LogEntry::new(LogLevel::Info, correlation_id, EventType::CommandReceived)
            .with_command_id(command_id)
            .with_door(door)
            .with_details(serde_json::json!({
                "command_type": command_type,
            }));
        self.log(&entry);
    }

    /// Logs an authentication validation result.
    pub fn log_auth_validation(&self, correlation_id: &str, command_id: &str, success: bool) {
        let level = if success {
            LogLevel::Info
        } else {
            LogLevel::Warn
        };
        let entry = LogEntry::new(level, correlation_id, EventType::AuthValidation)
            .with_command_id(command_id)
            .with_details(serde_json::json!({
                "success": success,
            }));
        self.log(&entry);
    }

    /// Logs a safety validation result.
    pub fn log_safety_validation(
        &self,
        correlation_id: &str,
        command_id: &str,
        door: Door,
        success: bool,
        reason: Option<&str>,
    ) {
        let level = if success {
            LogLevel::Info
        } else {
            LogLevel::Warn
        };
        let mut details = serde_json::json!({ "success": success });
        if let Some(r) = reason {
            details["reason"] = serde_json::Value::String(r.to_string());
        }
        let entry = LogEntry::new(level, correlation_id, EventType::SafetyValidation)
            .with_command_id(command_id)
            .with_door(door)
            .with_details(details);
        self.log(&entry);
    }

    /// Logs an execution result.
    pub fn log_execution(
        &self,
        correlation_id: &str,
        command_id: &str,
        door: Door,
        operation: &str,
        success: bool,
    ) {
        let level = if success {
            LogLevel::Info
        } else {
            LogLevel::Error
        };
        let entry = LogEntry::new(level, correlation_id, EventType::Execution)
            .with_command_id(command_id)
            .with_door(door)
            .with_details(serde_json::json!({
                "operation": operation,
                "success": success,
            }));
        self.log(&entry);
    }

    /// Logs a state publication result.
    pub fn log_state_publish(
        &self,
        correlation_id: &str,
        command_id: &str,
        door: Door,
        is_locked: bool,
        success: bool,
    ) {
        let level = if success {
            LogLevel::Info
        } else {
            LogLevel::Warn
        };
        let entry = LogEntry::new(level, correlation_id, EventType::StatePublish)
            .with_command_id(command_id)
            .with_door(door)
            .with_details(serde_json::json!({
                "is_locked": is_locked,
                "success": success,
            }));
        self.log(&entry);
    }

    /// Logs command completion.
    pub fn log_command_complete(
        &self,
        correlation_id: &str,
        command_id: &str,
        success: bool,
        error_message: Option<&str>,
    ) {
        let level = if success {
            LogLevel::Info
        } else {
            LogLevel::Error
        };
        let mut details = serde_json::json!({ "success": success });
        if let Some(msg) = error_message {
            details["error_message"] = serde_json::Value::String(msg.to_string());
        }
        let entry = LogEntry::new(level, correlation_id, EventType::CommandComplete)
            .with_command_id(command_id)
            .with_details(details);
        self.log(&entry);
    }
}

/// Generates a new correlation ID.
pub fn generate_correlation_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("corr-{:x}", timestamp)
}

/// Initializes the tracing subscriber for structured logging.
pub fn init_tracing() {
    use tracing_subscriber::EnvFilter;

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::fmt().with_env_filter(filter).init();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::Door;

    #[test]
    fn test_log_entry_serialization() {
        let entry = LogEntry::new(LogLevel::Info, "corr-123", EventType::CommandReceived)
            .with_command_id("cmd-456")
            .with_door(Door::Driver)
            .with_details(serde_json::json!({"key": "value"}));

        let json = entry.to_json();
        assert!(json.contains("corr-123"));
        assert!(json.contains("cmd-456"));
        assert!(json.contains("command_received"));
    }

    #[test]
    fn test_generate_correlation_id() {
        let id1 = generate_correlation_id();
        let id2 = generate_correlation_id();

        assert!(id1.starts_with("corr-"));
        assert!(id2.starts_with("corr-"));
        // IDs should be different (unless generated at exact same nanosecond)
    }

    #[test]
    fn test_event_type_serialization() {
        let json = serde_json::to_string(&EventType::CommandReceived).unwrap();
        assert_eq!(json, "\"command_received\"");

        let json = serde_json::to_string(&EventType::SafetyValidation).unwrap();
        assert_eq!(json, "\"safety_validation\"");
    }
}
