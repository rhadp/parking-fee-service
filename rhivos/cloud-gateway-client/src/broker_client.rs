use crate::config::Config;
use crate::errors::BrokerError;
use crate::models::SignalUpdate;

/// Generated gRPC types from kuksa.val.v2 proto.
pub mod kuksa {
    pub mod val {
        pub mod v2 {
            tonic::include_proto!("kuksa.val.v2");
        }
    }
}

/// VSS signal path constants.
pub const SIGNAL_IS_LOCKED: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked";
pub const SIGNAL_LATITUDE: &str = "Vehicle.CurrentLocation.Latitude";
pub const SIGNAL_LONGITUDE: &str = "Vehicle.CurrentLocation.Longitude";
pub const SIGNAL_PARKING_ACTIVE: &str = "Vehicle.Parking.SessionActive";
pub const SIGNAL_COMMAND: &str = "Vehicle.Command.Door.Lock";
pub const SIGNAL_RESPONSE: &str = "Vehicle.Command.Door.Response";

/// Telemetry signal paths subscribed for aggregated publishing.
pub const TELEMETRY_SIGNALS: &[&str] = &[
    SIGNAL_IS_LOCKED,
    SIGNAL_LATITUDE,
    SIGNAL_LONGITUDE,
    SIGNAL_PARKING_ACTIVE,
];

/// Client for DATA_BROKER gRPC operations.
///
/// Wraps a `kuksa.val.v2.VAL` gRPC client behind a tokio Mutex and
/// provides domain-specific methods for writing command signals,
/// subscribing to telemetry signals, and subscribing to command
/// response signals.
pub struct BrokerClient {
    client: tokio::sync::Mutex<kuksa::val::v2::val_client::ValClient<tonic::transport::Channel>>,
}

impl BrokerClient {
    /// Connect to the DATA_BROKER at the configured address.
    ///
    /// Returns `Err(BrokerError::ConnectionFailed)` if the connection
    /// cannot be established.
    ///
    /// # Requirements
    /// - 04-REQ-3.1: Establish gRPC connection to DATA_BROKER
    /// - 04-REQ-3.E1: Exit with code 1 on connection failure
    pub async fn connect(config: &Config) -> Result<Self, BrokerError> {
        let addr = &config.databroker_addr;
        match kuksa::val::v2::val_client::ValClient::connect(addr.to_string()).await {
            Ok(client) => {
                tracing::info!(
                    addr = %addr,
                    "Connected to DATA_BROKER"
                );
                Ok(Self {
                    client: tokio::sync::Mutex::new(client),
                })
            }
            Err(e) => {
                tracing::error!(
                    addr = %addr,
                    error = %e,
                    "DATA_BROKER connection failed"
                );
                Err(BrokerError::ConnectionFailed(e.to_string()))
            }
        }
    }

    /// Write a command payload as a string to `Vehicle.Command.Door.Lock`.
    ///
    /// The payload is written as-is (passthrough fidelity). The service
    /// does not modify, enrich, or strip fields from the original command.
    ///
    /// # Requirements
    /// - 04-REQ-6.3: Write command payload as-is to DATA_BROKER via gRPC SetRequest
    pub async fn write_command(&self, payload: &str) -> Result<(), BrokerError> {
        let request = kuksa::val::v2::PublishValueRequest {
            signal_id: Some(kuksa::val::v2::SignalId {
                signal: Some(kuksa::val::v2::signal_id::Signal::Path(
                    SIGNAL_COMMAND.to_string(),
                )),
            }),
            data_point: Some(kuksa::val::v2::Datapoint {
                timestamp: None,
                value: Some(kuksa::val::v2::Value {
                    typed_value: Some(kuksa::val::v2::value::TypedValue::String(
                        payload.to_string(),
                    )),
                }),
            }),
        };

        self.client
            .lock()
            .await
            .publish_value(request)
            .await
            .map_err(|e| BrokerError::WriteFailed(format!("write_command: {e}")))?;

        tracing::info!("Command written to DATA_BROKER");
        Ok(())
    }

    /// Subscribe to `Vehicle.Command.Door.Response` and return a receiver
    /// for string values.
    ///
    /// Spawns a background task that streams updates from the DATA_BROKER
    /// subscription and forwards string values through a channel.
    ///
    /// # Requirements
    /// - 04-REQ-3.3: Subscribe to `Vehicle.Command.Door.Response`
    pub async fn subscribe_responses(
        &self,
    ) -> Result<tokio::sync::mpsc::Receiver<String>, BrokerError> {
        let request = kuksa::val::v2::SubscribeRequest {
            signal_paths: vec![SIGNAL_RESPONSE.to_string()],
            buffer_size: 0,
        };

        let response = self
            .client
            .lock()
            .await
            .subscribe(request)
            .await
            .map_err(|e| BrokerError::SubscribeFailed(format!("subscribe_responses: {e}")))?;

        let mut stream = response.into_inner();
        let (tx, rx) = tokio::sync::mpsc::channel(32);
        let signal_path = SIGNAL_RESPONSE.to_string();

        tokio::spawn(async move {
            use futures::StreamExt;
            while let Some(result) = stream.next().await {
                match result {
                    Ok(subscribe_response) => {
                        if let Some(datapoint) = subscribe_response.entries.get(&signal_path) {
                            if let Some(value) = &datapoint.value {
                                if let Some(kuksa::val::v2::value::TypedValue::String(s)) =
                                    &value.typed_value
                                {
                                    if tx.send(s.clone()).await.is_err() {
                                        tracing::debug!(
                                            "Response subscription receiver dropped"
                                        );
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            error = %e,
                            "Response subscription stream error"
                        );
                        break;
                    }
                }
            }
            tracing::info!("Response subscription stream ended");
        });

        tracing::info!(
            signal = SIGNAL_RESPONSE,
            "Subscribed to command responses"
        );
        Ok(rx)
    }

    /// Subscribe to telemetry signals and return a receiver for signal updates.
    ///
    /// Subscribes to: IsLocked, Latitude, Longitude, SessionActive.
    /// Spawns a background task that parses incoming datapoints into
    /// `SignalUpdate` variants and forwards them through a channel.
    ///
    /// # Requirements
    /// - 04-REQ-3.2: Subscribe to telemetry VSS signals
    pub async fn subscribe_telemetry(
        &self,
    ) -> Result<tokio::sync::mpsc::Receiver<SignalUpdate>, BrokerError> {
        let request = kuksa::val::v2::SubscribeRequest {
            signal_paths: TELEMETRY_SIGNALS.iter().map(|s| s.to_string()).collect(),
            buffer_size: 0,
        };

        let response = self
            .client
            .lock()
            .await
            .subscribe(request)
            .await
            .map_err(|e| BrokerError::SubscribeFailed(format!("subscribe_telemetry: {e}")))?;

        let mut stream = response.into_inner();
        let (tx, rx) = tokio::sync::mpsc::channel(32);

        tokio::spawn(async move {
            use futures::StreamExt;
            while let Some(result) = stream.next().await {
                match result {
                    Ok(subscribe_response) => {
                        for (path, datapoint) in &subscribe_response.entries {
                            if let Some(value) = &datapoint.value {
                                let update = match path.as_str() {
                                    SIGNAL_IS_LOCKED => {
                                        if let Some(
                                            kuksa::val::v2::value::TypedValue::Bool(v),
                                        ) = &value.typed_value
                                        {
                                            Some(SignalUpdate::IsLocked(*v))
                                        } else {
                                            None
                                        }
                                    }
                                    SIGNAL_LATITUDE => {
                                        if let Some(
                                            kuksa::val::v2::value::TypedValue::Double(v),
                                        ) = &value.typed_value
                                        {
                                            Some(SignalUpdate::Latitude(*v))
                                        } else {
                                            None
                                        }
                                    }
                                    SIGNAL_LONGITUDE => {
                                        if let Some(
                                            kuksa::val::v2::value::TypedValue::Double(v),
                                        ) = &value.typed_value
                                        {
                                            Some(SignalUpdate::Longitude(*v))
                                        } else {
                                            None
                                        }
                                    }
                                    SIGNAL_PARKING_ACTIVE => {
                                        if let Some(
                                            kuksa::val::v2::value::TypedValue::Bool(v),
                                        ) = &value.typed_value
                                        {
                                            Some(SignalUpdate::ParkingActive(*v))
                                        } else {
                                            None
                                        }
                                    }
                                    _ => None,
                                };

                                if let Some(update) = update {
                                    if tx.send(update).await.is_err() {
                                        tracing::debug!(
                                            "Telemetry subscription receiver dropped"
                                        );
                                        return;
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            error = %e,
                            "Telemetry subscription stream error"
                        );
                        break;
                    }
                }
            }
            tracing::info!("Telemetry subscription stream ended");
        });

        tracing::info!(
            signals = ?TELEMETRY_SIGNALS,
            "Subscribed to telemetry signals"
        );
        Ok(rx)
    }
}
