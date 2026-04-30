use std::time::{SystemTime, UNIX_EPOCH};

use async_nats::Client;
use tokio::time::{sleep, Duration};

use crate::config::Config;
use crate::errors::NatsError;
use crate::models::RegistrationMessage;

/// Client for NATS messaging operations.
///
/// Wraps an `async_nats::Client` and provides domain-specific methods
/// for command subscription, response/telemetry publishing, and
/// self-registration.
pub struct NatsClient {
    client: Client,
    vin: String,
}

impl NatsClient {
    /// Connect to the NATS server with exponential backoff retry.
    ///
    /// Retries up to 5 attempts with delays of 1s, 2s, 4s, 8s between
    /// successive failures. Returns `Err(NatsError::RetriesExhausted)` if
    /// all attempts fail.
    ///
    /// # Requirements
    /// - 04-REQ-2.1: Connect to NATS at configured URL
    /// - 04-REQ-2.2: Retry with exponential backoff, up to 5 attempts
    /// - 04-REQ-2.E1: Exit with error after retries exhausted
    pub async fn connect(config: &Config) -> Result<Self, NatsError> {
        const MAX_ATTEMPTS: u32 = 5;
        let backoff_delays = [1, 2, 4, 8]; // seconds between retries

        for attempt in 0..MAX_ATTEMPTS {
            match async_nats::connect(&config.nats_url).await {
                Ok(client) => {
                    tracing::info!(
                        url = %config.nats_url,
                        attempt = attempt + 1,
                        "Connected to NATS"
                    );
                    return Ok(Self {
                        client,
                        vin: config.vin.clone(),
                    });
                }
                Err(err) => {
                    tracing::error!(
                        url = %config.nats_url,
                        attempt = attempt + 1,
                        max_attempts = MAX_ATTEMPTS,
                        error = %err,
                        "NATS connection failed"
                    );

                    // Sleep before next retry (except after the last attempt)
                    if attempt < MAX_ATTEMPTS - 1 {
                        let delay = backoff_delays[attempt as usize];
                        tracing::info!(
                            delay_secs = delay,
                            "Retrying NATS connection"
                        );
                        sleep(Duration::from_secs(delay)).await;
                    }
                }
            }
        }

        tracing::error!(
            url = %config.nats_url,
            "NATS connection retries exhausted after {} attempts",
            MAX_ATTEMPTS
        );
        Err(NatsError::RetriesExhausted)
    }

    /// Subscribe to the command subject for this vehicle.
    ///
    /// Subscribes to `vehicles.{VIN}.commands` and returns a `Subscriber`
    /// that yields incoming NATS messages.
    ///
    /// # Requirements
    /// - 04-REQ-2.3: Subscribe to `vehicles.{VIN}.commands`
    pub async fn subscribe_commands(
        &self,
    ) -> Result<async_nats::Subscriber, NatsError> {
        let subject = format!("vehicles.{}.commands", self.vin);
        let subscriber = self
            .client
            .subscribe(subject.clone())
            .await
            .map_err(|e| NatsError::SubscribeFailed(e.to_string()))?;

        tracing::info!(subject = %subject, "Subscribed to commands");
        Ok(subscriber)
    }

    /// Publish a self-registration message to NATS.
    ///
    /// Publishes to `vehicles.{VIN}.status` with the vehicle's VIN,
    /// status "online", and current timestamp. Fire-and-forget: does
    /// not wait for acknowledgment.
    ///
    /// # Requirements
    /// - 04-REQ-4.1: Publish registration to `vehicles.{VIN}.status`
    /// - 04-REQ-4.2: Fire-and-forget, no acknowledgment
    pub async fn publish_registration(&self) -> Result<(), NatsError> {
        let subject = format!("vehicles.{}.status", self.vin);

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System clock is before UNIX epoch")
            .as_secs();

        let msg = RegistrationMessage {
            vin: self.vin.clone(),
            status: "online".to_string(),
            timestamp,
        };

        let payload = serde_json::to_string(&msg)
            .expect("RegistrationMessage serialization should not fail");

        self.client
            .publish(subject.clone(), payload.into())
            .await
            .map_err(|e| NatsError::PublishFailed(e.to_string()))?;

        tracing::info!(
            subject = %subject,
            vin = %self.vin,
            "Self-registration published"
        );
        Ok(())
    }

    /// Publish a command response to NATS.
    ///
    /// Publishes the JSON string verbatim to `vehicles.{VIN}.command_responses`.
    ///
    /// # Requirements
    /// - 04-REQ-7.1: Publish response to `vehicles.{VIN}.command_responses`
    pub async fn publish_response(&self, json: &str) -> Result<(), NatsError> {
        let subject = format!("vehicles.{}.command_responses", self.vin);

        self.client
            .publish(subject.clone(), json.to_string().into())
            .await
            .map_err(|e| NatsError::PublishFailed(e.to_string()))?;

        tracing::info!(
            subject = %subject,
            "Command response relayed"
        );
        Ok(())
    }

    /// Publish a telemetry message to NATS.
    ///
    /// Publishes the aggregated telemetry JSON to `vehicles.{VIN}.telemetry`.
    ///
    /// # Requirements
    /// - 04-REQ-8.1: Publish telemetry on signal change
    pub async fn publish_telemetry(&self, json: &str) -> Result<(), NatsError> {
        let subject = format!("vehicles.{}.telemetry", self.vin);

        self.client
            .publish(subject.clone(), json.to_string().into())
            .await
            .map_err(|e| NatsError::PublishFailed(e.to_string()))?;

        tracing::info!(
            subject = %subject,
            "Telemetry message published"
        );
        Ok(())
    }

    /// Returns a reference to the VIN this client is configured for.
    pub fn vin(&self) -> &str {
        &self.vin
    }
}
