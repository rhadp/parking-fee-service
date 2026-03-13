//! Trait abstraction for DATA_BROKER session state publishing.
//!
//! Enables unit testing with mock implementations.

/// Async trait for publishing session state to DATA_BROKER.
///
/// Implemented by [`super::publisher::BrokerSessionPublisher`] for real
/// gRPC calls and by mock implementations for testing.
#[tonic::async_trait]
pub trait SessionPublisher: Send + Sync {
    /// Write `Vehicle.Parking.SessionActive` to DATA_BROKER.
    ///
    /// Returns Ok(()) on success, or an error message on failure.
    /// Callers should log errors but continue operating (session
    /// state is authoritative; the signal may be stale on failure).
    async fn set_session_active(&self, active: bool) -> Result<(), String>;
}
