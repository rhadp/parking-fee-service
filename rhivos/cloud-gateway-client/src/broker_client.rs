//! DATA_BROKER gRPC client for the CLOUD_GATEWAY_CLIENT.
//!
//! Manages the gRPC connection to the Kuksa Databroker and provides methods
//! to write command signals, subscribe to command response signals, and
//! subscribe to telemetry signals.
//!
//! All `tonic` gRPC usage is encapsulated within this module.
//!
//! # Requirements
//!
//! - [04-REQ-3.1]: Establish gRPC connection to DATA_BROKER.
//! - [04-REQ-3.2]: Subscribe to telemetry signals (IsLocked, Latitude, Longitude, SessionActive).
//! - [04-REQ-3.3]: Subscribe to `Vehicle.Command.Door.Response`.
//! - [04-REQ-3.E1]: Exit with code 1 if connection fails at startup.
//! - [04-REQ-6.3]: Write command payload to `Vehicle.Command.Door.Lock`.
//! - [04-REQ-7.1]: Observe command response changes for relay.
//! - [04-REQ-7.E1]: Skip relay when response is not valid JSON.

use std::time::Duration;

use tokio::sync::mpsc;
use tonic::transport::Channel;

use crate::config::Config;
use crate::errors::BrokerError;
use crate::models::SignalUpdate;

/// Generated kuksa.val.v2 gRPC types and client.
///
/// Made public to allow integration tests to perform direct gRPC operations
/// against DATA_BROKER (e.g., reading back written values, injecting test
/// signals) without duplicating proto compilation.
pub mod kuksa {
    pub mod val {
        pub mod v2 {
            tonic::include_proto!("kuksa.val.v2");
        }
    }
}

use kuksa::val::v2::{
    signal_id::Signal, val_client::ValClient, value::TypedValue, Datapoint, PublishValueRequest,
    SignalId, SubscribeRequest, Value,
};

// VSS signal path constants.

/// Command signal written by CLOUD_GATEWAY_CLIENT for LOCKING_SERVICE.
const SIGNAL_COMMAND: &str = "Vehicle.Command.Door.Lock";

/// Response signal observed from LOCKING_SERVICE.
const SIGNAL_RESPONSE: &str = "Vehicle.Command.Door.Response";

/// Telemetry signal: door lock state.
const SIGNAL_IS_LOCKED: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked";

/// Telemetry signal: current latitude.
const SIGNAL_LATITUDE: &str = "Vehicle.CurrentLocation.Latitude";

/// Telemetry signal: current longitude.
const SIGNAL_LONGITUDE: &str = "Vehicle.CurrentLocation.Longitude";

/// Telemetry signal: parking session active state.
const SIGNAL_PARKING_ACTIVE: &str = "Vehicle.Parking.SessionActive";

/// gRPC client for communicating with DATA_BROKER using the kuksa.val.v2 API.
///
/// The client wraps a tonic `ValClient<Channel>` which is cheaply cloneable.
/// Each method clones the inner client to satisfy tonic's `&mut self`
/// requirement while keeping the public API on `&self`.
pub struct BrokerClient {
    client: ValClient<Channel>,
}

impl BrokerClient {
    /// Connects to DATA_BROKER at the configured address.
    ///
    /// Uses a 5-second connect timeout. This method does not retry --
    /// the caller (main startup sequence) should exit with code 1 on failure.
    ///
    /// # Errors
    ///
    /// Returns [`BrokerError::ConnectionFailed`] if the connection cannot be
    /// established.
    ///
    /// # Requirements
    ///
    /// - [04-REQ-3.1]: Establish gRPC connection to DATA_BROKER.
    /// - [04-REQ-3.E1]: Caller exits with code 1 on failure.
    pub async fn connect(config: &Config) -> Result<Self, BrokerError> {
        let addr = &config.databroker_addr;
        tracing::info!(addr, "connecting to DATA_BROKER");

        let endpoint = tonic::transport::Endpoint::from_shared(addr.to_string())
            .map_err(|e| {
                tracing::error!(addr, error = %e, "invalid DATA_BROKER address");
                BrokerError::ConnectionFailed(e.to_string())
            })?
            .connect_timeout(Duration::from_secs(5));

        let channel = endpoint.connect().await.map_err(|e| {
            tracing::error!(addr, error = %e, "failed to connect to DATA_BROKER");
            BrokerError::ConnectionFailed(e.to_string())
        })?;

        tracing::info!(addr, "connected to DATA_BROKER");

        Ok(Self {
            client: ValClient::new(channel),
        })
    }

    /// Writes a command payload (JSON string) to `Vehicle.Command.Door.Lock`.
    ///
    /// The payload is written as-is (no modification) via a `PublishValue` RPC.
    ///
    /// # Errors
    ///
    /// Returns [`BrokerError::WriteFailed`] if the gRPC call fails.
    ///
    /// # Requirements
    ///
    /// - [04-REQ-6.3]: Write command payload as-is to `Vehicle.Command.Door.Lock`.
    pub async fn write_command(&self, payload: &str) -> Result<(), BrokerError> {
        let mut client = self.client.clone();

        let request = PublishValueRequest {
            signal_id: Some(SignalId {
                signal: Some(Signal::Path(SIGNAL_COMMAND.to_string())),
            }),
            data_point: Some(Datapoint {
                timestamp: None,
                value: Some(Value {
                    typed_value: Some(TypedValue::String(payload.to_string())),
                }),
            }),
        };

        tracing::info!(signal = SIGNAL_COMMAND, "writing command to DATA_BROKER");

        client.publish_value(request).await.map_err(|e| {
            tracing::error!(
                signal = SIGNAL_COMMAND,
                error = %e,
                "failed to write command to DATA_BROKER"
            );
            BrokerError::WriteFailed(e.to_string())
        })?;

        tracing::info!(signal = SIGNAL_COMMAND, "command written to DATA_BROKER");
        Ok(())
    }

    /// Subscribes to `Vehicle.Command.Door.Response` and returns a receiver
    /// that streams response JSON strings.
    ///
    /// Spawns a background task that reads from the gRPC stream and forwards
    /// string-typed datapoint values. Invalid (non-JSON) responses are logged
    /// at ERROR level and skipped ([04-REQ-7.E1]).
    ///
    /// # Errors
    ///
    /// Returns [`BrokerError::SubscribeFailed`] if the subscription cannot
    /// be created.
    ///
    /// # Requirements
    ///
    /// - [04-REQ-3.3]: Subscribe to `Vehicle.Command.Door.Response`.
    /// - [04-REQ-7.1]: Read JSON value and relay to NATS.
    /// - [04-REQ-7.E1]: Log error and skip if response is not valid JSON.
    pub async fn subscribe_responses(&self) -> Result<mpsc::Receiver<String>, BrokerError> {
        let mut client = self.client.clone();

        let request = SubscribeRequest {
            signal_paths: vec![SIGNAL_RESPONSE.to_string()],
            buffer_size: 0,
        };

        tracing::info!(
            signal = SIGNAL_RESPONSE,
            "subscribing to command response signal"
        );

        let response = client.subscribe(request).await.map_err(|e| {
            tracing::error!(
                signal = SIGNAL_RESPONSE,
                error = %e,
                "failed to subscribe to command response signal"
            );
            BrokerError::SubscribeFailed(e.to_string())
        })?;

        let mut stream = response.into_inner();
        let (tx, rx) = mpsc::channel(32);

        tracing::info!(
            signal = SIGNAL_RESPONSE,
            "subscribed to command response signal"
        );

        tokio::spawn(async move {
            loop {
                match stream.message().await {
                    Ok(Some(subscribe_response)) => {
                        for (path, datapoint) in subscribe_response.entries {
                            if let Some(value) = datapoint.value {
                                if let Some(TypedValue::String(s)) = value.typed_value {
                                    // Validate that the response is valid JSON
                                    // before forwarding ([04-REQ-7.E1]).
                                    if serde_json::from_str::<serde_json::Value>(&s).is_err() {
                                        tracing::error!(
                                            signal = %path,
                                            value = %s,
                                            "response from DATA_BROKER is not valid JSON, skipping"
                                        );
                                        continue;
                                    }

                                    tracing::info!(
                                        signal = %path,
                                        "received command response from DATA_BROKER"
                                    );

                                    if tx.send(s).await.is_err() {
                                        // Receiver dropped, stop reading.
                                        return;
                                    }
                                }
                            }
                        }
                    }
                    Ok(None) => {
                        tracing::warn!(
                            signal = SIGNAL_RESPONSE,
                            "response subscription stream ended"
                        );
                        break;
                    }
                    Err(e) => {
                        tracing::error!(
                            signal = SIGNAL_RESPONSE,
                            error = %e,
                            "response subscription stream error"
                        );
                        break;
                    }
                }
            }
        });

        Ok(rx)
    }

    /// Subscribes to telemetry signals and returns a receiver that streams
    /// [`SignalUpdate`] values.
    ///
    /// Subscribes to:
    /// - `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` (bool)
    /// - `Vehicle.CurrentLocation.Latitude` (double)
    /// - `Vehicle.CurrentLocation.Longitude` (double)
    /// - `Vehicle.Parking.SessionActive` (bool)
    ///
    /// Spawns a background task that reads from the gRPC stream, maps each
    /// datapoint to the appropriate `SignalUpdate` variant, and forwards it.
    ///
    /// # Errors
    ///
    /// Returns [`BrokerError::SubscribeFailed`] if the subscription cannot
    /// be created.
    ///
    /// # Requirements
    ///
    /// - [04-REQ-3.2]: Subscribe to telemetry signals.
    pub async fn subscribe_telemetry(&self) -> Result<mpsc::Receiver<SignalUpdate>, BrokerError> {
        let mut client = self.client.clone();

        let signal_paths = vec![
            SIGNAL_IS_LOCKED.to_string(),
            SIGNAL_LATITUDE.to_string(),
            SIGNAL_LONGITUDE.to_string(),
            SIGNAL_PARKING_ACTIVE.to_string(),
        ];

        let request = SubscribeRequest {
            signal_paths: signal_paths.clone(),
            buffer_size: 0,
        };

        tracing::info!(
            signals = ?signal_paths,
            "subscribing to telemetry signals"
        );

        let response = client.subscribe(request).await.map_err(|e| {
            tracing::error!(
                signals = ?signal_paths,
                error = %e,
                "failed to subscribe to telemetry signals"
            );
            BrokerError::SubscribeFailed(e.to_string())
        })?;

        let mut stream = response.into_inner();
        let (tx, rx) = mpsc::channel(32);

        tracing::info!("subscribed to telemetry signals");

        tokio::spawn(async move {
            loop {
                match stream.message().await {
                    Ok(Some(subscribe_response)) => {
                        for (path, datapoint) in subscribe_response.entries {
                            let update = match path.as_str() {
                                SIGNAL_IS_LOCKED => {
                                    extract_bool(&datapoint).map(SignalUpdate::IsLocked)
                                }
                                SIGNAL_LATITUDE => {
                                    extract_double(&datapoint).map(SignalUpdate::Latitude)
                                }
                                SIGNAL_LONGITUDE => {
                                    extract_double(&datapoint).map(SignalUpdate::Longitude)
                                }
                                SIGNAL_PARKING_ACTIVE => {
                                    extract_bool(&datapoint).map(SignalUpdate::ParkingActive)
                                }
                                other => {
                                    tracing::warn!(
                                        signal = %other,
                                        "received update for unexpected signal, ignoring"
                                    );
                                    None
                                }
                            };

                            if let Some(signal_update) = update {
                                tracing::info!(
                                    signal = %path,
                                    "received telemetry update from DATA_BROKER"
                                );

                                if tx.send(signal_update).await.is_err() {
                                    // Receiver dropped, stop reading.
                                    return;
                                }
                            }
                        }
                    }
                    Ok(None) => {
                        tracing::warn!("telemetry subscription stream ended");
                        break;
                    }
                    Err(e) => {
                        tracing::error!(
                            error = %e,
                            "telemetry subscription stream error"
                        );
                        break;
                    }
                }
            }
        });

        Ok(rx)
    }
}

/// Extracts a boolean value from a datapoint.
fn extract_bool(datapoint: &Datapoint) -> Option<bool> {
    datapoint
        .value
        .as_ref()
        .and_then(|v| v.typed_value.as_ref())
        .and_then(|tv| match tv {
            TypedValue::Bool(b) => Some(*b),
            _ => None,
        })
}

/// Extracts a double (f64) value from a datapoint.
fn extract_double(datapoint: &Datapoint) -> Option<f64> {
    datapoint
        .value
        .as_ref()
        .and_then(|v| v.typed_value.as_ref())
        .and_then(|tv| match tv {
            TypedValue::Double(d) => Some(*d),
            TypedValue::Float(f) => Some(*f as f64),
            _ => None,
        })
}
