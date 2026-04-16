//! DATA_BROKER client abstraction (stub — full implementation in task group 4).

/// VSS signal paths used by PARKING_OPERATOR_ADAPTOR.
pub const SIGNAL_IS_LOCKED: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked";
pub const SIGNAL_SESSION_ACTIVE: &str = "Vehicle.Parking.SessionActive";

/// Errors from DATA_BROKER operations.
#[derive(Debug)]
pub enum BrokerError {
    ConnectionFailed(String),
    PublishFailed(String),
}

/// Abstraction over DATA_BROKER signal publishing.
///
/// Allows unit tests to inject a mock without a live DATA_BROKER.
/// No `Send + Sync` bound so that `RefCell`-based mocks work in
/// single-threaded (`current_thread`) tokio tests.
#[allow(async_fn_in_trait)]
pub trait SessionPublisher {
    async fn set_session_active(&self, active: bool) -> Result<(), BrokerError>;
}

/// Stub DATA_BROKER client — wired up fully in task group 4.
pub struct BrokerClient {
    _addr: String,
}

impl BrokerClient {
    pub async fn connect(_addr: &str) -> Result<Self, BrokerError> {
        todo!("BrokerClient::connect not yet implemented")
    }

    pub async fn set_bool(&self, _signal: &str, _value: bool) -> Result<(), BrokerError> {
        todo!("BrokerClient::set_bool not yet implemented")
    }

    pub async fn subscribe_bool(
        &self,
        _signal: &str,
    ) -> Result<tokio::sync::mpsc::Receiver<bool>, BrokerError> {
        todo!("BrokerClient::subscribe_bool not yet implemented")
    }
}

impl SessionPublisher for BrokerClient {
    async fn set_session_active(&self, active: bool) -> Result<(), BrokerError> {
        self.set_bool(SIGNAL_SESSION_ACTIVE, active).await
    }
}
