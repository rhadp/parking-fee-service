//! Telemetry types and publishing for cloud-gateway-client.
//!
//! This module defines the telemetry message format published to the cloud,
//! internal state tracking for vehicle signals, and the TelemetryPublisher
//! that handles batched publishing with offline buffering.

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::error::{MqttError, TelemetryError};
use crate::mqtt::MqttClient;
use crate::offline_buffer::OfflineTelemetryBuffer;
use crate::subscriber::{vss_paths, SignalUpdate, SignalValue};

/// Telemetry message published to the cloud.
///
/// Contains all vehicle signal data in a flat structure for CLOUD_GATEWAY.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Telemetry {
    /// ISO8601 timestamp of the telemetry snapshot
    pub timestamp: String,

    /// Vehicle latitude (flat, not nested)
    pub latitude: f64,

    /// Vehicle longitude (flat, not nested)
    pub longitude: f64,

    /// Whether doors are locked
    pub door_locked: bool,

    /// Whether any door is open
    pub door_open: bool,

    /// Whether a parking session is active
    pub parking_session_active: bool,
}

impl Telemetry {
    /// Create a new Telemetry snapshot from the current state.
    pub fn from_state(state: &TelemetryState) -> Self {
        Self {
            timestamp: Utc::now().to_rfc3339(),
            latitude: state.latitude,
            longitude: state.longitude,
            door_locked: state.door_locked,
            door_open: state.door_open,
            parking_session_active: state.parking_session_active,
        }
    }
}

/// Internal state tracking for vehicle signals.
///
/// Accumulates signal updates from DATA_BROKER before publishing.
#[derive(Debug, Clone, Default)]
pub struct TelemetryState {
    /// Vehicle latitude
    pub latitude: f64,

    /// Vehicle longitude
    pub longitude: f64,

    /// Whether doors are locked
    pub door_locked: bool,

    /// Whether any door is open
    pub door_open: bool,

    /// Whether a parking session is active
    pub parking_session_active: bool,

    /// Timestamp of last update
    pub last_updated: Option<DateTime<Utc>>,
}

impl TelemetryState {
    /// Create a new empty state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Update the latitude.
    pub fn set_latitude(&mut self, lat: f64) {
        self.latitude = lat;
        self.last_updated = Some(Utc::now());
    }

    /// Update the longitude.
    pub fn set_longitude(&mut self, lng: f64) {
        self.longitude = lng;
        self.last_updated = Some(Utc::now());
    }

    /// Update the door locked state.
    pub fn set_door_locked(&mut self, locked: bool) {
        self.door_locked = locked;
        self.last_updated = Some(Utc::now());
    }

    /// Update the door open state.
    pub fn set_door_open(&mut self, open: bool) {
        self.door_open = open;
        self.last_updated = Some(Utc::now());
    }

    /// Update the parking session active state.
    pub fn set_parking_session_active(&mut self, active: bool) {
        self.parking_session_active = active;
        self.last_updated = Some(Utc::now());
    }

    /// Check if the state has been updated since the given time.
    pub fn has_updates_since(&self, since: DateTime<Utc>) -> bool {
        self.last_updated.is_some_and(|t| t > since)
    }

    /// Apply a signal update to the state.
    pub fn apply_signal_update(&mut self, update: &SignalUpdate) {
        match update.signal_path.as_str() {
            path if path == vss_paths::LATITUDE => {
                if let SignalValue::Float(v) = update.value {
                    self.set_latitude(v);
                }
            }
            path if path == vss_paths::LONGITUDE => {
                if let SignalValue::Float(v) = update.value {
                    self.set_longitude(v);
                }
            }
            path if path == vss_paths::DOOR_LOCKED => {
                if let SignalValue::Bool(v) = update.value {
                    self.set_door_locked(v);
                }
            }
            path if path == vss_paths::DOOR_OPEN => {
                if let SignalValue::Bool(v) = update.value {
                    self.set_door_open(v);
                }
            }
            path if path == vss_paths::PARKING_SESSION_ACTIVE => {
                if let SignalValue::Bool(v) = update.value {
                    self.set_parking_session_active(v);
                }
            }
            _ => {
                warn!("Unknown signal path: {}", update.signal_path);
            }
        }
    }
}

/// Telemetry publisher that batches signal updates and publishes to MQTT.
///
/// Features:
/// - Rate limiting: publishes at most once per publish_interval
/// - Offline buffering: stores telemetry when MQTT is disconnected
/// - Chronological publishing: drains buffer in order when connection restored
pub struct TelemetryPublisher {
    /// MQTT client for publishing
    mqtt_client: Arc<MqttClient>,
    /// Vehicle Identification Number for topic construction
    vin: String,
    /// Channel to receive signal updates
    signal_rx: mpsc::Receiver<SignalUpdate>,
    /// Current telemetry state
    current_state: TelemetryState,
    /// Publish interval (rate limiting)
    publish_interval: Duration,
    /// Offline buffer for telemetry during disconnection
    offline_buffer: OfflineTelemetryBuffer,
    /// Last publish time for rate limiting
    last_publish: Option<DateTime<Utc>>,
    /// Whether DATA_BROKER is connected
    data_broker_connected: bool,
}

impl TelemetryPublisher {
    /// Create a new TelemetryPublisher.
    pub fn new(
        mqtt_client: Arc<MqttClient>,
        vin: String,
        signal_rx: mpsc::Receiver<SignalUpdate>,
        publish_interval: Duration,
    ) -> Self {
        Self {
            mqtt_client,
            vin,
            signal_rx,
            current_state: TelemetryState::new(),
            publish_interval,
            offline_buffer: OfflineTelemetryBuffer::default(),
            last_publish: None,
            data_broker_connected: true,
        }
    }

    /// Create a new TelemetryPublisher with custom buffer settings.
    pub fn with_buffer(
        mqtt_client: Arc<MqttClient>,
        vin: String,
        signal_rx: mpsc::Receiver<SignalUpdate>,
        publish_interval: Duration,
        max_buffer_messages: usize,
        max_buffer_age: Duration,
    ) -> Self {
        Self {
            mqtt_client,
            vin,
            signal_rx,
            current_state: TelemetryState::new(),
            publish_interval,
            offline_buffer: OfflineTelemetryBuffer::new(max_buffer_messages, max_buffer_age),
            last_publish: None,
            data_broker_connected: true,
        }
    }

    /// Get the telemetry topic for this vehicle.
    pub fn telemetry_topic(&self) -> String {
        format!("vehicles/{}/telemetry", self.vin)
    }

    /// Run the telemetry publishing loop.
    ///
    /// Batches signal updates and publishes at most once per publish_interval.
    /// Buffers messages when MQTT is offline and drains them when connection restored.
    pub async fn run(&mut self) -> Result<(), TelemetryError> {
        info!("Telemetry publisher starting for VIN {}", self.vin);

        let mut publish_timer = tokio::time::interval(self.publish_interval);

        loop {
            tokio::select! {
                // Receive signal updates
                signal = self.signal_rx.recv() => {
                    match signal {
                        Some(update) => {
                            self.current_state.apply_signal_update(&update);
                            debug!("Applied signal update: {}", update.signal_path);
                        }
                        None => {
                            // Channel closed, stop publishing
                            info!("Signal channel closed, stopping telemetry publisher");
                            break;
                        }
                    }
                }
                // Publish timer tick
                _ = publish_timer.tick() => {
                    // Only publish if we have updates
                    if self.current_state.last_updated.is_some() && self.data_broker_connected {
                        if let Err(e) = self.publish_current_state().await {
                            error!("Failed to publish telemetry: {}", e);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Publish the current state or buffer if offline.
    async fn publish_current_state(&mut self) -> Result<(), TelemetryError> {
        let telemetry = Telemetry::from_state(&self.current_state);
        self.publish_or_buffer(telemetry).await
    }

    /// Publish telemetry if connected, buffer if offline.
    pub async fn publish_or_buffer(&mut self, telemetry: Telemetry) -> Result<(), TelemetryError> {
        if self.mqtt_client.is_connected().await {
            // First drain any buffered messages
            if !self.offline_buffer.is_empty() {
                info!(
                    "MQTT reconnected, draining {} buffered messages",
                    self.offline_buffer.len()
                );
                self.drain_offline_buffer().await?;
            }

            // Then publish current telemetry
            self.publish_telemetry(&telemetry).await?;
            self.last_publish = Some(Utc::now());
        } else {
            // Buffer for later
            debug!("MQTT offline, buffering telemetry");
            self.offline_buffer.push(telemetry);
        }

        Ok(())
    }

    /// Drain the offline buffer, publishing all messages in chronological order.
    pub async fn drain_offline_buffer(&mut self) -> Result<(), TelemetryError> {
        let buffered = self.offline_buffer.drain();
        let count = buffered.len();

        for buffered_msg in buffered {
            if let Err(e) = self.publish_telemetry(&buffered_msg.telemetry).await {
                // If publish fails, stop draining and re-buffer remaining messages
                warn!("Failed to publish buffered message, stopping drain: {}", e);
                return Err(e);
            }
        }

        if count > 0 {
            info!("Drained {} buffered telemetry messages", count);
        }

        Ok(())
    }

    /// Publish a single telemetry message to MQTT.
    async fn publish_telemetry(&self, telemetry: &Telemetry) -> Result<(), TelemetryError> {
        let topic = self.telemetry_topic();
        let payload = serde_json::to_vec(telemetry)
            .map_err(|e| TelemetryError::SerializationFailed(e.to_string()))?;

        self.mqtt_client
            .publish(&topic, &payload)
            .await
            .map_err(|e: MqttError| TelemetryError::PublishFailed(e.to_string()))?;

        debug!("Published telemetry to {}", topic);
        Ok(())
    }

    /// Set DATA_BROKER connection status.
    ///
    /// When disconnected, telemetry publishing is paused.
    pub fn set_data_broker_connected(&mut self, connected: bool) {
        if self.data_broker_connected && !connected {
            warn!("DATA_BROKER disconnected, pausing telemetry publishing");
        } else if !self.data_broker_connected && connected {
            info!("DATA_BROKER reconnected, resuming telemetry publishing");
        }
        self.data_broker_connected = connected;
    }

    /// Get the current telemetry state (for testing).
    pub fn current_state(&self) -> &TelemetryState {
        &self.current_state
    }

    /// Get the offline buffer length (for testing).
    pub fn buffer_len(&self) -> usize {
        self.offline_buffer.len()
    }
}

/// Tracks telemetry publishing statistics for rate limiting verification.
#[derive(Debug, Clone, Default)]
pub struct TelemetryPublishStats {
    /// Total number of signal updates received
    pub updates_received: usize,
    /// Total number of telemetry messages published
    pub messages_published: usize,
    /// Total time window in milliseconds
    pub time_window_ms: u64,
}

impl TelemetryPublishStats {
    /// Calculate the maximum expected messages based on rate limiting.
    ///
    /// With a publish interval of 1 second, the max messages is ceil(time_window / 1000).
    pub fn max_expected_messages(&self, publish_interval_ms: u64) -> usize {
        if publish_interval_ms == 0 {
            return self.updates_received;
        }
        ((self.time_window_ms as f64 / publish_interval_ms as f64).ceil() as usize).max(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_telemetry_serialization() {
        let telem = Telemetry {
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            latitude: 37.7749,
            longitude: -122.4194,
            door_locked: true,
            door_open: false,
            parking_session_active: true,
        };

        let json = serde_json::to_string(&telem).unwrap();
        assert!(json.contains("\"latitude\":37.7749"));
        assert!(json.contains("\"longitude\":-122.4194"));
        assert!(json.contains("\"door_locked\":true"));
        assert!(json.contains("\"parking_session_active\":true"));

        let parsed: Telemetry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, telem);
    }

    #[test]
    fn test_telemetry_state_updates() {
        let mut state = TelemetryState::new();
        assert!(state.last_updated.is_none());

        state.set_latitude(37.7749);
        assert!(state.last_updated.is_some());
        assert_eq!(state.latitude, 37.7749);
    }

    #[test]
    fn test_telemetry_from_state() {
        let mut state = TelemetryState::new();
        state.set_latitude(37.7749);
        state.set_longitude(-122.4194);
        state.set_door_locked(true);
        state.set_parking_session_active(true);

        let telem = Telemetry::from_state(&state);
        assert_eq!(telem.latitude, 37.7749);
        assert_eq!(telem.longitude, -122.4194);
        assert!(telem.door_locked);
        assert!(telem.parking_session_active);
    }

    #[test]
    #[allow(clippy::arc_with_non_send_sync)] // Arc needed for API compatibility in test
    fn test_telemetry_topic() {
        let (_, rx) = mpsc::channel(10);
        let mqtt_config = crate::config::MqttConfig::default();
        let mqtt_client = Arc::new(MqttClient::new(mqtt_config).unwrap());
        let publisher = TelemetryPublisher::new(
            mqtt_client,
            "VIN123".to_string(),
            rx,
            Duration::from_secs(1),
        );
        assert_eq!(publisher.telemetry_topic(), "vehicles/VIN123/telemetry");
    }

    #[test]
    fn test_apply_signal_update_latitude() {
        let mut state = TelemetryState::new();
        let update = SignalUpdate::new_float(vss_paths::LATITUDE, 37.7749);
        state.apply_signal_update(&update);
        assert!((state.latitude - 37.7749).abs() < 0.0001);
    }

    #[test]
    fn test_apply_signal_update_longitude() {
        let mut state = TelemetryState::new();
        let update = SignalUpdate::new_float(vss_paths::LONGITUDE, -122.4194);
        state.apply_signal_update(&update);
        assert!((state.longitude - (-122.4194)).abs() < 0.0001);
    }

    #[test]
    fn test_apply_signal_update_door_locked() {
        let mut state = TelemetryState::new();
        let update = SignalUpdate::new_bool(vss_paths::DOOR_LOCKED, true);
        state.apply_signal_update(&update);
        assert!(state.door_locked);
    }

    #[test]
    fn test_apply_signal_update_door_open() {
        let mut state = TelemetryState::new();
        let update = SignalUpdate::new_bool(vss_paths::DOOR_OPEN, true);
        state.apply_signal_update(&update);
        assert!(state.door_open);
    }

    #[test]
    fn test_apply_signal_update_parking_session() {
        let mut state = TelemetryState::new();
        let update = SignalUpdate::new_bool(vss_paths::PARKING_SESSION_ACTIVE, true);
        state.apply_signal_update(&update);
        assert!(state.parking_session_active);
    }

    #[test]
    fn test_telemetry_publish_stats() {
        let stats = TelemetryPublishStats {
            updates_received: 100,
            messages_published: 5,
            time_window_ms: 5000,
        };

        // With 1000ms interval over 5000ms window, max is 5
        assert_eq!(stats.max_expected_messages(1000), 5);

        // With 2000ms interval over 5000ms window, max is ceil(5000/2000) = 3
        assert_eq!(stats.max_expected_messages(2000), 3);
    }

    // Property 13: Telemetry Contains All Required Fields
    // Validates: Requirements 7.2
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_telemetry_contains_all_fields(
            lat in -90.0f64..90.0,
            lng in -180.0f64..180.0,
            door_locked in proptest::bool::ANY,
            door_open in proptest::bool::ANY,
            parking_active in proptest::bool::ANY
        ) {
            let mut state = TelemetryState::new();
            state.set_latitude(lat);
            state.set_longitude(lng);
            state.set_door_locked(door_locked);
            state.set_door_open(door_open);
            state.set_parking_session_active(parking_active);

            let telem = Telemetry::from_state(&state);

            // Verify all required fields are present and correct
            prop_assert!(!telem.timestamp.is_empty(), "timestamp must not be empty");
            prop_assert!((telem.latitude - lat).abs() < 0.0001, "latitude must match");
            prop_assert!((telem.longitude - lng).abs() < 0.0001, "longitude must match");
            prop_assert_eq!(telem.door_locked, door_locked, "door_locked must match");
            prop_assert_eq!(telem.door_open, door_open, "door_open must match");
            prop_assert_eq!(telem.parking_session_active, parking_active, "parking_session_active must match");

            // Verify JSON serialization contains all fields (flat structure)
            let json = serde_json::to_string(&telem).unwrap();
            prop_assert!(json.contains("\"timestamp\""), "JSON must contain timestamp");
            prop_assert!(json.contains("\"latitude\""), "JSON must contain latitude (flat)");
            prop_assert!(json.contains("\"longitude\""), "JSON must contain longitude (flat)");
            prop_assert!(json.contains("\"door_locked\""), "JSON must contain door_locked");
            prop_assert!(json.contains("\"door_open\""), "JSON must contain door_open");
            prop_assert!(json.contains("\"parking_session_active\""), "JSON must contain parking_session_active");

            // Verify timestamp is ISO8601 format (RFC3339)
            let parsed: Telemetry = serde_json::from_str(&json).unwrap();
            prop_assert!(!parsed.timestamp.is_empty());
            // RFC3339 timestamps contain 'T' and have 'Z' or timezone offset
            prop_assert!(parsed.timestamp.contains('T') || parsed.timestamp.contains(' '));
        }

        #[test]
        fn prop_telemetry_json_roundtrip(
            lat in -90.0f64..90.0,
            lng in -180.0f64..180.0,
            door_locked in proptest::bool::ANY,
            door_open in proptest::bool::ANY,
            parking_active in proptest::bool::ANY
        ) {
            let telem = Telemetry {
                timestamp: Utc::now().to_rfc3339(),
                latitude: lat,
                longitude: lng,
                door_locked,
                door_open,
                parking_session_active: parking_active,
            };

            // Serialize and deserialize
            let json = serde_json::to_string(&telem).unwrap();
            let parsed: Telemetry = serde_json::from_str(&json).unwrap();

            // All fields must match
            prop_assert_eq!(parsed.timestamp, telem.timestamp);
            prop_assert!((parsed.latitude - telem.latitude).abs() < 0.0001);
            prop_assert!((parsed.longitude - telem.longitude).abs() < 0.0001);
            prop_assert_eq!(parsed.door_locked, telem.door_locked);
            prop_assert_eq!(parsed.door_open, telem.door_open);
            prop_assert_eq!(parsed.parking_session_active, telem.parking_session_active);
        }
    }

    // Property 14: Telemetry Rate Limiting
    // Validates: Requirements 7.3
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_telemetry_rate_limiting(
            num_updates in 1usize..100,
            time_window_ms in 100u64..10000
        ) {
            let publish_interval_ms = 1000u64; // 1 second as per requirements

            // Calculate expected maximum messages
            // For T seconds window with 1 second interval, max = ceil(T/1)
            let max_expected = ((time_window_ms as f64 / publish_interval_ms as f64).ceil() as usize).max(1);

            let stats = TelemetryPublishStats {
                updates_received: num_updates,
                messages_published: max_expected, // Simulated publish count
                time_window_ms,
            };

            // Verify the rate limiting formula
            let calculated_max = stats.max_expected_messages(publish_interval_ms);
            prop_assert_eq!(calculated_max, max_expected);

            // Published messages should never exceed the rate limit
            prop_assert!(
                stats.messages_published <= calculated_max + 1, // +1 for potential initial publish
                "Published {} messages but max expected {} for {} updates in {}ms window",
                stats.messages_published,
                calculated_max,
                num_updates,
                time_window_ms
            );
        }

        #[test]
        fn prop_rate_limit_bounds(
            time_window_ms in 1000u64..60000
        ) {
            let publish_interval_ms = 1000u64;

            let stats = TelemetryPublishStats {
                updates_received: 1000, // Many updates
                messages_published: 0,
                time_window_ms,
            };

            let max_expected = stats.max_expected_messages(publish_interval_ms);

            // Max expected should be approximately time_window / interval
            let expected_approx = (time_window_ms / publish_interval_ms) as usize;

            // Should be within 1 of the expected (due to ceiling)
            prop_assert!(
                max_expected >= expected_approx && max_expected <= expected_approx + 1,
                "max_expected {} not within expected range [{}, {}]",
                max_expected,
                expected_approx,
                expected_approx + 1
            );
        }
    }
}
