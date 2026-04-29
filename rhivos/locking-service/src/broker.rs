use std::fmt;

/// Errors that can occur during DATA_BROKER communication.
#[derive(Debug)]
pub enum BrokerError {
    /// Connection to DATA_BROKER failed.
    ConnectionFailed(String),
    /// An RPC call to DATA_BROKER failed.
    RpcError(String),
}

impl fmt::Display for BrokerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BrokerError::ConnectionFailed(msg) => write!(f, "connection failed: {msg}"),
            BrokerError::RpcError(msg) => write!(f, "rpc error: {msg}"),
        }
    }
}

impl std::error::Error for BrokerError {}

/// VSS signal path constants.
pub const SIGNAL_SPEED: &str = "Vehicle.Speed";
pub const SIGNAL_DOOR_OPEN: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen";
pub const SIGNAL_IS_LOCKED: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked";
pub const SIGNAL_COMMAND: &str = "Vehicle.Command.Door.Lock";
pub const SIGNAL_RESPONSE: &str = "Vehicle.Command.Door.Response";

/// Abstraction over the DATA_BROKER gRPC client for testability.
pub trait BrokerClient {
    /// Read a float signal value from DATA_BROKER.
    fn get_float(
        &self,
        signal: &str,
    ) -> impl std::future::Future<Output = Result<Option<f32>, BrokerError>>;

    /// Read a boolean signal value from DATA_BROKER.
    fn get_bool(
        &self,
        signal: &str,
    ) -> impl std::future::Future<Output = Result<Option<bool>, BrokerError>>;

    /// Write a boolean signal value to DATA_BROKER.
    fn set_bool(
        &self,
        signal: &str,
        value: bool,
    ) -> impl std::future::Future<Output = Result<(), BrokerError>>;

    /// Write a string signal value to DATA_BROKER.
    fn set_string(
        &self,
        signal: &str,
        value: &str,
    ) -> impl std::future::Future<Output = Result<(), BrokerError>>;
}
