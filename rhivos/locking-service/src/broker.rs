use std::fmt;

// Signal path constants
pub const SIGNAL_COMMAND: &str = "Vehicle.Command.Door.Lock";
pub const SIGNAL_SPEED: &str = "Vehicle.Speed";
pub const SIGNAL_IS_OPEN: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen";
pub const SIGNAL_IS_LOCKED: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked";
pub const SIGNAL_RESPONSE: &str = "Vehicle.Command.Door.Response";

#[derive(Debug)]
pub enum BrokerError {
    Connection(String),
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

pub trait BrokerClient {
    fn get_float(
        &self,
        signal: &str,
    ) -> impl std::future::Future<Output = Result<Option<f32>, BrokerError>>;

    fn get_bool(
        &self,
        signal: &str,
    ) -> impl std::future::Future<Output = Result<Option<bool>, BrokerError>>;

    fn set_bool(
        &self,
        signal: &str,
        value: bool,
    ) -> impl std::future::Future<Output = Result<(), BrokerError>>;

    fn set_string(
        &self,
        signal: &str,
        value: &str,
    ) -> impl std::future::Future<Output = Result<(), BrokerError>>;
}
