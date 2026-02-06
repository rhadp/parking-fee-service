//! Test utilities for cloud-gateway-client.
//!
//! This module provides mock implementations of external services
//! for integration testing.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tokio::sync::{mpsc, Mutex, RwLock};

use crate::command::{Command, CommandType, Door};
use crate::error::{ForwardError, TelemetryError};
use crate::forwarder::ForwardResult;
use crate::subscriber::{SignalUpdate, SignalValue};
use crate::telemetry::Telemetry;

/// Mock LOCKING_SERVICE for testing command forwarding.
#[derive(Debug, Clone)]
pub struct MockLockingService {
    /// Configured responses for lock/unlock commands
    responses: Arc<RwLock<HashMap<CommandType, MockResponse>>>,
    /// Recorded commands for verification
    recorded_commands: Arc<Mutex<Vec<Command>>>,
    /// Simulated delay before response
    delay: Option<Duration>,
}

/// Mock response configuration.
#[derive(Debug, Clone)]
pub struct MockResponse {
    /// Whether the response should be successful
    pub success: bool,
    /// Error message if not successful
    pub error_message: Option<String>,
}

impl Default for MockResponse {
    fn default() -> Self {
        Self {
            success: true,
            error_message: None,
        }
    }
}

impl MockLockingService {
    /// Create a new mock LOCKING_SERVICE that returns success for all commands.
    pub fn new() -> Self {
        Self {
            responses: Arc::new(RwLock::new(HashMap::new())),
            recorded_commands: Arc::new(Mutex::new(Vec::new())),
            delay: None,
        }
    }

    /// Create a mock that fails all commands.
    pub fn failing(error_message: &str) -> Self {
        let mut responses = HashMap::new();
        responses.insert(
            CommandType::Lock,
            MockResponse {
                success: false,
                error_message: Some(error_message.to_string()),
            },
        );
        responses.insert(
            CommandType::Unlock,
            MockResponse {
                success: false,
                error_message: Some(error_message.to_string()),
            },
        );

        Self {
            responses: Arc::new(RwLock::new(responses)),
            recorded_commands: Arc::new(Mutex::new(Vec::new())),
            delay: None,
        }
    }

    /// Set a delay for simulating slow responses (for timeout testing).
    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = Some(delay);
        self
    }

    /// Configure response for a specific command type.
    pub async fn set_response(&self, command_type: CommandType, response: MockResponse) {
        self.responses.write().await.insert(command_type, response);
    }

    /// Execute a lock command.
    pub async fn lock(&self, command: &Command) -> Result<ForwardResult, ForwardError> {
        self.execute_command(command, CommandType::Lock).await
    }

    /// Execute an unlock command.
    pub async fn unlock(&self, command: &Command) -> Result<ForwardResult, ForwardError> {
        self.execute_command(command, CommandType::Unlock).await
    }

    /// Execute a command and return the configured response.
    async fn execute_command(
        &self,
        command: &Command,
        expected_type: CommandType,
    ) -> Result<ForwardResult, ForwardError> {
        // Record the command
        self.recorded_commands.lock().await.push(command.clone());

        // Apply delay if configured
        if let Some(delay) = self.delay {
            tokio::time::sleep(delay).await;
        }

        // Get configured response or default to success
        let responses = self.responses.read().await;
        let response = responses.get(&expected_type).cloned().unwrap_or_default();

        if response.success {
            Ok(ForwardResult::success())
        } else {
            Err(ForwardError::ExecutionFailed(
                response
                    .error_message
                    .unwrap_or_else(|| "Mock error".to_string()),
            ))
        }
    }

    /// Get all recorded commands.
    pub async fn get_recorded_commands(&self) -> Vec<Command> {
        self.recorded_commands.lock().await.clone()
    }

    /// Clear recorded commands.
    pub async fn clear_recorded_commands(&self) {
        self.recorded_commands.lock().await.clear();
    }
}

impl Default for MockLockingService {
    fn default() -> Self {
        Self::new()
    }
}

/// Mock DATA_BROKER for testing signal subscription.
#[derive(Debug)]
pub struct MockDataBroker {
    /// Channel to send signals to subscribers
    signal_tx: mpsc::Sender<SignalUpdate>,
    /// Whether the broker is connected
    connected: Arc<RwLock<bool>>,
    /// Recorded subscriptions
    subscribed_paths: Arc<Mutex<Vec<String>>>,
}

impl MockDataBroker {
    /// Create a new mock DATA_BROKER with a signal channel.
    pub fn new(signal_tx: mpsc::Sender<SignalUpdate>) -> Self {
        Self {
            signal_tx,
            connected: Arc::new(RwLock::new(true)),
            subscribed_paths: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Simulate subscribing to a signal path.
    pub async fn subscribe(&self, path: &str) -> Result<(), TelemetryError> {
        if !*self.connected.read().await {
            return Err(TelemetryError::DataBrokerUnavailable(
                "Mock DATA_BROKER disconnected".to_string(),
            ));
        }

        self.subscribed_paths.lock().await.push(path.to_string());
        Ok(())
    }

    /// Send a signal update to subscribers.
    pub async fn send_signal(&self, update: SignalUpdate) -> Result<(), TelemetryError> {
        if !*self.connected.read().await {
            return Err(TelemetryError::DataBrokerUnavailable(
                "Mock DATA_BROKER disconnected".to_string(),
            ));
        }

        self.signal_tx
            .send(update)
            .await
            .map_err(|e| TelemetryError::SubscriptionFailed(e.to_string()))
    }

    /// Send a latitude update.
    pub async fn send_latitude(&self, lat: f64) -> Result<(), TelemetryError> {
        self.send_signal(SignalUpdate::new_float(
            "Vehicle.CurrentLocation.Latitude",
            lat,
        ))
        .await
    }

    /// Send a longitude update.
    pub async fn send_longitude(&self, lng: f64) -> Result<(), TelemetryError> {
        self.send_signal(SignalUpdate::new_float(
            "Vehicle.CurrentLocation.Longitude",
            lng,
        ))
        .await
    }

    /// Send a door locked update.
    pub async fn send_door_locked(&self, locked: bool) -> Result<(), TelemetryError> {
        self.send_signal(SignalUpdate::new_bool(
            "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked",
            locked,
        ))
        .await
    }

    /// Send a door open update.
    pub async fn send_door_open(&self, open: bool) -> Result<(), TelemetryError> {
        self.send_signal(SignalUpdate::new_bool(
            "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen",
            open,
        ))
        .await
    }

    /// Send a parking session update.
    pub async fn send_parking_session(&self, active: bool) -> Result<(), TelemetryError> {
        self.send_signal(SignalUpdate::new_bool(
            "Vehicle.Parking.SessionActive",
            active,
        ))
        .await
    }

    /// Simulate disconnection.
    pub async fn disconnect(&self) {
        *self.connected.write().await = false;
    }

    /// Simulate reconnection.
    pub async fn reconnect(&self) {
        *self.connected.write().await = true;
    }

    /// Check if connected.
    pub async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }

    /// Get subscribed paths.
    pub async fn get_subscribed_paths(&self) -> Vec<String> {
        self.subscribed_paths.lock().await.clone()
    }
}

/// Mock MQTT client for testing.
#[derive(Debug)]
pub struct MockMqttClient {
    /// Whether the client is connected
    connected: Arc<RwLock<bool>>,
    /// Published messages
    published_messages: Arc<Mutex<Vec<(String, Vec<u8>)>>>,
    /// Subscribed topics
    subscribed_topics: Arc<Mutex<Vec<String>>>,
}

impl MockMqttClient {
    /// Create a new mock MQTT client.
    pub fn new() -> Self {
        Self {
            connected: Arc::new(RwLock::new(true)),
            published_messages: Arc::new(Mutex::new(Vec::new())),
            subscribed_topics: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Check if connected.
    pub async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }

    /// Simulate connecting.
    pub async fn connect(&self) {
        *self.connected.write().await = true;
    }

    /// Simulate disconnecting.
    pub async fn disconnect(&self) {
        *self.connected.write().await = false;
    }

    /// Subscribe to a topic.
    pub async fn subscribe(&self, topic: &str) -> Result<(), crate::error::MqttError> {
        if !self.is_connected().await {
            return Err(crate::error::MqttError::SubscribeFailed(
                "Not connected".to_string(),
            ));
        }

        self.subscribed_topics.lock().await.push(topic.to_string());
        Ok(())
    }

    /// Publish a message.
    pub async fn publish(
        &self,
        topic: &str,
        payload: &[u8],
    ) -> Result<(), crate::error::MqttError> {
        if !self.is_connected().await {
            return Err(crate::error::MqttError::PublishFailed(
                "Not connected".to_string(),
            ));
        }

        self.published_messages
            .lock()
            .await
            .push((topic.to_string(), payload.to_vec()));
        Ok(())
    }

    /// Get all published messages.
    pub async fn get_published_messages(&self) -> Vec<(String, Vec<u8>)> {
        self.published_messages.lock().await.clone()
    }

    /// Get published telemetry messages.
    pub async fn get_published_telemetry(&self) -> Vec<Telemetry> {
        let messages = self.published_messages.lock().await;
        messages
            .iter()
            .filter(|(topic, _)| topic.contains("/telemetry"))
            .filter_map(|(_, payload)| serde_json::from_slice(payload).ok())
            .collect()
    }

    /// Clear published messages.
    pub async fn clear_published_messages(&self) {
        self.published_messages.lock().await.clear();
    }

    /// Get subscribed topics.
    pub async fn get_subscribed_topics(&self) -> Vec<String> {
        self.subscribed_topics.lock().await.clone()
    }
}

impl Default for MockMqttClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Test certificate utilities.
pub mod test_certs {
    use std::fs;
    use std::path::PathBuf;

    use tempfile::TempDir;

    /// Create a test certificate directory with mock certificates.
    pub fn create_test_cert_dir() -> (TempDir, PathBuf, PathBuf, PathBuf) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let base_path = temp_dir.path();

        let ca_path = base_path.join("ca.crt");
        let cert_path = base_path.join("client.crt");
        let key_path = base_path.join("client.key");

        // Write mock certificate content (PEM format headers)
        let mock_ca = "-----BEGIN CERTIFICATE-----\nMOCK_CA_CERT\n-----END CERTIFICATE-----\n";
        let mock_cert =
            "-----BEGIN CERTIFICATE-----\nMOCK_CLIENT_CERT\n-----END CERTIFICATE-----\n";
        let mock_key = "-----BEGIN PRIVATE KEY-----\nMOCK_PRIVATE_KEY\n-----END PRIVATE KEY-----\n";

        fs::write(&ca_path, mock_ca).expect("Failed to write CA cert");
        fs::write(&cert_path, mock_cert).expect("Failed to write client cert");
        fs::write(&key_path, mock_key).expect("Failed to write client key");

        (temp_dir, ca_path, cert_path, key_path)
    }

    /// Create an invalid certificate file.
    pub fn create_invalid_cert(path: &PathBuf) {
        fs::write(path, "INVALID CERTIFICATE DATA").expect("Failed to write invalid cert");
    }

    /// Update a certificate file to simulate hot-reload.
    pub fn update_cert(path: &PathBuf, content: &str) {
        fs::write(path, content).expect("Failed to update cert");
    }
}

/// Create a test command for testing.
pub fn create_test_command(command_id: &str, command_type: CommandType) -> Command {
    Command {
        command_id: command_id.to_string(),
        command_type,
        doors: vec![Door::All],
        auth_token: "test-token".to_string(),
        timestamp: Some(Utc::now().to_rfc3339()),
    }
}

/// Create a test telemetry message.
pub fn create_test_telemetry(lat: f64, lng: f64, door_locked: bool) -> Telemetry {
    Telemetry {
        timestamp: Utc::now().to_rfc3339(),
        latitude: lat,
        longitude: lng,
        door_locked,
        door_open: false,
        parking_session_active: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_locking_service_success() {
        let mock = MockLockingService::new();
        let command = create_test_command("cmd-1", CommandType::Lock);

        let result = mock.lock(&command).await;
        assert!(result.is_ok());

        let recorded = mock.get_recorded_commands().await;
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0].command_id, "cmd-1");
    }

    #[tokio::test]
    async fn test_mock_locking_service_failure() {
        let mock = MockLockingService::failing("Test error");
        let command = create_test_command("cmd-1", CommandType::Lock);

        let result = mock.lock(&command).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_data_broker() {
        let (tx, mut rx) = mpsc::channel(10);
        let mock = MockDataBroker::new(tx);

        mock.send_latitude(37.7749).await.unwrap();

        let update = rx.recv().await.unwrap();
        match update.value {
            SignalValue::Float(v) => assert!((v - 37.7749).abs() < 0.0001),
            _ => panic!("Expected float value"),
        }
    }

    #[tokio::test]
    async fn test_mock_data_broker_disconnect() {
        let (tx, _rx) = mpsc::channel(10);
        let mock = MockDataBroker::new(tx);

        mock.disconnect().await;

        let result = mock.send_latitude(37.7749).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_mqtt_client() {
        let mock = MockMqttClient::new();

        mock.publish("test/topic", b"hello").await.unwrap();

        let messages = mock.get_published_messages().await;
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].0, "test/topic");
        assert_eq!(messages[0].1, b"hello");
    }

    #[tokio::test]
    async fn test_mock_mqtt_client_disconnect() {
        let mock = MockMqttClient::new();
        mock.disconnect().await;

        let result = mock.publish("test/topic", b"hello").await;
        assert!(result.is_err());
    }

    #[test]
    fn test_create_test_certs() {
        let (temp_dir, ca_path, cert_path, key_path) = test_certs::create_test_cert_dir();

        assert!(ca_path.exists());
        assert!(cert_path.exists());
        assert!(key_path.exists());

        // Verify PEM format
        let ca_content = std::fs::read_to_string(&ca_path).unwrap();
        assert!(ca_content.contains("-----BEGIN CERTIFICATE-----"));

        drop(temp_dir); // Clean up
    }

    #[test]
    fn test_create_test_command() {
        let cmd = create_test_command("test-id", CommandType::Lock);
        assert_eq!(cmd.command_id, "test-id");
        assert_eq!(cmd.command_type, CommandType::Lock);
    }

    #[test]
    fn test_create_test_telemetry() {
        let telem = create_test_telemetry(37.7749, -122.4194, true);
        assert!((telem.latitude - 37.7749).abs() < 0.0001);
        assert!((telem.longitude - (-122.4194)).abs() < 0.0001);
        assert!(telem.door_locked);
    }
}
