//! DATA_BROKER gRPC client for the CLOUD_GATEWAY_CLIENT.
//!
//! Connects to the Kuksa Databroker via gRPC over Unix Domain Socket.
//! Provides methods to write signal values and subscribe to signal changes.
//! Implements retry with exponential backoff for connection resilience.

use std::collections::HashMap;

use hyper_util::rt::TokioIo;
use tokio::net::UnixStream;
use tonic::transport::{Channel, Endpoint, Uri};
use tower::service_fn;
use tracing::{info, warn};

use crate::kuksa_proto;
use crate::kuksa_proto::val_client::ValClient;
use crate::kuksa_proto::value::TypedValue;
use crate::kuksa_proto::{
    Datapoint, GetValueRequest, GetValueResponse, PublishValueRequest, SignalId, SubscribeRequest,
    SubscribeResponse,
};

/// Maximum backoff delay for connection retries (30 seconds).
const MAX_BACKOFF_SECS: u64 = 30;

/// Initial backoff delay for connection retries (1 second).
const INITIAL_BACKOFF_SECS: u64 = 1;

/// A signal value returned from DATA_BROKER.
#[derive(Debug, Clone)]
pub enum SignalValue {
    String(String),
    Bool(bool),
    Float(f32),
    Double(f64),
    Int32(i32),
    Int64(i64),
    Uint32(u32),
    Uint64(u64),
}

/// Client for communicating with DATA_BROKER via gRPC over Unix Domain Socket.
pub struct DatabrokerClient {
    client: ValClient<Channel>,
    uds_path: String,
}

impl DatabrokerClient {
    /// Connect to DATA_BROKER at the given UDS path with exponential backoff retry.
    ///
    /// Retries connection with exponential backoff (1s, 2s, 4s, ..., max 30s)
    /// until a connection is established.
    pub async fn connect(uds_path: &str) -> Result<Self, tonic::transport::Error> {
        let mut backoff_secs = INITIAL_BACKOFF_SECS;

        loop {
            match Self::try_connect(uds_path).await {
                Ok(client) => {
                    info!(path = %uds_path, "Connected to DATA_BROKER");
                    return Ok(client);
                }
                Err(e) => {
                    warn!(
                        path = %uds_path,
                        backoff_secs = backoff_secs,
                        error = %e,
                        "DATA_BROKER unreachable, retrying"
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(backoff_secs)).await;
                    backoff_secs = (backoff_secs * 2).min(MAX_BACKOFF_SECS);
                }
            }
        }
    }

    /// Attempt a single connection to DATA_BROKER via UDS.
    ///
    /// This is public so callers can implement their own retry or timeout logic.
    pub async fn try_connect(uds_path: &str) -> Result<Self, tonic::transport::Error> {
        let uds_path_owned = uds_path.to_string();

        // tonic doesn't natively support UDS, so we use a custom connector.
        // The URI is ignored; the actual connection goes through the UDS path.
        let channel = Endpoint::try_from("http://[::]:50051")?
            .connect_with_connector(service_fn(move |_: Uri| {
                let path = uds_path_owned.clone();
                async move { UnixStream::connect(path).await.map(TokioIo::new) }
            }))
            .await?;

        let client = ValClient::new(channel);
        Ok(DatabrokerClient {
            client,
            uds_path: uds_path.to_string(),
        })
    }

    /// Returns the UDS path this client is connected to.
    pub fn uds_path(&self) -> &str {
        &self.uds_path
    }

    /// Subscribe to signal changes at the given VSS paths.
    ///
    /// Returns a tonic streaming response of `SubscribeResponse` messages.
    /// Each message contains a map of signal paths to their current datapoints.
    pub async fn subscribe_signals(
        &mut self,
        paths: &[&str],
    ) -> Result<tonic::Streaming<SubscribeResponse>, tonic::Status> {
        let request = SubscribeRequest {
            signal_paths: paths.iter().map(|p| p.to_string()).collect(),
            buffer_size: 0,
            filter: None,
        };
        let response = self.client.subscribe(request).await?;
        Ok(response.into_inner())
    }

    /// Subscribe to signal changes at a single VSS path.
    pub async fn subscribe_signal(
        &mut self,
        path: &str,
    ) -> Result<tonic::Streaming<SubscribeResponse>, tonic::Status> {
        self.subscribe_signals(&[path]).await
    }

    /// Read the current value of a signal from DATA_BROKER.
    ///
    /// Returns `None` if the signal exists but has no value set,
    /// or if the signal is not found.
    pub async fn get_signal(&mut self, path: &str) -> Result<Option<SignalValue>, tonic::Status> {
        let request = GetValueRequest {
            signal_id: Some(SignalId {
                signal: Some(kuksa_proto::signal_id::Signal::Path(path.to_string())),
            }),
        };

        let response: GetValueResponse = self.client.get_value(request).await?.into_inner();

        Ok(extract_signal_value(response.data_point))
    }

    /// Write a string value to a signal in DATA_BROKER.
    pub async fn set_signal_string(
        &mut self,
        path: &str,
        value: &str,
    ) -> Result<(), tonic::Status> {
        let request = PublishValueRequest {
            signal_id: Some(SignalId {
                signal: Some(kuksa_proto::signal_id::Signal::Path(path.to_string())),
            }),
            data_point: Some(Datapoint {
                timestamp: None,
                value: Some(kuksa_proto::Value {
                    typed_value: Some(TypedValue::String(value.to_string())),
                }),
            }),
        };

        self.client.publish_value(request).await?;
        Ok(())
    }

    /// Write a boolean value to a signal in DATA_BROKER.
    pub async fn set_signal_bool(
        &mut self,
        path: &str,
        value: bool,
    ) -> Result<(), tonic::Status> {
        let request = PublishValueRequest {
            signal_id: Some(SignalId {
                signal: Some(kuksa_proto::signal_id::Signal::Path(path.to_string())),
            }),
            data_point: Some(Datapoint {
                timestamp: None,
                value: Some(kuksa_proto::Value {
                    typed_value: Some(TypedValue::Bool(value)),
                }),
            }),
        };

        self.client.publish_value(request).await?;
        Ok(())
    }
}

/// Extract a `SignalValue` from an optional `Datapoint`.
fn extract_signal_value(datapoint: Option<Datapoint>) -> Option<SignalValue> {
    let dp = datapoint?;
    let value = dp.value?;
    let typed = value.typed_value?;

    match typed {
        TypedValue::String(s) => Some(SignalValue::String(s)),
        TypedValue::Bool(b) => Some(SignalValue::Bool(b)),
        TypedValue::Float(f) => Some(SignalValue::Float(f)),
        TypedValue::Double(d) => Some(SignalValue::Double(d)),
        TypedValue::Int32(i) => Some(SignalValue::Int32(i)),
        TypedValue::Int64(i) => Some(SignalValue::Int64(i)),
        TypedValue::Uint32(u) => Some(SignalValue::Uint32(u)),
        TypedValue::Uint64(u) => Some(SignalValue::Uint64(u)),
        _ => None,
    }
}

/// Extract signal values from a `SubscribeResponse` entries map.
pub fn extract_entries(
    entries: &HashMap<String, Datapoint>,
) -> HashMap<String, Option<SignalValue>> {
    entries
        .iter()
        .map(|(path, dp)| (path.clone(), extract_signal_value(Some(dp.clone()))))
        .collect()
}
