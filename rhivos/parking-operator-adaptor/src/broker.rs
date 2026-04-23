use std::fmt;
use std::time::Duration;

use tokio::sync::mpsc;
use tonic::transport::Channel;

/// Generated code from vendored kuksa.val.v1 proto files.
/// Retained for type re-exports (GetRequest, etc.) but no longer used
/// for RPC calls — v1 Set is non-functional in kuksa-databroker 0.5.0.
/// See docs/errata/04_kuksa_v2_api_migration.md.
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

/// Generated code from vendored parking_adaptor.v1 proto files.
pub mod parking_adaptor {
    pub mod v1 {
        tonic::include_proto!("parking_adaptor.v1");
    }
}

use kuksa::val::v2::{
    self,
    val_client::ValClient as V2Client,
    value::TypedValue,
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

/// gRPC client for the kuksa DATA_BROKER.
///
/// Uses the v2 API (`kuksa.val.v2.VAL`) for publishing values and
/// subscribing to signal changes. The v1 API's `Set` RPC is
/// non-functional in kuksa-databroker 0.5.0.
pub struct GrpcBrokerClient {
    client: V2Client<Channel>,
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
            match V2Client::connect(addr.to_string()).await {
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
    ///
    /// Uses the v2 Subscribe RPC which returns `map<string, Datapoint>`
    /// keyed by signal path.
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

    /// Build a v2 `SubscribeRequest` for a single VSS signal path.
    fn make_subscribe_request(signal: &str) -> v2::SubscribeRequest {
        v2::SubscribeRequest {
            signal_paths: vec![signal.to_string()],
            buffer_size: 0,
        }
    }

    /// Run the subscription loop, attempting to resubscribe on stream errors.
    async fn subscription_loop(
        client: &mut V2Client<Channel>,
        mut stream: tonic::Streaming<v2::SubscribeResponse>,
        signal: &str,
        tx: &mpsc::Sender<bool>,
    ) {
        let mut resubscribe_attempts = 0;

        loop {
            match stream.message().await {
                Ok(Some(response)) => {
                    // Reset resubscribe counter on successful message.
                    resubscribe_attempts = 0;
                    // v2 response.entries is a map<string, Datapoint>
                    if let Some(dp) = response.entries.get(signal) {
                        if let Some(v2::Value {
                            typed_value: Some(TypedValue::Bool(b)),
                        }) = &dp.value
                        {
                            if tx.send(*b).await.is_err() {
                                tracing::info!("subscription receiver dropped, stopping");
                                return;
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
        client: &mut V2Client<Channel>,
        signal: &str,
        stream: &mut tonic::Streaming<v2::SubscribeResponse>,
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
    /// Write a boolean signal value to DATA_BROKER using v2 PublishValue.
    ///
    /// The v1 Set RPC is non-functional in kuksa-databroker 0.5.0, so all
    /// writes use the v2 PublishValue RPC instead.
    async fn set_bool(&self, signal: &str, value: bool) -> Result<(), BrokerError> {
        let request = tonic::Request::new(v2::PublishValueRequest {
            signal_id: Some(v2::SignalId {
                signal: Some(v2::signal_id::Signal::Path(signal.to_string())),
            }),
            data_point: Some(v2::Datapoint {
                timestamp: None,
                value: Some(v2::Value {
                    typed_value: Some(TypedValue::Bool(value)),
                }),
            }),
        });

        self.client
            .clone()
            .publish_value(request)
            .await
            .map_err(|e| BrokerError::RpcFailed(e.to_string()))?;
        Ok(())
    }
}
