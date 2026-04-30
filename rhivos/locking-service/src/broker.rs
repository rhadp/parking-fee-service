use std::fmt;

/// Generated gRPC types from kuksa.val.v2 proto.
pub mod kuksa {
    pub mod val {
        pub mod v2 {
            tonic::include_proto!("kuksa.val.v2");
        }
    }
}

/// Errors that can occur during DATA_BROKER communication.
#[derive(Debug)]
pub enum BrokerError {
    /// Connection to DATA_BROKER failed.
    ConnectionFailed(String),
    /// An RPC call to DATA_BROKER failed.
    RpcError(String),
}

impl fmt::Display for BrokerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BrokerError::ConnectionFailed(msg) => write!(f, "connection failed: {msg}"),
            BrokerError::RpcError(msg) => write!(f, "rpc error: {msg}"),
        }
    }
}

impl std::error::Error for BrokerError {}

/// VSS signal path constants.
pub const SIGNAL_SPEED: &str = "Vehicle.Speed";
pub const SIGNAL_DOOR_OPEN: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen";
pub const SIGNAL_IS_LOCKED: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked";
pub const SIGNAL_COMMAND: &str = "Vehicle.Command.Door.Lock";
pub const SIGNAL_RESPONSE: &str = "Vehicle.Command.Door.Response";

/// Abstraction over the DATA_BROKER gRPC client for testability.
pub trait BrokerClient {
    /// Read a float signal value from DATA_BROKER.
    fn get_float(
        &self,
        signal: &str,
    ) -> impl std::future::Future<Output = Result<Option<f32>, BrokerError>>;

    /// Read a boolean signal value from DATA_BROKER.
    fn get_bool(
        &self,
        signal: &str,
    ) -> impl std::future::Future<Output = Result<Option<bool>, BrokerError>>;

    /// Write a boolean signal value to DATA_BROKER.
    fn set_bool(
        &self,
        signal: &str,
        value: bool,
    ) -> impl std::future::Future<Output = Result<(), BrokerError>>;

    /// Write a string signal value to DATA_BROKER.
    fn set_string(
        &self,
        signal: &str,
        value: &str,
    ) -> impl std::future::Future<Output = Result<(), BrokerError>>;
}

/// gRPC-backed implementation of `BrokerClient` using kuksa.val.v2.
pub struct GrpcBrokerClient {
    client: tokio::sync::Mutex<kuksa::val::v2::val_client::ValClient<tonic::transport::Channel>>,
}

impl GrpcBrokerClient {
    /// Connect to DATA_BROKER with exponential backoff.
    ///
    /// Attempts up to 5 connections with delays of 1s, 2s, 4s, 8s between
    /// attempts. Returns `BrokerError::ConnectionFailed` if all attempts fail.
    pub async fn connect(addr: &str) -> Result<Self, BrokerError> {
        const MAX_ATTEMPTS: u32 = 5;
        let mut last_err = String::new();

        for attempt in 0..MAX_ATTEMPTS {
            match kuksa::val::v2::val_client::ValClient::connect(addr.to_string()).await {
                Ok(client) => {
                    tracing::info!("connected to DATA_BROKER at {addr}");
                    return Ok(Self {
                        client: tokio::sync::Mutex::new(client),
                    });
                }
                Err(e) => {
                    last_err = e.to_string();
                    if attempt < MAX_ATTEMPTS - 1 {
                        let delay = std::time::Duration::from_secs(1 << attempt);
                        tracing::warn!(
                            "DATA_BROKER connection attempt {} failed: {e}, retrying in {delay:?}",
                            attempt + 1
                        );
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        tracing::error!(
            "failed to connect to DATA_BROKER after {MAX_ATTEMPTS} attempts: {last_err}"
        );
        Err(BrokerError::ConnectionFailed(last_err))
    }

    /// Subscribe to a VSS signal, returning an mpsc receiver for string values.
    ///
    /// Spawns a background task that streams updates from the DATA_BROKER
    /// subscription and forwards string values through the channel.
    pub async fn subscribe(
        &self,
        signal: &str,
    ) -> Result<tokio::sync::mpsc::Receiver<String>, BrokerError> {
        let request = kuksa::val::v2::SubscribeRequest {
            signal_paths: vec![signal.to_string()],
            buffer_size: 0,
        };

        let response = self
            .client
            .lock()
            .await
            .subscribe(request)
            .await
            .map_err(|e| BrokerError::RpcError(format!("subscribe failed: {e}")))?;

        let mut stream = response.into_inner();
        let (tx, rx) = tokio::sync::mpsc::channel(32);
        let signal_path = signal.to_string();

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
                                            "subscription receiver dropped for {signal_path}"
                                        );
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("subscription stream error for {signal_path}: {e}");
                        break;
                    }
                }
            }
            tracing::info!("subscription stream ended for {signal_path}");
        });

        Ok(rx)
    }
}

impl BrokerClient for GrpcBrokerClient {
    async fn get_float(&self, signal: &str) -> Result<Option<f32>, BrokerError> {
        let request = kuksa::val::v2::GetValueRequest {
            signal_id: Some(kuksa::val::v2::SignalId {
                signal: Some(kuksa::val::v2::signal_id::Signal::Path(signal.to_string())),
            }),
        };

        let response = self
            .client
            .lock()
            .await
            .get_value(request)
            .await
            .map_err(|e| BrokerError::RpcError(format!("get_float({signal}): {e}")))?;

        let inner = response.into_inner();
        if let Some(dp) = inner.data_point {
            if let Some(value) = dp.value {
                return Ok(match value.typed_value {
                    Some(kuksa::val::v2::value::TypedValue::Float(f)) => Some(f),
                    Some(kuksa::val::v2::value::TypedValue::Double(d)) => Some(d as f32),
                    _ => None,
                });
            }
        }
        Ok(None)
    }

    async fn get_bool(&self, signal: &str) -> Result<Option<bool>, BrokerError> {
        let request = kuksa::val::v2::GetValueRequest {
            signal_id: Some(kuksa::val::v2::SignalId {
                signal: Some(kuksa::val::v2::signal_id::Signal::Path(signal.to_string())),
            }),
        };

        let response = self
            .client
            .lock()
            .await
            .get_value(request)
            .await
            .map_err(|e| BrokerError::RpcError(format!("get_bool({signal}): {e}")))?;

        let inner = response.into_inner();
        if let Some(dp) = inner.data_point {
            if let Some(value) = dp.value {
                return Ok(match value.typed_value {
                    Some(kuksa::val::v2::value::TypedValue::Bool(b)) => Some(b),
                    _ => None,
                });
            }
        }
        Ok(None)
    }

    async fn set_bool(&self, signal: &str, value: bool) -> Result<(), BrokerError> {
        let request = kuksa::val::v2::PublishValueRequest {
            signal_id: Some(kuksa::val::v2::SignalId {
                signal: Some(kuksa::val::v2::signal_id::Signal::Path(signal.to_string())),
            }),
            data_point: Some(kuksa::val::v2::Datapoint {
                timestamp: None,
                value: Some(kuksa::val::v2::Value {
                    typed_value: Some(kuksa::val::v2::value::TypedValue::Bool(value)),
                }),
            }),
        };

        self.client
            .lock()
            .await
            .publish_value(request)
            .await
            .map_err(|e| BrokerError::RpcError(format!("set_bool({signal}, {value}): {e}")))?;
        Ok(())
    }

    async fn set_string(&self, signal: &str, value: &str) -> Result<(), BrokerError> {
        let request = kuksa::val::v2::PublishValueRequest {
            signal_id: Some(kuksa::val::v2::SignalId {
                signal: Some(kuksa::val::v2::signal_id::Signal::Path(signal.to_string())),
            }),
            data_point: Some(kuksa::val::v2::Datapoint {
                timestamp: None,
                value: Some(kuksa::val::v2::Value {
                    typed_value: Some(kuksa::val::v2::value::TypedValue::String(
                        value.to_string(),
                    )),
                }),
            }),
        };

        self.client
            .lock()
            .await
            .publish_value(request)
            .await
            .map_err(|e| BrokerError::RpcError(format!("set_string({signal}): {e}")))?;
        Ok(())
    }
}
