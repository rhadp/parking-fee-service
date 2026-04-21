use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_nats::Subscriber;
use tracing::{error, info, warn};

use crate::config::Config;
use crate::errors::NatsError;
use crate::models::RegistrationMessage;

/// NATS client managing connection lifecycle, subscriptions, and publications.
pub struct NatsClient {
    client: async_nats::Client,
    vin: String,
}

impl NatsClient {
    /// Connect to NATS with exponential-backoff retry.
    ///
    /// Makes up to 5 connection attempts. Delays between successive attempts:
    /// 1 s, 2 s, 4 s, 8 s (four intervals for five total attempts).
    ///
    /// Returns `Err(NatsError::RetriesExhausted)` after all 5 attempts fail.
    ///
    /// See `docs/errata/04_cloud_gateway_client.md` §E1 for why the delay
    /// sequence has four entries rather than the three listed in REQ-2.2.
    pub async fn connect(config: &Config) -> Result<Self, NatsError> {
        let delays: &[Duration] = &[
            Duration::from_secs(1),
            Duration::from_secs(2),
            Duration::from_secs(4),
            Duration::from_secs(8),
        ];

        let mut last_error = String::new();
        for attempt in 0..5_usize {
            match async_nats::connect(&config.nats_url).await {
                Ok(client) => {
                    info!(url = %config.nats_url, "Connected to NATS");
                    return Ok(NatsClient {
                        client,
                        vin: config.vin.clone(),
                    });
                }
                Err(e) => {
                    last_error = e.to_string();
                    if attempt < delays.len() {
                        warn!(
                            attempt = attempt + 1,
                            max_attempts = 5,
                            delay_secs = delays[attempt].as_secs(),
                            error = %last_error,
                            "NATS connection failed, will retry"
                        );
                        tokio::time::sleep(delays[attempt]).await;
                    }
                }
            }
        }

        error!(error = %last_error, "NATS server unreachable after 5 attempts");
        Err(NatsError::RetriesExhausted)
    }

    /// Subscribe to `vehicles.{VIN}.commands` to receive inbound commands.
    ///
    /// Returns an async stream of NATS messages. Callers iterate this stream
    /// to receive individual command messages.
    ///
    /// Validates: [04-REQ-2.3]
    pub async fn subscribe_commands(&self) -> Result<Subscriber, NatsError> {
        let subject = format!("vehicles.{}.commands", self.vin);
        self.client
            .subscribe(subject.clone())
            .await
            .map_err(|e| {
                error!(subject = %subject, error = %e, "Failed to subscribe to command subject");
                NatsError::SubscribeFailed(e.to_string())
            })
    }

    /// Publish a self-registration message to `vehicles.{VIN}.status`.
    ///
    /// Fire-and-forget — no acknowledgment is awaited.
    ///
    /// Validates: [04-REQ-4.1], [04-REQ-4.2]
    pub async fn publish_registration(&self) -> Result<(), NatsError> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let msg = RegistrationMessage {
            vin: self.vin.clone(),
            status: "online".to_string(),
            timestamp,
        };

        let json = serde_json::to_string(&msg)
            .map_err(|e| NatsError::PublishFailed(e.to_string()))?;

        let subject = format!("vehicles.{}.status", self.vin);
        self.client
            .publish(subject.clone(), json.as_bytes().to_vec().into())
            .await
            .map_err(|e| {
                error!(subject = %subject, error = %e, "Failed to publish self-registration");
                NatsError::PublishFailed(e.to_string())
            })?;

        info!(subject = %subject, "Self-registration published");
        Ok(())
    }

    /// Relay a command response JSON string to `vehicles.{VIN}.command_responses`.
    ///
    /// The payload is published verbatim without modification.
    ///
    /// Validates: [04-REQ-7.1], [04-REQ-7.2]
    pub async fn publish_response(&self, json: &str) -> Result<(), NatsError> {
        let subject = format!("vehicles.{}.command_responses", self.vin);
        self.client
            .publish(subject.clone(), json.as_bytes().to_vec().into())
            .await
            .map_err(|e| {
                error!(subject = %subject, error = %e, "Failed to publish command response");
                NatsError::PublishFailed(e.to_string())
            })?;

        info!(subject = %subject, "Command response relayed");
        Ok(())
    }

    /// Publish aggregated telemetry JSON to `vehicles.{VIN}.telemetry`.
    ///
    /// Validates: [04-REQ-8.1]
    pub async fn publish_telemetry(&self, json: &str) -> Result<(), NatsError> {
        let subject = format!("vehicles.{}.telemetry", self.vin);
        self.client
            .publish(subject.clone(), json.as_bytes().to_vec().into())
            .await
            .map_err(|e| {
                error!(subject = %subject, error = %e, "Failed to publish telemetry");
                NatsError::PublishFailed(e.to_string())
            })?;

        info!(subject = %subject, "Telemetry published");
        Ok(())
    }
}
