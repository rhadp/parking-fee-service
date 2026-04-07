use std::fmt;

/// Error type for DATA_BROKER operations.
#[derive(Debug)]
pub enum BrokerError {
    /// Connection to DATA_BROKER failed.
    ConnectionFailed(String),
    /// A DATA_BROKER operation failed.
    OperationFailed(String),
}

impl fmt::Display for BrokerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BrokerError::ConnectionFailed(msg) => write!(f, "connection failed: {msg}"),
            BrokerError::OperationFailed(msg) => write!(f, "operation failed: {msg}"),
        }
    }
}

impl std::error::Error for BrokerError {}

/// Trait abstracting DATA_BROKER gRPC client operations.
#[allow(async_fn_in_trait)]
pub trait DataBrokerClient {
    /// Set a boolean signal value in DATA_BROKER.
    async fn set_bool(&self, signal: &str, value: bool) -> Result<(), BrokerError>;
}

/// Backoff delays for DATA_BROKER connection retries: 1s, 2s, 4s.
const BROKER_BACKOFF_MS: [u64; 5] = [1000, 2000, 4000, 4000, 4000];

/// Maximum connection attempts for DATA_BROKER.
const MAX_CONNECT_ATTEMPTS: usize = 5;

/// Live DATA_BROKER gRPC client wrapping the kuksa.val.v2 API.
pub struct KuksaBrokerClient {
    client: crate::proto::kuksa::val_client::ValClient<tonic::transport::Channel>,
}

impl KuksaBrokerClient {
    /// Connect to DATA_BROKER with retry logic.
    ///
    /// Retries up to 5 times with exponential backoff (1s, 2s, 4s, 4s, 4s).
    /// Returns `BrokerError::ConnectionFailed` after all attempts fail.
    pub async fn connect(addr: &str) -> Result<Self, BrokerError> {
        let mut last_error = String::new();

        for attempt in 0..MAX_CONNECT_ATTEMPTS {
            if attempt > 0 {
                let delay_ms = BROKER_BACKOFF_MS[attempt - 1];
                tracing::warn!(
                    attempt = attempt + 1,
                    delay_ms,
                    "retrying DATA_BROKER connection"
                );
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            }

            match crate::proto::kuksa::val_client::ValClient::connect(addr.to_string()).await {
                Ok(client) => {
                    tracing::info!("connected to DATA_BROKER at {addr}");
                    return Ok(Self { client });
                }
                Err(e) => {
                    last_error = format!("{e}");
                    tracing::warn!(
                        attempt = attempt + 1,
                        error = %e,
                        "DATA_BROKER connection failed"
                    );
                }
            }
        }

        Err(BrokerError::ConnectionFailed(format!(
            "failed to connect after {MAX_CONNECT_ATTEMPTS} attempts: {last_error}"
        )))
    }

    /// Subscribe to a boolean signal in DATA_BROKER.
    ///
    /// Returns a tonic streaming response of `SubscribeResponse` messages.
    pub async fn subscribe(
        &mut self,
        signal_path: &str,
    ) -> Result<tonic::Streaming<crate::proto::kuksa::SubscribeResponse>, BrokerError> {
        let request = crate::proto::kuksa::SubscribeRequest {
            signal_paths: vec![signal_path.to_string()],
            buffer_size: 0,
        };

        let response = self
            .client
            .subscribe(request)
            .await
            .map_err(|e| BrokerError::OperationFailed(format!("subscribe failed: {e}")))?;

        Ok(response.into_inner())
    }
}

impl DataBrokerClient for KuksaBrokerClient {
    async fn set_bool(&self, signal: &str, value: bool) -> Result<(), BrokerError> {
        let request = crate::proto::kuksa::PublishValueRequest {
            signal_id: Some(crate::proto::kuksa::SignalId {
                signal: Some(crate::proto::kuksa::signal_id::Signal::Path(
                    signal.to_string(),
                )),
            }),
            data_point: Some(crate::proto::kuksa::Datapoint {
                timestamp: None,
                value: Some(crate::proto::kuksa::Value {
                    typed_value: Some(crate::proto::kuksa::value::TypedValue::Bool(value)),
                }),
            }),
        };

        let mut client = self.client.clone();
        client
            .publish_value(request)
            .await
            .map_err(|e| BrokerError::OperationFailed(format!("set_bool failed: {e}")))?;

        Ok(())
    }
}
