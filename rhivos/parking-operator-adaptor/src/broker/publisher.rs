//! DATA_BROKER publisher for `Vehicle.Parking.SessionActive`.
//!
//! [`BrokerSessionPublisher`] implements [`super::SessionPublisher`] by
//! writing the signal over a kuksa.val.v1 gRPC channel.
//!
//! Requirements: 08-REQ-6.2, 08-REQ-6.E2

use super::SessionPublisher;

/// Concrete publisher that writes `Vehicle.Parking.SessionActive` to DATA_BROKER.
///
/// # Stub
/// Not yet connected to a real gRPC channel — task group 4 adds the kuksa
/// client implementation.
pub struct BrokerSessionPublisher;

impl BrokerSessionPublisher {
    /// Create a new `BrokerSessionPublisher`.
    ///
    /// # Stub — task group 4 accepts a channel/address parameter.
    pub fn new() -> Self {
        Self
    }
}

impl Default for BrokerSessionPublisher {
    fn default() -> Self {
        Self::new()
    }
}

#[tonic::async_trait]
impl SessionPublisher for BrokerSessionPublisher {
    async fn set_session_active(&self, _active: bool) -> Result<(), String> {
        // STUB: task group 4 implements the real gRPC set call.
        Err("BrokerSessionPublisher::set_session_active not yet implemented".to_string())
    }
}
