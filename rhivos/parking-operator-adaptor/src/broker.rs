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

/// Generated code from vendored parking_adaptor.v1 proto files.
pub mod parking_adaptor {
    pub mod v1 {
        tonic::include_proto!("parking_adaptor.v1");
    }
}

use kuksa::val::v1::{
    datapoint::Value, val_client::ValClient, DataEntry, Datapoint, EntryUpdate, Field, SetRequest,
    SubscribeEntry, SubscribeRequest, View,
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

/// VSS signal path for door lock state.
pub const SIGNAL_IS_LOCKED: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked";
/// VSS signal path for parking session active state.
pub const SIGNAL_SESSION_ACTIVE: &str = "Vehicle.Parking.SessionActive";

/// Abstraction over the DATA_BROKER gRPC client.
///
/// Uses async fn in trait (AFIT). The trait is used with concrete types
/// (generics, not dyn), so auto trait bounds on the returned futures are
/// not needed. The MockBrokerClient uses RefCell (not Send/Sync) and
/// requires single-threaded tokio runtime.
#[allow(async_fn_in_trait)]
pub trait BrokerClient {
    /// Write a boolean signal value to DATA_BROKER.
    async fn set_bool(&self, signal: &str, value: bool) -> Result<(), BrokerError>;
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
    /// are exhausted. (08-REQ-3.E3)
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
                        attempt = attempt_num + 1,
                        max_attempts = MAX_CONNECT_ATTEMPTS,
                        error = %e,
                        "DATA_BROKER connection attempt failed"
                    );
                }
            }
        }

        Err(BrokerError::ConnectionFailed(last_error))
    }

    /// Subscribe to a boolean VSS signal, returning a channel receiver.
    ///
    /// Spawns a background task that reads from the gRPC stream and forwards
    /// boolean values through the returned channel. On stream interruption,
    /// attempts to resubscribe up to `MAX_RESUBSCRIBE_ATTEMPTS` times.
    /// (08-REQ-3.2)
    pub async fn subscribe_bool(
        &self,
        signal: &str,
    ) -> Result<mpsc::Receiver<bool>, BrokerError> {
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
        tx: &mpsc::Sender<bool>,
    ) {
        let mut resubscribe_attempts = 0;

        loop {
            match stream.message().await {
                Ok(Some(response)) => {
                    // Reset resubscribe counter on successful message.
                    resubscribe_attempts = 0;
                    for update in response.updates {
                        if let Some(value) = update.value {
                            if let Some(Value::BoolValue(b)) = value.value {
                                if tx.send(b).await.is_err() {
                                    tracing::info!("subscription receiver dropped, stopping");
                                    return;
                                }
                            }
                        }
                    }
                }
                Ok(None) => {
                    tracing::warn!("subscription stream ended for {signal}");
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
                    tracing::warn!(error = %e, "subscription stream error for {signal}");
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
            attempt = *attempts,
            max_attempts = MAX_RESUBSCRIBE_ATTEMPTS,
            "resubscribing, waiting {delay:?}",
        );
        tokio::time::sleep(delay).await;

        let request = Self::make_subscribe_request(signal);
        match client.subscribe(tonic::Request::new(request)).await {
            Ok(response) => {
                *stream = response.into_inner();
                tracing::info!("resubscribed to {signal} successfully");
                true
            }
            Err(e) => {
                tracing::error!(error = %e, "resubscribe to {signal} failed");
                // Recurse to try again (if attempts remain).
                Box::pin(Self::try_resubscribe(client, signal, stream, attempts)).await
            }
        }
    }
}

impl BrokerClient for GrpcBrokerClient {
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
}
