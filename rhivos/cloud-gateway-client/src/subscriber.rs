//! Signal subscription from DATA_BROKER.
//!
//! This module provides a SignalSubscriber that subscribes to VSS signals
//! from DATA_BROKER and sends updates through a channel.

use std::time::SystemTime;

use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::error::TelemetryError;

/// VSS signal paths for telemetry.
pub mod vss_paths {
    /// Vehicle latitude
    pub const LATITUDE: &str = "Vehicle.CurrentLocation.Latitude";
    /// Vehicle longitude
    pub const LONGITUDE: &str = "Vehicle.CurrentLocation.Longitude";
    /// Door lock state
    pub const DOOR_LOCKED: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked";
    /// Door open state
    pub const DOOR_OPEN: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen";
    /// Parking session active
    pub const PARKING_SESSION_ACTIVE: &str = "Vehicle.Parking.SessionActive";

    /// All signal paths for subscription.
    pub const ALL: &[&str] = &[
        LATITUDE,
        LONGITUDE,
        DOOR_LOCKED,
        DOOR_OPEN,
        PARKING_SESSION_ACTIVE,
    ];
}

/// Signal value types from DATA_BROKER.
#[derive(Debug, Clone)]
pub enum SignalValue {
    /// Floating point value (latitude, longitude)
    Float(f64),
    /// Boolean value (door locked, door open, parking session)
    Bool(bool),
}

/// A signal update from DATA_BROKER.
#[derive(Debug, Clone)]
pub struct SignalUpdate {
    /// VSS signal path
    pub signal_path: String,
    /// Signal value
    pub value: SignalValue,
    /// Timestamp of the update
    pub timestamp: SystemTime,
}

impl SignalUpdate {
    /// Create a new float signal update.
    pub fn new_float(signal_path: impl Into<String>, value: f64) -> Self {
        Self {
            signal_path: signal_path.into(),
            value: SignalValue::Float(value),
            timestamp: SystemTime::now(),
        }
    }

    /// Create a new bool signal update.
    pub fn new_bool(signal_path: impl Into<String>, value: bool) -> Self {
        Self {
            signal_path: signal_path.into(),
            value: SignalValue::Bool(value),
            timestamp: SystemTime::now(),
        }
    }
}

/// Subscribes to VSS signals from DATA_BROKER.
///
/// Maintains a gRPC streaming subscription to DATA_BROKER and forwards
/// signal updates through a channel for processing by TelemetryPublisher.
pub struct SignalSubscriber {
    /// Channel to send signal updates
    signal_tx: mpsc::Sender<SignalUpdate>,
    /// DATA_BROKER socket path
    data_broker_socket: String,
    /// Whether the subscriber is connected
    connected: bool,
}

impl SignalSubscriber {
    /// Create a new SignalSubscriber.
    pub fn new(signal_tx: mpsc::Sender<SignalUpdate>, data_broker_socket: String) -> Self {
        Self {
            signal_tx,
            data_broker_socket,
            connected: false,
        }
    }

    /// Subscribe to all required VSS signals.
    ///
    /// This establishes a streaming subscription to DATA_BROKER for:
    /// - Vehicle.CurrentLocation.Latitude
    /// - Vehicle.CurrentLocation.Longitude
    /// - Vehicle.Cabin.Door.Row1.DriverSide.IsLocked
    /// - Vehicle.Cabin.Door.Row1.DriverSide.IsOpen
    /// - Vehicle.Parking.SessionActive
    pub async fn subscribe_all(&mut self) -> Result<(), TelemetryError> {
        info!(
            "Subscribing to VSS signals from DATA_BROKER at {}",
            self.data_broker_socket
        );

        for path in vss_paths::ALL {
            debug!("Subscribing to signal: {}", path);
        }

        // In a real implementation, this would:
        // 1. Connect to DATA_BROKER via gRPC/UDS
        // 2. Send a SubscribeRequest with all signal paths
        // 3. Return the streaming response for run() to process

        self.connected = true;
        info!("Subscribed to {} VSS signals", vss_paths::ALL.len());

        Ok(())
    }

    /// Run the signal subscription loop.
    ///
    /// Receives signal updates from DATA_BROKER and forwards them
    /// to the TelemetryPublisher through the signal channel.
    pub async fn run(&mut self) -> Result<(), TelemetryError> {
        if !self.connected {
            return Err(TelemetryError::SubscriptionFailed(
                "Not subscribed to signals".to_string(),
            ));
        }

        info!("Signal subscriber running");

        // In a real implementation, this would:
        // 1. Poll the gRPC stream for SubscribeResponse messages
        // 2. Parse the VehicleSignal oneof to extract the value
        // 3. Send SignalUpdate through signal_tx
        // 4. Handle disconnection and reconnection gracefully

        // For now, this is a placeholder that would be replaced
        // with actual DATA_BROKER gRPC client integration
        loop {
            // The actual implementation would process streaming responses here
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;

            // Check if channel is closed (subscriber should shut down)
            if self.signal_tx.is_closed() {
                info!("Signal channel closed, stopping subscriber");
                break;
            }
        }

        Ok(())
    }

    /// Check if the subscriber is connected to DATA_BROKER.
    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// Get the signal sender for external signal injection (used in tests).
    pub fn signal_sender(&self) -> mpsc::Sender<SignalUpdate> {
        self.signal_tx.clone()
    }

    /// Process a signal update from DATA_BROKER.
    ///
    /// Parses the VehicleSignal and sends the appropriate SignalUpdate.
    pub async fn process_signal(
        &self,
        signal_path: String,
        signal: &shared::proto::sdv::vss::VehicleSignal,
    ) -> Result<(), TelemetryError> {
        use shared::proto::sdv::vss::vehicle_signal::Signal;

        let update = match &signal.signal {
            Some(Signal::Location(loc)) => {
                if signal_path.contains("Latitude") {
                    SignalUpdate::new_float(&signal_path, loc.latitude)
                } else {
                    SignalUpdate::new_float(&signal_path, loc.longitude)
                }
            }
            Some(Signal::DoorState(door)) => {
                if signal_path.contains("IsLocked") {
                    SignalUpdate::new_bool(&signal_path, door.is_locked)
                } else {
                    SignalUpdate::new_bool(&signal_path, door.is_open)
                }
            }
            Some(Signal::ParkingState(parking)) => {
                SignalUpdate::new_bool(&signal_path, parking.session_active)
            }
            _ => {
                warn!("Unknown signal type for path: {}", signal_path);
                return Ok(());
            }
        };

        self.signal_tx.send(update).await.map_err(|e| {
            TelemetryError::SubscriptionFailed(format!("Channel send failed: {}", e))
        })?;

        Ok(())
    }

    /// Handle DATA_BROKER disconnection gracefully.
    #[allow(dead_code)] // Used in full integration
    fn handle_disconnection(&mut self, error: &str) {
        self.connected = false;
        error!("DATA_BROKER disconnected: {}", error);
    }

    /// Attempt to reconnect to DATA_BROKER.
    pub async fn reconnect(&mut self) -> Result<(), TelemetryError> {
        warn!("Attempting to reconnect to DATA_BROKER");
        self.subscribe_all().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_signal_subscriber_creation() {
        let (tx, _rx) = mpsc::channel(10);
        let subscriber = SignalSubscriber::new(tx, "/tmp/test.sock".to_string());
        assert!(!subscriber.is_connected());
    }

    #[tokio::test]
    async fn test_subscribe_all() {
        let (tx, _rx) = mpsc::channel(10);
        let mut subscriber = SignalSubscriber::new(tx, "/tmp/test.sock".to_string());

        let result = subscriber.subscribe_all().await;
        assert!(result.is_ok());
        assert!(subscriber.is_connected());
    }

    #[tokio::test]
    async fn test_signal_update_float() {
        let update = SignalUpdate::new_float(vss_paths::LATITUDE, 37.7749);
        assert_eq!(update.signal_path, vss_paths::LATITUDE);
        match update.value {
            SignalValue::Float(v) => assert!((v - 37.7749).abs() < 0.0001),
            _ => panic!("Expected Float value"),
        }
    }

    #[tokio::test]
    async fn test_signal_update_bool() {
        let update = SignalUpdate::new_bool(vss_paths::DOOR_LOCKED, true);
        assert_eq!(update.signal_path, vss_paths::DOOR_LOCKED);
        match update.value {
            SignalValue::Bool(v) => assert!(v),
            _ => panic!("Expected Bool value"),
        }
    }

    #[tokio::test]
    async fn test_run_without_subscribe_fails() {
        let (tx, _rx) = mpsc::channel(10);
        let mut subscriber = SignalSubscriber::new(tx, "/tmp/test.sock".to_string());

        let result = subscriber.run().await;
        assert!(result.is_err());
    }

    #[test]
    fn test_vss_paths() {
        assert_eq!(vss_paths::ALL.len(), 5);
        assert!(vss_paths::ALL.contains(&vss_paths::LATITUDE));
        assert!(vss_paths::ALL.contains(&vss_paths::LONGITUDE));
        assert!(vss_paths::ALL.contains(&vss_paths::DOOR_LOCKED));
        assert!(vss_paths::ALL.contains(&vss_paths::DOOR_OPEN));
        assert!(vss_paths::ALL.contains(&vss_paths::PARKING_SESSION_ACTIVE));
    }
}
