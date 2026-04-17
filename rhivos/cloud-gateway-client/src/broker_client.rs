//! DATA_BROKER gRPC client for CLOUD_GATEWAY_CLIENT.
//!
//! Provides `BrokerClient` for connecting to Eclipse Kuksa Databroker,
//! writing command signals, and subscribing to response and telemetry signals.
//!
//! All DATA_BROKER communication uses the `kuksa.val.v2.VAL` gRPC API
//! as exposed by `ghcr.io/eclipse-kuksa/kuksa-databroker:0.5.0`.

use std::time::Duration;

use futures::StreamExt;
use tonic::transport::{Channel, Endpoint};
use tracing::{debug, error, info, warn};

use crate::config::Config;
use crate::errors::BrokerError;
use crate::models::SignalUpdate;

/// Generated types from `proto/kuksa/val/v2/val.proto`.
///
/// The `enum_variant_names` lint is suppressed because the generated `typed_value`
/// enum has variants like `String`, `Bool`, etc. that clippy may flag.
#[allow(clippy::enum_variant_names)]
mod kuksa_v2 {
    tonic::include_proto!("kuksa.val.v2");
}

use kuksa_v2::val_client::ValClient;
use kuksa_v2::{Datapoint, PublishValueRequest, SignalId, SubscribeRequest, Value};
use kuksa_v2::value::TypedValue;

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
/// `ValClient` per call is cheap and does not open new connections.
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

    /// Create a fresh `ValClient` per call.
    ///
    /// The underlying `Channel` is multiplexed, so cloning is cheap.
    fn client(&self) -> ValClient<Channel> {
        ValClient::new(self.channel.clone())
    }

    /// Write `payload` verbatim to `Vehicle.Command.Door.Lock` in DATA_BROKER.
    ///
    /// The cloud-gateway-client acts as the "provider" publishing the command
    /// payload for the LOCKING_SERVICE to observe via its subscription. The
    /// `kuksa.val.v2.VAL/PublishValue` RPC is used so that the write succeeds
    /// even when no actuator provider is registered for the signal.
    ///
    /// The payload is the original JSON bytes from the NATS command message,
    /// forwarded as-is per Property 3 (Command Passthrough Fidelity).
    ///
    /// [04-REQ-6.3]
    pub async fn write_command(&self, payload: &str) -> Result<(), BrokerError> {
        let mut client = self.client();
        client
            .publish_value(PublishValueRequest {
                signal_id: Some(SignalId {
                    signal: Some(kuksa_v2::signal_id::Signal::Path(
                        SIGNAL_COMMAND_LOCK.to_owned(),
                    )),
                }),
                data_point: Some(Datapoint {
                    value: Some(Value {
                        typed_value: Some(TypedValue::String(payload.to_owned())),
                    }),
                }),
            })
            .await
            .map_err(|e| {
                BrokerError::WriteFailed(format!(
                    "PublishValue {} failed: {e}",
                    SIGNAL_COMMAND_LOCK
                ))
            })?;

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
                signal_paths: vec![SIGNAL_COMMAND_RESPONSE.to_owned()],
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
                        for (path, dp) in response.entries {
                            if path == SIGNAL_COMMAND_RESPONSE {
                                match dp.value {
                                    Some(Value {
                                        typed_value: Some(TypedValue::String(s)),
                                    }) => {
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
                signal_paths: vec![
                    SIGNAL_IS_LOCKED.to_owned(),
                    SIGNAL_LATITUDE.to_owned(),
                    SIGNAL_LONGITUDE.to_owned(),
                    SIGNAL_PARKING_ACTIVE.to_owned(),
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
                        for (path, dp) in response.entries {
                            if let Some(update) = parse_signal_update(&path, dp.value) {
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

/// Parse a DATA_BROKER signal value into a typed `SignalUpdate`.
///
/// Returns `None` if the path is unknown or the value type is unexpected
/// (both cases are logged at WARN level).
fn parse_signal_update(path: &str, value: Option<Value>) -> Option<SignalUpdate> {
    let v = value?;
    let typed = v.typed_value?;

    if path == SIGNAL_IS_LOCKED {
        if let TypedValue::Bool(b) = typed {
            Some(SignalUpdate::IsLocked(b))
        } else {
            warn!(signal = path, "Unexpected value type for IsLocked signal");
            None
        }
    } else if path == SIGNAL_LATITUDE {
        match typed {
            TypedValue::Double(d) => Some(SignalUpdate::Latitude(d)),
            TypedValue::Float(f) => Some(SignalUpdate::Latitude(f64::from(f))),
            _ => {
                warn!(signal = path, "Unexpected value type for Latitude signal");
                None
            }
        }
    } else if path == SIGNAL_LONGITUDE {
        match typed {
            TypedValue::Double(d) => Some(SignalUpdate::Longitude(d)),
            TypedValue::Float(f) => Some(SignalUpdate::Longitude(f64::from(f))),
            _ => {
                warn!(signal = path, "Unexpected value type for Longitude signal");
                None
            }
        }
    } else if path == SIGNAL_PARKING_ACTIVE {
        if let TypedValue::Bool(b) = typed {
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

    fn make_value(tv: TypedValue) -> Option<Value> {
        Some(Value { typed_value: Some(tv) })
    }

    /// Verify parse_signal_update correctly maps IsLocked bool values.
    #[test]
    fn test_parse_signal_is_locked() {
        let result = parse_signal_update(SIGNAL_IS_LOCKED, make_value(TypedValue::Bool(true)));
        assert!(matches!(result, Some(SignalUpdate::IsLocked(true))));
    }

    /// Verify parse_signal_update correctly maps Latitude double values.
    #[test]
    fn test_parse_signal_latitude_double() {
        let result = parse_signal_update(SIGNAL_LATITUDE, make_value(TypedValue::Double(48.1351)));
        assert!(matches!(result, Some(SignalUpdate::Latitude(v)) if (v - 48.1351).abs() < 1e-9));
    }

    /// Verify parse_signal_update correctly maps Latitude float values (upcast to f64).
    #[test]
    fn test_parse_signal_latitude_float() {
        let result = parse_signal_update(SIGNAL_LATITUDE, make_value(TypedValue::Float(48.0_f32)));
        assert!(matches!(result, Some(SignalUpdate::Latitude(_))));
    }

    /// Verify parse_signal_update correctly maps Longitude double values.
    #[test]
    fn test_parse_signal_longitude() {
        let result = parse_signal_update(SIGNAL_LONGITUDE, make_value(TypedValue::Double(11.582)));
        assert!(matches!(result, Some(SignalUpdate::Longitude(v)) if (v - 11.582).abs() < 1e-9));
    }

    /// Verify parse_signal_update correctly maps ParkingActive bool values.
    #[test]
    fn test_parse_signal_parking_active() {
        let result = parse_signal_update(SIGNAL_PARKING_ACTIVE, make_value(TypedValue::Bool(false)));
        assert!(matches!(result, Some(SignalUpdate::ParkingActive(false))));
    }

    /// Verify parse_signal_update returns None for unknown signal paths.
    #[test]
    fn test_parse_signal_unknown_path() {
        let result = parse_signal_update("Vehicle.Unknown.Signal", make_value(TypedValue::Bool(true)));
        assert!(result.is_none());
    }

    /// Verify parse_signal_update returns None when value is absent.
    #[test]
    fn test_parse_signal_none_datapoint() {
        let result = parse_signal_update(SIGNAL_IS_LOCKED, None);
        assert!(result.is_none());
    }

    /// Verify parse_signal_update returns None for wrong value type.
    #[test]
    fn test_parse_signal_wrong_type() {
        // IsLocked expects bool, not string
        let result = parse_signal_update(SIGNAL_IS_LOCKED, make_value(TypedValue::String("true".to_owned())));
        assert!(result.is_none());
    }
}
