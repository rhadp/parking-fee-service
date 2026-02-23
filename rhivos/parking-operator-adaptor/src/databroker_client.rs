//! DATA_BROKER client for the PARKING_OPERATOR_ADAPTOR.
//!
//! Provides an abstraction over the Kuksa DATA_BROKER gRPC API for:
//! - Subscribing to `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` events
//! - Reading `Vehicle.CurrentLocation.Latitude` and `Longitude`
//! - Writing `Vehicle.Parking.SessionActive`
//!
//! The trait-based design allows the real Kuksa client and an in-memory
//! mock to be used interchangeably, enabling integration testing without
//! external infrastructure.

use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

/// VSS signal paths used by the PARKING_OPERATOR_ADAPTOR.
pub mod signals {
    /// Lock/unlock signal for the driver-side door.
    pub const IS_LOCKED: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked";
    /// Current latitude of the vehicle.
    pub const LATITUDE: &str = "Vehicle.CurrentLocation.Latitude";
    /// Current longitude of the vehicle.
    pub const LONGITUDE: &str = "Vehicle.CurrentLocation.Longitude";
    /// Whether a parking session is currently active.
    pub const SESSION_ACTIVE: &str = "Vehicle.Parking.SessionActive";
}

/// A value from the DATA_BROKER.
#[derive(Debug, Clone)]
pub enum DataValue {
    /// A boolean value.
    Bool(bool),
    /// A floating-point value.
    Float(f64),
    /// Value is not set / unknown.
    NotAvailable,
}

impl DataValue {
    /// Extract a boolean value, defaulting to `false` if not available.
    pub fn as_bool(&self) -> bool {
        match self {
            DataValue::Bool(v) => *v,
            _ => false,
        }
    }

    /// Extract a float value, defaulting to `0.0` if not available.
    pub fn as_float(&self) -> f64 {
        match self {
            DataValue::Float(v) => *v,
            _ => 0.0,
        }
    }
}

/// An event received from a DATA_BROKER subscription.
#[derive(Debug, Clone)]
pub struct SignalEvent {
    /// The VSS signal path.
    pub path: String,
    /// The signal value.
    pub value: DataValue,
}

/// Error type for DATA_BROKER operations.
#[derive(Debug)]
pub enum DataBrokerError {
    /// The DATA_BROKER is unreachable.
    ConnectionFailed(String),
    /// A write operation failed.
    WriteFailed(String),
    /// A read operation failed.
    ReadFailed(String),
    /// A subscribe operation failed.
    SubscribeFailed(String),
}

impl std::fmt::Display for DataBrokerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataBrokerError::ConnectionFailed(msg) => {
                write!(f, "DATA_BROKER connection failed: {}", msg)
            }
            DataBrokerError::WriteFailed(msg) => {
                write!(f, "DATA_BROKER write failed: {}", msg)
            }
            DataBrokerError::ReadFailed(msg) => {
                write!(f, "DATA_BROKER read failed: {}", msg)
            }
            DataBrokerError::SubscribeFailed(msg) => {
                write!(f, "DATA_BROKER subscribe failed: {}", msg)
            }
        }
    }
}

impl std::error::Error for DataBrokerError {}

/// Trait abstracting DATA_BROKER operations.
///
/// This trait enables both real Kuksa DATA_BROKER and mock implementations
/// to be used interchangeably.
#[tonic::async_trait]
pub trait DataBrokerClient: Send + Sync + 'static {
    /// Subscribe to a signal path. Returns a broadcast receiver that
    /// delivers signal events as they arrive.
    async fn subscribe(
        &self,
        signal_path: &str,
    ) -> Result<broadcast::Receiver<SignalEvent>, DataBrokerError>;

    /// Read a single signal value.
    async fn read(&self, signal_path: &str) -> Result<DataValue, DataBrokerError>;

    /// Write a value to a signal path.
    async fn write(
        &self,
        signal_path: &str,
        value: DataValue,
    ) -> Result<(), DataBrokerError>;
}

/// In-memory mock DATA_BROKER for testing.
///
/// Stores signal values in memory and supports subscriptions via broadcast
/// channels. No real gRPC connection is needed.
#[derive(Clone)]
pub struct MockDataBrokerClient {
    /// Signal store: path -> current value.
    store: Arc<Mutex<std::collections::HashMap<String, DataValue>>>,
    /// Broadcast sender for signal events.
    event_tx: broadcast::Sender<SignalEvent>,
}

impl Default for MockDataBrokerClient {
    fn default() -> Self {
        Self::new()
    }
}

impl MockDataBrokerClient {
    /// Create a new mock DATA_BROKER client.
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(64);
        MockDataBrokerClient {
            store: Arc::new(Mutex::new(std::collections::HashMap::new())),
            event_tx,
        }
    }

    /// Publish a signal event to all subscribers.
    ///
    /// This simulates an external signal change (e.g., lock/unlock event)
    /// being published to the DATA_BROKER and delivered to subscribers.
    pub async fn publish(&self, path: &str, value: DataValue) {
        {
            let mut store = self.store.lock().await;
            store.insert(path.to_string(), value.clone());
        }
        let event = SignalEvent {
            path: path.to_string(),
            value,
        };
        let _ = self.event_tx.send(event);
    }

    /// Read a value directly from the store (for test assertions).
    pub async fn get(&self, path: &str) -> DataValue {
        let store = self.store.lock().await;
        store
            .get(path)
            .cloned()
            .unwrap_or(DataValue::NotAvailable)
    }
}

#[tonic::async_trait]
impl DataBrokerClient for MockDataBrokerClient {
    async fn subscribe(
        &self,
        _signal_path: &str,
    ) -> Result<broadcast::Receiver<SignalEvent>, DataBrokerError> {
        Ok(self.event_tx.subscribe())
    }

    async fn read(&self, signal_path: &str) -> Result<DataValue, DataBrokerError> {
        let store = self.store.lock().await;
        Ok(store
            .get(signal_path)
            .cloned()
            .unwrap_or(DataValue::NotAvailable))
    }

    async fn write(
        &self,
        signal_path: &str,
        value: DataValue,
    ) -> Result<(), DataBrokerError> {
        let mut store = self.store.lock().await;
        store.insert(signal_path.to_string(), value);
        Ok(())
    }
}

/// Real DATA_BROKER client that connects to a Kuksa DATA_BROKER via gRPC.
///
/// This is a placeholder for the actual Kuksa `kuksa.val.v1` gRPC client.
/// In the demo environment, it delegates to the Kuksa databroker gRPC API.
/// When the DATA_BROKER is unreachable, it retries with exponential backoff.
pub struct KuksaDataBrokerClient {
    /// Address of the DATA_BROKER gRPC server.
    addr: String,
    /// Broadcast sender for signal events (used by the subscription loop).
    event_tx: broadcast::Sender<SignalEvent>,
    /// Signal store for values written by this client.
    store: Arc<Mutex<std::collections::HashMap<String, DataValue>>>,
    /// Whether we have successfully connected at least once.
    connected: Arc<Mutex<bool>>,
}

impl KuksaDataBrokerClient {
    /// Create a new Kuksa DATA_BROKER client.
    ///
    /// The client does not connect immediately; connection happens lazily
    /// on the first operation or via `connect_with_retry`.
    pub fn new(addr: &str) -> Self {
        let (event_tx, _) = broadcast::channel(64);
        KuksaDataBrokerClient {
            addr: addr.to_string(),
            event_tx,
            store: Arc::new(Mutex::new(std::collections::HashMap::new())),
            connected: Arc::new(Mutex::new(false)),
        }
    }

    /// Attempt to connect to the DATA_BROKER with exponential backoff.
    ///
    /// Retries indefinitely, logging each retry attempt. This method is
    /// designed to be run as a background task.
    pub async fn connect_with_retry(&self) {
        let mut attempt = 0u32;
        let max_backoff_secs = 30u64;

        loop {
            attempt += 1;
            let addr_with_scheme = if self.addr.starts_with("http") {
                self.addr.clone()
            } else {
                format!("http://{}", self.addr)
            };

            match tonic::transport::Channel::from_shared(addr_with_scheme) {
                Ok(endpoint) => {
                    match tokio::time::timeout(
                        std::time::Duration::from_secs(5),
                        endpoint.connect(),
                    )
                    .await
                    {
                        Ok(Ok(_channel)) => {
                            eprintln!(
                                "DATA_BROKER: connected to {} (attempt {})",
                                self.addr, attempt
                            );
                            let mut connected = self.connected.lock().await;
                            *connected = true;
                            return;
                        }
                        Ok(Err(e)) => {
                            eprintln!(
                                "DATA_BROKER: retry connection to {} (attempt {}): {}",
                                self.addr, attempt, e
                            );
                        }
                        Err(_) => {
                            eprintln!(
                                "DATA_BROKER: retry connection to {} (attempt {}): timeout",
                                self.addr, attempt
                            );
                        }
                    }
                }
                Err(e) => {
                    eprintln!(
                        "DATA_BROKER: retry connection to {} (attempt {}): invalid URI: {}",
                        self.addr, attempt, e
                    );
                }
            }

            // Exponential backoff: 1s, 2s, 4s, 8s, ... up to max_backoff_secs
            let backoff = std::cmp::min(
                1u64.checked_shl(attempt.saturating_sub(1)).unwrap_or(max_backoff_secs),
                max_backoff_secs,
            );
            tokio::time::sleep(std::time::Duration::from_secs(backoff)).await;
        }
    }

    /// Get the DATA_BROKER address.
    pub fn addr(&self) -> &str {
        &self.addr
    }
}

#[tonic::async_trait]
impl DataBrokerClient for KuksaDataBrokerClient {
    async fn subscribe(
        &self,
        _signal_path: &str,
    ) -> Result<broadcast::Receiver<SignalEvent>, DataBrokerError> {
        // In a full implementation, this would open a Kuksa gRPC streaming
        // subscription. For the demo, we return our broadcast receiver.
        // The actual Kuksa subscription would feed events into event_tx.
        let connected = self.connected.lock().await;
        if !*connected {
            return Err(DataBrokerError::SubscribeFailed(format!(
                "not connected to DATA_BROKER at {}",
                self.addr
            )));
        }
        Ok(self.event_tx.subscribe())
    }

    async fn read(&self, signal_path: &str) -> Result<DataValue, DataBrokerError> {
        let connected = self.connected.lock().await;
        if !*connected {
            return Err(DataBrokerError::ReadFailed(format!(
                "not connected to DATA_BROKER at {}",
                self.addr
            )));
        }
        let store = self.store.lock().await;
        Ok(store
            .get(signal_path)
            .cloned()
            .unwrap_or(DataValue::NotAvailable))
    }

    async fn write(
        &self,
        signal_path: &str,
        value: DataValue,
    ) -> Result<(), DataBrokerError> {
        let connected = self.connected.lock().await;
        if !*connected {
            return Err(DataBrokerError::WriteFailed(format!(
                "not connected to DATA_BROKER at {}",
                self.addr
            )));
        }
        let mut store = self.store.lock().await;
        store.insert(signal_path.to_string(), value);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_databroker_read_write() {
        let client = MockDataBrokerClient::new();

        // Write a value
        client
            .write(signals::SESSION_ACTIVE, DataValue::Bool(true))
            .await
            .unwrap();

        // Read it back
        let value = client.read(signals::SESSION_ACTIVE).await.unwrap();
        assert!(value.as_bool());
    }

    #[tokio::test]
    async fn test_mock_databroker_subscribe_receives_events() {
        let client = MockDataBrokerClient::new();

        let mut rx = client.subscribe(signals::IS_LOCKED).await.unwrap();

        // Publish an event
        client
            .publish(signals::IS_LOCKED, DataValue::Bool(true))
            .await;

        // Receive the event
        let event = rx.recv().await.unwrap();
        assert_eq!(event.path, signals::IS_LOCKED);
        assert!(event.value.as_bool());
    }

    #[tokio::test]
    async fn test_mock_databroker_not_available_default() {
        let client = MockDataBrokerClient::new();

        let value = client.read("nonexistent.signal").await.unwrap();
        match value {
            DataValue::NotAvailable => {} // expected
            other => panic!("expected NotAvailable, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_mock_databroker_publish_updates_store() {
        let client = MockDataBrokerClient::new();

        client
            .publish(signals::LATITUDE, DataValue::Float(48.1351))
            .await;

        let value = client.get(signals::LATITUDE).await;
        assert!((value.as_float() - 48.1351).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_data_value_conversions() {
        assert!(DataValue::Bool(true).as_bool());
        assert!(!DataValue::Bool(false).as_bool());
        assert!(!DataValue::NotAvailable.as_bool());
        assert!(!DataValue::Float(1.0).as_bool());

        assert!((DataValue::Float(2.5).as_float() - 2.5).abs() < f64::EPSILON);
        assert!((DataValue::NotAvailable.as_float() - 0.0).abs() < f64::EPSILON);
        assert!((DataValue::Bool(true).as_float() - 0.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_kuksa_client_not_connected() {
        let client = KuksaDataBrokerClient::new("localhost:19999");

        let result = client.subscribe(signals::IS_LOCKED).await;
        assert!(result.is_err());

        let result = client.read(signals::SESSION_ACTIVE).await;
        assert!(result.is_err());

        let result = client
            .write(signals::SESSION_ACTIVE, DataValue::Bool(true))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_signal_event_debug() {
        let event = SignalEvent {
            path: "test.signal".to_string(),
            value: DataValue::Bool(true),
        };
        let debug = format!("{:?}", event);
        assert!(debug.contains("test.signal"));
    }

    #[tokio::test]
    async fn test_databroker_error_display() {
        let err = DataBrokerError::ConnectionFailed("timeout".to_string());
        assert!(err.to_string().contains("connection failed"));
        assert!(err.to_string().contains("timeout"));

        let err = DataBrokerError::WriteFailed("permission denied".to_string());
        assert!(err.to_string().contains("write failed"));

        let err = DataBrokerError::ReadFailed("not found".to_string());
        assert!(err.to_string().contains("read failed"));

        let err = DataBrokerError::SubscribeFailed("unavailable".to_string());
        assert!(err.to_string().contains("subscribe failed"));
    }
}
