//! NATS client module for cloud-gateway-client.
//!
//! Manages the NATS connection lifecycle: connect with exponential backoff retry,
//! subscribe to commands, and publish responses/telemetry/status messages.
//! Encapsulates all async-nats API usage.

use crate::config::Config;
use crate::errors::NatsError;
use crate::models::RegistrationMessage;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};

/// NATS client wrapping the async-nats connection.
///
/// Stores the underlying connection and the VIN used to construct subject paths.
pub struct NatsClient {
    client: async_nats::Client,
    vin: String,
}

impl NatsClient {
    /// Connect to the NATS server at the configured URL with exponential backoff retry.
    ///
    /// Attempts connection up to 5 times. Waits 1s, 2s, 4s, 8s between successive
    /// attempts. Returns [`NatsError::RetriesExhausted`] if all 5 attempts fail.
    ///
    /// Implements: [04-REQ-2.1], [04-REQ-2.2], [04-REQ-2.E1]
    pub async fn connect(config: &Config) -> Result<Self, NatsError> {
        const MAX_ATTEMPTS: u32 = 5;
        // Delays (seconds) between attempt N and attempt N+1.
        // 4 delays cover the gaps between 5 attempts.
        const DELAYS_SECS: [u64; 4] = [1, 2, 4, 8];

        for attempt in 1..=MAX_ATTEMPTS {
            match async_nats::connect(&config.nats_url).await {
                Ok(client) => {
                    info!("Connected to NATS at {}", config.nats_url);
                    return Ok(NatsClient {
                        client,
                        vin: config.vin.clone(),
                    });
                }
                Err(e) => {
                    if attempt == MAX_ATTEMPTS {
                        error!(
                            "NATS server at {} is unreachable after {} attempts: {}",
                            config.nats_url, MAX_ATTEMPTS, e
                        );
                    } else {
                        let delay = DELAYS_SECS[(attempt - 1) as usize];
                        warn!(
                            "NATS connection attempt {}/{} failed ({}). Retrying in {}s...",
                            attempt, MAX_ATTEMPTS, e, delay
                        );
                        sleep(Duration::from_secs(delay)).await;
                    }
                }
            }
        }

        Err(NatsError::RetriesExhausted)
    }

    /// Subscribe to `vehicles.{VIN}.commands` to receive incoming commands.
    ///
    /// Returns an [`async_nats::Subscriber`] for the caller to iterate over.
    ///
    /// Implements: [04-REQ-2.3]
    pub async fn subscribe_commands(&self) -> Result<async_nats::Subscriber, NatsError> {
        let subject = format!("vehicles.{}.commands", self.vin);
        match self.client.subscribe(subject.clone()).await {
            Ok(subscriber) => {
                info!("Subscribed to NATS subject: {}", subject);
                Ok(subscriber)
            }
            Err(e) => {
                error!("Failed to subscribe to {}: {}", subject, e);
                Err(NatsError::SubscribeFailed(e.to_string()))
            }
        }
    }

    /// Publish a self-registration message to `vehicles.{VIN}.status`.
    ///
    /// The message format is `{"vin":"<vin>","status":"online","timestamp":<unix_ts>}`.
    /// This is fire-and-forget; no acknowledgment is awaited.
    ///
    /// Implements: [04-REQ-4.1], [04-REQ-4.2]
    pub async fn publish_registration(&self) -> Result<(), NatsError> {
        let subject = format!("vehicles.{}.status", self.vin);

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let msg = RegistrationMessage {
            vin: self.vin.clone(),
            status: "online".to_string(),
            timestamp,
        };

        let json = serde_json::to_string(&msg).map_err(|e| {
            error!("Failed to serialize registration message: {}", e);
            NatsError::PublishFailed(e.to_string())
        })?;

        match self.client.publish(subject.clone(), json.into()).await {
            Ok(()) => {
                info!("Self-registration published to {}", subject);
                Ok(())
            }
            Err(e) => {
                error!("Failed to publish registration to {}: {}", subject, e);
                Err(NatsError::PublishFailed(e.to_string()))
            }
        }
    }

    /// Publish a command response JSON verbatim to `vehicles.{VIN}.command_responses`.
    ///
    /// The `json` payload is published as-is without modification.
    ///
    /// Implements: [04-REQ-7.1]
    pub async fn publish_response(&self, json: &str) -> Result<(), NatsError> {
        let subject = format!("vehicles.{}.command_responses", self.vin);

        match self
            .client
            .publish(subject.clone(), json.to_owned().into())
            .await
        {
            Ok(()) => {
                info!("Command response relayed to {}", subject);
                Ok(())
            }
            Err(e) => {
                error!("Failed to publish command response to {}: {}", subject, e);
                Err(NatsError::PublishFailed(e.to_string()))
            }
        }
    }

    /// Publish an aggregated telemetry JSON to `vehicles.{VIN}.telemetry`.
    ///
    /// The `json` payload is published as-is without modification.
    ///
    /// Implements: [04-REQ-8.1]
    pub async fn publish_telemetry(&self, json: &str) -> Result<(), NatsError> {
        let subject = format!("vehicles.{}.telemetry", self.vin);

        match self
            .client
            .publish(subject.clone(), json.to_owned().into())
            .await
        {
            Ok(()) => {
                info!("Telemetry message published to {}", subject);
                Ok(())
            }
            Err(e) => {
                error!("Failed to publish telemetry to {}: {}", subject, e);
                Err(NatsError::PublishFailed(e.to_string()))
            }
        }
    }
}
