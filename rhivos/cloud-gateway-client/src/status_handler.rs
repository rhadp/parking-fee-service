//! Status request handler — reads vehicle state from DATA_BROKER and
//! publishes a [`StatusResponse`] to MQTT.
//!
//! When CLOUD_GATEWAY sends a status request via MQTT, this module reads
//! all relevant vehicle signals from DATA_BROKER (locked state, door state,
//! speed, location, parking session) and responds with a `StatusResponse`.
//!
//! Missing signals are represented as `None` (serialized as JSON `null`).
//!
//! # Requirements
//!
//! - 03-REQ-3.5: On status request via MQTT, read DATA_BROKER and publish
//!   status response to `vehicles/{vin}/status_response` (QoS 2).

use tracing::{error, info, warn};

use crate::messages::StatusRequest;
use crate::mqtt::MqttClient;

/// Trait abstracting DATA_BROKER read operations for testability.
#[allow(async_fn_in_trait)]
pub trait DataBrokerReader: Send + Sync {
    /// Read `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked`.
    async fn get_is_locked(&self) -> Result<Option<bool>, Box<dyn std::error::Error + Send + Sync>>;

    /// Read `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen`.
    async fn get_is_door_open(&self) -> Result<Option<bool>, Box<dyn std::error::Error + Send + Sync>>;

    /// Read `Vehicle.Speed`.
    async fn get_speed(&self) -> Result<Option<f64>, Box<dyn std::error::Error + Send + Sync>>;

    /// Read `Vehicle.CurrentLocation.Latitude`.
    async fn get_latitude(&self) -> Result<Option<f64>, Box<dyn std::error::Error + Send + Sync>>;

    /// Read `Vehicle.CurrentLocation.Longitude`.
    async fn get_longitude(&self) -> Result<Option<f64>, Box<dyn std::error::Error + Send + Sync>>;

    /// Read `Vehicle.Parking.SessionActive`.
    async fn get_parking_session_active(
        &self,
    ) -> Result<Option<bool>, Box<dyn std::error::Error + Send + Sync>>;
}

/// Read all vehicle signals from DATA_BROKER and return a [`VehicleState`].
///
/// Each signal is read independently; failures are logged and the
/// corresponding field is set to `None`.
pub async fn read_vehicle_state<R: DataBrokerReader>(reader: &R) -> VehicleState {
    VehicleState {
        is_locked: read_signal(reader.get_is_locked(), "IsLocked").await,
        is_door_open: read_signal(reader.get_is_door_open(), "IsOpen").await,
        speed: read_signal(reader.get_speed(), "Speed").await,
        latitude: read_signal(reader.get_latitude(), "Latitude").await,
        longitude: read_signal(reader.get_longitude(), "Longitude").await,
        parking_session_active: read_signal(
            reader.get_parking_session_active(),
            "ParkingSessionActive",
        )
        .await,
    }
}

/// Collected vehicle state from DATA_BROKER.
#[derive(Debug, Clone, Default)]
pub struct VehicleState {
    pub is_locked: Option<bool>,
    pub is_door_open: Option<bool>,
    pub speed: Option<f64>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub parking_session_active: Option<bool>,
}

/// Read a single signal, logging any errors and returning `None` on failure.
async fn read_signal<T>(
    future: impl std::future::Future<Output = Result<Option<T>, Box<dyn std::error::Error + Send + Sync>>>,
    name: &str,
) -> Option<T> {
    match future.await {
        Ok(value) => value,
        Err(e) => {
            warn!(signal = name, error = %e, "failed to read signal from DATA_BROKER");
            None
        }
    }
}

/// Process a raw MQTT status request payload.
///
/// 1. Parse JSON → [`StatusRequest`].
/// 2. Read all vehicle signals from DATA_BROKER.
/// 3. Construct and publish a [`StatusResponse`] to MQTT.
///
/// Returns `true` if the response was published successfully.
pub async fn handle_status_request<R: DataBrokerReader>(
    payload: &[u8],
    vin: &str,
    reader: &R,
    mqtt: &MqttClient,
) -> bool {
    // 1. Parse the status request.
    let req: StatusRequest = match serde_json::from_slice(payload) {
        Ok(r) => r,
        Err(e) => {
            warn!(error = %e, "received invalid status request JSON, discarding");
            return false;
        }
    };

    info!(request_id = %req.request_id, "received status request");

    // 2. Read vehicle state from DATA_BROKER.
    let state = read_vehicle_state(reader).await;

    // 3. Construct and publish response.
    let response = crate::messages::StatusResponse {
        request_id: req.request_id.clone(),
        vin: vin.to_string(),
        is_locked: state.is_locked,
        is_door_open: state.is_door_open,
        speed: state.speed,
        latitude: state.latitude,
        longitude: state.longitude,
        parking_session_active: state.parking_session_active,
        timestamp: chrono_timestamp(),
    };

    let payload = match serde_json::to_vec(&response) {
        Ok(p) => p,
        Err(e) => {
            error!(error = %e, "failed to serialize StatusResponse");
            return false;
        }
    };

    if let Err(e) = mqtt.publish_status_response(&payload).await {
        error!(
            request_id = %req.request_id,
            error = %e,
            "failed to publish status response to MQTT"
        );
        return false;
    }

    info!(request_id = %req.request_id, "published status response");
    true
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
                speed: Some(0.0),
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
        async fn get_is_locked(&self) -> Result<Option<bool>, Box<dyn std::error::Error + Send + Sync>> {
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

        async fn get_speed(&self) -> Result<Option<f64>, Box<dyn std::error::Error + Send + Sync>> {
            if self.fail_signal.as_deref() == Some("Speed") {
                return Err("mock read failure".into());
            }
            Ok(self.speed)
        }

        async fn get_latitude(&self) -> Result<Option<f64>, Box<dyn std::error::Error + Send + Sync>> {
            if self.fail_signal.as_deref() == Some("Latitude") {
                return Err("mock read failure".into());
            }
            Ok(self.latitude)
        }

        async fn get_longitude(&self) -> Result<Option<f64>, Box<dyn std::error::Error + Send + Sync>> {
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

    #[tokio::test]
    async fn read_vehicle_state_full() {
        let reader = MockReader::full_state();
        let state = read_vehicle_state(&reader).await;

        assert_eq!(state.is_locked, Some(true));
        assert_eq!(state.is_door_open, Some(false));
        assert_eq!(state.speed, Some(0.0));
        assert_eq!(state.latitude, Some(48.1351));
        assert_eq!(state.longitude, Some(11.582));
        assert_eq!(state.parking_session_active, Some(false));
    }

    #[tokio::test]
    async fn read_vehicle_state_empty() {
        let reader = MockReader::empty_state();
        let state = read_vehicle_state(&reader).await;

        assert!(state.is_locked.is_none());
        assert!(state.is_door_open.is_none());
        assert!(state.speed.is_none());
        assert!(state.latitude.is_none());
        assert!(state.longitude.is_none());
        assert!(state.parking_session_active.is_none());
    }

    #[tokio::test]
    async fn read_vehicle_state_partial_failure() {
        let reader = MockReader::full_state().with_failure("Speed");
        let state = read_vehicle_state(&reader).await;

        assert_eq!(state.is_locked, Some(true));
        assert_eq!(state.is_door_open, Some(false));
        // Speed should be None due to failure.
        assert!(state.speed.is_none());
        assert_eq!(state.latitude, Some(48.1351));
        assert_eq!(state.longitude, Some(11.582));
        assert_eq!(state.parking_session_active, Some(false));
    }

    #[tokio::test]
    async fn status_response_serialization_full() {
        let response = crate::messages::StatusResponse {
            request_id: "req-1".to_string(),
            vin: "DEMO0000000000001".to_string(),
            is_locked: Some(true),
            is_door_open: Some(false),
            speed: Some(0.0),
            latitude: Some(48.1351),
            longitude: Some(11.582),
            parking_session_active: Some(false),
            timestamp: 1708300802,
        };

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["request_id"], "req-1");
        assert_eq!(json["vin"], "DEMO0000000000001");
        assert_eq!(json["is_locked"], true);
        assert_eq!(json["is_door_open"], false);
        assert_eq!(json["speed"], 0.0);
        assert_eq!(json["latitude"], 48.1351);
        assert_eq!(json["longitude"], 11.582);
        assert_eq!(json["parking_session_active"], false);
    }

    #[tokio::test]
    async fn status_response_serialization_null_fields() {
        let response = crate::messages::StatusResponse {
            request_id: "req-2".to_string(),
            vin: "DEMO0000000000001".to_string(),
            is_locked: None,
            is_door_open: None,
            speed: None,
            latitude: None,
            longitude: None,
            parking_session_active: None,
            timestamp: 1708300802,
        };

        let json = serde_json::to_value(&response).unwrap();
        assert!(json["is_locked"].is_null());
        assert!(json["is_door_open"].is_null());
        assert!(json["speed"].is_null());
        assert!(json["latitude"].is_null());
        assert!(json["longitude"].is_null());
        assert!(json["parking_session_active"].is_null());
    }

    #[test]
    fn chrono_timestamp_is_reasonable() {
        let ts = chrono_timestamp();
        assert!(ts > 1_577_836_800, "timestamp too small: {ts}");
        assert!(ts < 4_102_444_800, "timestamp too large: {ts}");
    }

    #[test]
    fn status_request_parsing_valid() {
        let payload = br#"{"request_id":"req-1","timestamp":1708300802}"#;
        let req: StatusRequest = serde_json::from_slice(payload).unwrap();
        assert_eq!(req.request_id, "req-1");
        assert_eq!(req.timestamp, 1708300802);
    }

    #[test]
    fn status_request_parsing_invalid() {
        let payload = b"not valid json";
        let result = serde_json::from_slice::<StatusRequest>(payload);
        assert!(result.is_err());
    }

    #[test]
    fn vehicle_state_default() {
        let state = VehicleState::default();
        assert!(state.is_locked.is_none());
        assert!(state.is_door_open.is_none());
        assert!(state.speed.is_none());
        assert!(state.latitude.is_none());
        assert!(state.longitude.is_none());
        assert!(state.parking_session_active.is_none());
    }
}
