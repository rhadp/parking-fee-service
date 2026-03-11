//! DATA_BROKER gRPC client for communicating with Kuksa Databroker via UDS.

use std::time::Duration;

use hyper_util::rt::TokioIo;
use tokio::net::UnixStream;
use tonic::transport::{Channel, Endpoint, Uri};
use tower::service_fn;
use tracing::{error, info, warn};

/// Re-export generated protobuf types.
pub mod kuksa {
    pub mod val {
        pub mod v2 {
            tonic::include_proto!("kuksa.val.v2");
        }
    }
}

use kuksa::val::v2::{
    val_client::ValClient, GetValueRequest, PublishValueRequest, SignalId, SubscribeRequest,
    SubscribeResponse,
};

/// Signal value variants that can be read from or written to DATA_BROKER.
#[derive(Debug, Clone)]
pub enum SignalValue {
    String(String),
    Bool(bool),
    Double(f64),
}

/// A client for communicating with DATA_BROKER via gRPC over Unix Domain Socket.
#[derive(Clone)]
pub struct DataBrokerClient {
    client: ValClient<Channel>,
}

impl DataBrokerClient {
    /// Maximum backoff duration for retry attempts.
    const MAX_BACKOFF: Duration = Duration::from_secs(30);

    /// Connect to DATA_BROKER via Unix Domain Socket with exponential backoff retry.
    ///
    /// Retries with backoff (1s, 2s, 4s, ..., max 30s) until a connection is established.
    pub async fn connect(uds_path: &str) -> Result<Self, tonic::transport::Error> {
        let mut backoff = Duration::from_secs(1);

        loop {
            match Self::try_connect(uds_path).await {
                Ok(client) => {
                    info!("Connected to DATA_BROKER at {}", uds_path);
                    return Ok(client);
                }
                Err(e) => {
                    warn!(
                        "Failed to connect to DATA_BROKER at {}: {}. Retrying in {:?}...",
                        uds_path, e, backoff
                    );
                    tokio::time::sleep(backoff).await;
                    backoff = std::cmp::min(backoff * 2, Self::MAX_BACKOFF);
                }
            }
        }
    }

    /// Attempt a single connection to DATA_BROKER via UDS.
    async fn try_connect(uds_path: &str) -> Result<Self, tonic::transport::Error> {
        let uds_path = uds_path.to_string();

        // tonic requires a valid URI even for UDS; the actual connection uses the socket path.
        let channel = Endpoint::try_from("http://[::]:50051")?
            .connect_with_connector(service_fn(move |_: Uri| {
                let path = uds_path.clone();
                async move { UnixStream::connect(path).await.map(TokioIo::new) }
            }))
            .await?;

        let client = ValClient::new(channel);
        Ok(DataBrokerClient { client })
    }

    /// Write a string signal value to DATA_BROKER.
    ///
    /// Used for writing command JSON to `Vehicle.Command.Door.Lock`.
    pub async fn set_signal(
        &mut self,
        path: &str,
        value: SignalValue,
    ) -> Result<(), tonic::Status> {
        let typed_value = match value {
            SignalValue::String(s) => kuksa::val::v2::value::TypedValue::String(s),
            SignalValue::Bool(b) => kuksa::val::v2::value::TypedValue::Bool(b),
            SignalValue::Double(d) => kuksa::val::v2::value::TypedValue::Double(d),
        };

        let request = PublishValueRequest {
            signal_id: Some(SignalId {
                signal: Some(kuksa::val::v2::signal_id::Signal::Path(path.to_string())),
            }),
            data_point: Some(kuksa::val::v2::Datapoint {
                timestamp: None,
                value: Some(kuksa::val::v2::Value {
                    typed_value: Some(typed_value),
                }),
            }),
        };

        self.client.publish_value(request).await?;
        Ok(())
    }

    /// Subscribe to signal changes on the given signal paths.
    ///
    /// Returns a streaming response of signal updates.
    pub async fn subscribe_signals(
        &mut self,
        paths: &[&str],
    ) -> Result<tonic::Streaming<SubscribeResponse>, tonic::Status> {
        let request = SubscribeRequest {
            signal_paths: paths.iter().map(|p| p.to_string()).collect(),
            buffer_size: 0,
        };

        let response = self.client.subscribe(request).await?;
        Ok(response.into_inner())
    }

    /// Read the current value of a signal from DATA_BROKER.
    ///
    /// Returns `None` if the signal has no current value.
    pub async fn get_signal(&mut self, path: &str) -> Result<Option<SignalValue>, tonic::Status> {
        let request = GetValueRequest {
            signal_id: Some(SignalId {
                signal: Some(kuksa::val::v2::signal_id::Signal::Path(path.to_string())),
            }),
        };

        match self.client.get_value(request).await {
            Ok(response) => {
                let dp = response.into_inner().data_point;
                match dp.and_then(|dp| dp.value).and_then(|v| v.typed_value) {
                    Some(kuksa::val::v2::value::TypedValue::String(s)) => {
                        Ok(Some(SignalValue::String(s)))
                    }
                    Some(kuksa::val::v2::value::TypedValue::Bool(b)) => {
                        Ok(Some(SignalValue::Bool(b)))
                    }
                    Some(kuksa::val::v2::value::TypedValue::Double(d)) => {
                        Ok(Some(SignalValue::Double(d)))
                    }
                    _ => Ok(None),
                }
            }
            Err(status) if status.code() == tonic::Code::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Connect to DATA_BROKER via TCP (host:port).
    ///
    /// Used primarily for integration tests where the UDS socket is inside a container.
    pub async fn connect_tcp(addr: &str) -> Result<Self, tonic::transport::Error> {
        let uri = format!("http://{}", addr);
        let channel = Endpoint::try_from(uri)?.connect().await?;
        let client = ValClient::new(channel);
        Ok(DataBrokerClient { client })
    }

    /// Reconnect to DATA_BROKER with exponential backoff.
    ///
    /// Used for connection recovery during operation.
    pub async fn reconnect(&mut self, uds_path: &str) -> Result<(), tonic::transport::Error> {
        error!("DATA_BROKER connection lost. Attempting to reconnect...");
        let new_client = Self::connect(uds_path).await?;
        self.client = new_client.client;
        info!("Reconnected to DATA_BROKER");
        Ok(())
    }
}
