//! Outbound telemetry pipeline: DATA_BROKER -> NATS.
//!
//! Subscribes to vehicle state signals on DATA_BROKER and publishes
//! telemetry JSON messages to `vehicles.{VIN}.telemetry` on NATS
//! whenever signal values change.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info, warn};

use crate::databroker_client::{DatabrokerClient, SignalValue};
use crate::nats_client::NatsClient;

/// VSS signal paths to subscribe to for telemetry.
const TELEMETRY_SIGNALS: &[&str] = &[
    "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked",
    "Vehicle.CurrentLocation.Latitude",
    "Vehicle.CurrentLocation.Longitude",
    "Vehicle.Parking.SessionActive",
];

/// A telemetry message published to NATS.
#[derive(Debug, serde::Serialize)]
struct TelemetryMessage {
    vin: String,
    signal: String,
    value: serde_json::Value,
    timestamp: i64,
}

/// Run the telemetry publishing loop.
///
/// Subscribes to vehicle state signals on DATA_BROKER and publishes
/// a JSON telemetry message to NATS whenever a signal value changes.
///
/// Only publishes on actual value changes, not on initial subscription delivery
/// or repeated identical values.
///
/// If the DATA_BROKER stream terminates, the function returns so the caller
/// can decide whether to restart it.
pub async fn run(databroker: Arc<Mutex<DatabrokerClient>>, nats_client: NatsClient) {
    use futures::StreamExt;

    info!("Telemetry publisher started");

    // Subscribe to all telemetry signals on DATA_BROKER
    let mut stream = {
        let mut db = databroker.lock().await;
        match db.subscribe_signals(TELEMETRY_SIGNALS).await {
            Ok(stream) => stream,
            Err(e) => {
                error!(
                    error = %e,
                    "Failed to subscribe to telemetry signals on DATA_BROKER"
                );
                return;
            }
        }
    };

    info!(
        signals = ?TELEMETRY_SIGNALS,
        "Subscribed to telemetry signals on DATA_BROKER"
    );

    // Track last-known values to detect actual changes
    let mut last_values: HashMap<String, String> = HashMap::new();

    // Track whether we've received the initial subscription delivery
    let mut initial_delivery = true;

    while let Some(response) = stream.next().await {
        match response {
            Ok(subscribe_response) => {
                for (path, datapoint) in &subscribe_response.entries {
                    let signal_value = match extract_signal_value(datapoint) {
                        Some(v) => v,
                        None => continue, // No value present, skip
                    };

                    let json_value = signal_value_to_json(&signal_value);
                    let value_str = json_value.to_string();

                    // Skip if value hasn't changed (dedup)
                    if let Some(last) = last_values.get(path.as_str()) {
                        if *last == value_str {
                            continue;
                        }
                    }

                    // On initial delivery from DATA_BROKER, record values but
                    // only publish if they are genuinely new (not None/default).
                    // After initial delivery, always publish on change.
                    last_values.insert(path.clone(), value_str);

                    if initial_delivery {
                        // Still record but don't publish initial snapshot
                        continue;
                    }

                    let timestamp = chrono::Utc::now().timestamp();

                    let msg = TelemetryMessage {
                        vin: nats_client.vin().to_string(),
                        signal: path.clone(),
                        value: json_value,
                        timestamp,
                    };

                    let payload = match serde_json::to_vec(&msg) {
                        Ok(p) => p,
                        Err(e) => {
                            error!(
                                signal = %path,
                                error = %e,
                                "Failed to serialize telemetry message"
                            );
                            continue;
                        }
                    };

                    info!(
                        signal = %path,
                        "Publishing telemetry to NATS"
                    );

                    if let Err(e) = nats_client
                        .publish_telemetry(bytes::Bytes::from(payload))
                        .await
                    {
                        error!(
                            signal = %path,
                            error = %e,
                            "Failed to publish telemetry to NATS"
                        );
                    }
                }

                // After processing the first response, mark initial delivery as done
                if initial_delivery {
                    initial_delivery = false;
                }
            }
            Err(e) => {
                error!(
                    error = %e,
                    "DATA_BROKER telemetry stream error, ending publisher"
                );
                break;
            }
        }
    }

    warn!("Telemetry publisher stream ended");
}

/// Extract a `SignalValue` from a Datapoint, if present.
fn extract_signal_value(datapoint: &crate::kuksa_proto::Datapoint) -> Option<SignalValue> {
    use crate::kuksa_proto::value::TypedValue;

    let value = datapoint.value.as_ref()?;
    let typed = value.typed_value.as_ref()?;

    match typed {
        TypedValue::String(s) => Some(SignalValue::String(s.clone())),
        TypedValue::Bool(b) => Some(SignalValue::Bool(*b)),
        TypedValue::Float(f) => Some(SignalValue::Float(*f)),
        TypedValue::Double(d) => Some(SignalValue::Double(*d)),
        TypedValue::Int32(i) => Some(SignalValue::Int32(*i)),
        TypedValue::Int64(i) => Some(SignalValue::Int64(*i)),
        TypedValue::Uint32(u) => Some(SignalValue::Uint32(*u)),
        TypedValue::Uint64(u) => Some(SignalValue::Uint64(*u)),
        _ => None,
    }
}

/// Convert a `SignalValue` to a `serde_json::Value`.
fn signal_value_to_json(value: &SignalValue) -> serde_json::Value {
    match value {
        SignalValue::String(s) => serde_json::Value::String(s.clone()),
        SignalValue::Bool(b) => serde_json::Value::Bool(*b),
        SignalValue::Float(f) => serde_json::json!(*f),
        SignalValue::Double(d) => serde_json::json!(*d),
        SignalValue::Int32(i) => serde_json::json!(*i),
        SignalValue::Int64(i) => serde_json::json!(*i),
        SignalValue::Uint32(u) => serde_json::json!(*u),
        SignalValue::Uint64(u) => serde_json::json!(*u),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_message_serialization() {
        let msg = TelemetryMessage {
            vin: "TEST_VIN".to_string(),
            signal: "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked".to_string(),
            value: serde_json::Value::Bool(true),
            timestamp: 1700000002,
        };

        let json = serde_json::to_string(&msg).expect("should serialize");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("should parse");

        assert_eq!(parsed["vin"], "TEST_VIN");
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
            vin: "TEST_VIN".to_string(),
            signal: "Vehicle.CurrentLocation.Latitude".to_string(),
            value: serde_json::json!(48.1351),
            timestamp: 1700000003,
        };

        let json = serde_json::to_string(&msg).expect("should serialize");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("should parse");

        assert_eq!(parsed["signal"], "Vehicle.CurrentLocation.Latitude");
        assert_eq!(parsed["value"], 48.1351);
    }

    #[test]
    fn test_signal_value_to_json_bool() {
        let v = signal_value_to_json(&SignalValue::Bool(true));
        assert_eq!(v, serde_json::Value::Bool(true));
    }

    #[test]
    fn test_signal_value_to_json_double() {
        let v = signal_value_to_json(&SignalValue::Double(11.582));
        assert_eq!(v, serde_json::json!(11.582));
    }

    #[test]
    fn test_signal_value_to_json_string() {
        let v = signal_value_to_json(&SignalValue::String("hello".to_string()));
        assert_eq!(v, serde_json::Value::String("hello".to_string()));
    }

    #[test]
    fn test_signal_value_to_json_int() {
        let v = signal_value_to_json(&SignalValue::Int32(42));
        assert_eq!(v, serde_json::json!(42));
    }
}
