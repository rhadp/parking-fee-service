//! MQTT client wrapper for CLOUD_GATEWAY_CLIENT.
//!
//! Manages the connection to Eclipse Mosquitto via `rumqttc`, subscribes to
//! vehicle-specific command and status request topics, and publishes the
//! registration message on startup.
//!
//! # Architecture
//!
//! The MQTT client is split into two parts:
//! - [`MqttClient`]: Owns the `AsyncClient` handle for publishing messages.
//! - [`run_event_loop`]: Drives the `rumqttc` event loop and dispatches
//!   incoming messages via a callback / channel (deferred to task group 6).
//!
//! # Requirements
//!
//! - 03-REQ-3.1: Subscribe to `vehicles/{vin}/commands` (QoS 2) and
//!   `vehicles/{vin}/status_request` (QoS 2).
//! - 03-REQ-5.2: Publish registration message on startup.

use rumqttc::{AsyncClient, EventLoop, MqttOptions, QoS};
use std::time::Duration;
use tracing::{info, warn};

use crate::messages::{
    self, RegistrationMessage, TOPIC_COMMAND_RESPONSES, TOPIC_COMMANDS, TOPIC_REGISTRATION,
    TOPIC_STATUS_REQUEST, TOPIC_STATUS_RESPONSE, TOPIC_TELEMETRY,
};
use crate::vin::VinData;

/// Wrapper around the rumqttc async MQTT client.
///
/// Provides high-level methods for publishing messages with the correct QoS
/// and topic patterns.
#[derive(Clone)]
pub struct MqttClient {
    client: AsyncClient,
    vin: String,
}

/// Creates and connects an MQTT client, subscribes to vehicle topics, and
/// publishes the initial registration message.
///
/// Returns the [`MqttClient`] handle for publishing and the [`EventLoop`]
/// that must be polled to drive the connection.
///
/// # Arguments
///
/// * `broker_addr` — MQTT broker address in `host:port` format.
/// * `vin_data` — The vehicle's VIN and pairing PIN.
///
/// # Errors
///
/// Returns an error if subscribing or publishing the registration message fails.
pub async fn connect_and_register(
    broker_addr: &str,
    vin_data: &VinData,
) -> Result<(MqttClient, EventLoop), MqttError> {
    let client_id = format!("cgc-{}", &vin_data.vin);

    let mut opts = MqttOptions::new(&client_id, parse_host(broker_addr), parse_port(broker_addr));
    opts.set_keep_alive(Duration::from_secs(30));
    opts.set_clean_session(true);
    // Cap in-flight to a reasonable number for QoS 2 handshakes.
    opts.set_inflight(10);

    let (client, event_loop) = AsyncClient::new(opts, 100);

    let mqtt = MqttClient {
        client,
        vin: vin_data.vin.clone(),
    };

    // Subscribe to inbound topics.
    let commands_topic = messages::topic_for(TOPIC_COMMANDS, &vin_data.vin);
    let status_req_topic = messages::topic_for(TOPIC_STATUS_REQUEST, &vin_data.vin);

    mqtt.client
        .subscribe(&commands_topic, QoS::ExactlyOnce)
        .await
        .map_err(|e| MqttError::Subscribe {
            topic: commands_topic.clone(),
            source: e.to_string(),
        })?;
    info!(topic = %commands_topic, "subscribed to commands (QoS 2)");

    mqtt.client
        .subscribe(&status_req_topic, QoS::ExactlyOnce)
        .await
        .map_err(|e| MqttError::Subscribe {
            topic: status_req_topic.clone(),
            source: e.to_string(),
        })?;
    info!(topic = %status_req_topic, "subscribed to status requests (QoS 2)");

    // Publish registration message.
    mqtt.publish_registration(vin_data).await?;

    Ok((mqtt, event_loop))
}

impl MqttClient {
    /// Publish the vehicle registration message to MQTT (QoS 2).
    ///
    /// This is called on every startup so CLOUD_GATEWAY learns about (or
    /// re-registers) this vehicle.
    pub async fn publish_registration(&self, vin_data: &VinData) -> Result<(), MqttError> {
        let msg = RegistrationMessage {
            vin: vin_data.vin.clone(),
            pairing_pin: vin_data.pairing_pin.clone(),
            timestamp: chrono_timestamp(),
        };

        let topic = messages::topic_for(TOPIC_REGISTRATION, &self.vin);
        let payload = serde_json::to_vec(&msg).expect("RegistrationMessage serializes to JSON");

        self.client
            .publish(&topic, QoS::ExactlyOnce, false, payload)
            .await
            .map_err(|e| MqttError::Publish {
                topic: topic.clone(),
                source: e.to_string(),
            })?;

        info!(
            vin = %vin_data.vin,
            topic = %topic,
            "published registration message"
        );
        Ok(())
    }

    /// Publish a command response to MQTT (QoS 2).
    pub async fn publish_command_response(&self, payload: &[u8]) -> Result<(), MqttError> {
        let topic = messages::topic_for(TOPIC_COMMAND_RESPONSES, &self.vin);
        self.client
            .publish(&topic, QoS::ExactlyOnce, false, payload.to_vec())
            .await
            .map_err(|e| MqttError::Publish {
                topic: topic.clone(),
                source: e.to_string(),
            })?;
        Ok(())
    }

    /// Publish a status response to MQTT (QoS 2).
    pub async fn publish_status_response(&self, payload: &[u8]) -> Result<(), MqttError> {
        let topic = messages::topic_for(TOPIC_STATUS_RESPONSE, &self.vin);
        self.client
            .publish(&topic, QoS::ExactlyOnce, false, payload.to_vec())
            .await
            .map_err(|e| MqttError::Publish {
                topic: topic.clone(),
                source: e.to_string(),
            })?;
        Ok(())
    }

    /// Publish a telemetry message to MQTT (QoS 0).
    pub async fn publish_telemetry(&self, payload: &[u8]) -> Result<(), MqttError> {
        let topic = messages::topic_for(TOPIC_TELEMETRY, &self.vin);
        self.client
            .publish(&topic, QoS::AtMostOnce, false, payload.to_vec())
            .await
            .map_err(|e| MqttError::Publish {
                topic: topic.clone(),
                source: e.to_string(),
            })?;
        Ok(())
    }

    /// Returns the VIN this client is associated with.
    pub fn vin(&self) -> &str {
        &self.vin
    }
}

/// Drive the rumqttc event loop, logging connection events.
///
/// This function runs forever (or until the connection is permanently closed).
/// Incoming publish messages are sent to `msg_tx` for processing by the
/// command handler (task group 6 will wire this up).
///
/// For now (task group 5) this simply keeps the connection alive and logs
/// events.
pub async fn run_event_loop(mut event_loop: EventLoop) {
    use rumqttc::Event;

    loop {
        match event_loop.poll().await {
            Ok(Event::Incoming(incoming)) => {
                // Log connection-level events. Publish messages will be
                // handled in task group 6 via the command handler.
                match &incoming {
                    rumqttc::Packet::ConnAck(_) => {
                        info!("MQTT connected");
                    }
                    rumqttc::Packet::SubAck(_) => {
                        // Subscription acknowledgements are expected.
                    }
                    rumqttc::Packet::Publish(p) => {
                        // Log inbound messages for now; command handling is
                        // wired in task group 6.
                        info!(topic = %p.topic, "received MQTT message (handler not yet wired)");
                    }
                    _ => {}
                }
            }
            Ok(Event::Outgoing(_)) => {
                // Outgoing events (PubAck, PubRec, etc.) — no action needed.
            }
            Err(e) => {
                warn!(error = %e, "MQTT connection error, will retry...");
                // rumqttc auto-reconnects; sleep briefly to avoid tight loop on
                // persistent failures.
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

/// Returns the current Unix timestamp in seconds.
fn chrono_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock before Unix epoch")
        .as_secs() as i64
}

/// Parse the host portion from a `host:port` address string.
fn parse_host(addr: &str) -> &str {
    addr.rsplit_once(':').map(|(h, _)| h).unwrap_or(addr)
}

/// Parse the port from a `host:port` address string, defaulting to 1883.
fn parse_port(addr: &str) -> u16 {
    addr.rsplit_once(':')
        .and_then(|(_, p)| p.parse().ok())
        .unwrap_or(1883)
}

/// Errors that can occur during MQTT operations.
#[derive(Debug)]
pub enum MqttError {
    /// Failed to subscribe to a topic.
    Subscribe { topic: String, source: String },
    /// Failed to publish a message.
    Publish { topic: String, source: String },
}

impl std::fmt::Display for MqttError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MqttError::Subscribe { topic, source } => {
                write!(f, "MQTT subscribe to '{topic}' failed: {source}")
            }
            MqttError::Publish { topic, source } => {
                write!(f, "MQTT publish to '{topic}' failed: {source}")
            }
        }
    }
}

impl std::error::Error for MqttError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_host_with_port() {
        assert_eq!(parse_host("localhost:1883"), "localhost");
        assert_eq!(parse_host("mosquitto.example.com:8883"), "mosquitto.example.com");
    }

    #[test]
    fn parse_host_without_port() {
        assert_eq!(parse_host("localhost"), "localhost");
    }

    #[test]
    fn parse_port_with_port() {
        assert_eq!(parse_port("localhost:1883"), 1883);
        assert_eq!(parse_port("mosquitto:8883"), 8883);
    }

    #[test]
    fn parse_port_without_port() {
        assert_eq!(parse_port("localhost"), 1883);
    }

    #[test]
    fn parse_port_invalid() {
        assert_eq!(parse_port("localhost:notaport"), 1883);
    }

    #[test]
    fn chrono_timestamp_is_reasonable() {
        let ts = chrono_timestamp();
        // Should be after 2020-01-01 and before 2100-01-01.
        assert!(ts > 1_577_836_800, "timestamp too small: {ts}");
        assert!(ts < 4_102_444_800, "timestamp too large: {ts}");
    }

    #[test]
    fn mqtt_error_display() {
        let err = MqttError::Subscribe {
            topic: "test/topic".to_string(),
            source: "connection refused".to_string(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("test/topic"));
        assert!(msg.contains("connection refused"));

        let err = MqttError::Publish {
            topic: "out/topic".to_string(),
            source: "timeout".to_string(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("out/topic"));
        assert!(msg.contains("timeout"));
    }

    #[test]
    fn mqtt_client_clone() {
        // MqttClient derives Clone — verify it compiles.
        fn assert_clone<T: Clone>() {}
        assert_clone::<MqttClient>();
    }
}
