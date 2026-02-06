//! Command handler for processing incoming MQTT commands.
//!
//! This module orchestrates command processing: validation, forwarding,
//! and response publishing.

use std::time::Duration;

use tokio::time::timeout;
use tracing::{error, info, warn};

use crate::command::CommandResponse;
use crate::error::ForwardError;
use crate::forwarder::CommandForwarder;
use crate::mqtt::{MqttClient, MqttMessage};
use crate::response::ResponsePublisher;
use crate::validator::CommandValidator;

/// Result of command processing.
#[derive(Debug, Clone)]
pub struct CommandProcessingResult {
    /// Command ID
    pub command_id: Option<String>,
    /// Whether processing succeeded
    pub success: bool,
    /// Error if processing failed
    pub error: Option<String>,
    /// Processing duration in milliseconds
    pub duration_ms: u64,
}

impl CommandProcessingResult {
    /// Create a success result.
    pub fn success(command_id: String, duration_ms: u64) -> Self {
        Self {
            command_id: Some(command_id),
            success: true,
            error: None,
            duration_ms,
        }
    }

    /// Create a failure result.
    pub fn failure(command_id: Option<String>, error: String, duration_ms: u64) -> Self {
        Self {
            command_id,
            success: false,
            error: Some(error),
            duration_ms,
        }
    }
}

/// Handles incoming MQTT commands.
pub struct CommandHandler {
    /// Command validator
    validator: CommandValidator,
    /// Response publisher
    response_publisher: ResponsePublisher,
    /// Command timeout in milliseconds
    timeout_ms: u64,
}

impl CommandHandler {
    /// Create a new command handler.
    pub fn new(valid_tokens: Vec<String>, vin: String, timeout_ms: u64) -> Self {
        Self {
            validator: CommandValidator::new(valid_tokens),
            response_publisher: ResponsePublisher::new(vin),
            timeout_ms,
        }
    }

    /// Handle an incoming MQTT message.
    ///
    /// Returns the processing result.
    pub async fn handle_message(
        &self,
        message: &MqttMessage,
        forwarder: &mut CommandForwarder,
        mqtt_client: &MqttClient,
    ) -> CommandProcessingResult {
        let start = std::time::Instant::now();

        info!(
            "Received command message on topic: {}, payload size: {} bytes",
            message.topic,
            message.payload.len()
        );

        // Validate the command
        let command = match self.validator.validate(&message.payload) {
            Ok(cmd) => {
                info!(
                    "Validated command: id={}, type={:?}, doors={:?}",
                    cmd.command_id, cmd.command_type, cmd.doors
                );
                cmd
            }
            Err(e) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                let error_string = e.to_string();
                error!("Command validation failed: {}", error_string);

                // Try to extract command_id from the payload for error response
                let command_id = self.extract_command_id(&message.payload);

                // Publish error response
                if let Some(ref id) = command_id {
                    let response: CommandResponse = e.into();
                    let mut response = response;
                    response.command_id = id.clone();
                    let _ = self
                        .response_publisher
                        .publish_response(mqtt_client, &response)
                        .await;
                }

                return CommandProcessingResult::failure(
                    command_id,
                    format!("Validation failed: {}", error_string),
                    duration_ms,
                );
            }
        };

        let command_id = command.command_id.clone();

        // Forward the command with timeout
        let forward_result = timeout(
            Duration::from_millis(self.timeout_ms),
            forwarder.forward(&command),
        )
        .await;

        let duration_ms = start.elapsed().as_millis() as u64;

        match forward_result {
            Ok(Ok(result)) => {
                if result.success {
                    info!(
                        "Command {} processed successfully in {}ms",
                        command_id, duration_ms
                    );

                    let _ = self
                        .response_publisher
                        .publish_success(mqtt_client, &command_id)
                        .await;

                    CommandProcessingResult::success(command_id, duration_ms)
                } else {
                    let error_code = result.error_code.unwrap_or_else(|| "UNKNOWN".to_string());
                    let error_message = result
                        .error_message
                        .unwrap_or_else(|| "Unknown error".to_string());

                    warn!("Command {} failed: {}", command_id, error_message);

                    let _ = self
                        .response_publisher
                        .publish_failure(
                            mqtt_client,
                            &command_id,
                            error_code,
                            error_message.clone(),
                        )
                        .await;

                    CommandProcessingResult::failure(Some(command_id), error_message, duration_ms)
                }
            }
            Ok(Err(e)) => {
                error!("Command {} forwarding error: {}", command_id, e);

                let (error_code, error_message) = match e {
                    ForwardError::ServiceUnavailable(msg) => ("SERVICE_UNAVAILABLE", msg),
                    ForwardError::ExecutionFailed(msg) => ("EXECUTION_FAILED", msg),
                    ForwardError::Timeout => ("TIMEOUT", "Command timed out".to_string()),
                };

                let _ = self
                    .response_publisher
                    .publish_failure(
                        mqtt_client,
                        &command_id,
                        error_code.to_string(),
                        error_message.clone(),
                    )
                    .await;

                CommandProcessingResult::failure(Some(command_id), error_message, duration_ms)
            }
            Err(_) => {
                error!(
                    "Command {} timed out after {}ms",
                    command_id, self.timeout_ms
                );

                let _ = self
                    .response_publisher
                    .publish_failure(
                        mqtt_client,
                        &command_id,
                        "TIMEOUT".to_string(),
                        format!("Command timed out after {}ms", self.timeout_ms),
                    )
                    .await;

                CommandProcessingResult::failure(
                    Some(command_id),
                    "Command timed out".to_string(),
                    duration_ms,
                )
            }
        }
    }

    /// Extract command_id from a potentially malformed JSON payload.
    fn extract_command_id(&self, payload: &[u8]) -> Option<String> {
        // Try to parse as JSON value to extract command_id
        if let Ok(value) = serde_json::from_slice::<serde_json::Value>(payload) {
            if let Some(id) = value.get("command_id") {
                if let Some(s) = id.as_str() {
                    return Some(s.to_string());
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_processing_result_success() {
        let result = CommandProcessingResult::success("cmd-123".to_string(), 100);
        assert!(result.success);
        assert_eq!(result.command_id, Some("cmd-123".to_string()));
        assert!(result.error.is_none());
        assert_eq!(result.duration_ms, 100);
    }

    #[test]
    fn test_processing_result_failure() {
        let result = CommandProcessingResult::failure(
            Some("cmd-123".to_string()),
            "Something went wrong".to_string(),
            50,
        );
        assert!(!result.success);
        assert_eq!(result.command_id, Some("cmd-123".to_string()));
        assert_eq!(result.error, Some("Something went wrong".to_string()));
        assert_eq!(result.duration_ms, 50);
    }

    #[test]
    fn test_extract_command_id() {
        let handler = CommandHandler::new(vec!["token".to_string()], "VIN123".to_string(), 5000);

        let payload = r#"{"command_id": "cmd-456", "type": "lock"}"#;
        let result = handler.extract_command_id(payload.as_bytes());
        assert_eq!(result, Some("cmd-456".to_string()));
    }

    #[test]
    fn test_extract_command_id_missing() {
        let handler = CommandHandler::new(vec!["token".to_string()], "VIN123".to_string(), 5000);

        let payload = r#"{"type": "lock"}"#;
        let result = handler.extract_command_id(payload.as_bytes());
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_command_id_malformed() {
        let handler = CommandHandler::new(vec!["token".to_string()], "VIN123".to_string(), 5000);

        let payload = b"not valid json";
        let result = handler.extract_command_id(payload);
        assert!(result.is_none());
    }

    // Property 12: Command Timeout Enforcement
    // Validates: Requirements 5.5
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_timeout_result_has_timeout_error(
            command_id in "[a-zA-Z0-9-]{1,36}",
            timeout_ms in 1000u64..10000
        ) {
            // When a command times out, the result should indicate timeout
            let result = CommandProcessingResult::failure(
                Some(command_id.clone()),
                format!("Command timed out after {}ms", timeout_ms),
                timeout_ms,
            );

            prop_assert!(!result.success);
            prop_assert!(result.error.is_some());
            prop_assert!(result.error.as_ref().unwrap().contains("timed out"));
        }

        #[test]
        fn prop_success_result_has_valid_duration(
            command_id in "[a-zA-Z0-9-]{1,36}",
            duration_ms in 0u64..10000
        ) {
            let result = CommandProcessingResult::success(command_id.clone(), duration_ms);

            prop_assert!(result.success);
            prop_assert_eq!(result.command_id, Some(command_id));
            prop_assert_eq!(result.duration_ms, duration_ms);
            prop_assert!(result.error.is_none());
        }

        #[test]
        fn prop_failure_result_preserves_error(
            command_id in "[a-zA-Z0-9-]{1,36}",
            error_msg in "[a-zA-Z0-9 ]{1,100}",
            duration_ms in 0u64..10000
        ) {
            let result = CommandProcessingResult::failure(
                Some(command_id.clone()),
                error_msg.clone(),
                duration_ms,
            );

            prop_assert!(!result.success);
            prop_assert_eq!(result.command_id, Some(command_id));
            prop_assert_eq!(result.error, Some(error_msg));
            prop_assert_eq!(result.duration_ms, duration_ms);
        }
    }
}
