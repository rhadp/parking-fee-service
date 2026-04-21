//! DATA_BROKER client abstraction.
//!
//! Wraps a Kuksa Databroker gRPC client and provides typed subscribe/set
//! operations on VSS signals.
//!
//! NOTE: Actual gRPC code generation (tonic-build + kuksa.val.v2 proto) is
//! wired in task group 4.  This module exposes trait-based abstractions used
//! by event_loop for testing without a live DATA_BROKER.

use async_trait::async_trait;

/// Errors produced by DATA_BROKER operations.
#[derive(Debug, Clone)]
pub enum BrokerError {
    /// DATA_BROKER is unreachable or returned a gRPC error.
    Unavailable(String),
}

impl std::fmt::Display for BrokerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BrokerError::Unavailable(msg) => write!(f, "broker unavailable: {msg}"),
        }
    }
}

/// Trait for publishing VSS `Vehicle.Parking.SessionActive` to DATA_BROKER.
///
/// Implementations must be `Send + Sync` to support trait-object usage across
/// async task boundaries.
#[async_trait]
pub trait SessionPublisher: Send + Sync {
    /// Set `Vehicle.Parking.SessionActive` to `active` in DATA_BROKER.
    ///
    /// Failure must be tolerated: log the error and continue (08-REQ-4.E1).
    async fn set_session_active(&self, active: bool) -> Result<(), BrokerError>;
}

/// Concrete DATA_BROKER client backed by the Kuksa gRPC service.
///
/// Connection and subscription behaviour wired in task group 4.
pub struct BrokerClient;

impl BrokerClient {
    /// Connect to DATA_BROKER at `addr`, retrying with exponential backoff.
    ///
    /// Exits with a non-zero code after 5 failed attempts (08-REQ-3.E3).
    pub async fn connect(_addr: &str) -> Result<Self, BrokerError> {
        todo!("implement BrokerClient::connect")
    }
}

#[async_trait]
impl SessionPublisher for BrokerClient {
    async fn set_session_active(&self, _active: bool) -> Result<(), BrokerError> {
        todo!("implement BrokerClient::set_session_active")
    }
}
