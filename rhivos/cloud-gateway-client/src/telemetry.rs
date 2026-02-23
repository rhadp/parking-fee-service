//! Telemetry subscription and MQTT publishing.
//!
//! Subscribes to vehicle state signals in DATA_BROKER and publishes
//! telemetry messages to the MQTT topic `vehicles/{vin}/telemetry`
//! when signal values change.

use databroker_client::{DataValue, DatabrokerClient};
use rumqttc::AsyncClient;
use tokio_stream::StreamExt;
use tracing::{debug, error, info, warn};

use crate::commands::{build_telemetry_message, current_timestamp, TelemetrySignal};

/// VSS signal paths for telemetry subscription.
pub const TELEMETRY_SIGNALS: &[&str] = &[
    "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked",
    "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen",
    "Vehicle.CurrentLocation.Latitude",
    "Vehicle.CurrentLocation.Longitude",
    "Vehicle.Speed",
];

/// Run the telemetry subscription loop.
///
/// Subscribes to all telemetry signals in DATA_BROKER and publishes
/// changes to the MQTT telemetry topic.
///
/// This function runs indefinitely until the subscription is broken.
///
/// # Arguments
///
/// * `db_client` - The DATA_BROKER client for signal subscriptions.
/// * `mqtt_client` - The MQTT client for publishing telemetry.
/// * `telemetry_topic` - The MQTT topic to publish telemetry to.
pub async fn run_telemetry_loop(
    db_client: &DatabrokerClient,
    mqtt_client: &AsyncClient,
    telemetry_topic: &str,
) {
    info!(
        signals = ?TELEMETRY_SIGNALS,
        "subscribing to telemetry signals in DATA_BROKER"
    );

    let mut stream = match db_client.subscribe(TELEMETRY_SIGNALS).await {
        Ok(s) => s,
        Err(e) => {
            error!(error = %e, "failed to subscribe to telemetry signals");
            return;
        }
    };

    info!("telemetry subscription active");

    while let Some(result) = stream.next().await {
        match result {
            Ok(updates) => {
                for update in &updates {
                    if let Some(ref value) = update.value {
                        let json_value = datavalue_to_json(value);
                        let signal = TelemetrySignal {
                            path: update.path.clone(),
                            value: json_value,
                            timestamp: current_timestamp(),
                        };

                        let message = build_telemetry_message(&[signal]);

                        debug!(
                            topic = %telemetry_topic,
                            path = %update.path,
                            "publishing telemetry"
                        );

                        if let Err(e) = mqtt_client
                            .publish(
                                telemetry_topic,
                                rumqttc::QoS::AtLeastOnce,
                                false,
                                message.as_bytes(),
                            )
                            .await
                        {
                            warn!(error = %e, "failed to publish telemetry to MQTT");
                        }
                    }
                }
            }
            Err(e) => {
                error!(error = %e, "telemetry subscription error");
                break;
            }
        }
    }

    warn!("telemetry subscription ended");
}

/// Convert a `DataValue` to a `serde_json::Value`.
fn datavalue_to_json(value: &DataValue) -> serde_json::Value {
    match value {
        DataValue::Bool(b) => serde_json::Value::Bool(*b),
        DataValue::Float(f) => serde_json::json!(*f),
        DataValue::Double(d) => serde_json::json!(*d),
        DataValue::String(s) => serde_json::Value::String(s.clone()),
        DataValue::Int32(i) => serde_json::json!(*i),
        DataValue::Int64(i) => serde_json::json!(*i),
        DataValue::Uint32(u) => serde_json::json!(*u),
        DataValue::Uint64(u) => serde_json::json!(*u),
    }
}

/// Run the response relay loop.
///
/// Subscribes to Vehicle.Command.Door.Response in DATA_BROKER and
/// publishes responses to the MQTT command_responses topic.
///
/// # Arguments
///
/// * `db_client` - The DATA_BROKER client for signal subscriptions.
/// * `mqtt_client` - The MQTT client for publishing responses.
/// * `response_topic` - The MQTT topic to publish responses to.
pub async fn run_response_relay(
    db_client: &DatabrokerClient,
    mqtt_client: &AsyncClient,
    response_topic: &str,
) {
    const RESPONSE_SIGNAL: &str = "Vehicle.Command.Door.Response";

    info!("subscribing to {} in DATA_BROKER", RESPONSE_SIGNAL);

    let mut stream = match db_client.subscribe(&[RESPONSE_SIGNAL]).await {
        Ok(s) => s,
        Err(e) => {
            error!(error = %e, "failed to subscribe to response signal");
            return;
        }
    };

    info!("response relay active");

    while let Some(result) = stream.next().await {
        match result {
            Ok(updates) => {
                for update in &updates {
                    if update.path != RESPONSE_SIGNAL {
                        continue;
                    }

                    if let Some(DataValue::String(ref response_json)) = update.value {
                        debug!(
                            topic = %response_topic,
                            response = %response_json,
                            "relaying command response to MQTT"
                        );

                        if let Err(e) = mqtt_client
                            .publish(
                                response_topic,
                                rumqttc::QoS::AtLeastOnce,
                                false,
                                response_json.as_bytes(),
                            )
                            .await
                        {
                            warn!(error = %e, "failed to publish response to MQTT");
                        }
                    }
                }
            }
            Err(e) => {
                error!(error = %e, "response subscription error");
                break;
            }
        }
    }

    warn!("response relay subscription ended");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_datavalue_to_json_bool() {
        assert_eq!(
            datavalue_to_json(&DataValue::Bool(true)),
            serde_json::Value::Bool(true)
        );
        assert_eq!(
            datavalue_to_json(&DataValue::Bool(false)),
            serde_json::Value::Bool(false)
        );
    }

    #[test]
    fn test_datavalue_to_json_float() {
        let v = datavalue_to_json(&DataValue::Float(42.5));
        assert!(v.is_number());
    }

    #[test]
    fn test_datavalue_to_json_double() {
        let v = datavalue_to_json(&DataValue::Double(3.14159));
        assert!(v.is_number());
    }

    #[test]
    fn test_datavalue_to_json_string() {
        assert_eq!(
            datavalue_to_json(&DataValue::String("hello".to_string())),
            serde_json::Value::String("hello".to_string())
        );
    }

    #[test]
    fn test_telemetry_signals_coverage() {
        // Verify all required signals are included (02-REQ-5.1)
        assert!(TELEMETRY_SIGNALS.contains(&"Vehicle.Cabin.Door.Row1.DriverSide.IsLocked"));
        assert!(TELEMETRY_SIGNALS.contains(&"Vehicle.Cabin.Door.Row1.DriverSide.IsOpen"));
        assert!(TELEMETRY_SIGNALS.contains(&"Vehicle.CurrentLocation.Latitude"));
        assert!(TELEMETRY_SIGNALS.contains(&"Vehicle.CurrentLocation.Longitude"));
        assert!(TELEMETRY_SIGNALS.contains(&"Vehicle.Speed"));
    }
}
