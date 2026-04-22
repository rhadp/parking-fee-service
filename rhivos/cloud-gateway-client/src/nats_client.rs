//! NATS client for cloud-gateway-client.
//!
//! Manages the NATS connection lifecycle: connect with retry, subscribe
//! to commands, publish responses/telemetry/status. Encapsulates all
//! `async-nats` API usage.

use std::collections::HashMap;
use std::time::Duration;

use async_nats::Client;
use tokio::time::sleep;
use tracing::{error, info, warn};

use crate::config::Config;
use crate::errors::NatsError;
use crate::models::RegistrationMessage;

/// Maximum number of connection attempts before giving up.
const MAX_RETRIES: u32 = 5;

/// NATS client wrapper with domain-specific publish/subscribe methods.
pub struct NatsClient {
    client: Client,
    vin: String,
}

impl NatsClient {
    /// Connect to the NATS server with exponential backoff retry.
    ///
    /// Makes up to [`MAX_RETRIES`] connection attempts. Delays between
    /// attempts follow exponential backoff: 1s, 2s, 4s, 8s.
    ///
    /// # Errors
    ///
    /// Returns [`NatsError::RetriesExhausted`] if all attempts fail.
    pub async fn connect(config: &Config) -> Result<Self, NatsError> {
        for attempt in 1..=MAX_RETRIES {
            match async_nats::connect(&config.nats_url).await {
                Ok(client) => {
                    info!(
                        url = %config.nats_url,
                        attempt,
                        "Connected to NATS"
                    );
                    return Ok(NatsClient {
                        client,
                        vin: config.vin.clone(),
                    });
                }
                Err(e) => {
                    if attempt < MAX_RETRIES {
                        let delay = Duration::from_secs(1 << (attempt - 1));
                        warn!(
                            url = %config.nats_url,
                            attempt,
                            max_retries = MAX_RETRIES,
                            delay_secs = delay.as_secs(),
                            error = %e,
                            "NATS connection failed, retrying"
                        );
                        sleep(delay).await;
                    } else {
                        error!(
                            url = %config.nats_url,
                            attempt,
                            error = %e,
                            "NATS connection failed, all retries exhausted"
                        );
                    }
                }
            }
        }

        Err(NatsError::RetriesExhausted)
    }

    /// Subscribe to the command subject for this vehicle.
    ///
    /// Returns a subscriber for `vehicles.{VIN}.commands`.
    pub async fn subscribe_commands(
        &self,
    ) -> Result<async_nats::Subscriber, NatsError> {
        let subject = format!("vehicles.{}.commands", self.vin);
        let subscriber = self
            .client
            .subscribe(subject.clone())
            .await
            .map_err(|e| NatsError::SubscribeFailed(e.to_string()))?;
        info!(subject = %subject, "Subscribed to commands");
        Ok(subscriber)
    }

    /// Publish the self-registration message.
    ///
    /// Publishes a registration message to `vehicles.{VIN}.status` with
    /// the current timestamp. This is fire-and-forget per REQ-4.2.
    pub async fn publish_registration(&self) -> Result<(), NatsError> {
        let subject = format!("vehicles.{}.status", self.vin);
        let msg = RegistrationMessage {
            vin: self.vin.clone(),
            status: "online".to_string(),
            timestamp: unix_timestamp(),
        };
        let payload = serde_json::to_string(&msg)
            .expect("registration message serialization should not fail");

        self.client
            .publish(subject.clone(), payload.into())
            .await
            .map_err(|e| NatsError::PublishFailed(e.to_string()))?;

        info!(
            subject = %subject,
            vin = %self.vin,
            "Published self-registration"
        );
        Ok(())
    }

    /// Publish a command response to NATS.
    ///
    /// Publishes the JSON string verbatim to `vehicles.{VIN}.command_responses`.
    pub async fn publish_response(&self, json: &str) -> Result<(), NatsError> {
        let subject = format!("vehicles.{}.command_responses", self.vin);
        self.client
            .publish(subject.clone(), json.to_owned().into())
            .await
            .map_err(|e| NatsError::PublishFailed(e.to_string()))?;

        info!(
            subject = %subject,
            "Relayed command response"
        );
        Ok(())
    }

    /// Publish a telemetry message to NATS.
    ///
    /// Publishes the aggregated telemetry JSON to `vehicles.{VIN}.telemetry`.
    pub async fn publish_telemetry(&self, json: &str) -> Result<(), NatsError> {
        let subject = format!("vehicles.{}.telemetry", self.vin);
        self.client
            .publish(subject.clone(), json.to_owned().into())
            .await
            .map_err(|e| NatsError::PublishFailed(e.to_string()))?;

        info!(
            subject = %subject,
            "Published telemetry"
        );
        Ok(())
    }

    /// Extract headers from a NATS message into a HashMap.
    ///
    /// Used by command processing to pass headers to the bearer token
    /// validator which uses `HashMap<String, String>`.
    pub fn extract_headers(message: &async_nats::Message) -> HashMap<String, String> {
        let mut map = HashMap::new();
        if let Some(headers) = &message.headers {
            for (key, values) in headers.iter() {
                // Use the first value for each header key
                if let Some(value) = values.iter().next() {
                    map.insert(key.to_string(), value.to_string());
                }
            }
        }
        map
    }
}

/// Get the current UNIX timestamp in seconds.
fn unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock before UNIX epoch")
        .as_secs()
}
