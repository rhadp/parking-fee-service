use std::fmt;
use std::time::Duration;

use tokio::sync::mpsc;
use tonic::transport::Channel;

/// Generated code from vendored kuksa.val.v1 proto files.
pub mod kuksa {
    pub mod val {
        pub mod v1 {
            tonic::include_proto!("kuksa.val.v1");
        }
    }
}

use kuksa::val::v1::{
    datapoint::Value, val_client::ValClient, DataEntry, Datapoint, EntryRequest, EntryUpdate,
    Field, GetRequest, SetRequest, SubscribeEntry, SubscribeRequest, View,
};

/// Error type for broker operations.
#[derive(Debug)]
pub enum BrokerError {
    /// Connection to DATA_BROKER failed.
    ConnectionFailed(String),
    /// A gRPC call failed.
    RpcFailed(String),
}

impl fmt::Display for BrokerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BrokerError::ConnectionFailed(msg) => write!(f, "connection failed: {msg}"),
            BrokerError::RpcFailed(msg) => write!(f, "rpc failed: {msg}"),
        }
    }
}

impl std::error::Error for BrokerError {}

/// VSS signal path for vehicle speed.
pub const SIGNAL_SPEED: &str = "Vehicle.Speed";
/// VSS signal path for door open state.
pub const SIGNAL_DOOR_OPEN: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen";
/// VSS signal path for door lock state.
pub const SIGNAL_IS_LOCKED: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked";
/// VSS signal path for incoming lock/unlock commands.
pub const SIGNAL_COMMAND: &str = "Vehicle.Command.Door.Lock";
/// VSS signal path for command responses.
pub const SIGNAL_RESPONSE: &str = "Vehicle.Command.Door.Response";

/// Abstraction over the DATA_BROKER gRPC client.
///
/// Uses async fn in trait (AFIT). The trait is used with concrete types
/// (generics, not dyn), so auto trait bounds on the returned futures are
/// not needed. The MockBrokerClient uses RefCell (not Send/Sync) and
/// requires single-threaded tokio runtime.
#[allow(async_fn_in_trait)]
pub trait BrokerClient {
    /// Read a float signal value from DATA_BROKER.
    async fn get_float(&self, signal: &str) -> Result<Option<f32>, BrokerError>;
    /// Read a boolean signal value from DATA_BROKER.
    async fn get_bool(&self, signal: &str) -> Result<Option<bool>, BrokerError>;
    /// Write a boolean signal value to DATA_BROKER.
    async fn set_bool(&self, signal: &str, value: bool) -> Result<(), BrokerError>;
    /// Write a string signal value to DATA_BROKER.
    async fn set_string(&self, signal: &str, value: &str) -> Result<(), BrokerError>;
}

/// Maximum number of connection attempts before giving up.
const MAX_CONNECT_ATTEMPTS: usize = 5;
/// Exponential backoff delays between connection attempts (seconds).
/// 5 attempts = 4 inter-attempt gaps: 1s, 2s, 4s, 8s.
const BACKOFF_DELAYS_SECS: [u64; 4] = [1, 2, 4, 8];

/// Maximum number of resubscription attempts after stream interruption.
const MAX_RESUBSCRIBE_ATTEMPTS: usize = 5;

/// gRPC client for the kuksa.val.v1 DATA_BROKER.
///
/// Wraps a tonic-generated `ValClient` and provides the `BrokerClient` trait
/// implementation plus connection retry and subscription management.
pub struct GrpcBrokerClient {
    client: ValClient<Channel>,
}

impl GrpcBrokerClient {
    /// Connect to DATA_BROKER with exponential backoff retry.
    ///
    /// Attempts up to `MAX_CONNECT_ATTEMPTS` times with delays of 1s, 2s, 4s, 8s
    /// between attempts. Returns `BrokerError::ConnectionFailed` if all attempts
    /// are exhausted.
    pub async fn connect(addr: &str) -> Result<Self, BrokerError> {
        let mut last_error = String::new();

        // First attempt (no delay), then retry with exponential backoff delays.
        for (attempt_num, delay) in std::iter::once(None)
            .chain(BACKOFF_DELAYS_SECS.iter().map(|&d| Some(d)))
            .take(MAX_CONNECT_ATTEMPTS)
            .enumerate()
        {
            if let Some(secs) = delay {
                tokio::time::sleep(Duration::from_secs(secs)).await;
            }
            match ValClient::connect(addr.to_string()).await {
                Ok(client) => {
                    tracing::info!("connected to DATA_BROKER on attempt {}", attempt_num + 1);
                    return Ok(Self { client });
                }
                Err(e) => {
                    last_error = e.to_string();
                    tracing::warn!(
                        "connection attempt {} failed: {e}",
                        attempt_num + 1
                    );
                }
            }
        }

        Err(BrokerError::ConnectionFailed(last_error))
    }

    /// Subscribe to a VSS signal, returning a channel receiver for incoming values.
    ///
    /// Spawns a background task that reads from the gRPC stream and forwards
    /// string values through the returned channel. On stream interruption,
    /// attempts to resubscribe up to `MAX_RESUBSCRIBE_ATTEMPTS` times.
    pub async fn subscribe(
        &self,
        signal: &str,
    ) -> Result<mpsc::Receiver<String>, BrokerError> {
        let request = Self::make_subscribe_request(signal);

        let stream = self
            .client
            .clone()
            .subscribe(tonic::Request::new(request))
            .await
            .map_err(|e| BrokerError::RpcFailed(e.to_string()))?
            .into_inner();

        let (tx, rx) = mpsc::channel(32);
        let mut client = self.client.clone();
        let signal = signal.to_string();

        tokio::spawn(async move {
            Self::subscription_loop(&mut client, stream, &signal, &tx).await;
        });

        Ok(rx)
    }

    /// Build a `SubscribeRequest` for a single VSS signal path.
    fn make_subscribe_request(signal: &str) -> SubscribeRequest {
        SubscribeRequest {
            entries: vec![SubscribeEntry {
                path: signal.to_string(),
                view: View::CurrentValue as i32,
                fields: vec![Field::Value as i32],
            }],
        }
    }

    /// Run the subscription loop, attempting to resubscribe on stream errors.
    async fn subscription_loop(
        client: &mut ValClient<Channel>,
        mut stream: tonic::Streaming<kuksa::val::v1::SubscribeResponse>,
        signal: &str,
        tx: &mpsc::Sender<String>,
    ) {
        let mut resubscribe_attempts = 0;

        loop {
            match stream.message().await {
                Ok(Some(response)) => {
                    // Reset resubscribe counter on successful message.
                    resubscribe_attempts = 0;
                    for update in response.updates {
                        if let Some(value) = update.value {
                            if let Some(Value::StringValue(s)) = value.value {
                                if tx.send(s).await.is_err() {
                                    tracing::info!("subscription receiver dropped, stopping");
                                    return;
                                }
                            }
                        }
                    }
                }
                Ok(None) => {
                    tracing::warn!("subscription stream ended");
                    if !Self::try_resubscribe(
                        client,
                        signal,
                        &mut stream,
                        &mut resubscribe_attempts,
                    )
                    .await
                    {
                        return;
                    }
                }
                Err(e) => {
                    tracing::warn!("subscription stream error: {e}");
                    if !Self::try_resubscribe(
                        client,
                        signal,
                        &mut stream,
                        &mut resubscribe_attempts,
                    )
                    .await
                    {
                        return;
                    }
                }
            }
        }
    }

    /// Attempt to resubscribe after a stream interruption.
    ///
    /// Returns `true` if resubscription succeeded, `false` if max attempts
    /// are exhausted.
    async fn try_resubscribe(
        client: &mut ValClient<Channel>,
        signal: &str,
        stream: &mut tonic::Streaming<kuksa::val::v1::SubscribeResponse>,
        attempts: &mut usize,
    ) -> bool {
        *attempts += 1;
        if *attempts > MAX_RESUBSCRIBE_ATTEMPTS {
            tracing::error!(
                "max resubscribe attempts ({MAX_RESUBSCRIBE_ATTEMPTS}) exhausted, giving up"
            );
            return false;
        }

        let delay = Duration::from_secs(BACKOFF_DELAYS_SECS[(*attempts - 1).min(3)]);
        tracing::warn!(
            "resubscribing (attempt {}/{}), waiting {delay:?}",
            attempts,
            MAX_RESUBSCRIBE_ATTEMPTS
        );
        tokio::time::sleep(delay).await;

        let request = Self::make_subscribe_request(signal);
        match client.subscribe(tonic::Request::new(request)).await {
            Ok(response) => {
                *stream = response.into_inner();
                tracing::info!("resubscribed successfully");
                true
            }
            Err(e) => {
                tracing::error!("resubscribe failed: {e}");
                // Recurse to try again (if attempts remain).
                Box::pin(Self::try_resubscribe(client, signal, stream, attempts)).await
            }
        }
    }
}

impl BrokerClient for GrpcBrokerClient {
    async fn get_float(&self, signal: &str) -> Result<Option<f32>, BrokerError> {
        let request = tonic::Request::new(GetRequest {
            entries: vec![EntryRequest {
                path: signal.to_string(),
                view: View::CurrentValue as i32,
                fields: vec![Field::Value as i32],
            }],
        });

        let response = self
            .client
            .clone()
            .get(request)
            .await
            .map_err(|e| BrokerError::RpcFailed(e.to_string()))?;

        let entries = response.into_inner().entries;
        if let Some(entry) = entries.first() {
            if let Some(ref dp) = entry.value {
                return Ok(match dp.value {
                    Some(Value::FloatValue(f)) => Some(f),
                    Some(Value::DoubleValue(d)) => Some(d as f32),
                    _ => None,
                });
            }
        }
        Ok(None)
    }

    async fn get_bool(&self, signal: &str) -> Result<Option<bool>, BrokerError> {
        let request = tonic::Request::new(GetRequest {
            entries: vec![EntryRequest {
                path: signal.to_string(),
                view: View::CurrentValue as i32,
                fields: vec![Field::Value as i32],
            }],
        });

        let response = self
            .client
            .clone()
            .get(request)
            .await
            .map_err(|e| BrokerError::RpcFailed(e.to_string()))?;

        let entries = response.into_inner().entries;
        if let Some(entry) = entries.first() {
            if let Some(ref dp) = entry.value {
                return Ok(match dp.value {
                    Some(Value::BoolValue(b)) => Some(b),
                    _ => None,
                });
            }
        }
        Ok(None)
    }

    async fn set_bool(&self, signal: &str, value: bool) -> Result<(), BrokerError> {
        let request = tonic::Request::new(SetRequest {
            updates: vec![EntryUpdate {
                entry: Some(DataEntry {
                    path: signal.to_string(),
                    value: Some(Datapoint {
                        timestamp: 0,
                        value: Some(Value::BoolValue(value)),
                    }),
                    actuator_target: None,
                }),
                fields: vec![Field::Value as i32],
            }],
        });

        self.client
            .clone()
            .set(request)
            .await
            .map_err(|e| BrokerError::RpcFailed(e.to_string()))?;
        Ok(())
    }

    async fn set_string(&self, signal: &str, value: &str) -> Result<(), BrokerError> {
        let request = tonic::Request::new(SetRequest {
            updates: vec![EntryUpdate {
                entry: Some(DataEntry {
                    path: signal.to_string(),
                    value: Some(Datapoint {
                        timestamp: 0,
                        value: Some(Value::StringValue(value.to_string())),
                    }),
                    actuator_target: None,
                }),
                fields: vec![Field::Value as i32],
            }],
        });

        self.client
            .clone()
            .set(request)
            .await
            .map_err(|e| BrokerError::RpcFailed(e.to_string()))?;
        Ok(())
    }
}
