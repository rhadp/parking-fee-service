//! DATA_BROKER gRPC client module for cloud-gateway-client.
//!
//! Manages the DATA_BROKER gRPC connection. Provides methods to write command
//! signals, subscribe to telemetry signals, and subscribe to command response
//! signals. Encapsulates all tonic gRPC usage.
//!
//! Uses the Kuksa Databroker v2 gRPC API (kuksa.val.v2.VAL).

use crate::config::Config;
use crate::errors::BrokerError;
use crate::models::SignalUpdate;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

/// Generated gRPC types for kuksa.val.v2.
#[allow(clippy::enum_variant_names)]
#[allow(clippy::doc_lazy_continuation)]
mod kuksa {
    tonic::include_proto!("kuksa.val.v2");
}

use kuksa::val_client::ValClient;
use kuksa::{
    Datapoint, PublishValueRequest, SignalId, SubscribeRequest, Value,
};
use tonic::transport::Channel;

/// VSS signal path for lock/unlock command input.
const SIGNAL_COMMAND_DOOR_LOCK: &str = "Vehicle.Command.Door.Lock";

/// VSS signal path for command response output.
const SIGNAL_COMMAND_DOOR_RESPONSE: &str = "Vehicle.Command.Door.Response";

/// VSS signal paths for telemetry subscription.
const SIGNAL_IS_LOCKED: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked";
const SIGNAL_LATITUDE: &str = "Vehicle.CurrentLocation.Latitude";
const SIGNAL_LONGITUDE: &str = "Vehicle.CurrentLocation.Longitude";
const SIGNAL_PARKING_ACTIVE: &str = "Vehicle.Parking.SessionActive";

/// Helper to create a SignalID from a path string.
fn signal_id(path: &str) -> Option<SignalId> {
    Some(SignalId {
        signal: Some(kuksa::signal_id::Signal::Path(path.to_string())),
    })
}

/// gRPC client for DATA_BROKER (Eclipse Kuksa Databroker) communication.
#[derive(Clone)]
pub struct BrokerClient {
    client: ValClient<Channel>,
}

impl BrokerClient {
    /// Connect to the DATA_BROKER at the configured address.
    ///
    /// Returns [`BrokerError::ConnectionFailed`] if the connection cannot be
    /// established.
    ///
    /// Implements: [04-REQ-3.1], [04-REQ-3.E1]
    pub async fn connect(config: &Config) -> Result<Self, BrokerError> {
        match ValClient::connect(config.databroker_addr.clone()).await {
            Ok(client) => {
                info!("Connected to DATA_BROKER at {}", config.databroker_addr);
                Ok(BrokerClient { client })
            }
            Err(e) => {
                error!(
                    "Failed to connect to DATA_BROKER at {}: {}",
                    config.databroker_addr, e
                );
                Err(BrokerError::ConnectionFailed(format!(
                    "DATA_BROKER at {} unreachable: {}",
                    config.databroker_addr, e
                )))
            }
        }
    }

    /// Write a command payload to `Vehicle.Command.Door.Lock` in DATA_BROKER.
    ///
    /// The payload is written as a string value via gRPC PublishValueRequest.
    /// The original JSON is passed through without modification.
    ///
    /// Implements: [04-REQ-6.3]
    pub async fn write_command(&self, payload: &str) -> Result<(), BrokerError> {
        let request = PublishValueRequest {
            signal_id: signal_id(SIGNAL_COMMAND_DOOR_LOCK),
            data_point: Some(Datapoint {
                timestamp: None,
                value: Some(Value {
                    typed_value: Some(kuksa::value::TypedValue::String(
                        payload.to_string(),
                    )),
                }),
            }),
        };

        self.client
            .clone()
            .publish_value(request)
            .await
            .map_err(|e| {
                error!("Failed to write command to DATA_BROKER: {}", e);
                BrokerError::WriteFailed(format!("publish_value command failed: {}", e))
            })?;

        info!(
            "Command written to {} in DATA_BROKER",
            SIGNAL_COMMAND_DOOR_LOCK
        );
        Ok(())
    }

    /// Subscribe to `Vehicle.Command.Door.Response` to observe command results.
    ///
    /// Returns a channel receiver that yields JSON string values whenever
    /// the response signal changes in DATA_BROKER. Invalid (non-JSON) values
    /// are logged and skipped.
    ///
    /// Implements: [04-REQ-3.3], [04-REQ-7.1], [04-REQ-7.2], [04-REQ-7.E1]
    pub async fn subscribe_responses(
        &self,
    ) -> Result<mpsc::Receiver<String>, BrokerError> {
        let request = SubscribeRequest {
            signal_paths: vec![SIGNAL_COMMAND_DOOR_RESPONSE.to_string()],
            buffer_size: 0,
        };

        let response = self
            .client
            .clone()
            .subscribe(request)
            .await
            .map_err(|e| {
                error!(
                    "Failed to subscribe to {} in DATA_BROKER: {}",
                    SIGNAL_COMMAND_DOOR_RESPONSE, e
                );
                BrokerError::SubscribeFailed(format!(
                    "subscribe to {} failed: {}",
                    SIGNAL_COMMAND_DOOR_RESPONSE, e
                ))
            })?;

        info!(
            "Subscribed to {} in DATA_BROKER",
            SIGNAL_COMMAND_DOOR_RESPONSE
        );

        let (tx, rx) = mpsc::channel(32);
        let mut stream = response.into_inner();

        tokio::spawn(async move {
            while let Ok(Some(msg)) = stream.message().await {
                for (path, dp) in msg.entries {
                    if path != SIGNAL_COMMAND_DOOR_RESPONSE {
                        continue;
                    }
                    if let Some(value) = dp.value {
                        if let Some(kuksa::value::TypedValue::String(s)) = value.typed_value {
                            // Validate that the value is valid JSON before relaying
                            // Implements: [04-REQ-7.E1]
                            if serde_json::from_str::<serde_json::Value>(&s).is_err() {
                                error!(
                                    "Invalid JSON in command response from DATA_BROKER, skipping: {}",
                                    s
                                );
                                continue;
                            }

                            info!("Command response received from DATA_BROKER");
                            if tx.send(s).await.is_err() {
                                warn!("Response channel closed, stopping response subscription");
                                return;
                            }
                        }
                    }
                }
            }
            warn!("DATA_BROKER response subscription stream ended");
        });

        Ok(rx)
    }

    /// Subscribe to telemetry signals in DATA_BROKER.
    ///
    /// Subscribes to the following VSS signals:
    /// - `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked`
    /// - `Vehicle.CurrentLocation.Latitude`
    /// - `Vehicle.CurrentLocation.Longitude`
    /// - `Vehicle.Parking.SessionActive`
    ///
    /// Returns a channel receiver that yields [`SignalUpdate`] values whenever
    /// any subscribed signal changes.
    ///
    /// Implements: [04-REQ-3.2]
    pub async fn subscribe_telemetry(
        &self,
    ) -> Result<mpsc::Receiver<SignalUpdate>, BrokerError> {
        let request = SubscribeRequest {
            signal_paths: vec![
                SIGNAL_IS_LOCKED.to_string(),
                SIGNAL_LATITUDE.to_string(),
                SIGNAL_LONGITUDE.to_string(),
                SIGNAL_PARKING_ACTIVE.to_string(),
            ],
            buffer_size: 0,
        };

        let response = self
            .client
            .clone()
            .subscribe(request)
            .await
            .map_err(|e| {
                error!("Failed to subscribe to telemetry signals in DATA_BROKER: {}", e);
                BrokerError::SubscribeFailed(format!(
                    "subscribe to telemetry signals failed: {}",
                    e
                ))
            })?;

        info!(
            "Subscribed to telemetry signals in DATA_BROKER: {}, {}, {}, {}",
            SIGNAL_IS_LOCKED, SIGNAL_LATITUDE, SIGNAL_LONGITUDE, SIGNAL_PARKING_ACTIVE
        );

        let (tx, rx) = mpsc::channel(32);
        let mut stream = response.into_inner();

        tokio::spawn(async move {
            while let Ok(Some(msg)) = stream.message().await {
                for (path, dp) in msg.entries {
                    let typed_value = dp.value.and_then(|v| v.typed_value);
                    let update = match path.as_str() {
                        p if p == SIGNAL_IS_LOCKED => match typed_value {
                            Some(kuksa::value::TypedValue::Bool(v)) => {
                                Some(SignalUpdate::IsLocked(v))
                            }
                            _ => {
                                warn!(
                                    "Unexpected value type for {}: {:?}",
                                    SIGNAL_IS_LOCKED, typed_value
                                );
                                None
                            }
                        },
                        p if p == SIGNAL_LATITUDE => match typed_value {
                            Some(kuksa::value::TypedValue::Double(v)) => {
                                Some(SignalUpdate::Latitude(v))
                            }
                            Some(kuksa::value::TypedValue::Float(v)) => {
                                Some(SignalUpdate::Latitude(v as f64))
                            }
                            _ => {
                                warn!(
                                    "Unexpected value type for {}: {:?}",
                                    SIGNAL_LATITUDE, typed_value
                                );
                                None
                            }
                        },
                        p if p == SIGNAL_LONGITUDE => match typed_value {
                            Some(kuksa::value::TypedValue::Double(v)) => {
                                Some(SignalUpdate::Longitude(v))
                            }
                            Some(kuksa::value::TypedValue::Float(v)) => {
                                Some(SignalUpdate::Longitude(v as f64))
                            }
                            _ => {
                                warn!(
                                    "Unexpected value type for {}: {:?}",
                                    SIGNAL_LONGITUDE, typed_value
                                );
                                None
                            }
                        },
                        p if p == SIGNAL_PARKING_ACTIVE => match typed_value {
                            Some(kuksa::value::TypedValue::Bool(v)) => {
                                Some(SignalUpdate::ParkingActive(v))
                            }
                            _ => {
                                warn!(
                                    "Unexpected value type for {}: {:?}",
                                    SIGNAL_PARKING_ACTIVE, typed_value
                                );
                                None
                            }
                        },
                        other => {
                            warn!("Received update for unknown signal: {}", other);
                            None
                        }
                    };

                    if let Some(signal_update) = update {
                        info!("Telemetry signal update: {:?}", signal_update);
                        if tx.send(signal_update).await.is_err() {
                            warn!(
                                "Telemetry channel closed, stopping telemetry subscription"
                            );
                            return;
                        }
                    }
                }
            }
            warn!("DATA_BROKER telemetry subscription stream ended");
        });

        Ok(rx)
    }
}
