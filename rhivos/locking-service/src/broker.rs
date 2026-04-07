/// Error type for broker operations.
#[derive(Debug)]
pub enum BrokerError {
    /// Connection failed.
    ConnectionFailed(String),
    /// Operation failed.
    OperationFailed(String),
}

impl std::fmt::Display for BrokerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BrokerError::ConnectionFailed(msg) => write!(f, "connection failed: {}", msg),
            BrokerError::OperationFailed(msg) => write!(f, "operation failed: {}", msg),
        }
    }
}

impl std::error::Error for BrokerError {}

/// Trait abstracting DATA_BROKER gRPC client operations.
#[allow(async_fn_in_trait)]
pub trait BrokerClient {
    /// Read a float signal value.
    async fn get_float(&self, signal: &str) -> Result<Option<f32>, BrokerError>;
    /// Read a boolean signal value.
    async fn get_bool(&self, signal: &str) -> Result<Option<bool>, BrokerError>;
    /// Write a boolean signal value.
    async fn set_bool(&self, signal: &str, value: bool) -> Result<(), BrokerError>;
    /// Write a string signal value.
    async fn set_string(&self, signal: &str, value: &str) -> Result<(), BrokerError>;
}

/// Generated gRPC types for kuksa.val.v1.
pub mod kuksa {
    tonic::include_proto!("kuksa");
}

use kuksa::val_client::ValClient;
use kuksa::{DataEntry, Datapoint, GetRequest, SetRequest, SubscribeRequest};
use tokio::sync::mpsc;
use tonic::transport::Channel;
use tracing::{info, warn};

/// gRPC-backed broker client for DATA_BROKER communication.
pub struct GrpcBrokerClient {
    client: ValClient<Channel>,
}

impl GrpcBrokerClient {
    /// Connect to DATA_BROKER with exponential backoff (5 attempts).
    ///
    /// Delays between attempts: 1s, 2s, 4s, 8s.
    pub async fn connect(addr: &str) -> Result<Self, BrokerError> {
        let max_attempts = 5;
        let mut delay = std::time::Duration::from_secs(1);

        for attempt in 1..=max_attempts {
            match ValClient::connect(addr.to_string()).await {
                Ok(client) => {
                    info!("connected to DATA_BROKER at {} (attempt {})", addr, attempt);
                    return Ok(Self { client });
                }
                Err(e) => {
                    if attempt == max_attempts {
                        return Err(BrokerError::ConnectionFailed(format!(
                            "failed after {} attempts: {}",
                            max_attempts, e
                        )));
                    }
                    warn!(
                        "connection attempt {}/{} failed: {}. Retrying in {:?}...",
                        attempt, max_attempts, e, delay
                    );
                    tokio::time::sleep(delay).await;
                    delay *= 2;
                }
            }
        }

        unreachable!()
    }

    /// Subscribe to a VSS signal path, returning an mpsc receiver for values.
    ///
    /// The subscription runs in a background task that forwards string values
    /// from the gRPC stream to the returned channel.
    pub async fn subscribe(
        &mut self,
        signal: &str,
    ) -> Result<mpsc::Receiver<String>, BrokerError> {
        let request = SubscribeRequest {
            paths: vec![signal.to_string()],
        };

        let response = self
            .client
            .subscribe(request)
            .await
            .map_err(|e| BrokerError::OperationFailed(format!("subscribe failed: {}", e)))?;

        let (tx, rx) = mpsc::channel(32);
        let mut stream = response.into_inner();

        tokio::spawn(async move {
            while let Ok(Some(msg)) = stream.message().await {
                for entry in msg.entries {
                    if let Some(dp) = entry.value {
                        if let Some(kuksa::datapoint::Value::StringValue(s)) = dp.value {
                            if tx.send(s).await.is_err() {
                                return; // receiver dropped
                            }
                        }
                    }
                }
            }
        });

        Ok(rx)
    }
}

impl BrokerClient for GrpcBrokerClient {
    async fn get_float(&self, signal: &str) -> Result<Option<f32>, BrokerError> {
        let request = GetRequest {
            paths: vec![signal.to_string()],
        };

        let response = self
            .client
            .clone()
            .get(request)
            .await
            .map_err(|e| BrokerError::OperationFailed(format!("get_float failed: {}", e)))?;

        for entry in response.into_inner().entries {
            if let Some(dp) = entry.value {
                match dp.value {
                    Some(kuksa::datapoint::Value::FloatValue(v)) => return Ok(Some(v)),
                    Some(kuksa::datapoint::Value::DoubleValue(v)) => return Ok(Some(v as f32)),
                    _ => {}
                }
            }
        }

        Ok(None)
    }

    async fn get_bool(&self, signal: &str) -> Result<Option<bool>, BrokerError> {
        let request = GetRequest {
            paths: vec![signal.to_string()],
        };

        let response = self
            .client
            .clone()
            .get(request)
            .await
            .map_err(|e| BrokerError::OperationFailed(format!("get_bool failed: {}", e)))?;

        for entry in response.into_inner().entries {
            if let Some(dp) = entry.value {
                if let Some(kuksa::datapoint::Value::BoolValue(v)) = dp.value {
                    return Ok(Some(v));
                }
            }
        }

        Ok(None)
    }

    async fn set_bool(&self, signal: &str, value: bool) -> Result<(), BrokerError> {
        let request = SetRequest {
            entries: vec![DataEntry {
                path: signal.to_string(),
                value: Some(Datapoint {
                    timestamp: 0,
                    value: Some(kuksa::datapoint::Value::BoolValue(value)),
                }),
            }],
        };

        let response = self
            .client
            .clone()
            .set(request)
            .await
            .map_err(|e| BrokerError::OperationFailed(format!("set_bool failed: {}", e)))?;

        let set_resp = response.into_inner();
        if !set_resp.success {
            return Err(BrokerError::OperationFailed(format!(
                "set_bool rejected: {}",
                set_resp.error
            )));
        }

        Ok(())
    }

    async fn set_string(&self, signal: &str, value: &str) -> Result<(), BrokerError> {
        let request = SetRequest {
            entries: vec![DataEntry {
                path: signal.to_string(),
                value: Some(Datapoint {
                    timestamp: 0,
                    value: Some(kuksa::datapoint::Value::StringValue(value.to_string())),
                }),
            }],
        };

        let response = self
            .client
            .clone()
            .set(request)
            .await
            .map_err(|e| BrokerError::OperationFailed(format!("set_string failed: {}", e)))?;

        let set_resp = response.into_inner();
        if !set_resp.success {
            return Err(BrokerError::OperationFailed(format!(
                "set_string rejected: {}",
                set_resp.error
            )));
        }

        Ok(())
    }
}
