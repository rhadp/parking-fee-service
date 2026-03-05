//! NATS connection management for the CLOUD_GATEWAY_CLIENT.
//!
//! Handles connecting to NATS (with optional TLS), subscribing to subjects,
//! and publishing messages. Leverages async-nats built-in reconnection.

use async_nats::Client;
use tracing::{error, info, warn};

use crate::config::Config;

/// Wrapper around the async-nats client with VIN-scoped subject helpers.
#[derive(Clone)]
pub struct NatsClient {
    client: Client,
    vin: String,
}

impl NatsClient {
    /// Connect to the NATS server using the provided configuration.
    ///
    /// When `nats_tls_enabled` is true, connects with TLS via rustls.
    /// Leverages async-nats built-in reconnection on connection loss.
    pub async fn connect(config: &Config) -> Result<Self, async_nats::ConnectError> {
        let connect_options = async_nats::ConnectOptions::new()
            .event_callback(|event| async move {
                match event {
                    async_nats::Event::Connected => {
                        info!("NATS connected");
                    }
                    async_nats::Event::Disconnected => {
                        warn!("NATS disconnected, will attempt reconnection");
                    }
                    async_nats::Event::ServerError(err) => {
                        error!("NATS server error: {err}");
                    }
                    _ => {
                        info!("NATS event: {event}");
                    }
                }
            });

        let client = if config.nats_tls_enabled {
            info!(url = %config.nats_url, "Connecting to NATS with TLS");
            connect_options
                .require_tls(true)
                .connect(&config.nats_url)
                .await?
        } else {
            info!(url = %config.nats_url, "Connecting to NATS (plain)");
            connect_options.connect(&config.nats_url).await?
        };

        info!(vin = %config.vin, "NATS client connected for VIN");

        Ok(Self {
            client,
            vin: config.vin.clone(),
        })
    }

    /// Subscribe to the VIN-scoped command subject: `vehicles.{VIN}.commands`.
    pub async fn subscribe_commands(
        &self,
    ) -> Result<async_nats::Subscriber, async_nats::SubscribeError> {
        let subject = self.command_subject();
        info!(subject = %subject, "Subscribing to command subject");
        self.client.subscribe(subject).await
    }

    /// Publish a message to the VIN-scoped command responses subject.
    pub async fn publish_command_response(
        &self,
        payload: bytes::Bytes,
    ) -> Result<(), async_nats::PublishError> {
        let subject = self.command_response_subject();
        self.client.publish(subject, payload).await
    }

    /// Publish a message to the VIN-scoped telemetry subject.
    pub async fn publish_telemetry(
        &self,
        payload: bytes::Bytes,
    ) -> Result<(), async_nats::PublishError> {
        let subject = self.telemetry_subject();
        self.client.publish(subject, payload).await
    }

    /// Returns the command subject for this VIN.
    pub fn command_subject(&self) -> String {
        format!("vehicles.{}.commands", self.vin)
    }

    /// Returns the command response subject for this VIN.
    pub fn command_response_subject(&self) -> String {
        format!("vehicles.{}.command_responses", self.vin)
    }

    /// Returns the telemetry subject for this VIN.
    pub fn telemetry_subject(&self) -> String {
        format!("vehicles.{}.telemetry", self.vin)
    }

    /// Returns the VIN this client is configured for.
    pub fn vin(&self) -> &str {
        &self.vin
    }

    /// Returns a reference to the underlying async-nats client.
    pub fn inner(&self) -> &Client {
        &self.client
    }
}
