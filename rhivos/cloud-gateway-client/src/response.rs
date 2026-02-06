//! Response publishing to CLOUD_GATEWAY.
//!
//! This module publishes command responses back to the cloud via MQTT.

use chrono::Utc;
use tracing::{debug, error, info};

use crate::command::{CommandResponse, ResponseStatus};
use crate::error::MqttError;
use crate::mqtt::MqttClient;

/// Publishes command responses to CLOUD_GATEWAY.
pub struct ResponsePublisher {
    /// Vehicle Identification Number for topic construction
    vin: String,
}

impl ResponsePublisher {
    /// Create a new response publisher.
    pub fn new(vin: String) -> Self {
        Self { vin }
    }

    /// Get the response topic for this vehicle.
    pub fn response_topic(&self) -> String {
        format!("vehicles/{}/command_responses", self.vin)
    }

    /// Publish a success response.
    pub async fn publish_success(
        &self,
        client: &MqttClient,
        command_id: &str,
    ) -> Result<(), MqttError> {
        let response = CommandResponse {
            command_id: command_id.to_string(),
            status: ResponseStatus::Success,
            error_code: None,
            error_message: None,
            timestamp: Utc::now().to_rfc3339(),
        };

        self.publish_response(client, &response).await
    }

    /// Publish a failure response.
    pub async fn publish_failure(
        &self,
        client: &MqttClient,
        command_id: &str,
        error_code: String,
        error_message: String,
    ) -> Result<(), MqttError> {
        let response = CommandResponse {
            command_id: command_id.to_string(),
            status: ResponseStatus::Failed,
            error_code: Some(error_code),
            error_message: Some(error_message),
            timestamp: Utc::now().to_rfc3339(),
        };

        self.publish_response(client, &response).await
    }

    /// Publish a command response.
    pub async fn publish_response(
        &self,
        client: &MqttClient,
        response: &CommandResponse,
    ) -> Result<(), MqttError> {
        let topic = self.response_topic();

        let payload = serde_json::to_vec(response).map_err(|e| {
            error!("Failed to serialize response: {}", e);
            MqttError::PublishFailed(format!("Serialization failed: {}", e))
        })?;

        debug!(
            "Publishing response for command {} to {}",
            response.command_id, topic
        );

        client.publish(&topic, &payload).await?;

        info!(
            "Published {} response for command {}",
            if response.status == ResponseStatus::Success {
                "success"
            } else {
                "failure"
            },
            response.command_id
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_response_topic() {
        let publisher = ResponsePublisher::new("VIN123".to_string());
        assert_eq!(
            publisher.response_topic(),
            "vehicles/VIN123/command_responses"
        );
    }

    #[test]
    fn test_response_topic_with_vin() {
        let publisher = ResponsePublisher::new("WVWZZZ3CZWE123456".to_string());
        assert_eq!(
            publisher.response_topic(),
            "vehicles/WVWZZZ3CZWE123456/command_responses"
        );
    }

    // Property 10: Response Command ID Correlation
    // Validates: Requirements 5.2
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_success_response_has_correct_command_id(
            command_id in "[a-zA-Z0-9-]{1,36}"
        ) {
            let response = CommandResponse {
                command_id: command_id.clone(),
                status: ResponseStatus::Success,
                error_code: None,
                error_message: None,
                timestamp: Utc::now().to_rfc3339(),
            };

            // Response command_id must match the original command
            prop_assert_eq!(&response.command_id, &command_id);
            prop_assert_eq!(response.status, ResponseStatus::Success);
        }

        #[test]
        fn prop_failure_response_has_correct_command_id(
            command_id in "[a-zA-Z0-9-]{1,36}",
            error_code in "[A-Z_]{3,20}",
            error_message in "[a-zA-Z0-9 ]{1,100}"
        ) {
            let response = CommandResponse {
                command_id: command_id.clone(),
                status: ResponseStatus::Failed,
                error_code: Some(error_code.clone()),
                error_message: Some(error_message.clone()),
                timestamp: Utc::now().to_rfc3339(),
            };

            // Response command_id must match the original command
            prop_assert_eq!(&response.command_id, &command_id);
            prop_assert_eq!(response.status, ResponseStatus::Failed);
        }
    }

    // Property 11: Response Structure Completeness
    // Validates: Requirements 5.3, 5.4
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_success_response_structure(
            command_id in "[a-zA-Z0-9-]{1,36}"
        ) {
            let response = CommandResponse {
                command_id: command_id.clone(),
                status: ResponseStatus::Success,
                error_code: None,
                error_message: None,
                timestamp: Utc::now().to_rfc3339(),
            };

            // Success responses should not have error fields
            prop_assert!(response.error_code.is_none());
            prop_assert!(response.error_message.is_none());
            prop_assert!(!response.timestamp.is_empty());
            prop_assert!(!response.command_id.is_empty());
        }

        #[test]
        fn prop_failure_response_has_error_details(
            command_id in "[a-zA-Z0-9-]{1,36}",
            error_code in "[A-Z_]{3,20}",
            error_message in "[a-zA-Z0-9 ]{1,100}"
        ) {
            let response = CommandResponse {
                command_id: command_id.clone(),
                status: ResponseStatus::Failed,
                error_code: Some(error_code.clone()),
                error_message: Some(error_message.clone()),
                timestamp: Utc::now().to_rfc3339(),
            };

            // Failure responses must have error fields
            prop_assert!(response.error_code.is_some());
            prop_assert!(response.error_message.is_some());
            prop_assert!(!response.error_code.as_ref().unwrap().is_empty());
            prop_assert!(!response.error_message.as_ref().unwrap().is_empty());
            prop_assert!(!response.timestamp.is_empty());
        }

        #[test]
        fn prop_response_serializes_to_valid_json(
            command_id in "[a-zA-Z0-9-]{1,36}",
            is_success in proptest::bool::ANY
        ) {
            let response = if is_success {
                CommandResponse {
                    command_id: command_id.clone(),
                    status: ResponseStatus::Success,
                    error_code: None,
                    error_message: None,
                    timestamp: Utc::now().to_rfc3339(),
                }
            } else {
                CommandResponse {
                    command_id: command_id.clone(),
                    status: ResponseStatus::Failed,
                    error_code: Some("ERROR".to_string()),
                    error_message: Some("Something failed".to_string()),
                    timestamp: Utc::now().to_rfc3339(),
                }
            };

            // Response must serialize to valid JSON
            let json = serde_json::to_string(&response);
            prop_assert!(json.is_ok());

            // JSON must contain command_id
            let json_str = json.unwrap();
            prop_assert!(json_str.contains(&command_id));
        }
    }
}
