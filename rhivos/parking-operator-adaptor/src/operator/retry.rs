//! Retry wrapper for [`super::OperatorApi`].
//!
//! [`RetryOperatorClient`] delegates to an inner [`OperatorApi`] implementation
//! and retries up to 3 times with exponential backoff (1 s, 2 s) on failure.
//!
//! Requirements: 08-REQ-1.E2, 08-REQ-2.E2

use super::client::OperatorError;
use super::models::{StartResponse, StopResponse};
use super::OperatorApi;

/// Wraps any [`OperatorApi`] with automatic retry logic.
///
/// The `inner` field is public so test code can inspect call counts.
///
/// # Stub
/// Does **not** yet retry — task group 4 adds the real backoff loop.
pub struct RetryOperatorClient<T: OperatorApi> {
    /// The underlying operator client.
    pub inner: T,
}

impl<T: OperatorApi> RetryOperatorClient<T> {
    /// Wrap `inner` with retry logic.
    pub fn new(inner: T) -> Self {
        Self { inner }
    }
}

#[tonic::async_trait]
impl<T: OperatorApi + Send + Sync + 'static> OperatorApi for RetryOperatorClient<T> {
    async fn start_session(
        &self,
        vehicle_id: &str,
        zone_id: &str,
    ) -> Result<StartResponse, OperatorError> {
        // STUB: forwards once without retry — task group 4 implements backoff.
        self.inner.start_session(vehicle_id, zone_id).await
    }

    async fn stop_session(&self, session_id: &str) -> Result<StopResponse, OperatorError> {
        // STUB: forwards once without retry — task group 4 implements backoff.
        self.inner.stop_session(session_id).await
    }
}
