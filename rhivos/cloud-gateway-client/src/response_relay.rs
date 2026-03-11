//! Outbound response relay pipeline: DATA_BROKER -> NATS.
//!
//! Subscribes to `Vehicle.Command.Door.Response` on DATA_BROKER and publishes
//! response JSON to `vehicles.{VIN}.command_responses` on NATS.

use bytes::Bytes;
use tracing::{error, info, warn};

use crate::databroker_client::DataBrokerClient;
use crate::nats_client::NatsClient;

/// The VSS signal path for door command responses.
const DOOR_RESPONSE_SIGNAL: &str = "Vehicle.Command.Door.Response";

/// Run the response relay loop.
///
/// Subscribes to `Vehicle.Command.Door.Response` on DATA_BROKER and publishes
/// each response update to NATS on `vehicles.{VIN}.command_responses`.
pub async fn run(
    mut databroker: DataBrokerClient,
    nats: NatsClient,
    databroker_uds_path: String,
) {
    info!("Response relay started");

    loop {
        // Subscribe to the response signal on DATA_BROKER
        let mut stream = match databroker
            .subscribe_signals(&[DOOR_RESPONSE_SIGNAL])
            .await
        {
            Ok(s) => {
                info!(
                    "Subscribed to DATA_BROKER signal: {}",
                    DOOR_RESPONSE_SIGNAL
                );
                s
            }
            Err(e) => {
                error!(
                    "Failed to subscribe to {} on DATA_BROKER: {}. Retrying...",
                    DOOR_RESPONSE_SIGNAL, e
                );
                // Attempt reconnection
                if let Err(re) = databroker.reconnect(&databroker_uds_path).await {
                    error!("DATA_BROKER reconnection failed: {}", re);
                }
                continue;
            }
        };

        // Process stream updates
        loop {
            match stream.message().await {
                Ok(Some(response)) => {
                    for (path, datapoint) in &response.entries {
                        if path != DOOR_RESPONSE_SIGNAL {
                            continue;
                        }

                        // Extract string value from datapoint
                        let json_str = match extract_string_value(datapoint) {
                            Some(s) => s,
                            None => {
                                warn!(
                                    "Response from DATA_BROKER on {} has no string value, skipping",
                                    path
                                );
                                continue;
                            }
                        };

                        // Validate that the response is parseable JSON
                        if serde_json::from_str::<serde_json::Value>(&json_str).is_err() {
                            warn!(
                                "Response JSON from DATA_BROKER on {} is not valid JSON, skipping: {}",
                                path, json_str
                            );
                            continue;
                        }

                        // Publish to NATS
                        if let Err(e) = nats
                            .publish_command_response(Bytes::from(json_str.clone()))
                            .await
                        {
                            error!(
                                "Failed to publish response to NATS: {}. Response lost.",
                                e
                            );
                        } else {
                            info!("Relayed command response to NATS: {}", json_str);
                        }
                    }
                }
                Ok(None) => {
                    warn!("DATA_BROKER subscription stream for {} ended", DOOR_RESPONSE_SIGNAL);
                    break;
                }
                Err(e) => {
                    error!(
                        "DATA_BROKER subscription stream error for {}: {}",
                        DOOR_RESPONSE_SIGNAL, e
                    );
                    break;
                }
            }
        }

        // Stream ended or errored; attempt reconnection
        warn!("Response relay lost DATA_BROKER stream, attempting reconnection...");
        if let Err(e) = databroker.reconnect(&databroker_uds_path).await {
            error!("DATA_BROKER reconnection failed: {}", e);
        }
    }
}

/// Extract a string value from a Kuksa Datapoint.
fn extract_string_value(
    datapoint: &crate::databroker_client::kuksa::val::v2::Datapoint,
) -> Option<String> {
    let value = datapoint.value.as_ref()?;
    match &value.typed_value {
        Some(crate::databroker_client::kuksa::val::v2::value::TypedValue::String(s)) => {
            Some(s.clone())
        }
        _ => None,
    }
}
