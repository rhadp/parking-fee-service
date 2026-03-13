//! Trait abstraction for the PARKING_OPERATOR REST client.
//!
//! Enables unit testing with mock implementations.

use super::models::{StartResponse, StopResponse};
use super::client::OperatorError;

/// Async trait for interacting with the PARKING_OPERATOR REST API.
///
/// Implemented by [`super::client::OperatorClient`] for real HTTP calls
/// and by mock implementations for testing.
#[tonic::async_trait]
pub trait OperatorApi: Send + Sync {
    /// Start a parking session.
    async fn start_session(
        &self,
        vehicle_id: &str,
        zone_id: &str,
    ) -> Result<StartResponse, OperatorError>;

    /// Stop a parking session.
    async fn stop_session(
        &self,
        session_id: &str,
    ) -> Result<StopResponse, OperatorError>;
}
