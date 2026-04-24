use std::fmt;
use std::time::Duration;

// Signal path constants
pub const SIGNAL_COMMAND: &str = "Vehicle.Command.Door.Lock";
pub const SIGNAL_SPEED: &str = "Vehicle.Speed";
pub const SIGNAL_IS_OPEN: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen";
pub const SIGNAL_IS_LOCKED: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked";
pub const SIGNAL_RESPONSE: &str = "Vehicle.Command.Door.Response";

/// Generated kuksa.val.v2 gRPC types and client.
mod kuksa {
    pub mod val {
        pub mod v2 {
            tonic::include_proto!("kuksa.val.v2");
        }
    }
}

use kuksa::val::v2::{
    signal_id::Signal, val_client::ValClient, value::TypedValue, Datapoint, GetValueRequest,
    PublishValueRequest, SignalId, SubscribeRequest, Value,
};
use tokio::sync::mpsc;
use tonic::transport::Channel;

#[derive(Debug)]
pub enum BrokerError {
    Connection(String),
    Rpc(String),
}

impl fmt::Display for BrokerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BrokerError::Connection(msg) => write!(f, "connection error: {msg}"),
            BrokerError::Rpc(msg) => write!(f, "RPC error: {msg}"),
        }
    }
}

impl std::error::Error for BrokerError {}

pub trait BrokerClient {
    fn get_float(
        &self,
        signal: &str,
    ) -> impl std::future::Future<Output = Result<Option<f32>, BrokerError>>;

    fn get_bool(
        &self,
        signal: &str,
    ) -> impl std::future::Future<Output = Result<Option<bool>, BrokerError>>;

    fn set_bool(
        &self,
        signal: &str,
        value: bool,
    ) -> impl std::future::Future<Output = Result<(), BrokerError>>;

    fn set_string(
        &self,
        signal: &str,
        value: &str,
    ) -> impl std::future::Future<Output = Result<(), BrokerError>>;
}

/// gRPC client for communicating with DATA_BROKER using kuksa.val.v2 API.
///
/// The client wraps a tonic `ValClient<Channel>` which is cheaply cloneable.
/// Each trait method clones the inner client to satisfy tonic's `&mut self`
/// requirement while keeping the `BrokerClient` trait methods on `&self`.
pub struct GrpcBrokerClient {
    client: ValClient<Channel>,
}

impl GrpcBrokerClient {
    /// Connect to DATA_BROKER with exponential backoff retry.
    ///
    /// Attempts up to 5 connections with delays of 1s, 2s, 4s, 8s between
    /// attempts. Returns a connected client or an error after exhausting retries.
    pub async fn connect(addr: &str) -> Result<Self, BrokerError> {
        let delays_secs: &[u64] = &[1, 2, 4, 8];
        let max_attempts = delays_secs.len() + 1; // 5 attempts, 4 delays between them
        let mut last_err = String::new();

        let endpoint = tonic::transport::Endpoint::from_shared(addr.to_string())
            .map_err(|e| BrokerError::Connection(e.to_string()))?
            .connect_timeout(Duration::from_secs(5));

        for attempt in 0..max_attempts {
            match endpoint.connect().await {
                Ok(channel) => {
                    tracing::info!(addr, "connected to DATA_BROKER");
                    return Ok(Self {
                        client: ValClient::new(channel),
                    });
                }
                Err(e) => {
                    last_err = e.to_string();
                    if let Some(&delay) = delays_secs.get(attempt) {
                        tracing::warn!(
                            attempt = attempt + 1,
                            max_attempts,
                            delay_secs = delay,
                            error = %e,
                            "DATA_BROKER connection failed, retrying"
                        );
                        tokio::time::sleep(Duration::from_secs(delay)).await;
                    }
                }
            }
        }

        Err(BrokerError::Connection(format!(
            "failed to connect after {max_attempts} attempts: {last_err}"
        )))
    }

    /// Subscribe to a VSS signal, returning an mpsc receiver that streams
    /// string values from the subscription.
    ///
    /// Spawns a background task that reads from the gRPC stream and forwards
    /// string-typed datapoint values to the returned channel. The channel is
    /// closed when the stream ends or errors.
    pub async fn subscribe(
        &self,
        signal: &str,
    ) -> Result<mpsc::Receiver<String>, BrokerError> {
        let mut client = self.client.clone();
        let request = SubscribeRequest {
            signal_paths: vec![signal.to_string()],
            buffer_size: 0,
        };

        let response = client
            .subscribe(request)
            .await
            .map_err(|e| BrokerError::Rpc(e.to_string()))?;

        let mut stream = response.into_inner();
        let (tx, rx) = mpsc::channel(32);
        let signal_path = signal.to_string();

        tokio::spawn(async move {
            loop {
                match stream.message().await {
                    Ok(Some(subscribe_response)) => {
                        for (_path, datapoint) in subscribe_response.entries {
                            if let Some(value) = datapoint.value {
                                if let Some(TypedValue::String(s)) = value.typed_value {
                                    if tx.send(s).await.is_err() {
                                        // Receiver dropped, stop reading
                                        return;
                                    }
                                }
                            }
                        }
                    }
                    Ok(None) => {
                        // Stream ended normally
                        tracing::warn!(signal = %signal_path, "subscription stream ended");
                        break;
                    }
                    Err(e) => {
                        tracing::error!(
                            signal = %signal_path,
                            error = %e,
                            "subscription stream error"
                        );
                        break;
                    }
                }
            }
        });

        Ok(rx)
    }
}

impl BrokerClient for GrpcBrokerClient {
    async fn get_float(&self, signal: &str) -> Result<Option<f32>, BrokerError> {
        let mut client = self.client.clone();
        let request = GetValueRequest {
            signal_id: Some(SignalId {
                signal: Some(Signal::Path(signal.to_string())),
            }),
        };

        let response = client
            .get_value(request)
            .await
            .map_err(|e| BrokerError::Rpc(e.to_string()))?;

        let dp = response.into_inner().data_point;
        match dp.and_then(|d| d.value).and_then(|v| v.typed_value) {
            Some(TypedValue::Float(f)) => Ok(Some(f)),
            Some(TypedValue::Double(d)) => Ok(Some(d as f32)),
            None => Ok(None),
            _ => Ok(None),
        }
    }

    async fn get_bool(&self, signal: &str) -> Result<Option<bool>, BrokerError> {
        let mut client = self.client.clone();
        let request = GetValueRequest {
            signal_id: Some(SignalId {
                signal: Some(Signal::Path(signal.to_string())),
            }),
        };

        let response = client
            .get_value(request)
            .await
            .map_err(|e| BrokerError::Rpc(e.to_string()))?;

        let dp = response.into_inner().data_point;
        match dp.and_then(|d| d.value).and_then(|v| v.typed_value) {
            Some(TypedValue::Bool(b)) => Ok(Some(b)),
            None => Ok(None),
            _ => Ok(None),
        }
    }

    async fn set_bool(&self, signal: &str, value: bool) -> Result<(), BrokerError> {
        let mut client = self.client.clone();
        let request = PublishValueRequest {
            signal_id: Some(SignalId {
                signal: Some(Signal::Path(signal.to_string())),
            }),
            data_point: Some(Datapoint {
                timestamp: None,
                value: Some(Value {
                    typed_value: Some(TypedValue::Bool(value)),
                }),
            }),
        };

        client
            .publish_value(request)
            .await
            .map_err(|e| BrokerError::Rpc(e.to_string()))?;
        Ok(())
    }

    async fn set_string(&self, signal: &str, value: &str) -> Result<(), BrokerError> {
        let mut client = self.client.clone();
        let request = PublishValueRequest {
            signal_id: Some(SignalId {
                signal: Some(Signal::Path(signal.to_string())),
            }),
            data_point: Some(Datapoint {
                timestamp: None,
                value: Some(Value {
                    typed_value: Some(TypedValue::String(value.to_string())),
                }),
            }),
        };

        client
            .publish_value(request)
            .await
            .map_err(|e| BrokerError::Rpc(e.to_string()))?;
        Ok(())
    }
}
