use async_nats::Client;
use tracing::{error, info, warn};

use crate::config::Config;

/// Manages the NATS connection and provides typed subscribe/publish methods.
#[derive(Clone)]
pub struct NatsClient {
    client: Client,
    vin: String,
}

impl NatsClient {
    /// Connect to the NATS server using the provided configuration.
    ///
    /// Uses async-nats built-in reconnection mechanism. When `NATS_TLS_ENABLED`
    /// is true, connects with TLS via rustls (requires the server to support TLS).
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
                        error!("NATS server error: {}", err);
                    }
                    _ => {
                        info!("NATS event: {:?}", event);
                    }
                }
            });

        let client = if config.nats_tls_enabled {
            info!(
                "Connecting to NATS at {} with TLS enabled",
                config.nats_url
            );
            connect_options
                .require_tls(true)
                .connect(&config.nats_url)
                .await?
        } else {
            info!(
                "Connecting to NATS at {} (plain, no TLS)",
                config.nats_url
            );
            connect_options.connect(&config.nats_url).await?
        };

        info!("NATS client connected successfully");

        Ok(NatsClient {
            client,
            vin: config.vin.clone(),
        })
    }

    /// Returns the NATS subject for commands: `vehicles.{VIN}.commands`.
    pub fn commands_subject(&self) -> String {
        format!("vehicles.{}.commands", self.vin)
    }

    /// Returns the NATS subject for command responses: `vehicles.{VIN}.command_responses`.
    pub fn command_responses_subject(&self) -> String {
        format!("vehicles.{}.command_responses", self.vin)
    }

    /// Returns the NATS subject for telemetry: `vehicles.{VIN}.telemetry`.
    pub fn telemetry_subject(&self) -> String {
        format!("vehicles.{}.telemetry", self.vin)
    }

    /// Subscribe to the command subject for this VIN.
    pub async fn subscribe_commands(
        &self,
    ) -> Result<async_nats::Subscriber, async_nats::SubscribeError> {
        let subject = self.commands_subject();
        info!("Subscribing to NATS subject: {}", subject);
        self.client.subscribe(subject).await
    }

    /// Publish a message to the given NATS subject.
    pub async fn publish(
        &self,
        subject: &str,
        payload: bytes::Bytes,
    ) -> Result<(), async_nats::PublishError> {
        self.client.publish(subject.to_string(), payload).await
    }

    /// Publish a command response to the command_responses subject.
    pub async fn publish_command_response(
        &self,
        payload: bytes::Bytes,
    ) -> Result<(), async_nats::PublishError> {
        let subject = self.command_responses_subject();
        self.client.publish(subject, payload).await
    }

    /// Publish a telemetry message to the telemetry subject.
    pub async fn publish_telemetry(
        &self,
        payload: bytes::Bytes,
    ) -> Result<(), async_nats::PublishError> {
        let subject = self.telemetry_subject();
        self.client.publish(subject, payload).await
    }

    /// Returns the VIN configured for this client.
    pub fn vin(&self) -> &str {
        &self.vin
    }
}
