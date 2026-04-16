//! NATS client for CLOUD_GATEWAY_CLIENT.
//!
//! Manages connection lifecycle with exponential-backoff retry, subscribes to
//! the commands subject, and publishes registration, response, and telemetry
//! messages to the cloud.

use std::time::Duration;

use tracing::{error, info, warn};

use crate::config::Config;
use crate::errors::NatsError;
use crate::models::RegistrationMessage;

/// Maximum NATS connection attempts (1 initial + 4 retries = 5 total).
///
/// Wait intervals: 1s, 2s, 4s, 8s  →  cumulative: t=0, t+1s, t+3s, t+7s, t+15s.
/// This satisfies [04-REQ-2.2] and TS-04-15.
const MAX_ATTEMPTS: u32 = 5;

/// Base backoff delay in milliseconds (doubles each retry).
const BASE_BACKOFF_MS: u64 = 1_000;

/// NATS client wrapper for CLOUD_GATEWAY_CLIENT.
///
/// Holds a live `async_nats::Client` and the VIN used to build subject paths.
/// All publish/subscribe operations use VIN-scoped subjects per the spec:
/// - Commands:          `vehicles.{VIN}.commands`
/// - Status:            `vehicles.{VIN}.status`
/// - Command responses: `vehicles.{VIN}.command_responses`
/// - Telemetry:         `vehicles.{VIN}.telemetry`
pub struct NatsClient {
    client: async_nats::Client,
    vin: String,
}

impl NatsClient {
    /// Connect to the NATS server at the URL specified in `config`.
    ///
    /// Uses exponential-backoff retry:
    /// - Attempt 1: immediate (t=0)
    /// - Attempt 2: after 1 s  (t~1 s)
    /// - Attempt 3: after 2 s  (t~3 s)
    /// - Attempt 4: after 4 s  (t~7 s)
    /// - Attempt 5: after 8 s  (t~15 s)
    ///
    /// Returns `Err(NatsError::RetriesExhausted)` after all 5 attempts fail,
    /// which the caller should treat as a fatal error (exit with code 1).
    ///
    /// Satisfies: [04-REQ-2.1], [04-REQ-2.2], [04-REQ-2.E1]
    pub async fn connect(config: &Config) -> Result<Self, NatsError> {
        let url = &config.nats_url;

        for attempt in 0..MAX_ATTEMPTS {
            if attempt > 0 {
                // delay_ms: 1000, 2000, 4000, 8000 for attempts 1..4
                let delay_ms = BASE_BACKOFF_MS * (1u64 << (attempt - 1));
                warn!(
                    attempt,
                    delay_ms, "NATS connection failed; retrying with exponential backoff"
                );
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            }

            match async_nats::connect(url.as_str()).await {
                Ok(client) => {
                    info!(url = %url, "Connected to NATS");
                    return Ok(NatsClient {
                        client,
                        vin: config.vin.clone(),
                    });
                }
                Err(e) => {
                    warn!(
                        attempt,
                        url = %url,
                        error = %e,
                        "Failed to connect to NATS"
                    );
                }
            }
        }

        error!(
            url = %url,
            attempts = MAX_ATTEMPTS,
            "NATS server unreachable after all retry attempts"
        );
        Err(NatsError::RetriesExhausted)
    }

    /// Subscribe to `vehicles.{VIN}.commands` and return the subscriber.
    ///
    /// The caller should iterate the returned `Subscriber` with `.next().await`
    /// (or `StreamExt::next`) to receive incoming command messages.
    ///
    /// Satisfies: [04-REQ-2.3]
    pub async fn subscribe_commands(&self) -> Result<async_nats::Subscriber, NatsError> {
        let subject = format!("vehicles.{}.commands", self.vin);
        info!(subject = %subject, "Subscribing to NATS commands subject");

        self.client
            .subscribe(subject.clone())
            .await
            .map_err(|e| {
                error!(
                    subject = %subject,
                    error = %e,
                    "Failed to subscribe to NATS commands subject"
                );
                NatsError::SubscribeFailed(e.to_string())
            })
    }

    /// Publish a self-registration message to `vehicles.{VIN}.status`.
    ///
    /// Fire-and-forget per [04-REQ-4.2]: the method does not wait for
    /// an acknowledgment beyond the NATS publish confirmation.
    ///
    /// Satisfies: [04-REQ-4.1], [04-REQ-4.2]
    pub async fn publish_registration(&self) -> Result<(), NatsError> {
        let subject = format!("vehicles.{}.status", self.vin);
        let msg = RegistrationMessage::new(self.vin.clone());
        let payload = serde_json::to_string(&msg)
            .map_err(|e| NatsError::PublishFailed(format!("Failed to serialize registration: {e}")))?;

        self.client
            .publish(subject.clone(), payload.into_bytes().into())
            .await
            .map_err(|e| {
                error!(
                    subject = %subject,
                    vin = %self.vin,
                    error = %e,
                    "Failed to publish self-registration to NATS"
                );
                NatsError::PublishFailed(e.to_string())
            })?;

        info!(vin = %self.vin, subject = %subject, "Self-registration published to NATS");
        Ok(())
    }

    /// Publish a command response JSON string to `vehicles.{VIN}.command_responses`.
    ///
    /// The `json` string is published verbatim — no modification is applied
    /// (Property 4: Response Relay Fidelity).
    ///
    /// Satisfies: [04-REQ-7.1]
    pub async fn publish_response(&self, json: &str) -> Result<(), NatsError> {
        let subject = format!("vehicles.{}.command_responses", self.vin);

        self.client
            .publish(subject.clone(), json.as_bytes().to_vec().into())
            .await
            .map_err(|e| {
                error!(
                    subject = %subject,
                    vin = %self.vin,
                    error = %e,
                    "Failed to publish command response to NATS"
                );
                NatsError::PublishFailed(e.to_string())
            })?;

        info!(vin = %self.vin, subject = %subject, "Command response relayed to NATS");
        Ok(())
    }

    /// Publish an aggregated telemetry JSON string to `vehicles.{VIN}.telemetry`.
    ///
    /// The `json` string is published as produced by `TelemetryState::update()`.
    ///
    /// Satisfies: [04-REQ-8.1]
    pub async fn publish_telemetry(&self, json: &str) -> Result<(), NatsError> {
        let subject = format!("vehicles.{}.telemetry", self.vin);

        self.client
            .publish(subject.clone(), json.as_bytes().to_vec().into())
            .await
            .map_err(|e| {
                error!(
                    subject = %subject,
                    vin = %self.vin,
                    error = %e,
                    "Failed to publish telemetry to NATS"
                );
                NatsError::PublishFailed(e.to_string())
            })?;

        info!(vin = %self.vin, subject = %subject, "Telemetry published to NATS");
        Ok(())
    }
}
