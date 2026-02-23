//! Kuksa Databroker gRPC client.
//!
//! Provides a typed client for reading, writing, and subscribing to VSS
//! signals via the `kuksa.val.v1` gRPC API. Supports connection over both
//! Unix Domain Sockets (UDS) and TCP endpoints.

use std::pin::Pin;

use tokio_stream::Stream;
use tonic::transport::{Channel, Endpoint, Uri};
use tower::service_fn;
use tracing::{debug, warn};

use crate::error::Error;
use crate::proto::kuksa::val::v1::{self as proto, val_client::ValClient};
use crate::value::DataValue;

/// A signal update received from a subscription.
#[derive(Debug, Clone)]
pub struct SignalUpdate {
    /// The VSS path of the signal that changed.
    pub path: String,
    /// The new value, or `None` if the signal was cleared.
    pub value: Option<DataValue>,
}

/// A stream of signal updates from a subscription.
pub type SignalStream = Pin<Box<dyn Stream<Item = Result<Vec<SignalUpdate>, Error>> + Send>>;

/// Endpoint prefix for Unix Domain Socket connections.
const UDS_PREFIX: &str = "unix://";

/// Default UDS socket path for Kuksa Databroker.
pub const DEFAULT_UDS_PATH: &str = "/tmp/kuksa-databroker.sock";

/// Default UDS endpoint URI.
pub const DEFAULT_UDS_ENDPOINT: &str = "unix:///tmp/kuksa-databroker.sock";

/// Parse an endpoint string into an enum for dispatch.
#[derive(Debug, Clone)]
pub(crate) enum EndpointKind {
    /// Unix Domain Socket with the socket file path.
    Uds(String),
    /// TCP/HTTP endpoint URI.
    Tcp(String),
}

/// Parse an endpoint string to determine its kind.
///
/// Endpoints starting with `unix://` are treated as UDS paths.
/// Everything else is treated as a TCP endpoint.
pub(crate) fn parse_endpoint(endpoint: &str) -> EndpointKind {
    if endpoint.starts_with(UDS_PREFIX) {
        let path = endpoint.strip_prefix(UDS_PREFIX).unwrap_or(DEFAULT_UDS_PATH);
        EndpointKind::Uds(path.to_string())
    } else if endpoint.ends_with(".sock") || endpoint.starts_with('/') {
        // Bare path like "/tmp/kuksa-databroker.sock"
        EndpointKind::Uds(endpoint.to_string())
    } else {
        EndpointKind::Tcp(endpoint.to_string())
    }
}

/// Typed client for the Kuksa Databroker gRPC API.
///
/// Wraps the `kuksa.val.v1.VAL` service with a high-level interface
/// for reading, writing, and subscribing to VSS signals.
#[derive(Clone)]
pub struct DatabrokerClient {
    client: ValClient<Channel>,
    /// Optional bearer token for authenticated requests.
    token: Option<String>,
}

impl DatabrokerClient {
    /// Connect to the Kuksa Databroker at the given endpoint.
    ///
    /// The endpoint can be:
    /// - A UDS path: `unix:///tmp/kuksa-databroker.sock` or `/tmp/kuksa-databroker.sock`
    /// - A TCP address: `http://localhost:55556`
    ///
    /// # Errors
    ///
    /// Returns an error if the connection cannot be established.
    pub async fn connect(endpoint: &str) -> Result<Self, Error> {
        let channel = match parse_endpoint(endpoint) {
            EndpointKind::Uds(path) => {
                debug!(path = %path, "connecting to Databroker via UDS");
                Self::connect_uds(&path).await?
            }
            EndpointKind::Tcp(addr) => {
                debug!(addr = %addr, "connecting to Databroker via TCP");
                Self::connect_tcp(&addr).await?
            }
        };

        Ok(Self {
            client: ValClient::new(channel),
            token: None,
        })
    }

    /// Set the bearer token for authenticated requests.
    ///
    /// The token will be sent as `authorization: Bearer <token>` metadata
    /// on each gRPC request.
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    /// Read the current value of a signal.
    ///
    /// # Arguments
    ///
    /// * `path` - The VSS signal path (e.g., `Vehicle.Speed`).
    ///
    /// # Returns
    ///
    /// The current value if one is set, or `Error::NoValue` if the signal
    /// exists but has no value, or `Error::SignalNotFound` if the path is unknown.
    pub async fn get_value(&self, path: &str) -> Result<DataValue, Error> {
        let mut client = self.client.clone();

        let request = proto::GetRequest {
            entries: vec![proto::EntryRequest {
                path: path.to_string(),
                view: proto::View::CurrentValue.into(),
                fields: vec![proto::Field::Value.into()],
            }],
        };

        let mut req = tonic::Request::new(request);
        self.attach_token(&mut req);

        let response = client.get(req).await?.into_inner();

        // Check for entry-level errors
        if let Some(entry_err) = response.errors.first() {
            if let Some(err) = &entry_err.error {
                // Code 404 = not found
                if err.code == 404 {
                    return Err(Error::SignalNotFound {
                        path: path.to_string(),
                    });
                }
                return Err(Error::EntryError {
                    path: entry_err.path.clone(),
                    code: err.code,
                    reason: err.reason.clone(),
                });
            }
        }

        // Check for global error
        if let Some(err) = &response.error {
            if err.code != 0 {
                return Err(Error::EntryError {
                    path: path.to_string(),
                    code: err.code,
                    reason: err.reason.clone(),
                });
            }
        }

        // Extract value from the first entry
        let entry = response.entries.first().ok_or(Error::SignalNotFound {
            path: path.to_string(),
        })?;

        let datapoint = entry.value.as_ref().ok_or(Error::NoValue {
            path: path.to_string(),
        })?;

        DataValue::from_datapoint(datapoint).ok_or(Error::NoValue {
            path: path.to_string(),
        })
    }

    /// Read the current value of a signal, returning `None` if not set.
    ///
    /// Unlike `get_value()`, this method returns `Ok(None)` instead of
    /// `Error::NoValue` when the signal exists but has no value.
    pub async fn get_value_opt(&self, path: &str) -> Result<Option<DataValue>, Error> {
        match self.get_value(path).await {
            Ok(v) => Ok(Some(v)),
            Err(Error::NoValue { .. }) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Write a value to a signal.
    ///
    /// For actuator signals (like command signals), this sets the
    /// actuator target. For sensor/attribute signals, this sets the
    /// current value.
    ///
    /// # Arguments
    ///
    /// * `path` - The VSS signal path.
    /// * `value` - The value to write.
    ///
    /// # Errors
    ///
    /// Returns an error if the write fails (unknown signal, permission denied, etc.).
    pub async fn set_value(&self, path: &str, value: DataValue) -> Result<(), Error> {
        let mut client = self.client.clone();

        let datapoint = value.to_datapoint();

        let request = proto::SetRequest {
            updates: vec![proto::EntryUpdate {
                entry: Some(proto::DataEntry {
                    path: path.to_string(),
                    value: Some(datapoint),
                    actuator_target: None,
                    metadata: None,
                }),
                fields: vec![proto::Field::Value.into()],
            }],
        };

        let mut req = tonic::Request::new(request);
        self.attach_token(&mut req);

        let response = client.set(req).await?.into_inner();

        // Check for entry-level errors
        if let Some(entry_err) = response.errors.first() {
            if let Some(err) = &entry_err.error {
                return Err(Error::EntryError {
                    path: entry_err.path.clone(),
                    code: err.code,
                    reason: err.reason.clone(),
                });
            }
        }

        // Check for global error
        if let Some(err) = &response.error {
            if err.code != 0 {
                return Err(Error::EntryError {
                    path: path.to_string(),
                    code: err.code,
                    reason: err.reason.clone(),
                });
            }
        }

        Ok(())
    }

    /// Set the actuator target value for a signal.
    ///
    /// Use this for actuator-type signals where you want to set the
    /// target value (as opposed to the current value).
    pub async fn set_target(&self, path: &str, value: DataValue) -> Result<(), Error> {
        let mut client = self.client.clone();

        let datapoint = value.to_datapoint();

        let request = proto::SetRequest {
            updates: vec![proto::EntryUpdate {
                entry: Some(proto::DataEntry {
                    path: path.to_string(),
                    value: None,
                    actuator_target: Some(datapoint),
                    metadata: None,
                }),
                fields: vec![proto::Field::ActuatorTarget.into()],
            }],
        };

        let mut req = tonic::Request::new(request);
        self.attach_token(&mut req);

        let response = client.set(req).await?.into_inner();

        // Check for entry-level errors
        if let Some(entry_err) = response.errors.first() {
            if let Some(err) = &entry_err.error {
                return Err(Error::EntryError {
                    path: entry_err.path.clone(),
                    code: err.code,
                    reason: err.reason.clone(),
                });
            }
        }

        // Check for global error
        if let Some(err) = &response.error {
            if err.code != 0 {
                return Err(Error::EntryError {
                    path: path.to_string(),
                    code: err.code,
                    reason: err.reason.clone(),
                });
            }
        }

        Ok(())
    }

    /// Subscribe to changes on one or more signals.
    ///
    /// Returns a stream of signal updates. Each update contains the path
    /// and new value for signals that changed.
    ///
    /// # Arguments
    ///
    /// * `paths` - The VSS signal paths to subscribe to.
    ///
    /// # Returns
    ///
    /// A stream that yields batches of signal updates.
    pub async fn subscribe(&self, paths: &[&str]) -> Result<SignalStream, Error> {
        let mut client = self.client.clone();

        let entries: Vec<proto::SubscribeEntry> = paths
            .iter()
            .map(|p| proto::SubscribeEntry {
                path: p.to_string(),
                view: proto::View::CurrentValue.into(),
                fields: vec![proto::Field::Value.into()],
            })
            .collect();

        let request = proto::SubscribeRequest { entries };

        let mut req = tonic::Request::new(request);
        self.attach_token(&mut req);

        let response = client.subscribe(req).await?;
        let stream = response.into_inner();

        let mapped = tokio_stream::StreamExt::map(stream, |result| {
            result
                .map(|resp| {
                    resp.updates
                        .into_iter()
                        .filter_map(|update| {
                            let entry = update.entry?;
                            let value = entry
                                .value
                                .as_ref()
                                .and_then(DataValue::from_datapoint);
                            Some(SignalUpdate {
                                path: entry.path,
                                value,
                            })
                        })
                        .collect()
                })
                .map_err(Error::from)
        });

        Ok(Box::pin(mapped))
    }

    /// Get metadata for a signal (data type, entry type, description, etc.).
    ///
    /// Useful for verifying signal existence and type.
    pub async fn get_metadata(
        &self,
        path: &str,
    ) -> Result<proto::Metadata, Error> {
        let mut client = self.client.clone();

        let request = proto::GetRequest {
            entries: vec![proto::EntryRequest {
                path: path.to_string(),
                view: proto::View::Metadata.into(),
                fields: vec![proto::Field::Metadata.into()],
            }],
        };

        let mut req = tonic::Request::new(request);
        self.attach_token(&mut req);

        let response = client.get(req).await?.into_inner();

        // Check for entry-level errors
        if let Some(entry_err) = response.errors.first() {
            if let Some(err) = &entry_err.error {
                if err.code == 404 {
                    return Err(Error::SignalNotFound {
                        path: path.to_string(),
                    });
                }
                return Err(Error::EntryError {
                    path: entry_err.path.clone(),
                    code: err.code,
                    reason: err.reason.clone(),
                });
            }
        }

        let entry = response.entries.first().ok_or(Error::SignalNotFound {
            path: path.to_string(),
        })?;

        entry.metadata.clone().ok_or(Error::SignalNotFound {
            path: path.to_string(),
        })
    }

    /// Get server information (name and version).
    pub async fn get_server_info(&self) -> Result<(String, String), Error> {
        let mut client = self.client.clone();

        let request = proto::GetServerInfoRequest {};
        let mut req = tonic::Request::new(request);
        self.attach_token(&mut req);

        let response = client.get_server_info(req).await?.into_inner();

        Ok((response.name, response.version))
    }

    // ---- Private helpers ----

    /// Connect to the Databroker via Unix Domain Socket.
    async fn connect_uds(path: &str) -> Result<Channel, Error> {
        let path = path.to_string();

        // For UDS, tonic requires a dummy URI; the actual connection is
        // handled by the connector closure.
        let channel = Endpoint::try_from("http://[::]:50051")
            .map_err(|e| Error::InvalidEndpoint(format!("failed to create UDS endpoint: {e}")))?
            .connect_with_connector(service_fn(move |_: Uri| {
                let path = path.clone();
                async move {
                    let stream = tokio::net::UnixStream::connect(&path).await?;
                    Ok::<_, std::io::Error>(hyper_util::rt::TokioIo::new(stream))
                }
            }))
            .await?;

        Ok(channel)
    }

    /// Connect to the Databroker via TCP.
    async fn connect_tcp(addr: &str) -> Result<Channel, Error> {
        // Ensure the address has a scheme
        let uri = if addr.starts_with("http://") || addr.starts_with("https://") {
            addr.to_string()
        } else {
            format!("http://{addr}")
        };

        let channel = Channel::from_shared(uri)
            .map_err(|e| Error::InvalidEndpoint(e.to_string()))?
            .connect()
            .await?;

        Ok(channel)
    }

    /// Attach the bearer token to a gRPC request if one is configured.
    fn attach_token<T>(&self, request: &mut tonic::Request<T>) {
        if let Some(token) = &self.token {
            let value = format!("Bearer {token}");
            match value.parse() {
                Ok(val) => {
                    request
                        .metadata_mut()
                        .insert("authorization", val);
                }
                Err(e) => {
                    warn!("failed to set authorization metadata: {e}");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_endpoint_uds_with_prefix() {
        let kind = parse_endpoint("unix:///tmp/kuksa-databroker.sock");
        match kind {
            EndpointKind::Uds(path) => assert_eq!(path, "/tmp/kuksa-databroker.sock"),
            _ => panic!("expected UDS endpoint"),
        }
    }

    #[test]
    fn test_parse_endpoint_uds_bare_path() {
        let kind = parse_endpoint("/tmp/kuksa-databroker.sock");
        match kind {
            EndpointKind::Uds(path) => assert_eq!(path, "/tmp/kuksa-databroker.sock"),
            _ => panic!("expected UDS endpoint"),
        }
    }

    #[test]
    fn test_parse_endpoint_uds_sock_extension() {
        let kind = parse_endpoint("/var/run/databroker.sock");
        match kind {
            EndpointKind::Uds(path) => assert_eq!(path, "/var/run/databroker.sock"),
            _ => panic!("expected UDS endpoint"),
        }
    }

    #[test]
    fn test_parse_endpoint_tcp_with_http() {
        let kind = parse_endpoint("http://localhost:55556");
        match kind {
            EndpointKind::Tcp(addr) => assert_eq!(addr, "http://localhost:55556"),
            _ => panic!("expected TCP endpoint"),
        }
    }

    #[test]
    fn test_parse_endpoint_tcp_without_scheme() {
        let kind = parse_endpoint("localhost:55556");
        match kind {
            EndpointKind::Tcp(addr) => assert_eq!(addr, "localhost:55556"),
            _ => panic!("expected TCP endpoint"),
        }
    }

    #[test]
    fn test_parse_endpoint_tcp_with_https() {
        let kind = parse_endpoint("https://databroker.example.com:55556");
        match kind {
            EndpointKind::Tcp(addr) => {
                assert_eq!(addr, "https://databroker.example.com:55556");
            }
            _ => panic!("expected TCP endpoint"),
        }
    }

    #[test]
    fn test_default_constants() {
        assert_eq!(DEFAULT_UDS_PATH, "/tmp/kuksa-databroker.sock");
        assert_eq!(DEFAULT_UDS_ENDPOINT, "unix:///tmp/kuksa-databroker.sock");
    }
}
