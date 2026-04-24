use std::fmt;

// Signal path constants
/// VSS signal for driver door lock state.
pub const SIGNAL_IS_LOCKED: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked";
/// Custom VSS signal for parking session active state.
pub const SIGNAL_SESSION_ACTIVE: &str = "Vehicle.Parking.SessionActive";

/// Generated kuksa.val.v2 gRPC types and client.
#[allow(dead_code)]
mod kuksa {
    pub mod val {
        pub mod v2 {
            tonic::include_proto!("kuksa.val.v2");
        }
    }
}

/// Error type for DATA_BROKER operations.
#[derive(Debug)]
pub enum BrokerError {
    /// Connection failed.
    Connection(String),
    /// RPC call failed.
    Rpc(String),
}

impl fmt::Display for BrokerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BrokerError::Connection(msg) => write!(f, "connection error: {msg}"),
            BrokerError::Rpc(msg) => write!(f, "RPC error: {msg}"),
        }
    }
}

impl std::error::Error for BrokerError {}

/// Trait for DATA_BROKER client operations.
///
/// Abstracting the broker client behind a trait allows event_loop tests
/// to inject a mock implementation.
pub trait DataBrokerClient {
    /// Set a boolean VSS signal value.
    fn set_bool(
        &self,
        signal: &str,
        value: bool,
    ) -> impl std::future::Future<Output = Result<(), BrokerError>>;
}
