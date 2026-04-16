//! DATA_BROKER gRPC client abstraction.
//!
//! Defines the `BrokerClient` trait, signal path constants, error types,
//! and a stub `GrpcBrokerClient` that will be implemented in a later task group.

// VSS signal paths used by LOCKING_SERVICE
pub const SIGNAL_COMMAND_LOCK: &str = "Vehicle.Command.Door.Lock";
pub const SIGNAL_SPEED: &str = "Vehicle.Speed";
pub const SIGNAL_IS_OPEN: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen";
pub const SIGNAL_IS_LOCKED: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked";
pub const SIGNAL_RESPONSE: &str = "Vehicle.Command.Door.Response";

/// Errors returned by DATA_BROKER operations.
#[derive(Debug)]
pub enum BrokerError {
    ConnectionFailed(String),
    PublishFailed(String),
    ReadFailed(String),
}

/// Abstraction over DATA_BROKER gRPC operations used by LOCKING_SERVICE.
///
/// All methods are async; native async-in-trait is supported on Rust ≥ 1.75.
/// The `async_fn_in_trait` lint is suppressed since this trait is used only
/// internally (no external crate consumers) and we do not require `Send` bounds.
#[allow(async_fn_in_trait)]
pub trait BrokerClient {
    async fn get_float(&self, signal: &str) -> Result<Option<f32>, BrokerError>;
    async fn get_bool(&self, signal: &str) -> Result<Option<bool>, BrokerError>;
    async fn set_bool(&self, signal: &str, value: bool) -> Result<(), BrokerError>;
    async fn set_string(&self, signal: &str, value: &str) -> Result<(), BrokerError>;
}

/// Production gRPC client for DATA_BROKER.
///
/// Implementation (tonic + kuksa.val.v1 generated code) lives in task group 3.
pub struct GrpcBrokerClient;

impl GrpcBrokerClient {
    /// Connect to DATA_BROKER at `addr` with exponential-backoff retry.
    pub async fn connect(_addr: &str) -> Result<Self, BrokerError> {
        todo!("Implement GrpcBrokerClient::connect in task group 3")
    }

    /// Subscribe to `signal`; returns a channel that receives new JSON values.
    pub async fn subscribe(
        &self,
        _signal: &str,
    ) -> Result<tokio::sync::mpsc::Receiver<String>, BrokerError> {
        todo!("Implement GrpcBrokerClient::subscribe in task group 3")
    }
}

impl BrokerClient for GrpcBrokerClient {
    async fn get_float(&self, _signal: &str) -> Result<Option<f32>, BrokerError> {
        todo!("Implement GrpcBrokerClient::get_float in task group 3")
    }

    async fn get_bool(&self, _signal: &str) -> Result<Option<bool>, BrokerError> {
        todo!("Implement GrpcBrokerClient::get_bool in task group 3")
    }

    async fn set_bool(&self, _signal: &str, _value: bool) -> Result<(), BrokerError> {
        todo!("Implement GrpcBrokerClient::set_bool in task group 3")
    }

    async fn set_string(&self, _signal: &str, _value: &str) -> Result<(), BrokerError> {
        todo!("Implement GrpcBrokerClient::set_string in task group 3")
    }
}
