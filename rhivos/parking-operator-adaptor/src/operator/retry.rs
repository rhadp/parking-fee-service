//! Retry wrapper for [`super::OperatorApi`].
//!
//! [`RetryOperatorClient`] delegates to an inner [`OperatorApi`] implementation
//! and retries up to 3 times with exponential backoff (1 s, 2 s) on failure.
//!
//! Requirements: 08-REQ-1.E2, 08-REQ-2.E2

use std::time::Duration;

use super::client::OperatorError;
use super::models::{StartResponse, StopResponse};
use super::OperatorApi;
use tracing::warn;

/// Number of attempts before giving up.
const MAX_ATTEMPTS: u32 = 3;

/// Initial backoff delay in seconds.
const INITIAL_BACKOFF_SECS: u64 = 1;

/// Wraps any [`OperatorApi`] with automatic retry logic.
///
/// The `inner` field is public so test code can inspect call counts.
///
/// On each transient failure the wrapper sleeps for 1 s, 2 s (exponential)
/// before the next attempt.  After 3 total attempts the last error is returned.
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
    /// Start a session with up to 3 attempts and exponential backoff.
    async fn start_session(
        &self,
        vehicle_id: &str,
        zone_id: &str,
    ) -> Result<StartResponse, OperatorError> {
        let mut delay = Duration::from_secs(INITIAL_BACKOFF_SECS);
        for attempt in 1..=MAX_ATTEMPTS {
            match self.inner.start_session(vehicle_id, zone_id).await {
                Ok(resp) => return Ok(resp),
                Err(e) => {
                    if attempt == MAX_ATTEMPTS {
                        return Err(e);
                    }
                    warn!(
                        attempt,
                        max = MAX_ATTEMPTS,
                        retry_in_ms = delay.as_millis(),
                        error = %e,
                        "start_session failed, retrying"
                    );
                    tokio::time::sleep(delay).await;
                    delay *= 2;
                }
            }
        }
        unreachable!()
    }

    /// Stop a session with up to 3 attempts and exponential backoff.
    async fn stop_session(&self, session_id: &str) -> Result<StopResponse, OperatorError> {
        let mut delay = Duration::from_secs(INITIAL_BACKOFF_SECS);
        for attempt in 1..=MAX_ATTEMPTS {
            match self.inner.stop_session(session_id).await {
                Ok(resp) => return Ok(resp),
                Err(e) => {
                    if attempt == MAX_ATTEMPTS {
                        return Err(e);
                    }
                    warn!(
                        attempt,
                        max = MAX_ATTEMPTS,
                        retry_in_ms = delay.as_millis(),
                        error = %e,
                        "stop_session failed, retrying"
                    );
                    tokio::time::sleep(delay).await;
                    delay *= 2;
                }
            }
        }
        unreachable!()
    }
}
