use async_trait::async_trait;
use tokio::sync::mpsc;

// VSS signal path constants
pub const SIGNAL_IS_LOCKED: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked";
pub const SIGNAL_IS_OPEN: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen";
pub const SIGNAL_SPEED: &str = "Vehicle.Speed";
pub const SIGNAL_COMMAND: &str = "Vehicle.Command.Door.Lock";
pub const SIGNAL_RESPONSE: &str = "Vehicle.Command.Door.Response";

#[derive(Debug)]
pub enum BrokerError {
    Connection(String),
    Transport(String),
    Signal(String),
}

impl std::fmt::Display for BrokerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BrokerError::Connection(e) => write!(f, "connection error: {e}"),
            BrokerError::Transport(e) => write!(f, "transport error: {e}"),
            BrokerError::Signal(e) => write!(f, "signal error: {e}"),
        }
    }
}

/// Trait abstracting DATA_BROKER gRPC communication.
#[async_trait(?Send)]
pub trait BrokerClient {
    async fn get_float(&self, signal: &str) -> Result<Option<f32>, BrokerError>;
    async fn get_bool(&self, signal: &str) -> Result<Option<bool>, BrokerError>;
    async fn set_bool(&self, signal: &str, value: bool) -> Result<(), BrokerError>;
    async fn set_string(&self, signal: &str, value: &str) -> Result<(), BrokerError>;
}

/// Real gRPC broker client (stub for task group 1; implemented in task group 3).
pub struct GrpcBrokerClient;

impl GrpcBrokerClient {
    /// Connect with exponential backoff (5 attempts: 1s, 2s, 4s, 8s delays).
    pub async fn connect(_addr: &str) -> Result<Self, BrokerError> {
        todo!("implemented in task group 3")
    }

    /// Subscribe to a VSS signal, returning an mpsc channel receiver.
    pub async fn subscribe(&mut self, _signal: &str) -> Result<mpsc::Receiver<String>, BrokerError> {
        todo!("implemented in task group 3")
    }
}

#[async_trait(?Send)]
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
