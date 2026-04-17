//! DATA_BROKER gRPC client for CLOUD_GATEWAY_CLIENT.
//!
//! Provides `BrokerClient` for connecting to Eclipse Kuksa Databroker,
//! writing command signals, and subscribing to response and telemetry signals.
//!
//! All DATA_BROKER communication uses the kuksa.val.v1 gRPC API.

use std::time::Duration;

use futures::StreamExt;
use tonic::transport::{Channel, Endpoint};
use tracing::{debug, error, info, warn};

use crate::config::Config;
use crate::errors::BrokerError;
use crate::models::SignalUpdate;

/// Generated types from `proto/kuksa/val.proto`.
///
/// The `enum_variant_names` lint is suppressed because the generated `Value`
/// enum has variants like `StringValue`, `BoolValue`, etc. — a protobuf
/// convention that clippy flags but we cannot change in generated code.
#[allow(clippy::enum_variant_names)]
mod kuksa {
    tonic::include_proto!("kuksa");
}

use kuksa::val_service_client::ValServiceClient;
use kuksa::{DataEntry, Datapoint, Field, SetRequest, SubscribeRequest};

// VSS signal paths used by CLOUD_GATEWAY_CLIENT
pub const SIGNAL_COMMAND_LOCK: &str = "Vehicle.Command.Door.Lock";
pub const SIGNAL_COMMAND_RESPONSE: &str = "Vehicle.Command.Door.Response";
pub const SIGNAL_IS_LOCKED: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked";
pub const SIGNAL_LATITUDE: &str = "Vehicle.CurrentLocation.Latitude";
pub const SIGNAL_LONGITUDE: &str = "Vehicle.CurrentLocation.Longitude";
pub const SIGNAL_PARKING_ACTIVE: &str = "Vehicle.Parking.SessionActive";

/// Maximum retry attempts for the initial DATA_BROKER connection.
const MAX_CONNECT_ATTEMPTS: u32 = 5;

/// Base backoff delay (ms) for exponential retry (doubles each attempt).
const BASE_BACKOFF_MS: u64 = 1_000;

/// gRPC client for Eclipse Kuksa Databroker (DATA_BROKER).
///
/// Uses a shared `Channel` (internally multiplexed by tonic) so that cloning
/// `ValServiceClient` per call is cheap and does not open new connections.
pub struct BrokerClient {
    channel: Channel,
}

impl BrokerClient {
    /// Connect to DATA_BROKER at the address specified in `config`, with
    /// exponential-backoff retry.
    ///
    /// Attempts: 1 (immediate) + up to 4 retries with delays 1s, 2s, 4s, 8s.
    /// Returns `Err(BrokerError::ConnectionFailed)` after exhausting all attempts.
    ///
    /// [04-REQ-3.1], [04-REQ-3.E1]
    pub async fn connect(config: &Config) -> Result<Self, BrokerError> {
        let addr = &config.databroker_addr;
        let endpoint = Endpoint::from_shared(addr.to_owned())
            .map_err(|e| {
                BrokerError::ConnectionFailed(format!("Invalid DATA_BROKER address {addr}: {e}"))
            })?
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(10));

        let mut last_err = String::new();

        for attempt in 0..MAX_CONNECT_ATTEMPTS {
            if attempt > 0 {
                let delay_ms = BASE_BACKOFF_MS * (1u64 << (attempt - 1));
                warn!(
                    attempt,
                    delay_ms,
                    addr = %addr,
                    "Retrying DATA_BROKER connection"
                );
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            }

            match endpoint.connect().await {
                Ok(channel) => {
                    info!(addr = %addr, "Connected to DATA_BROKER");
                    return Ok(BrokerClient { channel });
                }
                Err(e) => {
                    last_err = e.to_string();
                    warn!(attempt, error = %e, addr = %addr, "Failed to connect to DATA_BROKER");
                }
            }
        }

        error!(
            addr = %addr,
            attempts = MAX_CONNECT_ATTEMPTS,
            "DATA_BROKER unreachable after all retry attempts"
        );
        Err(BrokerError::ConnectionFailed(format!(
            "Failed to connect to DATA_BROKER at {addr} after {MAX_CONNECT_ATTEMPTS} attempts: {last_err}"
        )))
    }

    /// Create a fresh `ValServiceClient` per call.
    ///
    /// The underlying `Channel` is multiplexed, so cloning is cheap.
    fn client(&self) -> ValServiceClient<Channel> {
        ValServiceClient::new(self.channel.clone())
    }

    /// Write `payload` verbatim to `Vehicle.Command.Door.Lock` in DATA_BROKER.
    ///
    /// The payload is the original JSON bytes from the NATS command message,
    /// forwarded as-is per Property 3 (Command Passthrough Fidelity).
    ///
    /// [04-REQ-6.3]
    pub async fn write_command(&self, payload: &str) -> Result<(), BrokerError> {
        let mut client = self.client();
        let response = client
            .set(SetRequest {
                updates: vec![DataEntry {
                    path: SIGNAL_COMMAND_LOCK.to_owned(),
                    value: Some(Datapoint {
                        timestamp: 0,
                        value: Some(kuksa::datapoint::Value::StringValue(payload.to_owned())),
                    }),
                }],
            })
            .await
            .map_err(|e| {
                BrokerError::WriteFailed(format!(
                    "SET {} failed: {e}",
                    SIGNAL_COMMAND_LOCK
                ))
            })?
            .into_inner();

        // Check for field-level errors in the response.
        if !response.errors.is_empty() {
            let msg = response
                .errors
                .iter()
                .map(|e| format!("[{}] {}: {}", e.code, e.reason, e.message))
                .collect::<Vec<_>>()
                .join("; ");
            error!(
                signal = SIGNAL_COMMAND_LOCK,
                errors = %msg,
                "DATA_BROKER SET returned field-level errors"
            );
            return Err(BrokerError::WriteFailed(format!(
                "SET {} returned errors: {msg}",
                SIGNAL_COMMAND_LOCK
            )));
        }

        info!(signal = SIGNAL_COMMAND_LOCK, "Command forwarded to DATA_BROKER");
        Ok(())
    }

    /// Subscribe to `Vehicle.Command.Door.Response` and yield its string value
    /// on each change.
    ///
    /// Returns a `Stream` of `Ok(json_string)` items. If the subscription stream
    /// encounters an error, `Err(BrokerError::SubscribeFailed)` is yielded and
    /// the stream ends.
    ///
    /// [04-REQ-3.3], [04-REQ-7.1], [04-REQ-7.2], [04-REQ-7.E1]
    pub async fn subscribe_responses(
        &self,
    ) -> Result<impl futures::Stream<Item = Result<String, BrokerError>> + Send, BrokerError> {
        let mut client = self.client();
        let stream = client
            .subscribe(SubscribeRequest {
                entries: vec![Field {
                    path: SIGNAL_COMMAND_RESPONSE.to_owned(),
                }],
            })
            .await
            .map_err(|e| {
                BrokerError::SubscribeFailed(format!(
                    "Subscribe {} failed: {e}",
                    SIGNAL_COMMAND_RESPONSE
                ))
            })?
            .into_inner();

        info!(
            signal = SIGNAL_COMMAND_RESPONSE,
            "Subscribed to command responses from DATA_BROKER"
        );

        let (tx, rx) =
            futures::channel::mpsc::unbounded::<Result<String, BrokerError>>();

        tokio::spawn(async move {
            let mut stream = stream;
            while let Some(result) = stream.next().await {
                match result {
                    Ok(response) => {
                        for entry in response.updates {
                            if let Some(dp) = entry.value {
                                match dp.value {
                                    Some(kuksa::datapoint::Value::StringValue(s)) => {
                                        debug!(
                                            signal = SIGNAL_COMMAND_RESPONSE,
                                            "Command response received from DATA_BROKER"
                                        );
                                        if tx.unbounded_send(Ok(s)).is_err() {
                                            // Receiver dropped; stop streaming.
                                            return;
                                        }
                                    }
                                    Some(_) => {
                                        warn!(
                                            signal = SIGNAL_COMMAND_RESPONSE,
                                            "Unexpected non-string value type for response signal"
                                        );
                                    }
                                    None => {
                                        debug!(
                                            signal = SIGNAL_COMMAND_RESPONSE,
                                            "Response datapoint has no value"
                                        );
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!(
                            error = %e,
                            "DATA_BROKER command response stream error"
                        );
                        let _ = tx.unbounded_send(Err(BrokerError::SubscribeFailed(
                            e.to_string(),
                        )));
                        return;
                    }
                }
            }
            debug!("DATA_BROKER command response subscription stream closed");
        });

        Ok(rx)
    }

    /// Subscribe to all telemetry signals and yield a `SignalUpdate` on each change.
    ///
    /// Subscribed signals:
    /// - `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` → `SignalUpdate::IsLocked`
    /// - `Vehicle.CurrentLocation.Latitude` → `SignalUpdate::Latitude`
    /// - `Vehicle.CurrentLocation.Longitude` → `SignalUpdate::Longitude`
    /// - `Vehicle.Parking.SessionActive` → `SignalUpdate::ParkingActive`
    ///
    /// Returns a `Stream` of `Ok(SignalUpdate)` items. Unknown or untyped signals
    /// are silently skipped (logged at WARN). Errors terminate the stream with
    /// `Err(BrokerError::SubscribeFailed)`.
    ///
    /// [04-REQ-3.2]
    pub async fn subscribe_telemetry(
        &self,
    ) -> Result<impl futures::Stream<Item = Result<SignalUpdate, BrokerError>> + Send, BrokerError>
    {
        let mut client = self.client();
        let stream = client
            .subscribe(SubscribeRequest {
                entries: vec![
                    Field {
                        path: SIGNAL_IS_LOCKED.to_owned(),
                    },
                    Field {
                        path: SIGNAL_LATITUDE.to_owned(),
                    },
                    Field {
                        path: SIGNAL_LONGITUDE.to_owned(),
                    },
                    Field {
                        path: SIGNAL_PARKING_ACTIVE.to_owned(),
                    },
                ],
            })
            .await
            .map_err(|e| {
                BrokerError::SubscribeFailed(format!(
                    "Subscribe telemetry signals failed: {e}"
                ))
            })?
            .into_inner();

        info!("Subscribed to telemetry signals from DATA_BROKER");

        let (tx, rx) =
            futures::channel::mpsc::unbounded::<Result<SignalUpdate, BrokerError>>();

        tokio::spawn(async move {
            let mut stream = stream;
            while let Some(result) = stream.next().await {
                match result {
                    Ok(response) => {
                        for entry in response.updates {
                            let path = entry.path.clone();
                            if let Some(update) = parse_signal_update(&path, entry.value) {
                                debug!(
                                    signal = %path,
                                    "Telemetry signal update received from DATA_BROKER"
                                );
                                if tx.unbounded_send(Ok(update)).is_err() {
                                    // Receiver dropped; stop streaming.
                                    return;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!(
                            error = %e,
                            "DATA_BROKER telemetry stream error"
                        );
                        let _ = tx.unbounded_send(Err(BrokerError::SubscribeFailed(
                            e.to_string(),
                        )));
                        return;
                    }
                }
            }
            debug!("DATA_BROKER telemetry subscription stream closed");
        });

        Ok(rx)
    }
}

/// Parse a DATA_BROKER signal entry into a typed `SignalUpdate`.
///
/// Returns `None` if the path is unknown or the value type is unexpected
/// (both cases are logged at WARN level).
fn parse_signal_update(path: &str, datapoint: Option<Datapoint>) -> Option<SignalUpdate> {
    let dp = datapoint?;
    let value = dp.value?;

    if path == SIGNAL_IS_LOCKED {
        if let kuksa::datapoint::Value::BoolValue(b) = value {
            Some(SignalUpdate::IsLocked(b))
        } else {
            warn!(signal = path, "Unexpected value type for IsLocked signal");
            None
        }
    } else if path == SIGNAL_LATITUDE {
        match value {
            kuksa::datapoint::Value::DoubleValue(d) => Some(SignalUpdate::Latitude(d)),
            kuksa::datapoint::Value::FloatValue(f) => Some(SignalUpdate::Latitude(f64::from(f))),
            _ => {
                warn!(signal = path, "Unexpected value type for Latitude signal");
                None
            }
        }
    } else if path == SIGNAL_LONGITUDE {
        match value {
            kuksa::datapoint::Value::DoubleValue(d) => Some(SignalUpdate::Longitude(d)),
            kuksa::datapoint::Value::FloatValue(f) => Some(SignalUpdate::Longitude(f64::from(f))),
            _ => {
                warn!(signal = path, "Unexpected value type for Longitude signal");
                None
            }
        }
    } else if path == SIGNAL_PARKING_ACTIVE {
        if let kuksa::datapoint::Value::BoolValue(b) = value {
            Some(SignalUpdate::ParkingActive(b))
        } else {
            warn!(signal = path, "Unexpected value type for ParkingActive signal");
            None
        }
    } else {
        warn!(signal = path, "Received update for unknown telemetry signal");
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::SignalUpdate;

    /// Verify parse_signal_update correctly maps IsLocked bool values.
    #[test]
    fn test_parse_signal_is_locked() {
        let dp = Some(Datapoint {
            timestamp: 0,
            value: Some(kuksa::datapoint::Value::BoolValue(true)),
        });
        let result = parse_signal_update(SIGNAL_IS_LOCKED, dp);
        assert!(matches!(result, Some(SignalUpdate::IsLocked(true))));
    }

    /// Verify parse_signal_update correctly maps Latitude double values.
    #[test]
    fn test_parse_signal_latitude_double() {
        let dp = Some(Datapoint {
            timestamp: 0,
            value: Some(kuksa::datapoint::Value::DoubleValue(48.1351)),
        });
        let result = parse_signal_update(SIGNAL_LATITUDE, dp);
        assert!(matches!(result, Some(SignalUpdate::Latitude(v)) if (v - 48.1351).abs() < 1e-9));
    }

    /// Verify parse_signal_update correctly maps Latitude float values (upcast to f64).
    #[test]
    fn test_parse_signal_latitude_float() {
        let dp = Some(Datapoint {
            timestamp: 0,
            value: Some(kuksa::datapoint::Value::FloatValue(48.0_f32)),
        });
        let result = parse_signal_update(SIGNAL_LATITUDE, dp);
        assert!(matches!(result, Some(SignalUpdate::Latitude(_))));
    }

    /// Verify parse_signal_update correctly maps Longitude double values.
    #[test]
    fn test_parse_signal_longitude() {
        let dp = Some(Datapoint {
            timestamp: 0,
            value: Some(kuksa::datapoint::Value::DoubleValue(11.582)),
        });
        let result = parse_signal_update(SIGNAL_LONGITUDE, dp);
        assert!(matches!(result, Some(SignalUpdate::Longitude(v)) if (v - 11.582).abs() < 1e-9));
    }

    /// Verify parse_signal_update correctly maps ParkingActive bool values.
    #[test]
    fn test_parse_signal_parking_active() {
        let dp = Some(Datapoint {
            timestamp: 0,
            value: Some(kuksa::datapoint::Value::BoolValue(false)),
        });
        let result = parse_signal_update(SIGNAL_PARKING_ACTIVE, dp);
        assert!(matches!(result, Some(SignalUpdate::ParkingActive(false))));
    }

    /// Verify parse_signal_update returns None for unknown signal paths.
    #[test]
    fn test_parse_signal_unknown_path() {
        let dp = Some(Datapoint {
            timestamp: 0,
            value: Some(kuksa::datapoint::Value::BoolValue(true)),
        });
        let result = parse_signal_update("Vehicle.Unknown.Signal", dp);
        assert!(result.is_none());
    }

    /// Verify parse_signal_update returns None when datapoint is absent.
    #[test]
    fn test_parse_signal_none_datapoint() {
        let result = parse_signal_update(SIGNAL_IS_LOCKED, None);
        assert!(result.is_none());
    }

    /// Verify parse_signal_update returns None for wrong value type.
    #[test]
    fn test_parse_signal_wrong_type() {
        let dp = Some(Datapoint {
            timestamp: 0,
            // IsLocked expects bool, not string
            value: Some(kuksa::datapoint::Value::StringValue("true".to_owned())),
        });
        let result = parse_signal_update(SIGNAL_IS_LOCKED, dp);
        assert!(result.is_none());
    }
}
