//! MQTT client wrapper for CLOUD_GATEWAY_CLIENT.
//!
//! Provides connection management with automatic reconnection and
//! exponential backoff. Wraps the rumqttc async client to handle
//! subscriptions and publishing to the MQTT broker.

use rumqttc::{AsyncClient, Event, EventLoop, MqttOptions, Packet, QoS};
use tracing::{debug, error, info, warn};

/// Default MQTT broker address.
pub const DEFAULT_MQTT_BROKER: &str = "localhost";

/// Default MQTT broker port.
pub const DEFAULT_MQTT_PORT: u16 = 1883;

/// Default client ID for the CLOUD_GATEWAY_CLIENT.
const CLIENT_ID: &str = "cloud-gateway-client";

/// MQTT client wrapper with topic management.
pub struct MqttClient {
    /// The async MQTT client for publishing.
    client: AsyncClient,
    /// The event loop that drives MQTT communication.
    eventloop: EventLoop,
    /// The VIN used for topic construction.
    vin: String,
}

impl MqttClient {
    /// Create a new MQTT client and connect to the broker.
    ///
    /// The client subscribes to `vehicles/{vin}/commands` on connection.
    ///
    /// # Arguments
    ///
    /// * `broker_host` - MQTT broker hostname.
    /// * `broker_port` - MQTT broker port.
    /// * `vin` - Vehicle identification number for topic construction.
    pub fn new(broker_host: &str, broker_port: u16, vin: &str) -> Self {
        let mut mqttoptions = MqttOptions::new(CLIENT_ID, broker_host, broker_port);
        mqttoptions.set_keep_alive(std::time::Duration::from_secs(30));
        // Set clean session so we get a fresh state on reconnect
        mqttoptions.set_clean_session(true);

        let (client, eventloop) = AsyncClient::new(mqttoptions, 64);

        Self {
            client,
            eventloop,
            vin: vin.to_string(),
        }
    }

    /// Get a clone of the async MQTT client for publishing.
    pub fn client(&self) -> AsyncClient {
        self.client.clone()
    }

    /// Get the command topic for this vehicle.
    pub fn command_topic(&self) -> String {
        format!("vehicles/{}/commands", self.vin)
    }

    /// Get the command response topic for this vehicle.
    pub fn response_topic(&self) -> String {
        format!("vehicles/{}/command_responses", self.vin)
    }

    /// Get the telemetry topic for this vehicle.
    pub fn telemetry_topic(&self) -> String {
        format!("vehicles/{}/telemetry", self.vin)
    }

    /// Subscribe to the commands topic.
    pub async fn subscribe_commands(&self) -> Result<(), rumqttc::ClientError> {
        let topic = self.command_topic();
        info!(topic = %topic, "subscribing to command topic");
        self.client.subscribe(&topic, QoS::AtLeastOnce).await
    }

    /// Publish a message to the command responses topic.
    pub async fn publish_response(&self, payload: &str) -> Result<(), rumqttc::ClientError> {
        let topic = self.response_topic();
        debug!(topic = %topic, "publishing command response");
        self.client
            .publish(&topic, QoS::AtLeastOnce, false, payload.as_bytes())
            .await
    }

    /// Publish a message to the telemetry topic.
    pub async fn publish_telemetry(&self, payload: &str) -> Result<(), rumqttc::ClientError> {
        let topic = self.telemetry_topic();
        debug!(topic = %topic, "publishing telemetry");
        self.client
            .publish(&topic, QoS::AtLeastOnce, false, payload.as_bytes())
            .await
    }

    /// Run the MQTT event loop, calling the handler for each incoming message.
    ///
    /// This method handles reconnection with logging. It runs indefinitely
    /// until the event loop is terminated.
    ///
    /// The `on_message` callback is invoked for each message received on
    /// any subscribed topic. It receives the topic and payload as arguments.
    pub async fn run<F>(&mut self, mut on_message: F)
    where
        F: FnMut(String, Vec<u8>),
    {
        let mut connected = false;
        let mut retry_count: u32 = 0;
        let command_topic = self.command_topic();

        loop {
            match self.eventloop.poll().await {
                Ok(event) => {
                    match &event {
                        Event::Incoming(Packet::ConnAck(_)) => {
                            info!("connected to MQTT broker");
                            connected = true;
                            retry_count = 0;

                            // Re-subscribe on (re)connection using the client
                            // directly (avoids borrowing &self which contains
                            // the !Sync EventLoop)
                            info!(topic = %command_topic, "subscribing to command topic");
                            if let Err(e) = self
                                .client
                                .subscribe(&command_topic, QoS::AtLeastOnce)
                                .await
                            {
                                error!(error = %e, "failed to subscribe to commands topic");
                            }
                        }
                        Event::Incoming(Packet::Publish(publish)) => {
                            debug!(
                                topic = %publish.topic,
                                payload_len = publish.payload.len(),
                                "received MQTT message"
                            );
                            on_message(publish.topic.clone(), publish.payload.to_vec());
                        }
                        Event::Incoming(Packet::SubAck(_)) => {
                            debug!("subscription acknowledged");
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    if connected {
                        warn!(error = %e, "MQTT connection lost, will retry");
                        connected = false;
                    } else {
                        retry_count = retry_count.saturating_add(1);
                        let backoff = std::cmp::min(1u64 << retry_count.min(5), 32);
                        warn!(
                            error = %e,
                            retry = retry_count,
                            backoff_secs = backoff,
                            "MQTT connection failed, retrying"
                        );
                        tokio::time::sleep(std::time::Duration::from_secs(backoff)).await;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topic_construction() {
        let client = MqttClient::new("localhost", 1883, "VIN12345");
        assert_eq!(client.command_topic(), "vehicles/VIN12345/commands");
        assert_eq!(
            client.response_topic(),
            "vehicles/VIN12345/command_responses"
        );
        assert_eq!(client.telemetry_topic(), "vehicles/VIN12345/telemetry");
    }

    #[test]
    fn test_topic_construction_custom_vin() {
        let client = MqttClient::new("localhost", 1883, "ABC987");
        assert_eq!(client.command_topic(), "vehicles/ABC987/commands");
        assert_eq!(
            client.response_topic(),
            "vehicles/ABC987/command_responses"
        );
        assert_eq!(client.telemetry_topic(), "vehicles/ABC987/telemetry");
    }
}
