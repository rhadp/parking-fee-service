//! Outbound telemetry pipeline: DATA_BROKER -> NATS.
//!
//! Subscribes to vehicle state signals on DATA_BROKER and publishes telemetry
//! JSON messages to `vehicles.{VIN}.telemetry` on NATS when values change.

use bytes::Bytes;
use serde::Serialize;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{error, info, warn};

use crate::databroker_client::DataBrokerClient;
use crate::nats_client::NatsClient;

/// The DATA_BROKER signals to subscribe to for telemetry.
const TELEMETRY_SIGNALS: &[&str] = &[
    "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked",
    "Vehicle.CurrentLocation.Latitude",
    "Vehicle.CurrentLocation.Longitude",
    "Vehicle.Parking.SessionActive",
];

/// A telemetry message published to NATS.
#[derive(Debug, Clone, Serialize)]
pub struct TelemetryMessage {
    pub vin: String,
    pub signal: String,
    pub value: serde_json::Value,
    pub timestamp: u64,
}

/// Run the telemetry publishing loop.
///
/// Subscribes to vehicle state signals on DATA_BROKER and publishes telemetry
/// messages to NATS when signal values change.
pub async fn run(
    mut databroker: DataBrokerClient,
    nats: NatsClient,
    databroker_uds_path: String,
    vin: String,
) {
    info!("Telemetry publisher started");

    loop {
        // Subscribe to telemetry signals on DATA_BROKER
        let mut stream = match databroker.subscribe_signals(TELEMETRY_SIGNALS).await {
            Ok(s) => {
                info!(
                    "Subscribed to DATA_BROKER telemetry signals: {:?}",
                    TELEMETRY_SIGNALS
                );
                s
            }
            Err(e) => {
                error!(
                    "Failed to subscribe to telemetry signals on DATA_BROKER: {}. Retrying...",
                    e
                );
                if let Err(re) = databroker.reconnect(&databroker_uds_path).await {
                    error!("DATA_BROKER reconnection failed: {}", re);
                }
                continue;
            }
        };

        // Track last known values to only publish on changes
        let mut last_values: HashMap<String, serde_json::Value> = HashMap::new();

        // Process stream updates
        loop {
            match stream.message().await {
                Ok(Some(response)) => {
                    for (path, datapoint) in &response.entries {
                        // Extract the value as a JSON value
                        let json_value = match extract_json_value(datapoint) {
                            Some(v) => v,
                            None => {
                                warn!(
                                    "Telemetry signal {} has no extractable value, skipping",
                                    path
                                );
                                continue;
                            }
                        };

                        // Only publish on actual value changes
                        if let Some(last) = last_values.get(path) {
                            if *last == json_value {
                                continue;
                            }
                        }
                        last_values.insert(path.clone(), json_value.clone());

                        let timestamp = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();

                        let msg = TelemetryMessage {
                            vin: vin.clone(),
                            signal: path.clone(),
                            value: json_value,
                            timestamp,
                        };

                        match serde_json::to_vec(&msg) {
                            Ok(payload) => {
                                if let Err(e) =
                                    nats.publish_telemetry(Bytes::from(payload)).await
                                {
                                    error!(
                                        "Failed to publish telemetry to NATS: {}. Message lost.",
                                        e
                                    );
                                } else {
                                    info!(
                                        "Published telemetry for signal {}: {:?}",
                                        msg.signal, msg.value
                                    );
                                }
                            }
                            Err(e) => {
                                error!("Failed to serialize telemetry message: {}", e);
                            }
                        }
                    }
                }
                Ok(None) => {
                    warn!("DATA_BROKER telemetry subscription stream ended");
                    break;
                }
                Err(e) => {
                    error!("DATA_BROKER telemetry subscription stream error: {}", e);
                    break;
                }
            }
        }

        // Stream ended or errored; attempt reconnection
        warn!("Telemetry publisher lost DATA_BROKER stream, attempting reconnection...");
        if let Err(e) = databroker.reconnect(&databroker_uds_path).await {
            error!("DATA_BROKER reconnection failed: {}", e);
        }
    }
}

/// Extract a JSON-compatible value from a Kuksa Datapoint.
fn extract_json_value(
    datapoint: &crate::databroker_client::kuksa::val::v2::Datapoint,
) -> Option<serde_json::Value> {
    use crate::databroker_client::kuksa::val::v2::value::TypedValue;

    let value = datapoint.value.as_ref()?;
    match &value.typed_value {
        Some(TypedValue::String(s)) => Some(serde_json::Value::String(s.clone())),
        Some(TypedValue::Bool(b)) => Some(serde_json::Value::Bool(*b)),
        Some(TypedValue::Int32(i)) => Some(serde_json::json!(*i)),
        Some(TypedValue::Int64(i)) => Some(serde_json::json!(*i)),
        Some(TypedValue::Uint32(u)) => Some(serde_json::json!(*u)),
        Some(TypedValue::Uint64(u)) => Some(serde_json::json!(*u)),
        Some(TypedValue::Float(f)) => Some(serde_json::json!(*f)),
        Some(TypedValue::Double(d)) => Some(serde_json::json!(*d)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_message_serializes_correctly() {
        let msg = TelemetryMessage {
            vin: "TEST_VIN_001".to_string(),
            signal: "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked".to_string(),
            value: serde_json::Value::Bool(true),
            timestamp: 1700000002,
        };

        let json = serde_json::to_string(&msg).expect("Should serialize");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("Should parse");

        assert_eq!(parsed["vin"], "TEST_VIN_001");
        assert_eq!(
            parsed["signal"],
            "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked"
        );
        assert_eq!(parsed["value"], true);
        assert_eq!(parsed["timestamp"], 1700000002);
    }

    #[test]
    fn test_telemetry_message_with_double_value() {
        let msg = TelemetryMessage {
            vin: "TEST_VIN_001".to_string(),
            signal: "Vehicle.CurrentLocation.Latitude".to_string(),
            value: serde_json::json!(48.1351),
            timestamp: 1700000003,
        };

        let json = serde_json::to_string(&msg).expect("Should serialize");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("Should parse");

        assert_eq!(parsed["signal"], "Vehicle.CurrentLocation.Latitude");
        assert_eq!(parsed["value"], 48.1351);
    }

    #[test]
    fn test_telemetry_signals_list_is_complete() {
        assert_eq!(TELEMETRY_SIGNALS.len(), 4);
        assert!(TELEMETRY_SIGNALS.contains(&"Vehicle.Cabin.Door.Row1.DriverSide.IsLocked"));
        assert!(TELEMETRY_SIGNALS.contains(&"Vehicle.CurrentLocation.Latitude"));
        assert!(TELEMETRY_SIGNALS.contains(&"Vehicle.CurrentLocation.Longitude"));
        assert!(TELEMETRY_SIGNALS.contains(&"Vehicle.Parking.SessionActive"));
    }
}
