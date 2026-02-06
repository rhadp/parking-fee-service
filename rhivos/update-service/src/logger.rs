//! Operation logging for UPDATE_SERVICE.
//!
//! This module provides structured logging for all adapter operations
//! with correlation identifiers for request tracing.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

use crate::proto::AdapterState;
use crate::state::adapter_state_from_i32;

/// Container operation types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContainerOperation {
    /// Pull image from registry
    Pull,
    /// Install container from image
    Install,
    /// Start container
    Start,
    /// Stop container
    Stop,
    /// Remove container
    Remove,
}

impl std::fmt::Display for ContainerOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContainerOperation::Pull => write!(f, "pull"),
            ContainerOperation::Install => write!(f, "install"),
            ContainerOperation::Start => write!(f, "start"),
            ContainerOperation::Stop => write!(f, "stop"),
            ContainerOperation::Remove => write!(f, "remove"),
        }
    }
}

/// Operation outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationOutcome {
    /// Operation succeeded
    Success,
    /// Operation failed with reason
    Failure(String),
}

impl OperationOutcome {
    /// Check if the outcome is success.
    pub fn is_success(&self) -> bool {
        matches!(self, OperationOutcome::Success)
    }
}

/// Authentication events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthEvent {
    /// Token requested from registry
    TokenRequested,
    /// Token obtained successfully
    TokenObtained,
    /// Token cached for reuse
    TokenCached,
    /// Token refreshed before expiry
    TokenRefreshed,
    /// Authentication failed
    AuthenticationFailed(String),
    /// Anonymous access used
    AnonymousAccess,
}

/// Structured log entry format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// ISO 8601 timestamp
    pub timestamp: String,
    /// Log level (INFO, WARN, ERROR)
    pub level: String,
    /// Correlation ID for request tracing
    pub correlation_id: String,
    /// Service name
    pub service: String,
    /// Optional adapter ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapter_id: Option<String>,
    /// Event type (request, state_transition, container_op, auth)
    pub event_type: String,
    /// Human-readable message
    pub message: String,
    /// Additional structured data
    #[serde(default)]
    pub details: serde_json::Value,
}

impl LogEntry {
    /// Create a new log entry.
    pub fn new(
        level: &str,
        correlation_id: &str,
        service: &str,
        event_type: &str,
        message: &str,
    ) -> Self {
        Self {
            timestamp: Utc::now().to_rfc3339(),
            level: level.to_string(),
            correlation_id: correlation_id.to_string(),
            service: service.to_string(),
            adapter_id: None,
            event_type: event_type.to_string(),
            message: message.to_string(),
            details: serde_json::Value::Null,
        }
    }

    /// Set adapter ID.
    pub fn with_adapter_id(mut self, adapter_id: &str) -> Self {
        self.adapter_id = Some(adapter_id.to_string());
        self
    }

    /// Set additional details.
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = details;
        self
    }

    /// Convert to JSON string.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| format!("{:?}", self))
    }
}

/// Logger for structured events.
pub struct OperationLogger {
    service_name: String,
}

impl OperationLogger {
    /// Create a new OperationLogger.
    pub fn new(service_name: &str) -> Self {
        Self {
            service_name: service_name.to_string(),
        }
    }

    /// Log an incoming request.
    pub fn log_request(&self, correlation_id: &str, request_type: &str, adapter_id: &str) {
        let entry = LogEntry::new(
            "INFO",
            correlation_id,
            &self.service_name,
            "request",
            &format!("Received {} request", request_type),
        )
        .with_adapter_id(adapter_id)
        .with_details(serde_json::json!({
            "request_type": request_type
        }));

        info!(
            target: "structured",
            correlation_id = %correlation_id,
            request_type = %request_type,
            adapter_id = %adapter_id,
            "{}",
            entry.to_json()
        );
    }

    /// Log a state transition.
    pub fn log_state_transition(
        &self,
        correlation_id: &str,
        adapter_id: &str,
        previous_state: AdapterState,
        new_state: AdapterState,
        reason: Option<&str>,
    ) {
        let entry = LogEntry::new(
            "INFO",
            correlation_id,
            &self.service_name,
            "state_transition",
            &format!(
                "Adapter state changed: {:?} -> {:?}",
                previous_state, new_state
            ),
        )
        .with_adapter_id(adapter_id)
        .with_details(serde_json::json!({
            "previous_state": format!("{:?}", previous_state),
            "new_state": format!("{:?}", new_state),
            "reason": reason
        }));

        info!(
            target: "structured",
            correlation_id = %correlation_id,
            adapter_id = %adapter_id,
            previous_state = ?previous_state,
            new_state = ?new_state,
            "{}",
            entry.to_json()
        );
    }

    /// Log a state transition using i32 state values.
    pub fn log_state_transition_i32(
        &self,
        correlation_id: &str,
        adapter_id: &str,
        previous_state: i32,
        new_state: i32,
        reason: Option<&str>,
    ) {
        self.log_state_transition(
            correlation_id,
            adapter_id,
            adapter_state_from_i32(previous_state),
            adapter_state_from_i32(new_state),
            reason,
        );
    }

    /// Log a container operation.
    pub fn log_container_operation(
        &self,
        correlation_id: &str,
        adapter_id: &str,
        operation: ContainerOperation,
        outcome: OperationOutcome,
    ) {
        let level = if outcome.is_success() {
            "INFO"
        } else {
            "ERROR"
        };
        let entry = LogEntry::new(
            level,
            correlation_id,
            &self.service_name,
            "container_op",
            &format!(
                "Container operation '{}' completed: {:?}",
                operation, outcome
            ),
        )
        .with_adapter_id(adapter_id)
        .with_details(serde_json::json!({
            "operation": format!("{}", operation),
            "outcome": match &outcome {
                OperationOutcome::Success => "success".to_string(),
                OperationOutcome::Failure(msg) => format!("failure: {}", msg),
            }
        }));

        match outcome {
            OperationOutcome::Success => {
                info!(
                    target: "structured",
                    correlation_id = %correlation_id,
                    adapter_id = %adapter_id,
                    operation = %operation,
                    "{}",
                    entry.to_json()
                );
            }
            OperationOutcome::Failure(ref msg) => {
                error!(
                    target: "structured",
                    correlation_id = %correlation_id,
                    adapter_id = %adapter_id,
                    operation = %operation,
                    error = %msg,
                    "{}",
                    entry.to_json()
                );
            }
        }
    }

    /// Log an authentication event.
    pub fn log_auth_event(&self, correlation_id: &str, registry_url: &str, event: AuthEvent) {
        let (level, msg) = match &event {
            AuthEvent::TokenRequested => ("INFO", "Token requested from registry".to_string()),
            AuthEvent::TokenObtained => ("INFO", "Token obtained successfully".to_string()),
            AuthEvent::TokenCached => ("INFO", "Token cached for reuse".to_string()),
            AuthEvent::TokenRefreshed => ("INFO", "Token refreshed before expiry".to_string()),
            AuthEvent::AuthenticationFailed(reason) => {
                ("ERROR", format!("Authentication failed: {}", reason))
            }
            AuthEvent::AnonymousAccess => ("INFO", "Using anonymous access".to_string()),
        };

        let entry = LogEntry::new(level, correlation_id, &self.service_name, "auth", &msg)
            .with_details(serde_json::json!({
                "registry_url": registry_url,
                "event": format!("{:?}", event)
            }));

        match level {
            "ERROR" => {
                error!(
                    target: "structured",
                    correlation_id = %correlation_id,
                    registry_url = %registry_url,
                    "{}",
                    entry.to_json()
                );
            }
            "WARN" => {
                warn!(
                    target: "structured",
                    correlation_id = %correlation_id,
                    registry_url = %registry_url,
                    "{}",
                    entry.to_json()
                );
            }
            _ => {
                info!(
                    target: "structured",
                    correlation_id = %correlation_id,
                    registry_url = %registry_url,
                    "{}",
                    entry.to_json()
                );
            }
        }
    }

    /// Generate a new correlation ID.
    pub fn generate_correlation_id() -> String {
        uuid::Uuid::new_v4().to_string()
    }
}

/// Initialize tracing with structured logging.
pub fn init_tracing(log_level: &str) {
    use tracing_subscriber::fmt::format::FmtSpan;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::EnvFilter;

    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level));

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_span_events(FmtSpan::CLOSE)
        .json();

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
        let entry = LogEntry::new(
            "INFO",
            "corr-123",
            "update-service",
            "request",
            "Test message",
        );

        assert_eq!(entry.level, "INFO");
        assert_eq!(entry.correlation_id, "corr-123");
        assert_eq!(entry.service, "update-service");
        assert_eq!(entry.event_type, "request");
        assert!(entry.adapter_id.is_none());
    }

    #[test]
    fn test_log_entry_with_adapter_id() {
        let entry = LogEntry::new("INFO", "corr-123", "update-service", "request", "Test")
            .with_adapter_id("adapter-1");

        assert_eq!(entry.adapter_id, Some("adapter-1".to_string()));
    }

    #[test]
    fn test_log_entry_to_json() {
        let entry = LogEntry::new("INFO", "corr-123", "update-service", "request", "Test");
        let json = entry.to_json();

        assert!(json.contains("\"correlation_id\":\"corr-123\""));
        assert!(json.contains("\"event_type\":\"request\""));
    }

    #[test]
    fn test_operation_outcome() {
        assert!(OperationOutcome::Success.is_success());
        assert!(!OperationOutcome::Failure("error".to_string()).is_success());
    }

    #[test]
    fn test_generate_correlation_id() {
        let id1 = OperationLogger::generate_correlation_id();
        let id2 = OperationLogger::generate_correlation_id();

        assert_ne!(id1, id2);
        assert_eq!(id1.len(), 36); // UUID format
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 22: Request Logging with Correlation ID
        /// Validates: Requirements 12.1, 12.4
        #[test]
        fn prop_request_logging_includes_correlation_id(
            adapter_id in "[a-z][a-z0-9-]{3,20}",
            request_type in prop::sample::select(vec!["InstallAdapter", "UninstallAdapter", "ListAdapters"])
        ) {
            let correlation_id = OperationLogger::generate_correlation_id();

            let entry = LogEntry::new(
                "INFO",
                &correlation_id,
                "update-service",
                "request",
                &format!("Received {} request", request_type),
            )
            .with_adapter_id(&adapter_id)
            .with_details(serde_json::json!({
                "request_type": request_type
            }));

            let json = entry.to_json();

            prop_assert!(json.contains(&correlation_id));
            prop_assert!(json.contains(&adapter_id));
            prop_assert!(json.contains("request"));
            prop_assert!(!entry.timestamp.is_empty());
        }

        /// Property 23: State Transition Logging
        /// Validates: Requirements 12.2, 12.4
        #[test]
        fn prop_state_transition_logging(
            adapter_id in "[a-z][a-z0-9-]{3,20}",
            prev_state in 0i32..=5,
            new_state in 0i32..=5
        ) {
            let correlation_id = OperationLogger::generate_correlation_id();
            let prev = adapter_state_from_i32(prev_state);
            let new = adapter_state_from_i32(new_state);

            let entry = LogEntry::new(
                "INFO",
                &correlation_id,
                "update-service",
                "state_transition",
                &format!("State changed: {:?} -> {:?}", prev, new),
            )
            .with_adapter_id(&adapter_id)
            .with_details(serde_json::json!({
                "previous_state": format!("{:?}", prev),
                "new_state": format!("{:?}", new)
            }));

            let json = entry.to_json();
            let prev_str = format!("{:?}", prev);
            let new_str = format!("{:?}", new);

            prop_assert!(json.contains(&correlation_id));
            prop_assert!(json.contains("state_transition"));
            prop_assert!(json.contains(&prev_str));
            prop_assert!(json.contains(&new_str));
        }

        /// Property 24: Container Operation Logging
        /// Validates: Requirements 12.3, 12.4
        #[test]
        fn prop_container_operation_logging(
            adapter_id in "[a-z][a-z0-9-]{3,20}",
            op_idx in 0usize..5
        ) {
            let operations = [
                ContainerOperation::Pull,
                ContainerOperation::Install,
                ContainerOperation::Start,
                ContainerOperation::Stop,
                ContainerOperation::Remove,
            ];
            let operation = operations[op_idx];
            let correlation_id = OperationLogger::generate_correlation_id();

            let entry = LogEntry::new(
                "INFO",
                &correlation_id,
                "update-service",
                "container_op",
                &format!("Container operation '{}' completed", operation),
            )
            .with_adapter_id(&adapter_id)
            .with_details(serde_json::json!({
                "operation": format!("{}", operation),
                "outcome": "success"
            }));

            let json = entry.to_json();
            let op_str = format!("{}", operation);

            prop_assert!(json.contains(&correlation_id));
            prop_assert!(json.contains("container_op"));
            prop_assert!(json.contains(&op_str));
        }
    }
}
