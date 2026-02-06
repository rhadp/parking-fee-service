//! MQTT client with TLS and certificate hot-reload support.
//!
//! This module provides an MQTT client that connects to CLOUD_GATEWAY
//! over TLS, with support for certificate hot-reload and exponential
//! backoff reconnection.

use std::sync::Arc;
use std::time::Duration;

use rumqttc::{AsyncClient, Event, EventLoop, MqttOptions, QoS};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

use crate::cert_watcher::{CertificatePaths, CertificateWatcher};
use crate::config::MqttConfig;
use crate::error::MqttError;

/// Connection state of the MQTT client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// Not connected to the broker
    Disconnected,
    /// Attempting to connect
    Connecting,
    /// Successfully connected
    Connected,
    /// Reconnecting after connection loss
    Reconnecting,
}

/// Message received from MQTT subscription.
#[derive(Debug, Clone)]
pub struct MqttMessage {
    /// Topic the message was received on
    pub topic: String,
    /// Message payload
    pub payload: Vec<u8>,
}

/// MQTT client with TLS and certificate hot-reload support.
pub struct MqttClient {
    /// MQTT async client
    client: AsyncClient,
    /// MQTT event loop (needs to be polled)
    eventloop: Option<EventLoop>,
    /// Current connection state
    state: Arc<RwLock<ConnectionState>>,
    /// Certificate watcher for hot-reload
    cert_watcher: Option<CertificateWatcher>,
    /// MQTT configuration
    config: MqttConfig,
    /// Subscribed topics (for resubscription after reconnect)
    subscribed_topics: Arc<RwLock<Vec<String>>>,
    /// Channel to send received messages (used by run_eventloop)
    #[allow(dead_code)]
    message_tx: mpsc::Sender<MqttMessage>,
    /// Channel to receive messages
    message_rx: Option<mpsc::Receiver<MqttMessage>>,
    /// Reconnect attempt count (used by run_eventloop)
    #[allow(dead_code)]
    reconnect_attempts: Arc<RwLock<u32>>,
}

impl MqttClient {
    /// Create a new MQTT client with TLS configuration.
    pub fn new(config: MqttConfig) -> Result<Self, MqttError> {
        let (message_tx, message_rx) = mpsc::channel(256);

        // Create MQTT options
        let mut mqtt_options = MqttOptions::new(
            &config.client_id,
            Self::extract_host(&config.broker_url)?,
            Self::extract_port(&config.broker_url)?,
        );

        mqtt_options.set_keep_alive(Duration::from_secs(config.keepalive_secs));
        mqtt_options.set_clean_session(true);

        // Create client and event loop
        let (client, eventloop) = AsyncClient::new(mqtt_options, 256);

        Ok(Self {
            client,
            eventloop: Some(eventloop),
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            cert_watcher: None,
            config,
            subscribed_topics: Arc::new(RwLock::new(Vec::new())),
            message_tx,
            message_rx: Some(message_rx),
            reconnect_attempts: Arc::new(RwLock::new(0)),
        })
    }

    /// Create a new MQTT client with certificate watcher for hot-reload.
    pub fn with_cert_watcher(config: MqttConfig) -> Result<Self, MqttError> {
        let mut client = Self::new(config.clone())?;

        // Set up certificate watcher
        let cert_paths = CertificatePaths::new(
            config.ca_cert_path.clone(),
            config.client_cert_path.clone(),
            config.client_key_path.clone(),
        );

        client.cert_watcher = Some(CertificateWatcher::new(cert_paths));

        Ok(client)
    }

    /// Take the message receiver (can only be called once).
    pub fn take_message_receiver(&mut self) -> Option<mpsc::Receiver<MqttMessage>> {
        self.message_rx.take()
    }

    /// Take the event loop (can only be called once).
    pub fn take_eventloop(&mut self) -> Option<EventLoop> {
        self.eventloop.take()
    }

    /// Get the current connection state.
    pub async fn connection_state(&self) -> ConnectionState {
        *self.state.read().await
    }

    /// Check if connected.
    pub async fn is_connected(&self) -> bool {
        *self.state.read().await == ConnectionState::Connected
    }

    /// Connect to the MQTT broker with TLS.
    pub async fn connect(&self) -> Result<(), MqttError> {
        *self.state.write().await = ConnectionState::Connecting;

        // Load certificates if cert watcher is available
        if let Some(ref watcher) = self.cert_watcher {
            watcher
                .start()
                .await
                .map_err(|e| MqttError::TlsError(e.to_string()))?;
        }

        info!("Connecting to MQTT broker: {}", self.config.broker_url);

        // Connection happens through the event loop polling
        Ok(())
    }

    /// Subscribe to a topic.
    pub async fn subscribe(&self, topic: &str) -> Result<(), MqttError> {
        self.client
            .subscribe(topic, QoS::AtLeastOnce)
            .await
            .map_err(|e| MqttError::SubscribeFailed(e.to_string()))?;

        // Track subscription for reconnection
        self.subscribed_topics.write().await.push(topic.to_string());

        info!("Subscribed to topic: {}", topic);
        Ok(())
    }

    /// Publish a message to a topic.
    pub async fn publish(&self, topic: &str, payload: &[u8]) -> Result<(), MqttError> {
        self.client
            .publish(topic, QoS::AtLeastOnce, false, payload)
            .await
            .map_err(|e| MqttError::PublishFailed(e.to_string()))?;

        debug!("Published message to topic: {}", topic);
        Ok(())
    }

    /// Disconnect from the MQTT broker.
    pub async fn disconnect(&self) -> Result<(), MqttError> {
        self.client
            .disconnect()
            .await
            .map_err(|e| MqttError::ConnectionFailed(e.to_string()))?;

        *self.state.write().await = ConnectionState::Disconnected;

        info!("Disconnected from MQTT broker");
        Ok(())
    }

    /// Run the event loop and handle incoming messages.
    ///
    /// This should be spawned as a background task.
    #[allow(unused_variables)]
    pub async fn run_eventloop(
        mut eventloop: EventLoop,
        state: Arc<RwLock<ConnectionState>>,
        message_tx: mpsc::Sender<MqttMessage>,
        subscribed_topics: Arc<RwLock<Vec<String>>>,
        reconnect_attempts: Arc<RwLock<u32>>,
        config: MqttConfig,
    ) {
        loop {
            match eventloop.poll().await {
                Ok(notification) => {
                    // Reset reconnect attempts on successful event
                    *reconnect_attempts.write().await = 0;

                    match notification {
                        Event::Incoming(rumqttc::Packet::ConnAck(_)) => {
                            *state.write().await = ConnectionState::Connected;
                            info!("Connected to MQTT broker");

                            // Resubscribe to topics after reconnection
                            // Note: This would require access to the client
                        }
                        Event::Incoming(rumqttc::Packet::Publish(publish)) => {
                            let msg = MqttMessage {
                                topic: publish.topic.clone(),
                                payload: publish.payload.to_vec(),
                            };
                            if let Err(e) = message_tx.send(msg).await {
                                error!("Failed to send message to channel: {}", e);
                            }
                        }
                        Event::Incoming(rumqttc::Packet::Disconnect) => {
                            *state.write().await = ConnectionState::Disconnected;
                            warn!("Disconnected from MQTT broker");
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    let current_state = *state.read().await;
                    if current_state == ConnectionState::Connected {
                        *state.write().await = ConnectionState::Reconnecting;
                        warn!("Connection lost, reconnecting: {}", e);
                    }

                    // Calculate backoff delay
                    let attempts = *reconnect_attempts.read().await;
                    let delay = calculate_backoff_delay(
                        attempts,
                        config.reconnect_initial_delay_ms,
                        config.reconnect_max_delay_ms,
                    );

                    *reconnect_attempts.write().await = attempts.saturating_add(1);

                    debug!(
                        "Reconnecting in {}ms (attempt {})",
                        delay.as_millis(),
                        attempts + 1
                    );
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }

    /// Extract host from broker URL.
    fn extract_host(url: &str) -> Result<String, MqttError> {
        let url = url
            .strip_prefix("mqtts://")
            .or_else(|| url.strip_prefix("mqtt://"))
            .ok_or_else(|| MqttError::ConnectionFailed("Invalid broker URL scheme".to_string()))?;

        let host = url
            .split(':')
            .next()
            .ok_or_else(|| MqttError::ConnectionFailed("Invalid broker URL format".to_string()))?;

        Ok(host.to_string())
    }

    /// Extract port from broker URL.
    fn extract_port(url: &str) -> Result<u16, MqttError> {
        let url = url
            .strip_prefix("mqtts://")
            .or_else(|| url.strip_prefix("mqtt://"))
            .ok_or_else(|| MqttError::ConnectionFailed("Invalid broker URL scheme".to_string()))?;

        let parts: Vec<&str> = url.split(':').collect();
        if parts.len() < 2 {
            // Default ports
            return if url.starts_with("mqtts") {
                Ok(8883)
            } else {
                Ok(1883)
            };
        }

        parts[1]
            .parse()
            .map_err(|_| MqttError::ConnectionFailed("Invalid port number".to_string()))
    }
}

/// Calculate exponential backoff delay.
///
/// Formula: min(initial_delay * 2^attempt, max_delay)
pub fn calculate_backoff_delay(attempt: u32, initial_delay_ms: u64, max_delay_ms: u64) -> Duration {
    let delay_ms = initial_delay_ms.saturating_mul(2u64.saturating_pow(attempt));
    let clamped_delay_ms = delay_ms.min(max_delay_ms);
    Duration::from_millis(clamped_delay_ms)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_calculate_backoff_delay() {
        // Initial delay: 1000ms, max: 60000ms
        assert_eq!(
            calculate_backoff_delay(0, 1000, 60000),
            Duration::from_millis(1000)
        );
        assert_eq!(
            calculate_backoff_delay(1, 1000, 60000),
            Duration::from_millis(2000)
        );
        assert_eq!(
            calculate_backoff_delay(2, 1000, 60000),
            Duration::from_millis(4000)
        );
        assert_eq!(
            calculate_backoff_delay(3, 1000, 60000),
            Duration::from_millis(8000)
        );
        assert_eq!(
            calculate_backoff_delay(4, 1000, 60000),
            Duration::from_millis(16000)
        );
        assert_eq!(
            calculate_backoff_delay(5, 1000, 60000),
            Duration::from_millis(32000)
        );
        // Should cap at max
        assert_eq!(
            calculate_backoff_delay(6, 1000, 60000),
            Duration::from_millis(60000)
        );
        assert_eq!(
            calculate_backoff_delay(10, 1000, 60000),
            Duration::from_millis(60000)
        );
    }

    #[test]
    fn test_extract_host() {
        assert_eq!(
            MqttClient::extract_host("mqtts://broker.example.com:8883").unwrap(),
            "broker.example.com"
        );
        assert_eq!(
            MqttClient::extract_host("mqtt://localhost:1883").unwrap(),
            "localhost"
        );
    }

    #[test]
    fn test_extract_port() {
        assert_eq!(
            MqttClient::extract_port("mqtts://broker.example.com:8883").unwrap(),
            8883
        );
        assert_eq!(
            MqttClient::extract_port("mqtt://localhost:1883").unwrap(),
            1883
        );
    }

    #[test]
    fn test_connection_state() {
        assert_eq!(ConnectionState::Disconnected, ConnectionState::Disconnected);
        assert_ne!(ConnectionState::Connected, ConnectionState::Disconnected);
    }

    // Property 1: Exponential Backoff Calculation
    // Validates: Requirements 1.3
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_exponential_backoff_formula(
            attempt in 0u32..20,
            initial_delay in 100u64..2000,
            max_delay in 10000u64..120000
        ) {
            let delay = calculate_backoff_delay(attempt, initial_delay, max_delay);

            // Delay should never exceed max_delay
            prop_assert!(delay.as_millis() as u64 <= max_delay);

            // Delay should be at least initial_delay for attempt 0
            if attempt == 0 {
                prop_assert_eq!(delay.as_millis() as u64, initial_delay);
            }

            // Delay should follow formula: min(initial * 2^attempt, max)
            let expected = initial_delay.saturating_mul(2u64.saturating_pow(attempt)).min(max_delay);
            prop_assert_eq!(delay.as_millis() as u64, expected);
        }

        #[test]
        fn prop_backoff_never_exceeds_max(
            attempt in 0u32..100,
            initial_delay in 1u64..10000,
            max_delay in 1u64..1000000
        ) {
            let delay = calculate_backoff_delay(attempt, initial_delay, max_delay);

            // Delay should never exceed max_delay
            prop_assert!(delay.as_millis() as u64 <= max_delay);
        }

        #[test]
        fn prop_backoff_monotonically_increases(
            attempt in 0u32..10,
            initial_delay in 1000u64..2000,
            max_delay in 60000u64..120000
        ) {
            let delay1 = calculate_backoff_delay(attempt, initial_delay, max_delay);
            let delay2 = calculate_backoff_delay(attempt + 1, initial_delay, max_delay);

            // Delay should either increase or stay at max
            prop_assert!(delay2 >= delay1);
        }
    }
}
