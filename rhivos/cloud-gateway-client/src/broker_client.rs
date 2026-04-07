//! DATA_BROKER gRPC client module for cloud-gateway-client.
//!
//! Manages the DATA_BROKER gRPC connection. Provides methods to write command
//! signals, subscribe to telemetry signals, and subscribe to command response
//! signals. Encapsulates all tonic gRPC usage.

use crate::config::Config;
use crate::errors::BrokerError;
use crate::models::SignalUpdate;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

/// Generated gRPC types for kuksa.val.
#[allow(clippy::enum_variant_names)]
mod kuksa {
    tonic::include_proto!("kuksa");
}

use kuksa::val_client::ValClient;
use kuksa::{DataEntry, Datapoint, SetRequest, SubscribeRequest};
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
    /// The payload is written as a string value via gRPC SetRequest. The
    /// original JSON is passed through without modification.
    ///
    /// Implements: [04-REQ-6.3]
    pub async fn write_command(&self, payload: &str) -> Result<(), BrokerError> {
        let request = SetRequest {
            entries: vec![DataEntry {
                path: SIGNAL_COMMAND_DOOR_LOCK.to_string(),
                value: Some(Datapoint {
                    timestamp: 0,
                    value: Some(kuksa::datapoint::Value::StringValue(payload.to_string())),
                }),
            }],
        };

        let response = self
            .client
            .clone()
            .set(request)
            .await
            .map_err(|e| {
                error!("Failed to write command to DATA_BROKER: {}", e);
                BrokerError::WriteFailed(format!("set command failed: {}", e))
            })?;

        let set_resp = response.into_inner();
        if !set_resp.success {
            error!(
                "DATA_BROKER rejected command write: {}",
                set_resp.error
            );
            return Err(BrokerError::WriteFailed(format!(
                "DATA_BROKER rejected write: {}",
                set_resp.error
            )));
        }

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
            paths: vec![SIGNAL_COMMAND_DOOR_RESPONSE.to_string()],
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
                for entry in msg.entries {
                    if let Some(dp) = entry.value {
                        if let Some(kuksa::datapoint::Value::StringValue(s)) = dp.value {
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
            paths: vec![
                SIGNAL_IS_LOCKED.to_string(),
                SIGNAL_LATITUDE.to_string(),
                SIGNAL_LONGITUDE.to_string(),
                SIGNAL_PARKING_ACTIVE.to_string(),
            ],
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
                for entry in msg.entries {
                    if let Some(dp) = entry.value {
                        let update = match entry.path.as_str() {
                            p if p == SIGNAL_IS_LOCKED => {
                                if let Some(kuksa::datapoint::Value::BoolValue(v)) = dp.value {
                                    Some(SignalUpdate::IsLocked(v))
                                } else {
                                    warn!(
                                        "Unexpected value type for {}: {:?}",
                                        SIGNAL_IS_LOCKED, dp.value
                                    );
                                    None
                                }
                            }
                            p if p == SIGNAL_LATITUDE => match dp.value {
                                Some(kuksa::datapoint::Value::DoubleValue(v)) => {
                                    Some(SignalUpdate::Latitude(v))
                                }
                                Some(kuksa::datapoint::Value::FloatValue(v)) => {
                                    Some(SignalUpdate::Latitude(v as f64))
                                }
                                _ => {
                                    warn!(
                                        "Unexpected value type for {}: {:?}",
                                        SIGNAL_LATITUDE, dp.value
                                    );
                                    None
                                }
                            },
                            p if p == SIGNAL_LONGITUDE => match dp.value {
                                Some(kuksa::datapoint::Value::DoubleValue(v)) => {
                                    Some(SignalUpdate::Longitude(v))
                                }
                                Some(kuksa::datapoint::Value::FloatValue(v)) => {
                                    Some(SignalUpdate::Longitude(v as f64))
                                }
                                _ => {
                                    warn!(
                                        "Unexpected value type for {}: {:?}",
                                        SIGNAL_LONGITUDE, dp.value
                                    );
                                    None
                                }
                            },
                            p if p == SIGNAL_PARKING_ACTIVE => {
                                if let Some(kuksa::datapoint::Value::BoolValue(v)) = dp.value {
                                    Some(SignalUpdate::ParkingActive(v))
                                } else {
                                    warn!(
                                        "Unexpected value type for {}: {:?}",
                                        SIGNAL_PARKING_ACTIVE, dp.value
                                    );
                                    None
                                }
                            }
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
            }
            warn!("DATA_BROKER telemetry subscription stream ended");
        });

        Ok(rx)
    }
}
