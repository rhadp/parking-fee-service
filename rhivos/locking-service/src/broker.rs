//! DATA_BROKER gRPC client abstraction.
//!
//! Defines the `BrokerClient` trait and VSS signal path constants used throughout
//! the locking-service. The `GrpcBrokerClient` stub is wired up in task group 3.

#![allow(async_fn_in_trait)]
#![allow(dead_code)]

use std::fmt;

// ── VSS signal path constants ────────────────────────────────────────────────

pub const SIGNAL_COMMAND: &str = "Vehicle.Command.Door.Lock";
pub const SIGNAL_SPEED: &str = "Vehicle.Speed";
pub const SIGNAL_IS_OPEN: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen";
pub const SIGNAL_IS_LOCKED: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked";
pub const SIGNAL_RESPONSE: &str = "Vehicle.Command.Door.Response";

// ── BrokerError ──────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum BrokerError {
    Connection(String),
    NotFound(String),
    Other(String),
}

impl fmt::Display for BrokerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BrokerError::Connection(msg) => write!(f, "connection error: {msg}"),
            BrokerError::NotFound(msg) => write!(f, "not found: {msg}"),
            BrokerError::Other(msg) => write!(f, "broker error: {msg}"),
        }
    }
}

// ── BrokerClient trait ───────────────────────────────────────────────────────

/// Abstraction over the DATA_BROKER gRPC client.
///
/// Methods are async because they make network calls. Tests substitute
/// `MockBrokerClient` (from `testing.rs`) for deterministic unit testing.
pub trait BrokerClient {
    async fn get_float(&self, signal: &str) -> Result<Option<f32>, BrokerError>;
    async fn get_bool(&self, signal: &str) -> Result<Option<bool>, BrokerError>;
    async fn set_bool(&self, signal: &str, value: bool) -> Result<(), BrokerError>;
    async fn set_string(&self, signal: &str, value: &str) -> Result<(), BrokerError>;
}

// ── GrpcBrokerClient stub ────────────────────────────────────────────────────

/// Real gRPC-backed broker client. Full implementation in task group 3.
pub struct GrpcBrokerClient;

impl GrpcBrokerClient {
    /// Connect to DATA_BROKER with exponential backoff (5 attempts: 1s/2s/4s/8s).
    pub async fn connect(_addr: &str) -> Result<Self, BrokerError> {
        todo!("GrpcBrokerClient::connect — implemented in task group 3")
    }

    /// Subscribe to a VSS signal. Returns an mpsc Receiver of JSON payloads.
    pub async fn subscribe(
        &self,
        _signal: &str,
    ) -> Result<tokio::sync::mpsc::Receiver<String>, BrokerError> {
        todo!("GrpcBrokerClient::subscribe — implemented in task group 3")
    }
}

impl BrokerClient for GrpcBrokerClient {
    async fn get_float(&self, _signal: &str) -> Result<Option<f32>, BrokerError> {
        todo!()
    }
    async fn get_bool(&self, _signal: &str) -> Result<Option<bool>, BrokerError> {
        todo!()
    }
    async fn set_bool(&self, _signal: &str, _value: bool) -> Result<(), BrokerError> {
        todo!()
    }
    async fn set_string(&self, _signal: &str, _value: &str) -> Result<(), BrokerError> {
        todo!()
    }
}
