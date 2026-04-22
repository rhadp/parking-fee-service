use std::fmt;

/// Generated code from vendored kuksa.val.v1 proto files.
pub mod kuksa {
    pub mod val {
        pub mod v1 {
            tonic::include_proto!("kuksa.val.v1");
        }
    }
}

/// Generated code from vendored parking_adaptor.v1 proto files.
pub mod parking_adaptor {
    pub mod v1 {
        tonic::include_proto!("parking_adaptor.v1");
    }
}

/// Error type for broker operations.
#[derive(Debug)]
pub enum BrokerError {
    /// Connection to DATA_BROKER failed.
    ConnectionFailed(String),
    /// A gRPC call failed.
    RpcFailed(String),
}

impl fmt::Display for BrokerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BrokerError::ConnectionFailed(msg) => write!(f, "connection failed: {msg}"),
            BrokerError::RpcFailed(msg) => write!(f, "rpc failed: {msg}"),
        }
    }
}

impl std::error::Error for BrokerError {}

/// VSS signal path for door lock state.
pub const SIGNAL_IS_LOCKED: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked";
/// VSS signal path for parking session active state.
pub const SIGNAL_SESSION_ACTIVE: &str = "Vehicle.Parking.SessionActive";

/// Abstraction over the DATA_BROKER gRPC client.
///
/// Uses async fn in trait (AFIT). The trait is used with concrete types
/// (generics, not dyn), so auto trait bounds on the returned futures are
/// not needed. The MockBrokerClient uses RefCell (not Send/Sync) and
/// requires single-threaded tokio runtime.
#[allow(async_fn_in_trait)]
pub trait BrokerClient {
    /// Write a boolean signal value to DATA_BROKER.
    async fn set_bool(&self, signal: &str, value: bool) -> Result<(), BrokerError>;
}
