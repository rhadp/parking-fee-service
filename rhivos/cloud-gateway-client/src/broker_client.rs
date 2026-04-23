//! DATA_BROKER gRPC client for cloud-gateway-client.
//!
//! Manages the gRPC connection to Eclipse Kuksa Databroker. Provides
//! methods to write command signals, subscribe to telemetry signals,
//! and subscribe to command response signals.
//!
//! Uses the kuksa.val.v2 API for publishing values and subscribing to
//! changes. The v1 Set RPC is non-functional in kuksa-databroker 0.5.0,
//! so all writes use v2 PublishValue. See
//! `docs/errata/04_kuksa_v2_api_migration.md` for details.

use tokio::sync::mpsc;
use tonic::transport::Channel;
use tracing::{error, info, warn};

use crate::config::Config;
use crate::errors::BrokerError;
use crate::models::SignalUpdate;

/// Generated code from vendored kuksa.val.v1 proto files.
/// Retained for type re-exports used by integration tests.
pub mod kuksa {
    pub mod val {
        pub mod v1 {
            tonic::include_proto!("kuksa.val.v1");
        }
        pub mod v2 {
            tonic::include_proto!("kuksa.val.v2");
        }
    }
}

use kuksa::val::v2::{
    self,
    val_client::ValClient as V2Client,
    value::TypedValue,
};

/// VSS signal path for incoming lock/unlock commands.
const SIGNAL_COMMAND: &str = "Vehicle.Command.Door.Lock";
/// VSS signal path for command responses from LOCKING_SERVICE.
const SIGNAL_RESPONSE: &str = "Vehicle.Command.Door.Response";
/// VSS signal path for door lock state.
const SIGNAL_IS_LOCKED: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked";
/// VSS signal path for current latitude.
const SIGNAL_LATITUDE: &str = "Vehicle.CurrentLocation.Latitude";
/// VSS signal path for current longitude.
const SIGNAL_LONGITUDE: &str = "Vehicle.CurrentLocation.Longitude";
/// VSS signal path for parking session state.
const SIGNAL_PARKING: &str = "Vehicle.Parking.SessionActive";

/// Channel buffer size for subscription streams.
const CHANNEL_BUFFER: usize = 32;

/// gRPC client for the kuksa DATA_BROKER.
///
/// Uses the v2 API (`kuksa.val.v2.VAL`) for publishing values and
/// subscribing to signal changes. The v1 API's `Set` RPC is
/// non-functional in kuksa-databroker 0.5.0.
pub struct BrokerClient {
    v2_client: V2Client<Channel>,
}

impl BrokerClient {
    /// Connect to DATA_BROKER at the configured address.
    ///
    /// Makes a single connection attempt. On failure, returns
    /// `BrokerError::ConnectionFailed`.
    ///
    /// # Errors
    ///
    /// Returns [`BrokerError::ConnectionFailed`] if the gRPC connection
    /// cannot be established.
    pub async fn connect(config: &Config) -> Result<Self, BrokerError> {
        let addr = config.databroker_addr.clone();
        match V2Client::connect(addr.clone()).await {
            Ok(v2_client) => {
                info!(addr = %addr, "Connected to DATA_BROKER");
                Ok(BrokerClient { v2_client })
            }
            Err(e) => {
                error!(addr = %addr, error = %e, "Failed to connect to DATA_BROKER");
                Err(BrokerError::ConnectionFailed(e.to_string()))
            }
        }
    }

    /// Write a command payload to `Vehicle.Command.Door.Lock` in DATA_BROKER.
    ///
    /// The payload is written as-is (string value) to preserve the original
    /// JSON from NATS without modification (Property 3: Command Passthrough
    /// Fidelity).
    ///
    /// Uses the v2 PublishValue RPC because the v1 Set RPC is
    /// non-functional in kuksa-databroker 0.5.0.
    ///
    /// # Errors
    ///
    /// Returns [`BrokerError::WriteFailed`] if the gRPC PublishValue fails.
    pub async fn write_command(&self, payload: &str) -> Result<(), BrokerError> {
        let request = tonic::Request::new(v2::PublishValueRequest {
            signal_id: Some(v2::SignalId {
                signal: Some(v2::signal_id::Signal::Path(
                    SIGNAL_COMMAND.to_string(),
                )),
            }),
            data_point: Some(v2::Datapoint {
                timestamp: None,
                value: Some(v2::Value {
                    typed_value: Some(TypedValue::String(payload.to_string())),
                }),
            }),
        });

        self.v2_client
            .clone()
            .publish_value(request)
            .await
            .map_err(|e| {
                error!(
                    signal = SIGNAL_COMMAND,
                    error = %e,
                    "Failed to write command to DATA_BROKER"
                );
                BrokerError::WriteFailed(e.to_string())
            })?;

        info!(signal = SIGNAL_COMMAND, "Command written to DATA_BROKER");
        Ok(())
    }

    /// Subscribe to `Vehicle.Command.Door.Response` to observe command
    /// results from LOCKING_SERVICE.
    ///
    /// Returns a channel receiver that yields JSON response strings as
    /// they arrive from DATA_BROKER. Invalid (non-JSON) values from the
    /// broker are logged at ERROR level and skipped (REQ-7.E1).
    ///
    /// # Errors
    ///
    /// Returns [`BrokerError::SubscribeFailed`] if the initial gRPC
    /// subscription cannot be established.
    pub async fn subscribe_responses(
        &self,
    ) -> Result<mpsc::Receiver<String>, BrokerError> {
        let request = tonic::Request::new(v2::SubscribeRequest {
            signal_paths: vec![SIGNAL_RESPONSE.to_string()],
            buffer_size: 0,
        });

        let stream = self
            .v2_client
            .clone()
            .subscribe(request)
            .await
            .map_err(|e| {
                error!(
                    signal = SIGNAL_RESPONSE,
                    error = %e,
                    "Failed to subscribe to command responses"
                );
                BrokerError::SubscribeFailed(e.to_string())
            })?
            .into_inner();

        let (tx, rx) = mpsc::channel(CHANNEL_BUFFER);

        tokio::spawn(async move {
            Self::response_stream_loop(stream, tx).await;
        });

        info!(
            signal = SIGNAL_RESPONSE,
            "Subscribed to command responses"
        );
        Ok(rx)
    }

    /// Subscribe to telemetry signals: IsLocked, Latitude, Longitude,
    /// and SessionActive.
    ///
    /// Returns a channel receiver that yields [`SignalUpdate`] values as
    /// they change in DATA_BROKER.
    ///
    /// # Errors
    ///
    /// Returns [`BrokerError::SubscribeFailed`] if the initial gRPC
    /// subscription cannot be established.
    pub async fn subscribe_telemetry(
        &self,
    ) -> Result<mpsc::Receiver<SignalUpdate>, BrokerError> {
        let request = tonic::Request::new(v2::SubscribeRequest {
            signal_paths: vec![
                SIGNAL_IS_LOCKED.to_string(),
                SIGNAL_LATITUDE.to_string(),
                SIGNAL_LONGITUDE.to_string(),
                SIGNAL_PARKING.to_string(),
            ],
            buffer_size: 0,
        });

        let stream = self
            .v2_client
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

        let (tx, rx) = mpsc::channel(CHANNEL_BUFFER);

        tokio::spawn(async move {
            Self::telemetry_stream_loop(stream, tx).await;
        });

        info!("Subscribed to telemetry signals");
        Ok(rx)
    }

    /// Process the response subscription stream, forwarding string values
    /// to the channel.
    ///
    /// Validates that each value is valid JSON before forwarding (REQ-7.E1).
    /// Invalid JSON is logged at ERROR level and skipped.
    async fn response_stream_loop(
        mut stream: tonic::Streaming<v2::SubscribeResponse>,
        tx: mpsc::Sender<String>,
    ) {
        loop {
            match stream.message().await {
                Ok(Some(response)) => {
                    if let Some(dp) = response.entries.get(SIGNAL_RESPONSE) {
                        if let Some(v2::Value {
                            typed_value: Some(TypedValue::String(s)),
                        }) = &dp.value
                        {
                            // Validate JSON before relaying (REQ-7.E1)
                            if serde_json::from_str::<serde_json::Value>(s).is_err() {
                                error!(
                                    signal = SIGNAL_RESPONSE,
                                    value = %s,
                                    "Response value from DATA_BROKER is not valid JSON, skipping"
                                );
                                continue;
                            }
                            if tx.send(s.clone()).await.is_err() {
                                info!("Response receiver dropped, stopping stream");
                                return;
                            }
                        }
                    }
                }
                Ok(None) => {
                    warn!(
                        signal = SIGNAL_RESPONSE,
                        "Response subscription stream ended"
                    );
                    return;
                }
                Err(e) => {
                    error!(
                        signal = SIGNAL_RESPONSE,
                        error = %e,
                        "Response subscription stream error"
                    );
                    return;
                }
            }
        }
    }

    /// Process the telemetry subscription stream, converting DATA_BROKER
    /// signal updates to [`SignalUpdate`] values.
    async fn telemetry_stream_loop(
        mut stream: tonic::Streaming<v2::SubscribeResponse>,
        tx: mpsc::Sender<SignalUpdate>,
    ) {
        loop {
            match stream.message().await {
                Ok(Some(response)) => {
                    for (path, dp) in &response.entries {
                        let signal_update = match path.as_str() {
                            SIGNAL_IS_LOCKED => {
                                if let Some(v2::Value {
                                    typed_value: Some(TypedValue::Bool(v)),
                                }) = &dp.value
                                {
                                    Some(SignalUpdate::IsLocked(*v))
                                } else {
                                    None
                                }
                            }
                            SIGNAL_LATITUDE => {
                                if let Some(v2::Value {
                                    typed_value: Some(TypedValue::Double(v)),
                                }) = &dp.value
                                {
                                    Some(SignalUpdate::Latitude(*v))
                                } else if let Some(v2::Value {
                                    typed_value: Some(TypedValue::Float(v)),
                                }) = &dp.value
                                {
                                    Some(SignalUpdate::Latitude(*v as f64))
                                } else {
                                    None
                                }
                            }
                            SIGNAL_LONGITUDE => {
                                if let Some(v2::Value {
                                    typed_value: Some(TypedValue::Double(v)),
                                }) = &dp.value
                                {
                                    Some(SignalUpdate::Longitude(*v))
                                } else if let Some(v2::Value {
                                    typed_value: Some(TypedValue::Float(v)),
                                }) = &dp.value
                                {
                                    Some(SignalUpdate::Longitude(*v as f64))
                                } else {
                                    None
                                }
                            }
                            SIGNAL_PARKING => {
                                if let Some(v2::Value {
                                    typed_value: Some(TypedValue::Bool(v)),
                                }) = &dp.value
                                {
                                    Some(SignalUpdate::ParkingActive(*v))
                                } else {
                                    None
                                }
                            }
                            _ => {
                                warn!(
                                    signal = path.as_str(),
                                    "Unexpected signal path in telemetry stream"
                                );
                                None
                            }
                        };

                        if let Some(su) = signal_update {
                            if tx.send(su).await.is_err() {
                                info!("Telemetry receiver dropped, stopping stream");
                                return;
                            }
                        }
                    }
                }
                Ok(None) => {
                    warn!("Telemetry subscription stream ended");
                    return;
                }
                Err(e) => {
                    error!(
                        error = %e,
                        "Telemetry subscription stream error"
                    );
                    return;
                }
            }
        }
    }
}
