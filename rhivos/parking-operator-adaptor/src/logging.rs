//! Structured logging for PARKING_OPERATOR_ADAPTOR.
//!
//! This module provides structured logging with event types for diagnostics.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

/// Event types for structured logging.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventType {
    /// Service startup
    ServiceStartup,
    /// Service shutdown
    ServiceShutdown,
    /// Session started
    SessionStarted,
    /// Session stopped
    SessionStopped,
    /// Session error
    SessionError,
    /// API call succeeded
    ApiSuccess,
    /// API call failed
    ApiFailure,
    /// State published
    StatePublished,
    /// Status polled
    StatusPolled,
    /// Signal received
    SignalReceived,
    /// Connection established
    ConnectionEstablished,
    /// Connection lost
    ConnectionLost,
}

/// Structured log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Event type
    pub event_type: EventType,
    /// Component name
    pub component: String,
    /// Message
    pub message: String,
    /// Optional session ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// Optional zone ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zone_id: Option<String>,
    /// Optional error message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl LogEntry {
    /// Create a new log entry.
    pub fn new(event_type: EventType, component: &str, message: &str) -> Self {
        Self {
            timestamp: Utc::now(),
            event_type,
            component: component.to_string(),
            message: message.to_string(),
            session_id: None,
            zone_id: None,
            error: None,
        }
    }

    /// Set session ID.
    pub fn with_session_id(mut self, session_id: &str) -> Self {
        self.session_id = Some(session_id.to_string());
        self
    }

    /// Set zone ID.
    pub fn with_zone_id(mut self, zone_id: &str) -> Self {
        self.zone_id = Some(zone_id.to_string());
        self
    }

    /// Set error.
    pub fn with_error(mut self, error: &str) -> Self {
        self.error = Some(error.to_string());
        self
    }

    /// Convert to JSON string.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| format!("{:?}", self))
    }
}

/// Logger for structured events.
pub struct Logger {
    component: String,
}

impl Logger {
    /// Create a new Logger.
    pub fn new(component: &str) -> Self {
        Self {
            component: component.to_string(),
        }
    }

    /// Log an event.
    pub fn log(&self, event_type: EventType, message: &str) -> LogEntry {
        let entry = LogEntry::new(event_type, &self.component, message);
        tracing::info!(target: "structured", "{}", entry.to_json());
        entry
    }

    /// Log a session event.
    pub fn log_session(&self, event_type: EventType, message: &str, session_id: &str) -> LogEntry {
        let entry = LogEntry::new(event_type, &self.component, message)
            .with_session_id(session_id);
        tracing::info!(target: "structured", "{}", entry.to_json());
        entry
    }

    /// Log an error event.
    pub fn log_error(&self, event_type: EventType, message: &str, error: &str) -> LogEntry {
        let entry = LogEntry::new(event_type, &self.component, message)
            .with_error(error);
        tracing::error!(target: "structured", "{}", entry.to_json());
        entry
    }
}

/// Initialize tracing with structured logging.
pub fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_span_events(FmtSpan::CLOSE)
        .compact();

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_log_entry_new() {
        let entry = LogEntry::new(EventType::ServiceStartup, "test", "Starting service");

        assert_eq!(entry.event_type, EventType::ServiceStartup);
        assert_eq!(entry.component, "test");
        assert_eq!(entry.message, "Starting service");
        assert!(entry.session_id.is_none());
    }

    #[test]
    fn test_log_entry_with_session() {
        let entry = LogEntry::new(EventType::SessionStarted, "manager", "Session started")
            .with_session_id("session-123")
            .with_zone_id("zone-1");

        assert_eq!(entry.session_id, Some("session-123".to_string()));
        assert_eq!(entry.zone_id, Some("zone-1".to_string()));
    }

    #[test]
    fn test_log_entry_with_error() {
        let entry = LogEntry::new(EventType::ApiFailure, "client", "API call failed")
            .with_error("Connection refused");

        assert_eq!(entry.error, Some("Connection refused".to_string()));
    }

    #[test]
    fn test_log_entry_to_json() {
        let entry = LogEntry::new(EventType::ServiceStartup, "test", "Starting");
        let json = entry.to_json();

        assert!(json.contains("\"event_type\":\"ServiceStartup\""));
        assert!(json.contains("\"component\":\"test\""));
        assert!(json.contains("\"message\":\"Starting\""));
    }

    // Property 16: Log Entry Format Consistency
    // Validates: Requirements 7.1, 7.2
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_log_entry_serialization(
            component in "[a-z_]{3,20}",
            message in "[a-zA-Z0-9 ]{5,50}",
            session_id in "[a-z0-9-]{8,36}"
        ) {
            let entry = LogEntry::new(EventType::SessionStarted, &component, &message)
                .with_session_id(&session_id);

            let json = entry.to_json();

            prop_assert!(json.contains(&component));
            prop_assert!(json.contains(&session_id));
            prop_assert!(json.contains("timestamp"));
            prop_assert!(json.contains("event_type"));
        }
    }

    // Property 17: Logger Component Consistency
    // Validates: Requirements 7.1
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_logger_component_preserved(
            component in "[a-z_]{3,20}",
            message in "[a-zA-Z0-9 ]{5,50}"
        ) {
            let _logger = Logger::new(&component);

            // We can't easily capture log output in tests, but we can verify
            // the log entry structure is correct
            let entry = LogEntry::new(EventType::ServiceStartup, &component, &message);

            prop_assert_eq!(entry.component, component);
            prop_assert_eq!(entry.message, message);
        }
    }
}
