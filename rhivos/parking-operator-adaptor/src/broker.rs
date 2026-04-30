//! DATA_BROKER client abstraction.
//!
//! Wraps the tonic-generated kuksa.val.v2 gRPC client, providing typed
//! subscribe/set operations on VSS signals. Implements [`BrokerOps`]
//! for use in the event processing loop.

use crate::event_loop::BrokerOps;
use crate::proto::kuksa::val::v2;
use std::time::Duration;

/// Retry delays for DATA_BROKER connection (exponential backoff).
/// 5 total attempts with delays 1s, 2s, 4s, 8s between them.
const CONNECT_MAX_ATTEMPTS: u32 = 5;
const CONNECT_DELAYS_MS: [u64; 4] = [1000, 2000, 4000, 8000];

/// Error type for DATA_BROKER client operations.
#[derive(Debug)]
pub enum BrokerError {
    /// Failed to connect after all retry attempts.
    ConnectionFailed(String),
    /// A gRPC call to DATA_BROKER failed.
    RpcError(String),
}

impl std::fmt::Display for BrokerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BrokerError::ConnectionFailed(msg) => {
                write!(f, "DATA_BROKER connection failed: {msg}")
            }
            BrokerError::RpcError(msg) => {
                write!(f, "DATA_BROKER RPC error: {msg}")
            }
        }
    }
}

impl std::error::Error for BrokerError {}

/// DATA_BROKER gRPC client.
///
/// Provides typed operations for subscribing to VSS signals and
/// publishing signal values. Uses the kuksa.val.v2 gRPC API.
pub struct BrokerClient {
    client: tokio::sync::Mutex<v2::val_client::ValClient<tonic::transport::Channel>>,
}

impl BrokerClient {
    /// Connect to DATA_BROKER at the given address with retry.
    ///
    /// Retries connection up to 5 attempts with exponential backoff
    /// (1s, 2s, 4s, 8s). Returns error after all retries fail.
    pub async fn connect(addr: &str) -> Result<Self, BrokerError> {
        let mut last_error = String::new();

        for attempt in 0..CONNECT_MAX_ATTEMPTS {
            match v2::val_client::ValClient::connect(addr.to_string()).await {
                Ok(client) => {
                    tracing::info!(addr, "connected to DATA_BROKER");
                    return Ok(Self {
                        client: tokio::sync::Mutex::new(client),
                    });
                }
                Err(e) => {
                    last_error = e.to_string();
                    if attempt < CONNECT_MAX_ATTEMPTS - 1 {
                        let delay = Duration::from_millis(CONNECT_DELAYS_MS[attempt as usize]);
                        tracing::warn!(
                            attempt = attempt + 1,
                            max_attempts = CONNECT_MAX_ATTEMPTS,
                            error = %e,
                            delay_ms = delay.as_millis() as u64,
                            "retrying DATA_BROKER connection"
                        );
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        tracing::error!(
            error = %last_error,
            "DATA_BROKER connection failed after all retries"
        );
        Err(BrokerError::ConnectionFailed(last_error))
    }

    /// Subscribe to a boolean VSS signal.
    ///
    /// Returns an `mpsc::Receiver<bool>` that receives signal value
    /// updates. Spawns a background task to read the gRPC stream and
    /// forward values through the channel.
    pub async fn subscribe_bool(
        &self,
        signal: &str,
    ) -> Result<tokio::sync::mpsc::Receiver<bool>, BrokerError> {
        let request = v2::SubscribeRequest {
            signal_paths: vec![signal.to_string()],
            buffer_size: 0,
        };

        let response = self
            .client
            .lock()
            .await
            .subscribe(request)
            .await
            .map_err(|e| BrokerError::RpcError(format!("subscribe({signal}): {e}")))?;

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
                                if let Some(v2::value::TypedValue::Bool(b)) = &value.typed_value {
                                    if tx.send(*b).await.is_err() {
                                        tracing::debug!("subscriber channel closed");
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "subscription stream error");
                        break;
                    }
                }
            }
            tracing::debug!(signal = signal_path, "subscription stream ended");
        });

        Ok(rx)
    }

    /// Publish a boolean value to a VSS signal.
    ///
    /// Uses the kuksa.val.v2 `PublishValue` RPC.
    pub async fn publish_bool(
        &self,
        signal: &str,
        value: bool,
    ) -> Result<(), BrokerError> {
        let request = v2::PublishValueRequest {
            signal_id: Some(v2::SignalId {
                signal: Some(v2::signal_id::Signal::Path(signal.to_string())),
            }),
            data_point: Some(v2::Datapoint {
                timestamp: None,
                value: Some(v2::Value {
                    typed_value: Some(v2::value::TypedValue::Bool(value)),
                }),
            }),
        };

        self.client
            .lock()
            .await
            .publish_value(request)
            .await
            .map_err(|e| BrokerError::RpcError(format!("publish_bool({signal}): {e}")))?;

        Ok(())
    }
}

impl BrokerOps for BrokerClient {
    async fn set_bool(
        &self,
        signal: &str,
        value: bool,
    ) -> Result<(), String> {
        self.publish_bool(signal, value)
            .await
            .map_err(|e| e.to_string())
    }
}
