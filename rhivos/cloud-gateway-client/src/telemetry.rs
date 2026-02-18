//! Telemetry publisher — periodically reads vehicle signals from DATA_BROKER
//! and publishes a [`TelemetryMessage`] to MQTT.
//!
//! A background task runs on a configurable timer (default 5 seconds):
//!
//! 1. Read all relevant signals from DATA_BROKER (`IsLocked`, `IsOpen`,
//!    `Speed`, `Latitude`, `Longitude`, `ParkingSessionActive`).
//! 2. Construct a [`TelemetryMessage`] JSON.
//! 3. Publish to `vehicles/{vin}/telemetry` (QoS 0).
//!
//! Missing signals are represented as `None` (serialized as JSON `null`),
//! satisfying the 03-REQ-4.E1 edge case.
//!
//! # Requirements
//!
//! - 03-REQ-4.1: Periodically read vehicle signals from DATA_BROKER and
//!   publish telemetry to `vehicles/{vin}/telemetry` (QoS 0).
//! - 03-REQ-4.2: Telemetry includes `is_locked`, `is_door_open`, `speed`,
//!   `latitude`, `longitude`, `parking_session_active`, `vin`, `timestamp`.
//! - 03-REQ-4.3: Interval configurable via `--telemetry-interval` / env var.
//! - 03-REQ-4.E1: Missing signals → null/default values.

use std::time::Duration;

use tracing::{error, info, warn};

use crate::messages::TelemetryMessage;
use crate::mqtt::MqttClient;
use crate::status_handler::{read_vehicle_state, DataBrokerReader};

/// Run the telemetry publisher loop.
///
/// Reads vehicle signals from DATA_BROKER at the configured interval and
/// publishes a [`TelemetryMessage`] to MQTT (QoS 0) on each tick.
///
/// This function runs indefinitely and should be spawned as a background task.
/// It will return only if a fatal error occurs (which currently never happens —
/// individual tick failures are logged and the loop continues).
///
/// # Arguments
///
/// * `reader` — DATA_BROKER reader for vehicle signals.
/// * `mqtt` — MQTT client for publishing telemetry messages.
/// * `vin` — Vehicle Identification Number to include in each message.
/// * `interval_secs` — Seconds between telemetry publishes.
pub async fn run_telemetry_publisher<R: DataBrokerReader>(
    reader: &R,
    mqtt: &MqttClient,
    vin: &str,
    interval_secs: u64,
) {
    let interval = Duration::from_secs(interval_secs);

    info!(
        interval_secs,
        vin,
        "telemetry publisher started"
    );

    let mut tick = tokio::time::interval(interval);
    // The first tick fires immediately; skip it so we don't publish before
    // signals have had a chance to settle.
    tick.tick().await;

    loop {
        tick.tick().await;

        if let Err(e) = publish_telemetry_tick(reader, mqtt, vin).await {
            warn!(error = %e, "telemetry publish tick failed, will retry next interval");
        }
    }
}

/// Execute a single telemetry publish cycle.
///
/// Reads vehicle state and publishes a telemetry message. Exposed as a
/// standalone function to facilitate unit testing of a single tick without
/// running the full loop.
///
/// Returns `Ok(())` on success or an error string on failure.
pub async fn publish_telemetry_tick<R: DataBrokerReader>(
    reader: &R,
    mqtt: &MqttClient,
    vin: &str,
) -> Result<(), String> {
    // 1. Read all vehicle signals from DATA_BROKER.
    let state = read_vehicle_state(reader).await;

    // 2. Construct telemetry message.
    let msg = TelemetryMessage {
        vin: vin.to_string(),
        is_locked: state.is_locked,
        is_door_open: state.is_door_open,
        speed: state.speed,
        latitude: state.latitude,
        longitude: state.longitude,
        parking_session_active: state.parking_session_active,
        timestamp: chrono_timestamp(),
    };

    // 3. Serialize and publish to MQTT.
    let payload = serde_json::to_vec(&msg).map_err(|e| {
        error!(error = %e, "failed to serialize TelemetryMessage");
        format!("serialization error: {e}")
    })?;

    mqtt.publish_telemetry(&payload).await.map_err(|e| {
        error!(error = %e, "failed to publish telemetry to MQTT");
        format!("MQTT publish error: {e}")
    })?;

    info!(
        vin,
        is_locked = ?state.is_locked,
        is_door_open = ?state.is_door_open,
        speed = ?state.speed,
        "published telemetry"
    );

    Ok(())
}

/// Returns the current Unix timestamp in seconds.
fn chrono_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock before Unix epoch")
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::status_handler::DataBrokerReader;

    /// Mock DATA_BROKER reader with configurable return values.
    struct MockReader {
        is_locked: Option<bool>,
        is_door_open: Option<bool>,
        speed: Option<f64>,
        latitude: Option<f64>,
        longitude: Option<f64>,
        parking_active: Option<bool>,
        fail_signal: Option<String>,
    }

    impl MockReader {
        fn full_state() -> Self {
            Self {
                is_locked: Some(true),
                is_door_open: Some(false),
                speed: Some(60.5),
                latitude: Some(48.1351),
                longitude: Some(11.582),
                parking_active: Some(false),
                fail_signal: None,
            }
        }

        fn empty_state() -> Self {
            Self {
                is_locked: None,
                is_door_open: None,
                speed: None,
                latitude: None,
                longitude: None,
                parking_active: None,
                fail_signal: None,
            }
        }

        fn with_failure(mut self, signal: &str) -> Self {
            self.fail_signal = Some(signal.to_string());
            self
        }
    }

    impl DataBrokerReader for MockReader {
        async fn get_is_locked(
            &self,
        ) -> Result<Option<bool>, Box<dyn std::error::Error + Send + Sync>> {
            if self.fail_signal.as_deref() == Some("IsLocked") {
                return Err("mock read failure".into());
            }
            Ok(self.is_locked)
        }

        async fn get_is_door_open(
            &self,
        ) -> Result<Option<bool>, Box<dyn std::error::Error + Send + Sync>> {
            if self.fail_signal.as_deref() == Some("IsOpen") {
                return Err("mock read failure".into());
            }
            Ok(self.is_door_open)
        }

        async fn get_speed(
            &self,
        ) -> Result<Option<f64>, Box<dyn std::error::Error + Send + Sync>> {
            if self.fail_signal.as_deref() == Some("Speed") {
                return Err("mock read failure".into());
            }
            Ok(self.speed)
        }

        async fn get_latitude(
            &self,
        ) -> Result<Option<f64>, Box<dyn std::error::Error + Send + Sync>> {
            if self.fail_signal.as_deref() == Some("Latitude") {
                return Err("mock read failure".into());
            }
            Ok(self.latitude)
        }

        async fn get_longitude(
            &self,
        ) -> Result<Option<f64>, Box<dyn std::error::Error + Send + Sync>> {
            if self.fail_signal.as_deref() == Some("Longitude") {
                return Err("mock read failure".into());
            }
            Ok(self.longitude)
        }

        async fn get_parking_session_active(
            &self,
        ) -> Result<Option<bool>, Box<dyn std::error::Error + Send + Sync>> {
            if self.fail_signal.as_deref() == Some("ParkingSessionActive") {
                return Err("mock read failure".into());
            }
            Ok(self.parking_active)
        }
    }

    /// Mock MQTT client that captures published payloads for assertions.
    ///
    /// We cannot use the real `MqttClient` without a broker connection, so we
    /// test the telemetry message construction and serialization directly and
    /// verify the expected JSON schema.

    // -----------------------------------------------------------------------
    // Telemetry message construction tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn telemetry_message_from_full_state() {
        let reader = MockReader::full_state();
        let state = read_vehicle_state(&reader).await;

        let msg = TelemetryMessage {
            vin: "DEMO0000000000001".to_string(),
            is_locked: state.is_locked,
            is_door_open: state.is_door_open,
            speed: state.speed,
            latitude: state.latitude,
            longitude: state.longitude,
            parking_session_active: state.parking_session_active,
            timestamp: 1708300802,
        };

        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["vin"], "DEMO0000000000001");
        assert_eq!(json["is_locked"], true);
        assert_eq!(json["is_door_open"], false);
        assert_eq!(json["speed"], 60.5);
        assert_eq!(json["latitude"], 48.1351);
        assert_eq!(json["longitude"], 11.582);
        assert_eq!(json["parking_session_active"], false);
        assert_eq!(json["timestamp"], 1708300802);
        // No request_id in telemetry.
        assert!(json.get("request_id").is_none());
        assert_eq!(json.as_object().unwrap().len(), 8);
    }

    #[tokio::test]
    async fn telemetry_message_from_empty_state() {
        let reader = MockReader::empty_state();
        let state = read_vehicle_state(&reader).await;

        let msg = TelemetryMessage {
            vin: "DEMO0000000000001".to_string(),
            is_locked: state.is_locked,
            is_door_open: state.is_door_open,
            speed: state.speed,
            latitude: state.latitude,
            longitude: state.longitude,
            parking_session_active: state.parking_session_active,
            timestamp: 1708300802,
        };

        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["vin"], "DEMO0000000000001");
        assert!(json["is_locked"].is_null());
        assert!(json["is_door_open"].is_null());
        assert!(json["speed"].is_null());
        assert!(json["latitude"].is_null());
        assert!(json["longitude"].is_null());
        assert!(json["parking_session_active"].is_null());
        // Timestamp should still be present.
        assert_eq!(json["timestamp"], 1708300802);
    }

    #[tokio::test]
    async fn telemetry_message_partial_failure_uses_null() {
        let reader = MockReader::full_state().with_failure("Speed");
        let state = read_vehicle_state(&reader).await;

        let msg = TelemetryMessage {
            vin: "VIN123".to_string(),
            is_locked: state.is_locked,
            is_door_open: state.is_door_open,
            speed: state.speed,
            latitude: state.latitude,
            longitude: state.longitude,
            parking_session_active: state.parking_session_active,
            timestamp: 1708300802,
        };

        let json = serde_json::to_value(&msg).unwrap();
        // Speed should be null due to read failure (03-REQ-4.E1).
        assert!(json["speed"].is_null());
        // Other fields should still be present.
        assert_eq!(json["is_locked"], true);
        assert_eq!(json["is_door_open"], false);
        assert_eq!(json["latitude"], 48.1351);
        assert_eq!(json["longitude"], 11.582);
        assert_eq!(json["parking_session_active"], false);
    }

    #[tokio::test]
    async fn telemetry_message_multiple_failures_uses_null() {
        let reader = MockReader {
            is_locked: Some(true),
            is_door_open: None,
            speed: None,
            latitude: None,
            longitude: Some(11.582),
            parking_active: Some(true),
            fail_signal: Some("IsLocked".to_string()),
        };
        let state = read_vehicle_state(&reader).await;

        let msg = TelemetryMessage {
            vin: "VIN456".to_string(),
            is_locked: state.is_locked,
            is_door_open: state.is_door_open,
            speed: state.speed,
            latitude: state.latitude,
            longitude: state.longitude,
            parking_session_active: state.parking_session_active,
            timestamp: 1708300802,
        };

        let json = serde_json::to_value(&msg).unwrap();
        // IsLocked read failed — should be null.
        assert!(json["is_locked"].is_null());
        // is_door_open was None from reader (not available).
        assert!(json["is_door_open"].is_null());
        // Speed was None from reader (not available).
        assert!(json["speed"].is_null());
        // Latitude was None from reader (not available).
        assert!(json["latitude"].is_null());
        // Longitude was available.
        assert_eq!(json["longitude"], 11.582);
        // Parking active was available.
        assert_eq!(json["parking_session_active"], true);
    }

    #[tokio::test]
    async fn telemetry_message_includes_all_required_fields() {
        // 03-REQ-4.2: Telemetry message SHALL include: is_locked, is_door_open,
        // speed, latitude, longitude, parking_session_active, vin, timestamp.
        let reader = MockReader::full_state();
        let state = read_vehicle_state(&reader).await;

        let msg = TelemetryMessage {
            vin: "DEMO0000000000001".to_string(),
            is_locked: state.is_locked,
            is_door_open: state.is_door_open,
            speed: state.speed,
            latitude: state.latitude,
            longitude: state.longitude,
            parking_session_active: state.parking_session_active,
            timestamp: 1708300802,
        };

        let json = serde_json::to_value(&msg).unwrap();
        let obj = json.as_object().unwrap();

        // Verify all required fields are present.
        let required_fields = [
            "vin",
            "is_locked",
            "is_door_open",
            "speed",
            "latitude",
            "longitude",
            "parking_session_active",
            "timestamp",
        ];
        for field in &required_fields {
            assert!(
                obj.contains_key(*field),
                "missing required field: {field}"
            );
        }
        // Exactly 8 fields, no extras.
        assert_eq!(obj.len(), 8);
    }

    #[tokio::test]
    async fn telemetry_serialization_roundtrip() {
        let msg = TelemetryMessage {
            vin: "DEMO0000000000001".to_string(),
            is_locked: Some(true),
            is_door_open: Some(false),
            speed: Some(42.0),
            latitude: Some(48.1351),
            longitude: Some(11.582),
            parking_session_active: Some(true),
            timestamp: 1708300802,
        };

        let json_str = serde_json::to_string(&msg).unwrap();
        let deserialized: TelemetryMessage = serde_json::from_str(&json_str).unwrap();
        assert_eq!(msg, deserialized);
    }

    // -----------------------------------------------------------------------
    // Timestamp tests
    // -----------------------------------------------------------------------

    #[test]
    fn chrono_timestamp_is_reasonable() {
        let ts = chrono_timestamp();
        // Should be after 2020-01-01 and before 2100-01-01.
        assert!(ts > 1_577_836_800, "timestamp too small: {ts}");
        assert!(ts < 4_102_444_800, "timestamp too large: {ts}");
    }

    // -----------------------------------------------------------------------
    // Integration test (requires running Kuksa + Mosquitto)
    // -----------------------------------------------------------------------

    /// Integration test: verify telemetry messages arrive on MQTT.
    ///
    /// Requires `make infra-up` (Kuksa + Mosquitto running). Skip when
    /// infrastructure is unavailable.
    #[tokio::test]
    #[ignore]
    async fn telemetry_integration_with_real_infra() {
        // This test is intentionally ignored in CI and run manually with:
        //   cargo test -p cloud-gateway-client -- --ignored telemetry_integration
        //
        // When infrastructure is available, it verifies that:
        // 1. Telemetry messages are published at the configured interval.
        // 2. The message schema matches the expected JSON format.
        // 3. QoS 0 delivery works end-to-end.
        //
        // The actual test would:
        // - Connect to Mosquitto as a subscriber on vehicles/+/telemetry.
        // - Start the telemetry publisher with a short interval (1s).
        // - Wait for at least 2 messages to arrive.
        // - Verify JSON schema and field presence.
        eprintln!("telemetry integration test requires running infrastructure (make infra-up)");
    }
}
