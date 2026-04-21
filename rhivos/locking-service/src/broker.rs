use async_trait::async_trait;
use tokio::sync::mpsc;

// VSS signal path constants
pub const SIGNAL_IS_LOCKED: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked";
pub const SIGNAL_IS_OPEN: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen";
pub const SIGNAL_SPEED: &str = "Vehicle.Speed";
pub const SIGNAL_COMMAND: &str = "Vehicle.Command.Door.Lock";
pub const SIGNAL_RESPONSE: &str = "Vehicle.Command.Door.Response";

#[derive(Debug)]
pub enum BrokerError {
    Connection(String),
    Transport(String),
    Signal(String),
}

impl std::fmt::Display for BrokerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BrokerError::Connection(e) => write!(f, "connection error: {e}"),
            BrokerError::Transport(e) => write!(f, "transport error: {e}"),
            BrokerError::Signal(e) => write!(f, "signal error: {e}"),
        }
    }
}

/// Trait abstracting DATA_BROKER gRPC communication.
#[async_trait(?Send)]
pub trait BrokerClient {
    async fn get_float(&self, signal: &str) -> Result<Option<f32>, BrokerError>;
    async fn get_bool(&self, signal: &str) -> Result<Option<bool>, BrokerError>;
    async fn set_bool(&self, signal: &str, value: bool) -> Result<(), BrokerError>;
    async fn set_string(&self, signal: &str, value: &str) -> Result<(), BrokerError>;
}

mod proto {
    pub mod kuksa {
        pub mod val {
            pub mod v1 {
                tonic::include_proto!("kuksa.val.v1");
            }
        }
    }
}

use proto::kuksa::val::v1::{
    val_service_client::ValServiceClient,
    DataEntry, Datapoint, EntryRequest, EntryUpdate, Field,
    GetRequest, SetRequest, SubscribeEntry, SubscribeRequest,
    datapoint,
};

/// Real gRPC broker client backed by the kuksa.val.v1 VALService.
pub struct GrpcBrokerClient {
    client: ValServiceClient<tonic::transport::Channel>,
}

impl GrpcBrokerClient {
    /// Connect with exponential backoff (5 attempts: delays of 1s, 2s, 4s, 8s between attempts).
    pub async fn connect(addr: &str) -> Result<Self, BrokerError> {
        let delays_ms: &[u64] = &[1000, 2000, 4000, 8000];
        let mut last_err = String::new();

        for attempt in 0..5usize {
            if attempt > 0 {
                let delay = delays_ms[attempt - 1];
                tracing::warn!("Connection attempt {attempt} failed, retrying in {delay}ms");
                tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
            }

            match ValServiceClient::connect(addr.to_string()).await {
                Ok(client) => {
                    tracing::info!("Connected to DATA_BROKER at {addr}");
                    return Ok(Self { client });
                }
                Err(e) => {
                    last_err = e.to_string();
                    tracing::warn!("Failed to connect (attempt {}): {e}", attempt + 1);
                }
            }
        }

        Err(BrokerError::Connection(format!(
            "Failed to connect to {addr} after 5 attempts: {last_err}"
        )))
    }

    /// Subscribe to a VSS signal, returning an mpsc channel receiver.
    ///
    /// Spawns a background task that reads the stream and forwards string
    /// values to the returned channel.
    pub async fn subscribe(&mut self, signal: &str) -> Result<mpsc::Receiver<String>, BrokerError> {
        let request = SubscribeRequest {
            entries: vec![SubscribeEntry {
                path: signal.to_string(),
                fields: vec![Field::Value as i32],
            }],
        };

        let mut stream = self
            .client
            .subscribe(request)
            .await
            .map_err(|e| BrokerError::Transport(e.to_string()))?
            .into_inner();

        let (tx, rx) = mpsc::channel::<String>(64);

        tokio::spawn(async move {
            loop {
                match stream.message().await {
                    Ok(Some(response)) => {
                        for update in response.updates {
                            if let Some(entry) = update.entry {
                                if let Some(dp) = entry.value {
                                    if let Some(datapoint::Value::String(s)) = dp.value {
                                        if tx.send(s).await.is_err() {
                                            // Receiver dropped — exit the task.
                                            return;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Ok(None) => {
                        tracing::info!("Subscribe stream ended");
                        return;
                    }
                    Err(e) => {
                        tracing::error!("Subscribe stream error: {e}");
                        return;
                    }
                }
            }
        });

        Ok(rx)
    }
}

#[async_trait(?Send)]
impl BrokerClient for GrpcBrokerClient {
    async fn get_float(&self, signal: &str) -> Result<Option<f32>, BrokerError> {
        let request = GetRequest {
            entries: vec![EntryRequest {
                path: signal.to_string(),
                fields: vec![Field::Value as i32],
            }],
        };

        let mut client = self.client.clone();
        let response = client
            .get(request)
            .await
            .map_err(|e| BrokerError::Transport(e.to_string()))?
            .into_inner();

        for entry in response.entries {
            if entry.path == signal {
                if let Some(dp) = entry.value {
                    if let Some(datapoint::Value::Float(v)) = dp.value {
                        return Ok(Some(v));
                    }
                }
            }
        }

        Ok(None)
    }

    async fn get_bool(&self, signal: &str) -> Result<Option<bool>, BrokerError> {
        let request = GetRequest {
            entries: vec![EntryRequest {
                path: signal.to_string(),
                fields: vec![Field::Value as i32],
            }],
        };

        let mut client = self.client.clone();
        let response = client
            .get(request)
            .await
            .map_err(|e| BrokerError::Transport(e.to_string()))?
            .into_inner();

        for entry in response.entries {
            if entry.path == signal {
                if let Some(dp) = entry.value {
                    if let Some(datapoint::Value::Bool(v)) = dp.value {
                        return Ok(Some(v));
                    }
                }
            }
        }

        Ok(None)
    }

    async fn set_bool(&self, signal: &str, value: bool) -> Result<(), BrokerError> {
        let request = SetRequest {
            updates: vec![EntryUpdate {
                entry: Some(DataEntry {
                    path: signal.to_string(),
                    value: Some(Datapoint {
                        value: Some(datapoint::Value::Bool(value)),
                    }),
                }),
                fields: vec![Field::Value as i32],
            }],
        };

        let mut client = self.client.clone();
        let response = client
            .set(request)
            .await
            .map_err(|e| BrokerError::Transport(e.to_string()))?
            .into_inner();

        for entry_err in &response.errors {
            tracing::warn!("set_bool({signal}) per-entry error: {:?}", entry_err.error);
        }

        Ok(())
    }

    async fn set_string(&self, signal: &str, value: &str) -> Result<(), BrokerError> {
        let request = SetRequest {
            updates: vec![EntryUpdate {
                entry: Some(DataEntry {
                    path: signal.to_string(),
                    value: Some(Datapoint {
                        value: Some(datapoint::Value::String(value.to_string())),
                    }),
                }),
                fields: vec![Field::Value as i32],
            }],
        };

        let mut client = self.client.clone();
        let response = client
            .set(request)
            .await
            .map_err(|e| BrokerError::Transport(e.to_string()))?
            .into_inner();

        for entry_err in &response.errors {
            tracing::warn!("set_string({signal}) per-entry error: {:?}", entry_err.error);
        }

        Ok(())
    }
}
