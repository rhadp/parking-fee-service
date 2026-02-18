//! High-level client helper for the Kuksa Databroker `kuksa.val.v2` API.
//!
//! Provides a [`KuksaClient`] with typed convenience methods for reading,
//! writing, and subscribing to VSS signals via gRPC.

use std::collections::HashMap;

use tokio_stream::{Stream, StreamExt};
use tonic::transport::Channel;
use tracing::{debug, warn};

use crate::kuksa::val::v2::{
    self as proto,
    val_client::ValClient,
};

/// Errors returned by [`KuksaClient`] operations.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// gRPC transport or protocol error.
    #[error("gRPC error: {0}")]
    Grpc(#[from] tonic::Status),

    /// Failed to connect to the Kuksa Databroker.
    #[error("connection error: {0}")]
    Connection(#[from] tonic::transport::Error),

    /// The response did not contain the expected data.
    #[error("missing data in response for signal '{0}'")]
    MissingData(String),
}

/// Result type for [`KuksaClient`] operations.
pub type Result<T> = std::result::Result<T, Error>;

/// High-level Kuksa Databroker client wrapping the `kuksa.val.v2` gRPC API.
///
/// All operations are async and require a Tokio runtime.
#[derive(Debug, Clone)]
pub struct KuksaClient {
    inner: ValClient<Channel>,
}

impl KuksaClient {
    /// Connect to a Kuksa Databroker at the given address.
    ///
    /// The address should include the scheme, e.g. `http://localhost:55555`.
    pub async fn connect(addr: &str) -> Result<Self> {
        let inner = ValClient::connect(addr.to_string()).await?;
        Ok(Self { inner })
    }

    /// Create a client from an existing tonic `Channel`.
    pub fn from_channel(channel: Channel) -> Self {
        Self {
            inner: ValClient::new(channel),
        }
    }

    // ── Get operations ──────────────────────────────────────────────────

    /// Read a boolean signal value.
    ///
    /// Returns `Ok(None)` if the signal exists but has no value set.
    pub async fn get_bool(&self, path: &str) -> Result<Option<bool>> {
        let dp = self.get_value(path).await?;
        Ok(dp.and_then(|v| match v {
            proto::value::TypedValue::Bool(b) => Some(b),
            _ => {
                warn!(path, "expected bool, got different type");
                None
            }
        }))
    }

    /// Read a float (f32) signal value.
    pub async fn get_f32(&self, path: &str) -> Result<Option<f32>> {
        let dp = self.get_value(path).await?;
        Ok(dp.and_then(|v| match v {
            proto::value::TypedValue::Float(f) => Some(f),
            // Kuksa may store speed as double depending on VSS definition
            proto::value::TypedValue::Double(d) => Some(d as f32),
            _ => {
                warn!(path, "expected float, got different type");
                None
            }
        }))
    }

    /// Read a double (f64) signal value.
    pub async fn get_f64(&self, path: &str) -> Result<Option<f64>> {
        let dp = self.get_value(path).await?;
        Ok(dp.and_then(|v| match v {
            proto::value::TypedValue::Double(d) => Some(d),
            proto::value::TypedValue::Float(f) => Some(f64::from(f)),
            _ => {
                warn!(path, "expected double, got different type");
                None
            }
        }))
    }

    /// Read a string signal value.
    pub async fn get_string(&self, path: &str) -> Result<Option<String>> {
        let dp = self.get_value(path).await?;
        Ok(dp.and_then(|v| match v {
            proto::value::TypedValue::String(s) => Some(s),
            _ => {
                warn!(path, "expected string, got different type");
                None
            }
        }))
    }

    /// Low-level: read a signal's typed value via `GetValue`.
    async fn get_value(&self, path: &str) -> Result<Option<proto::value::TypedValue>> {
        let req = proto::GetValueRequest {
            signal_id: Some(proto::SignalId {
                signal: Some(proto::signal_id::Signal::Path(path.to_string())),
            }),
        };

        let resp = self.inner.clone().get_value(req).await;

        match resp {
            Ok(resp) => {
                let inner = resp.into_inner();
                Ok(inner
                    .data_point
                    .and_then(|dp| dp.value)
                    .and_then(|v| v.typed_value))
            }
            Err(status) if status.code() == tonic::Code::NotFound => {
                debug!(path, "signal not found");
                Ok(None)
            }
            Err(status) => Err(Error::Grpc(status)),
        }
    }

    // ── Set operations ──────────────────────────────────────────────────

    /// Write a boolean signal value.
    pub async fn set_bool(&self, path: &str, value: bool) -> Result<()> {
        self.publish_value(path, proto::value::TypedValue::Bool(value))
            .await
    }

    /// Write a float (f32) signal value.
    pub async fn set_f32(&self, path: &str, value: f32) -> Result<()> {
        self.publish_value(path, proto::value::TypedValue::Float(value))
            .await
    }

    /// Write a double (f64) signal value.
    pub async fn set_f64(&self, path: &str, value: f64) -> Result<()> {
        self.publish_value(path, proto::value::TypedValue::Double(value))
            .await
    }

    /// Write a string signal value.
    pub async fn set_string(&self, path: &str, value: &str) -> Result<()> {
        self.publish_value(
            path,
            proto::value::TypedValue::String(value.to_string()),
        )
        .await
    }

    /// Low-level: publish a typed value to a signal via `PublishValue`.
    async fn publish_value(
        &self,
        path: &str,
        typed_value: proto::value::TypedValue,
    ) -> Result<()> {
        let req = proto::PublishValueRequest {
            signal_id: Some(proto::SignalId {
                signal: Some(proto::signal_id::Signal::Path(path.to_string())),
            }),
            data_point: Some(proto::Datapoint {
                timestamp: None,
                value: Some(proto::Value {
                    typed_value: Some(typed_value),
                }),
            }),
        };

        self.inner.clone().publish_value(req).await?;
        Ok(())
    }

    // ── Subscribe operations ────────────────────────────────────────────

    /// Subscribe to a boolean signal, returning a stream of value changes.
    ///
    /// Each item in the stream is the new boolean value of the signal.
    /// The stream ends when the server closes the subscription or the
    /// connection drops.
    pub async fn subscribe_bool(
        &self,
        path: &str,
    ) -> Result<impl Stream<Item = Result<bool>>> {
        self.subscribe_typed(path, |v| match v {
            proto::value::TypedValue::Bool(b) => Some(b),
            _ => None,
        })
        .await
    }

    /// Subscribe to a string signal, returning a stream of value changes.
    pub async fn subscribe_string(
        &self,
        path: &str,
    ) -> Result<impl Stream<Item = Result<String>>> {
        self.subscribe_typed(path, |v| match v {
            proto::value::TypedValue::String(s) => Some(s),
            _ => None,
        })
        .await
    }

    /// Low-level: subscribe to a signal and map values through a converter.
    async fn subscribe_typed<T, F>(
        &self,
        path: &str,
        convert: F,
    ) -> Result<impl Stream<Item = Result<T>>>
    where
        F: Fn(proto::value::TypedValue) -> Option<T> + Send + 'static,
        T: Send + 'static,
    {
        let signal_path = path.to_string();
        let req = proto::SubscribeRequest {
            signal_paths: vec![signal_path.clone()],
            buffer_size: 0,
        };

        let resp = self.inner.clone().subscribe(req).await?;
        let stream = resp.into_inner();

        let mapped = stream.filter_map(move |item| {
            match item {
                Ok(subscribe_resp) => {
                    // Extract the value for our signal path from the entries map
                    extract_value_from_entries(
                        &signal_path,
                        subscribe_resp.entries,
                        &convert,
                    )
                }
                Err(status) => Some(Err(Error::Grpc(status))),
            }
        });

        Ok(mapped)
    }
}

/// Extract a typed value for a specific signal path from a subscribe response entries map.
fn extract_value_from_entries<T, F>(
    path: &str,
    entries: HashMap<String, proto::Datapoint>,
    convert: &F,
) -> Option<Result<T>>
where
    F: Fn(proto::value::TypedValue) -> Option<T>,
{
    let datapoint = entries.get(path)?;
    let value = datapoint.value.as_ref()?;
    let typed = value.typed_value.as_ref()?;
    match convert(typed.clone()) {
        Some(v) => Some(Ok(v)),
        None => {
            warn!(path, "unexpected value type in subscription");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_missing_data() {
        let err = Error::MissingData("Vehicle.Speed".into());
        let msg = err.to_string();
        assert!(msg.contains("Vehicle.Speed"));
        assert!(msg.contains("missing data"));
    }

    #[test]
    fn error_display_grpc() {
        let status = tonic::Status::not_found("signal not found");
        let err = Error::Grpc(status);
        let msg = err.to_string();
        assert!(msg.contains("gRPC error"));
    }

    #[test]
    fn error_display_connection() {
        // We can't easily construct a tonic::transport::Error, but we can
        // verify the variant exists and the enum is Debug.
        let err = Error::MissingData("test".into());
        let _debug = format!("{:?}", err);
    }

    #[test]
    fn kuksa_client_is_clone() {
        // KuksaClient must be Clone so it can be shared across tasks.
        fn _assert_clone<T: Clone>() {}
        _assert_clone::<KuksaClient>();
    }

    #[test]
    fn kuksa_client_is_debug() {
        fn _assert_debug<T: std::fmt::Debug>() {}
        _assert_debug::<KuksaClient>();
    }

    #[tokio::test]
    async fn connect_to_invalid_address_fails() {
        // Connecting to a non-existent server should fail.
        let result = KuksaClient::connect("http://127.0.0.1:1").await;
        assert!(result.is_err(), "expected connection to invalid address to fail");
    }

    #[test]
    fn extract_value_from_entries_bool() {
        let mut entries = HashMap::new();
        entries.insert(
            "Vehicle.Speed".to_string(),
            proto::Datapoint {
                timestamp: None,
                value: Some(proto::Value {
                    typed_value: Some(proto::value::TypedValue::Bool(true)),
                }),
            },
        );

        let result = extract_value_from_entries(
            "Vehicle.Speed",
            entries,
            &|v| match v {
                proto::value::TypedValue::Bool(b) => Some(b),
                _ => None,
            },
        );
        assert!(matches!(result, Some(Ok(true))));
    }

    #[test]
    fn extract_value_from_entries_missing_path() {
        let entries = HashMap::new();
        let result: Option<Result<bool>> = extract_value_from_entries(
            "Vehicle.Speed",
            entries,
            &|v| match v {
                proto::value::TypedValue::Bool(b) => Some(b),
                _ => None,
            },
        );
        assert!(result.is_none());
    }

    #[test]
    fn extract_value_from_entries_wrong_type() {
        let mut entries = HashMap::new();
        entries.insert(
            "Vehicle.Speed".to_string(),
            proto::Datapoint {
                timestamp: None,
                value: Some(proto::Value {
                    typed_value: Some(proto::value::TypedValue::Double(42.0)),
                }),
            },
        );

        // Asking for bool but the value is double → returns None
        let result: Option<Result<bool>> = extract_value_from_entries(
            "Vehicle.Speed",
            entries,
            &|v| match v {
                proto::value::TypedValue::Bool(b) => Some(b),
                _ => None,
            },
        );
        assert!(result.is_none());
    }

    #[test]
    fn extract_value_from_entries_no_value() {
        let mut entries = HashMap::new();
        entries.insert(
            "Vehicle.Speed".to_string(),
            proto::Datapoint {
                timestamp: None,
                value: None,
            },
        );

        let result: Option<Result<bool>> = extract_value_from_entries(
            "Vehicle.Speed",
            entries,
            &|v| match v {
                proto::value::TypedValue::Bool(b) => Some(b),
                _ => None,
            },
        );
        assert!(result.is_none());
    }

    /// Integration test: connect to a real Kuksa Databroker, write and read
    /// back a value. Requires `make infra-up`.
    #[tokio::test]
    #[ignore]
    async fn integration_set_and_get_bool() {
        let addr = std::env::var("DATABROKER_ADDR")
            .unwrap_or_else(|_| "http://localhost:55555".to_string());

        let client = KuksaClient::connect(&addr)
            .await
            .expect("failed to connect to Kuksa Databroker (is `make infra-up` running?)");

        // Write a boolean value
        client
            .set_bool(crate::signals::COMMAND_DOOR_LOCK, true)
            .await
            .expect("set_bool failed");

        // Read it back
        let value = client
            .get_bool(crate::signals::COMMAND_DOOR_LOCK)
            .await
            .expect("get_bool failed");

        assert_eq!(value, Some(true), "expected true after setting door lock");
    }

    /// Integration test: set and get a float value.
    #[tokio::test]
    #[ignore]
    async fn integration_set_and_get_f32() {
        let addr = std::env::var("DATABROKER_ADDR")
            .unwrap_or_else(|_| "http://localhost:55555".to_string());

        let client = KuksaClient::connect(&addr)
            .await
            .expect("failed to connect to Kuksa Databroker");

        client
            .set_f32(crate::signals::SPEED, 42.5)
            .await
            .expect("set_f32 failed");

        let value = client
            .get_f32(crate::signals::SPEED)
            .await
            .expect("get_f32 failed");

        assert!(value.is_some(), "expected Some value after setting speed");
        let speed = value.unwrap();
        assert!(
            (speed - 42.5).abs() < 0.1,
            "expected ~42.5, got {}",
            speed
        );
    }

    /// Integration test: set and get a string value.
    #[tokio::test]
    #[ignore]
    async fn integration_set_and_get_string() {
        let addr = std::env::var("DATABROKER_ADDR")
            .unwrap_or_else(|_| "http://localhost:55555".to_string());

        let client = KuksaClient::connect(&addr)
            .await
            .expect("failed to connect to Kuksa Databroker");

        client
            .set_string(crate::signals::LOCK_RESULT, "SUCCESS")
            .await
            .expect("set_string failed");

        let value = client
            .get_string(crate::signals::LOCK_RESULT)
            .await
            .expect("get_string failed");

        assert_eq!(value.as_deref(), Some("SUCCESS"));
    }

    /// Integration test: set and get a double (f64) value.
    #[tokio::test]
    #[ignore]
    async fn integration_set_and_get_f64() {
        let addr = std::env::var("DATABROKER_ADDR")
            .unwrap_or_else(|_| "http://localhost:55555".to_string());

        let client = KuksaClient::connect(&addr)
            .await
            .expect("failed to connect to Kuksa Databroker");

        client
            .set_f64(crate::signals::LOCATION_LAT, 48.8566)
            .await
            .expect("set_f64 failed");

        let value = client
            .get_f64(crate::signals::LOCATION_LAT)
            .await
            .expect("get_f64 failed");

        assert!(value.is_some());
        assert!(
            (value.unwrap() - 48.8566).abs() < 0.001,
            "expected ~48.8566, got {:?}",
            value
        );
    }
}
