use std::fmt;
use std::time::Duration;

use tokio::sync::mpsc;
use tonic::transport::Channel;

// Signal path constants
/// VSS signal for driver door lock state.
pub const SIGNAL_IS_LOCKED: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked";
/// Custom VSS signal for parking session active state.
pub const SIGNAL_SESSION_ACTIVE: &str = "Vehicle.Parking.SessionActive";

/// Generated kuksa.val.v2 gRPC types and client.
mod kuksa {
    pub mod val {
        pub mod v2 {
            tonic::include_proto!("kuksa.val.v2");
        }
    }
}

use self::kuksa::val::v2::{
    signal_id::Signal, val_client::ValClient, value::TypedValue, Datapoint,
    PublishValueRequest, SignalId, SubscribeRequest, Value,
};

/// Error type for DATA_BROKER operations.
#[derive(Debug)]
pub enum BrokerError {
    /// Connection failed.
    Connection(String),
    /// RPC call failed.
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

/// Trait for DATA_BROKER client operations.
///
/// Abstracting the broker client behind a trait allows event_loop tests
/// to inject a mock implementation.
pub trait DataBrokerClient {
    /// Set a boolean VSS signal value.
    fn set_bool(
        &self,
        signal: &str,
        value: bool,
    ) -> impl std::future::Future<Output = Result<(), BrokerError>>;
}

/// gRPC client for the Kuksa DATA_BROKER (v2 API).
///
/// Wraps a `ValClient<Channel>` (cheaply cloneable) and provides typed
/// operations for subscribing to boolean signals and publishing values.
pub struct GrpcBrokerClient {
    client: ValClient<Channel>,
}

impl GrpcBrokerClient {
    /// Connect to DATA_BROKER with exponential backoff retry.
    ///
    /// Makes up to 5 connection attempts with delays of 1s, 2s, 4s, 8s
    /// between attempts. Returns an error if all attempts fail.
    /// (08-REQ-3.E3)
    pub async fn connect(addr: &str) -> Result<Self, BrokerError> {
        let delays_secs: &[u64] = &[1, 2, 4, 8];
        let max_attempts = delays_secs.len() + 1; // 5 attempts
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

    /// Subscribe to a boolean VSS signal.
    ///
    /// Returns a channel receiver that yields boolean values whenever
    /// the signal changes. The subscription runs in a background task.
    /// (08-REQ-3.2)
    pub async fn subscribe_bool(
        &self,
        signal: &str,
    ) -> Result<mpsc::Receiver<bool>, BrokerError> {
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
                                if let Some(TypedValue::Bool(b)) = value.typed_value {
                                    if tx.send(b).await.is_err() {
                                        return;
                                    }
                                }
                            }
                        }
                    }
                    Ok(None) => {
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

impl DataBrokerClient for GrpcBrokerClient {
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
}
