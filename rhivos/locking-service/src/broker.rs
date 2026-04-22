use std::fmt;

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

/// VSS signal path for vehicle speed.
pub const SIGNAL_SPEED: &str = "Vehicle.Speed";
/// VSS signal path for door open state.
pub const SIGNAL_DOOR_OPEN: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen";
/// VSS signal path for door lock state.
pub const SIGNAL_IS_LOCKED: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked";
/// VSS signal path for incoming lock/unlock commands.
pub const SIGNAL_COMMAND: &str = "Vehicle.Command.Door.Lock";
/// VSS signal path for command responses.
pub const SIGNAL_RESPONSE: &str = "Vehicle.Command.Door.Response";

/// Abstraction over the DATA_BROKER gRPC client.
///
/// Uses async fn in trait (AFIT). The trait is used with concrete types
/// (generics, not dyn), so auto trait bounds on the returned futures are
/// not needed. The MockBrokerClient uses RefCell (not Send/Sync) and
/// requires single-threaded tokio runtime.
#[allow(async_fn_in_trait)]
pub trait BrokerClient {
    /// Read a float signal value from DATA_BROKER.
    async fn get_float(&self, signal: &str) -> Result<Option<f32>, BrokerError>;
    /// Read a boolean signal value from DATA_BROKER.
    async fn get_bool(&self, signal: &str) -> Result<Option<bool>, BrokerError>;
    /// Write a boolean signal value to DATA_BROKER.
    async fn set_bool(&self, signal: &str, value: bool) -> Result<(), BrokerError>;
    /// Write a string signal value to DATA_BROKER.
    async fn set_string(&self, signal: &str, value: &str) -> Result<(), BrokerError>;
}
