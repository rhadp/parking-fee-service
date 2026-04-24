use std::time::{SystemTime, UNIX_EPOCH};

use async_nats::Client;
use tokio::time::{sleep, Duration};

use crate::config::Config;
use crate::errors::NatsError;
use crate::models::RegistrationMessage;

/// NATS subject prefix template: `vehicles.{VIN}`.
fn subject_prefix(vin: &str) -> String {
    format!("vehicles.{vin}")
}

/// Inter-attempt delays for NATS connection retries.
///
/// With 5 total connection attempts, there are 4 inter-attempt delays.
/// The exponential backoff sequence is 1s, 2s, 4s, 8s.
///
/// Requirement [04-REQ-2.2]: retry with exponential backoff for up to 5 attempts.
const RETRY_DELAYS_SECS: [u64; 4] = [1, 2, 4, 8];

/// Maximum number of connection attempts (initial + retries).
const MAX_ATTEMPTS: usize = 5;

/// Manages the NATS connection lifecycle and message publishing/subscribing.
///
/// Encapsulates all `async-nats` API usage and provides typed methods
/// for each NATS operation required by the CLOUD_GATEWAY_CLIENT.
pub struct NatsClient {
    client: Client,
    vin: String,
}

impl NatsClient {
    /// Connects to the NATS server with exponential backoff retry.
    ///
    /// Attempts to connect up to [`MAX_ATTEMPTS`] times. On failure, waits
    /// according to [`RETRY_DELAYS_SECS`] before retrying.
    ///
    /// # Errors
    ///
    /// Returns [`NatsError::RetriesExhausted`] if all attempts fail.
    ///
    /// # Requirements
    ///
    /// - [04-REQ-2.1]: Connect to NATS at the configured URL.
    /// - [04-REQ-2.2]: Retry with exponential backoff (1s, 2s, 4s, 8s), max 5 attempts.
    /// - [04-REQ-2.E1]: Exit with code 1 when retries are exhausted.
    pub async fn connect(config: &Config) -> Result<Self, NatsError> {
        let url = &config.nats_url;
        let mut last_error = String::new();

        // Build the sequence of optional delays: [Some(1), Some(2), Some(4), Some(8), None].
        // Each entry corresponds to one attempt; the delay (if any) is applied after failure
        // before the next attempt. The last attempt has no delay because there is no next try.
        let attempts: Vec<Option<u64>> = RETRY_DELAYS_SECS
            .iter()
            .map(|&d| Some(d))
            .chain(std::iter::once(None))
            .collect();

        for (idx, delay_after) in attempts.iter().enumerate() {
            let attempt_num = idx + 1;
            tracing::info!(attempt = attempt_num, url, "attempting NATS connection");

            match async_nats::connect(url).await {
                Ok(client) => {
                    tracing::info!(url, "connected to NATS");
                    return Ok(Self {
                        client,
                        vin: config.vin.clone(),
                    });
                }
                Err(e) => {
                    last_error = e.to_string();
                    tracing::error!(
                        attempt = attempt_num,
                        max_attempts = MAX_ATTEMPTS,
                        error = %e,
                        "NATS connection failed"
                    );

                    if let Some(delay_secs) = delay_after {
                        tracing::info!(
                            delay_secs,
                            "waiting before next NATS connection attempt"
                        );
                        sleep(Duration::from_secs(*delay_secs)).await;
                    }
                }
            }
        }

        tracing::error!(
            url,
            attempts = MAX_ATTEMPTS,
            last_error,
            "NATS server unreachable after all retry attempts"
        );
        Err(NatsError::RetriesExhausted)
    }

    /// Subscribes to the command subject `vehicles.{VIN}.commands`.
    ///
    /// Returns a [`async_nats::Subscriber`] that yields incoming command messages.
    ///
    /// # Errors
    ///
    /// Returns [`NatsError::SubscribeFailed`] if the subscription cannot be created.
    ///
    /// # Requirements
    ///
    /// - [04-REQ-2.3]: Subscribe to `vehicles.{VIN}.commands`.
    pub async fn subscribe_commands(&self) -> Result<async_nats::Subscriber, NatsError> {
        let subject = format!("{}.commands", subject_prefix(&self.vin));
        tracing::info!(subject, "subscribing to command subject");

        self.client
            .subscribe(subject.clone())
            .await
            .map_err(|e| {
                tracing::error!(subject, error = %e, "failed to subscribe to commands");
                NatsError::SubscribeFailed(e.to_string())
            })
    }

    /// Publishes a self-registration message to `vehicles.{VIN}.status`.
    ///
    /// The registration message is fire-and-forget; the service does not wait
    /// for an acknowledgment ([04-REQ-4.2]).
    ///
    /// # Errors
    ///
    /// Returns [`NatsError::PublishFailed`] if the publish operation fails.
    ///
    /// # Requirements
    ///
    /// - [04-REQ-4.1]: Publish registration with `{"vin":"<vin>","status":"online","timestamp":<unix_ts>}`.
    /// - [04-REQ-4.2]: Fire-and-forget; no acknowledgment expected.
    pub async fn publish_registration(&self) -> Result<(), NatsError> {
        let subject = format!("{}.status", subject_prefix(&self.vin));

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let msg = RegistrationMessage {
            vin: self.vin.clone(),
            status: "online".to_string(),
            timestamp,
        };

        let payload = serde_json::to_string(&msg).map_err(|e| {
            tracing::error!(error = %e, "failed to serialize registration message");
            NatsError::PublishFailed(e.to_string())
        })?;

        tracing::info!(subject, vin = %self.vin, "publishing self-registration");

        self.client
            .publish(subject.clone(), payload.into())
            .await
            .map_err(|e| {
                tracing::error!(subject, error = %e, "failed to publish registration message");
                NatsError::PublishFailed(e.to_string())
            })?;

        tracing::info!(subject, "self-registration published");
        Ok(())
    }

    /// Publishes a command response to `vehicles.{VIN}.command_responses`.
    ///
    /// The JSON payload is published verbatim (no modification).
    ///
    /// # Errors
    ///
    /// Returns [`NatsError::PublishFailed`] if the publish operation fails.
    ///
    /// # Requirements
    ///
    /// - [04-REQ-7.1]: Publish command response verbatim to NATS.
    pub async fn publish_response(&self, json: &str) -> Result<(), NatsError> {
        let subject = format!("{}.command_responses", subject_prefix(&self.vin));

        tracing::info!(subject, "publishing command response");

        self.client
            .publish(subject.clone(), json.to_owned().into())
            .await
            .map_err(|e| {
                tracing::error!(subject, error = %e, "failed to publish command response");
                NatsError::PublishFailed(e.to_string())
            })?;

        tracing::info!(subject, "command response relayed");
        Ok(())
    }

    /// Publishes a telemetry message to `vehicles.{VIN}.telemetry`.
    ///
    /// # Errors
    ///
    /// Returns [`NatsError::PublishFailed`] if the publish operation fails.
    ///
    /// # Requirements
    ///
    /// - [04-REQ-8.1]: Publish aggregated telemetry on signal change.
    pub async fn publish_telemetry(&self, json: &str) -> Result<(), NatsError> {
        let subject = format!("{}.telemetry", subject_prefix(&self.vin));

        tracing::info!(subject, "publishing telemetry");

        self.client
            .publish(subject.clone(), json.to_owned().into())
            .await
            .map_err(|e| {
                tracing::error!(subject, error = %e, "failed to publish telemetry");
                NatsError::PublishFailed(e.to_string())
            })?;

        tracing::info!(subject, "telemetry message published");
        Ok(())
    }

    /// Returns the VIN associated with this client.
    pub fn vin(&self) -> &str {
        &self.vin
    }
}
