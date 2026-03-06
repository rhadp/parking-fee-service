//! Outbound response relay pipeline: DATA_BROKER -> NATS.
//!
//! Subscribes to `Vehicle.Command.Door.Response` on DATA_BROKER via gRPC.
//! When a response signal update is received, publishes the JSON string value
//! to `vehicles.{VIN}.command_responses` on NATS.

use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info, warn};

use crate::databroker_client::DatabrokerClient;
use crate::nats_client::NatsClient;

/// VSS signal path for command responses from the LOCKING_SERVICE.
const SIGNAL_COMMAND_RESPONSE: &str = "Vehicle.Command.Door.Response";

/// Run the response relay loop.
///
/// Subscribes to `Vehicle.Command.Door.Response` on DATA_BROKER and publishes
/// each response to NATS on `vehicles.{VIN}.command_responses`.
///
/// If the DATA_BROKER stream terminates, the function returns so the caller
/// can decide whether to restart it.
pub async fn run(databroker: Arc<Mutex<DatabrokerClient>>, nats_client: NatsClient) {
    use futures::StreamExt;

    info!("Response relay started");

    // Subscribe to the response signal on DATA_BROKER
    let mut stream = {
        let mut db = databroker.lock().await;
        match db.subscribe_signal(SIGNAL_COMMAND_RESPONSE).await {
            Ok(stream) => stream,
            Err(e) => {
                error!(
                    signal = SIGNAL_COMMAND_RESPONSE,
                    error = %e,
                    "Failed to subscribe to response signal on DATA_BROKER"
                );
                return;
            }
        }
    };

    info!(
        signal = SIGNAL_COMMAND_RESPONSE,
        "Subscribed to response signal on DATA_BROKER"
    );

    while let Some(response) = stream.next().await {
        match response {
            Ok(subscribe_response) => {
                for (path, datapoint) in &subscribe_response.entries {
                    // Extract the string value from the datapoint
                    let json_str = match extract_string_value(datapoint) {
                        Some(s) => s,
                        None => {
                            // Initial subscription may deliver None values; skip silently
                            continue;
                        }
                    };

                    // Validate that it is parseable JSON before relaying
                    if serde_json::from_str::<serde_json::Value>(&json_str).is_err() {
                        warn!(
                            signal = %path,
                            "Response JSON from DATA_BROKER is unparseable, skipping relay"
                        );
                        continue;
                    }

                    info!(
                        signal = %path,
                        "Relaying command response to NATS"
                    );

                    if let Err(e) = nats_client
                        .publish_command_response(bytes::Bytes::from(json_str))
                        .await
                    {
                        error!(
                            signal = %path,
                            error = %e,
                            "Failed to publish command response to NATS"
                        );
                    }
                }
            }
            Err(e) => {
                error!(
                    error = %e,
                    "DATA_BROKER response stream error, ending relay"
                );
                break;
            }
        }
    }

    warn!("Response relay stream ended");
}

/// Extract a string value from a Datapoint, if present.
fn extract_string_value(datapoint: &crate::kuksa_proto::Datapoint) -> Option<String> {
    let value = datapoint.value.as_ref()?;
    match &value.typed_value {
        Some(crate::kuksa_proto::value::TypedValue::String(s)) => Some(s.clone()),
        _ => None,
    }
}
