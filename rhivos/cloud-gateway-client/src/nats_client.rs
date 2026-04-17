//! NATS client module for CLOUD_GATEWAY_CLIENT.
//!
//! Manages the NATS connection lifecycle with exponential-backoff retry,
//! subscribes to the command subject, and publishes responses, telemetry,
//! and self-registration messages.
//!
//! Validates [04-REQ-2.1], [04-REQ-2.2], [04-REQ-2.3], [04-REQ-2.E1],
//!           [04-REQ-4.1], [04-REQ-4.2], [04-REQ-7.1], [04-REQ-8.1]

#![allow(dead_code)]

use std::time::{SystemTime, UNIX_EPOCH};

use async_nats::{Client, Subscriber};
use tracing::{error, info, warn};

use crate::config::Config;
use crate::errors::NatsError;
use crate::models::RegistrationMessage;

/// Maximum number of connection attempts (initial + 4 retries = 5 total).
const MAX_ATTEMPTS: u32 = 5;

/// Exponential-backoff delays in seconds between successive attempts.
/// Attempt 1 at t=0, wait 1s, attempt 2, wait 2s, ..., wait 8s, attempt 5.
const BACKOFF_DELAYS_SECS: [u64; 4] = [1, 2, 4, 8];

/// Wraps an `async_nats::Client` together with the VIN used to scope subjects.
///
/// `Clone` is derived because `async_nats::Client` is cheaply cloneable (it
/// wraps a shared connection internally).  This lets callers hand out per-task
/// copies without an extra `Arc`.
#[derive(Clone)]
pub struct NatsClient {
    client: Client,
    vin: String,
}

impl NatsClient {
    /// Connect to the NATS server at the URL specified in `config`.
    ///
    /// Retries with exponential backoff (1 s, 2 s, 4 s, 8 s) for up to 5 total
    /// attempts.  Returns `Err(NatsError::RetriesExhausted)` when all attempts
    /// are exhausted.
    ///
    /// Validates [04-REQ-2.1], [04-REQ-2.2], [04-REQ-2.E1]
    pub async fn connect(config: &Config) -> Result<Self, NatsError> {
        for attempt in 0..MAX_ATTEMPTS {
            if attempt > 0 {
                let delay = BACKOFF_DELAYS_SECS[(attempt - 1) as usize];
                warn!(
                    attempt,
                    delay_secs = delay,
                    "NATS connection attempt failed, retrying after backoff"
                );
                tokio::time::sleep(tokio::time::Duration::from_secs(delay)).await;
            }

            match async_nats::connect(&config.nats_url).await {
                Ok(client) => {
                    info!(url = %config.nats_url, "Connected to NATS");
                    return Ok(Self {
                        client,
                        vin: config.vin.clone(),
                    });
                }
                Err(e) => {
                    error!(
                        attempt = attempt + 1,
                        max_attempts = MAX_ATTEMPTS,
                        error = %e,
                        "NATS connection attempt failed"
                    );
                }
            }
        }

        error!(
            url = %config.nats_url,
            "NATS server unreachable after all retry attempts"
        );
        Err(NatsError::RetriesExhausted)
    }

    /// Subscribe to `vehicles.{VIN}.commands` to receive incoming lock/unlock commands.
    ///
    /// Validates [04-REQ-2.3]
    pub async fn subscribe_commands(&self) -> Result<Subscriber, NatsError> {
        let subject = format!("vehicles.{}.commands", self.vin);
        self.client
            .subscribe(subject.clone())
            .await
            .map_err(|e| {
                error!(subject = %subject, error = %e, "Failed to subscribe to commands subject");
                NatsError::SubscribeFailed(e.to_string())
            })
    }

    /// Publish a self-registration message to `vehicles.{VIN}.status`.
    ///
    /// Fire-and-forget; does not wait for acknowledgment.
    ///
    /// Validates [04-REQ-4.1], [04-REQ-4.2]
    pub async fn publish_registration(&self) -> Result<(), NatsError> {
        let subject = format!("vehicles.{}.status", self.vin);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let msg = RegistrationMessage {
            vin: self.vin.clone(),
            status: "online".to_string(),
            timestamp,
        };

        let payload = serde_json::to_string(&msg)
            .expect("RegistrationMessage serialization must not fail");

        self.client
            .publish(subject.clone(), payload.into())
            .await
            .map_err(|e| {
                error!(subject = %subject, error = %e, "Failed to publish registration message");
                NatsError::PublishFailed(e.to_string())
            })?;

        info!(vin = %self.vin, "Self-registration published");
        Ok(())
    }

    /// Publish a command response JSON string to `vehicles.{VIN}.command_responses`.
    ///
    /// Validates [04-REQ-7.1]
    pub async fn publish_response(&self, json: &str) -> Result<(), NatsError> {
        let subject = format!("vehicles.{}.command_responses", self.vin);
        self.client
            .publish(subject.clone(), json.to_owned().into())
            .await
            .map_err(|e| {
                error!(subject = %subject, error = %e, "Failed to publish command response");
                NatsError::PublishFailed(e.to_string())
            })?;

        info!(subject = %subject, "Command response relayed");
        Ok(())
    }

    /// Publish an aggregated telemetry JSON string to `vehicles.{VIN}.telemetry`.
    ///
    /// Validates [04-REQ-8.1]
    pub async fn publish_telemetry(&self, json: &str) -> Result<(), NatsError> {
        let subject = format!("vehicles.{}.telemetry", self.vin);
        self.client
            .publish(subject.clone(), json.to_owned().into())
            .await
            .map_err(|e| {
                error!(subject = %subject, error = %e, "Failed to publish telemetry");
                NatsError::PublishFailed(e.to_string())
            })?;

        info!(subject = %subject, "Telemetry message published");
        Ok(())
    }
}
