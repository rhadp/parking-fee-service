//! DATA_BROKER gRPC client module for CLOUD_GATEWAY_CLIENT.
//!
//! Manages the DATA_BROKER (Eclipse Kuksa Databroker) gRPC connection.
//! Provides methods to write command signals, subscribe to command response
//! signals, and subscribe to telemetry signals.
//!
//! All inter-service communication within the safety partition goes through
//! DATA_BROKER — this module never calls LOCKING_SERVICE directly.
//!
//! Validates [04-REQ-3.1], [04-REQ-3.2], [04-REQ-3.3], [04-REQ-3.E1],
//!           [04-REQ-6.3], [04-REQ-7.1], [04-REQ-7.2], [04-REQ-7.E1],
//!           [04-REQ-8.1], [04-REQ-10.2], [04-REQ-10.4]

#![allow(dead_code)]

use futures::Stream;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{error, info, warn};

use crate::config::Config;
use crate::errors::BrokerError;
use crate::models::SignalUpdate;

// ── Generated proto code ──────────────────────────────────────────────────────

#[allow(clippy::enum_variant_names)]
mod kuksa_val_v1 {
    tonic::include_proto!("kuksa.val.v1");
}

use kuksa_val_v1::datapoint::Value as DatapointValue;
use kuksa_val_v1::val_client::ValClient;
use kuksa_val_v1::{
    DataEntry, Datapoint, EntryUpdate, Field, SetRequest, SubscribeEntry, SubscribeRequest, View,
};
use tonic::transport::Channel;

// ── VSS signal path constants ─────────────────────────────────────────────────

/// Signal written when a lock/unlock command is forwarded to LOCKING_SERVICE.
/// Validates [04-REQ-6.3]
const SIGNAL_COMMAND: &str = "Vehicle.Command.Door.Lock";

/// Signal written by LOCKING_SERVICE with the result of a command.
/// Validates [04-REQ-3.3], [04-REQ-7.1], [04-REQ-7.2]
const SIGNAL_RESPONSE: &str = "Vehicle.Command.Door.Response";

/// Lock state of the driver-side front door.
/// Validates [04-REQ-3.2]
const SIGNAL_IS_LOCKED: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked";

/// Current vehicle latitude.
/// Validates [04-REQ-3.2]
const SIGNAL_LATITUDE: &str = "Vehicle.CurrentLocation.Latitude";

/// Current vehicle longitude.
/// Validates [04-REQ-3.2]
const SIGNAL_LONGITUDE: &str = "Vehicle.CurrentLocation.Longitude";

/// Whether a parking session is currently active.
/// Validates [04-REQ-3.2]
const SIGNAL_PARKING_ACTIVE: &str = "Vehicle.Parking.SessionActive";

// ── BrokerClient ──────────────────────────────────────────────────────────────

/// gRPC-backed DATA_BROKER client.
///
/// Wraps a `ValClient<Channel>` tonic client. All RPC methods clone the
/// underlying `Channel` before calling async methods, since `ValClient` wraps
/// a shared `Channel` and cloning is cheap.
pub struct BrokerClient {
    client: ValClient<Channel>,
}

impl BrokerClient {
    /// Connect to the DATA_BROKER at the address specified in `config`.
    ///
    /// Returns `Err(BrokerError::ConnectionFailed)` if the connection cannot be
    /// established. The caller (main startup sequence) is responsible for
    /// logging the failure and exiting with code 1 ([04-REQ-3.E1], [04-REQ-9.2]).
    ///
    /// Validates [04-REQ-3.1], [04-REQ-3.E1]
    pub async fn connect(config: &Config) -> Result<Self, BrokerError> {
        ValClient::connect(config.databroker_addr.clone())
            .await
            .map(|client| {
                info!(addr = %config.databroker_addr, "Connected to DATA_BROKER");
                Self { client }
            })
            .map_err(|e| {
                error!(
                    addr = %config.databroker_addr,
                    error = %e,
                    "Failed to connect to DATA_BROKER"
                );
                BrokerError::ConnectionFailed(e.to_string())
            })
    }

    /// Write a command payload to `Vehicle.Command.Door.Lock` in DATA_BROKER.
    ///
    /// The `payload` string is written verbatim as a `StringValue` — no
    /// re-serialization is performed, ensuring passthrough fidelity.
    /// This satisfies Property 3 from the design: the value written to
    /// DATA_BROKER is identical to the original payload received from NATS.
    ///
    /// Validates [04-REQ-6.3], Property 3
    pub async fn write_command(&self, payload: &str) -> Result<(), BrokerError> {
        let request = SetRequest {
            updates: vec![EntryUpdate {
                entry: Some(DataEntry {
                    path: SIGNAL_COMMAND.to_string(),
                    value: Some(Datapoint {
                        timestamp: 0,
                        value: Some(DatapointValue::StringValue(payload.to_string())),
                    }),
                    actuator_target: None,
                    metadata: None,
                }),
                fields: vec![Field::Value as i32],
            }],
        };

        self.client
            .clone()
            .set(request)
            .await
            .map_err(|e| {
                error!(
                    signal = SIGNAL_COMMAND,
                    error = %e,
                    "Failed to write command to DATA_BROKER"
                );
                BrokerError::WriteFailed(e.to_string())
            })?;

        info!(signal = SIGNAL_COMMAND, "Command forwarded to DATA_BROKER");
        Ok(())
    }

    /// Subscribe to `Vehicle.Command.Door.Response` and stream valid JSON responses.
    ///
    /// Returns a stream of `Ok(json_string)` for each valid JSON response
    /// received from DATA_BROKER. Invalid JSON values are logged as errors
    /// and dropped from the stream — they are never published to NATS
    /// ([04-REQ-7.E1]).
    ///
    /// The stream ends (`Err(BrokerError::SubscribeFailed(...))`) if the
    /// underlying gRPC subscription stream encounters an error.
    ///
    /// Validates [04-REQ-3.3], [04-REQ-7.1], [04-REQ-7.2], [04-REQ-7.E1]
    pub async fn subscribe_responses(
        &self,
    ) -> Result<impl Stream<Item = Result<String, BrokerError>>, BrokerError> {
        let request = SubscribeRequest {
            entries: vec![SubscribeEntry {
                path: SIGNAL_RESPONSE.to_string(),
                view: View::CurrentValue as i32,
                fields: vec![Field::Value as i32],
            }],
        };

        let mut stream = self
            .client
            .clone()
            .subscribe(request)
            .await
            .map_err(|e| {
                error!(
                    signal = SIGNAL_RESPONSE,
                    error = %e,
                    "Failed to subscribe to command response signal"
                );
                BrokerError::SubscribeFailed(e.to_string())
            })?
            .into_inner();

        let (tx, rx) = mpsc::channel::<Result<String, BrokerError>>(100);

        tokio::spawn(async move {
            loop {
                match stream.message().await {
                    Ok(Some(response)) => {
                        for update in response.updates {
                            if let Some(entry) = update.entry {
                                if let Some(dp) = entry.value {
                                    if let Some(DatapointValue::StringValue(s)) = dp.value {
                                        // Validate that the response is valid JSON [04-REQ-7.E1].
                                        if serde_json::from_str::<serde_json::Value>(&s).is_err()
                                        {
                                            error!(
                                                signal = SIGNAL_RESPONSE,
                                                value = %s,
                                                "Non-JSON response from DATA_BROKER; discarding"
                                            );
                                            continue;
                                        }
                                        if tx.send(Ok(s)).await.is_err() {
                                            // Receiver dropped — stop forwarding.
                                            return;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Ok(None) => {
                        warn!(signal = SIGNAL_RESPONSE, "Response subscription stream ended");
                        return;
                    }
                    Err(e) => {
                        error!(
                            signal = SIGNAL_RESPONSE,
                            error = %e,
                            "Response subscription stream error"
                        );
                        // Propagate the error downstream so callers can detect failure.
                        let _ = tx
                            .send(Err(BrokerError::SubscribeFailed(e.to_string())))
                            .await;
                        return;
                    }
                }
            }
        });

        Ok(ReceiverStream::new(rx))
    }

    /// Subscribe to telemetry signals and stream typed signal updates.
    ///
    /// Subscribes to all four telemetry signals in a single gRPC call:
    /// - `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` → `SignalUpdate::IsLocked`
    /// - `Vehicle.CurrentLocation.Latitude`             → `SignalUpdate::Latitude`
    /// - `Vehicle.CurrentLocation.Longitude`            → `SignalUpdate::Longitude`
    /// - `Vehicle.Parking.SessionActive`                → `SignalUpdate::ParkingActive`
    ///
    /// Updates for unrecognized signal paths or mismatched value types are
    /// logged as warnings and dropped.
    ///
    /// Validates [04-REQ-3.2], [04-REQ-8.1]
    pub async fn subscribe_telemetry(
        &self,
    ) -> Result<impl Stream<Item = Result<SignalUpdate, BrokerError>>, BrokerError> {
        let entries: Vec<SubscribeEntry> = [
            SIGNAL_IS_LOCKED,
            SIGNAL_LATITUDE,
            SIGNAL_LONGITUDE,
            SIGNAL_PARKING_ACTIVE,
        ]
        .iter()
        .map(|path| SubscribeEntry {
            path: path.to_string(),
            view: View::CurrentValue as i32,
            fields: vec![Field::Value as i32],
        })
        .collect();

        let request = SubscribeRequest { entries };

        let mut stream = self
            .client
            .clone()
            .subscribe(request)
            .await
            .map_err(|e| {
                error!(
                    error = %e,
                    "Failed to subscribe to telemetry signals"
                );
                BrokerError::SubscribeFailed(e.to_string())
            })?
            .into_inner();

        let (tx, rx) = mpsc::channel::<Result<SignalUpdate, BrokerError>>(100);

        tokio::spawn(async move {
            loop {
                match stream.message().await {
                    Ok(Some(response)) => {
                        for update in response.updates {
                            if let Some(entry) = update.entry {
                                let path = entry.path.clone();
                                if let Some(dp) = entry.value {
                                    match parse_telemetry_signal(&path, dp) {
                                        Some(signal_update) => {
                                            if tx.send(Ok(signal_update)).await.is_err() {
                                                // Receiver dropped — stop forwarding.
                                                return;
                                            }
                                        }
                                        None => {
                                            warn!(
                                                path = %path,
                                                "Unexpected or unrecognized telemetry signal; skipping"
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Ok(None) => {
                        warn!("Telemetry subscription stream ended");
                        return;
                    }
                    Err(e) => {
                        error!(error = %e, "Telemetry subscription stream error");
                        let _ = tx
                            .send(Err(BrokerError::SubscribeFailed(e.to_string())))
                            .await;
                        return;
                    }
                }
            }
        });

        Ok(ReceiverStream::new(rx))
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Map a telemetry signal path and its datapoint value to a typed `SignalUpdate`.
///
/// Returns `None` for unrecognized signal paths or mismatched value types,
/// allowing the caller to log a warning and continue processing.
fn parse_telemetry_signal(path: &str, dp: Datapoint) -> Option<SignalUpdate> {
    let value = dp.value?;

    if path == SIGNAL_IS_LOCKED {
        if let DatapointValue::BoolValue(b) = value {
            return Some(SignalUpdate::IsLocked(b));
        }
    } else if path == SIGNAL_LATITUDE {
        if let DatapointValue::DoubleValue(d) = value {
            return Some(SignalUpdate::Latitude(d));
        }
    } else if path == SIGNAL_LONGITUDE {
        if let DatapointValue::DoubleValue(d) = value {
            return Some(SignalUpdate::Longitude(d));
        }
    } else if path == SIGNAL_PARKING_ACTIVE {
        if let DatapointValue::BoolValue(b) = value {
            return Some(SignalUpdate::ParkingActive(b));
        }
    }

    None
}
