//! Mock implementations for unit testing.
//!
//! Provides [`MockOperatorClient`] and [`MockBrokerPublisher`] for testing
//! the autonomous session loop and gRPC handlers without real HTTP or gRPC
//! infrastructure.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;

use crate::broker::SessionPublisher;
use crate::operator::client::OperatorError;
use crate::operator::models::{StartResponse, StopResponse};
use crate::operator::OperatorApi;

/// Mock operator client with configurable responses and call tracking.
pub struct MockOperatorClient {
    /// Whether start_session should fail.
    pub start_should_fail: AtomicBool,
    /// Whether stop_session should fail.
    pub stop_should_fail: AtomicBool,
    /// Number of times start_session was called.
    pub start_call_count: AtomicU32,
    /// Number of times stop_session was called.
    pub stop_call_count: AtomicU32,
    /// The session_id to return on success.
    pub session_id: Mutex<String>,
    /// Records of (vehicle_id, zone_id) passed to start_session.
    pub start_calls: Mutex<Vec<(String, String)>>,
    /// Records of session_id passed to stop_session.
    pub stop_calls: Mutex<Vec<String>>,
}

impl MockOperatorClient {
    /// Create a new MockOperatorClient that succeeds by default.
    pub fn new() -> Self {
        Self {
            start_should_fail: AtomicBool::new(false),
            stop_should_fail: AtomicBool::new(false),
            start_call_count: AtomicU32::new(0),
            stop_call_count: AtomicU32::new(0),
            session_id: Mutex::new("mock-session-001".to_string()),
            start_calls: Mutex::new(Vec::new()),
            stop_calls: Mutex::new(Vec::new()),
        }
    }

    /// Configure whether start_session should fail.
    pub fn set_start_should_fail(&self, fail: bool) {
        self.start_should_fail.store(fail, Ordering::SeqCst);
    }

    /// Configure whether stop_session should fail.
    pub fn set_stop_should_fail(&self, fail: bool) {
        self.stop_should_fail.store(fail, Ordering::SeqCst);
    }

    /// Reset all call counts and recorded calls.
    pub fn reset(&self) {
        self.start_call_count.store(0, Ordering::SeqCst);
        self.stop_call_count.store(0, Ordering::SeqCst);
        self.start_calls.lock().unwrap().clear();
        self.stop_calls.lock().unwrap().clear();
    }

    /// Get the number of start_session calls.
    pub fn start_count(&self) -> u32 {
        self.start_call_count.load(Ordering::SeqCst)
    }

    /// Get the number of stop_session calls.
    pub fn stop_count(&self) -> u32 {
        self.stop_call_count.load(Ordering::SeqCst)
    }
}

impl Default for MockOperatorClient {
    fn default() -> Self {
        Self::new()
    }
}

#[tonic::async_trait]
impl OperatorApi for MockOperatorClient {
    async fn start_session(
        &self,
        vehicle_id: &str,
        zone_id: &str,
    ) -> Result<StartResponse, OperatorError> {
        self.start_call_count.fetch_add(1, Ordering::SeqCst);
        self.start_calls
            .lock()
            .unwrap()
            .push((vehicle_id.to_string(), zone_id.to_string()));

        if self.start_should_fail.load(Ordering::SeqCst) {
            return Err(OperatorError::Unreachable(
                "mock operator unavailable".to_string(),
            ));
        }

        let session_id = self.session_id.lock().unwrap().clone();
        Ok(StartResponse {
            session_id,
            status: "active".to_string(),
            rate: Some(crate::session::Rate {
                rate_type: "per_hour".to_string(),
                amount: 2.50,
                currency: "EUR".to_string(),
            }),
        })
    }

    async fn stop_session(
        &self,
        session_id: &str,
    ) -> Result<StopResponse, OperatorError> {
        self.stop_call_count.fetch_add(1, Ordering::SeqCst);
        self.stop_calls
            .lock()
            .unwrap()
            .push(session_id.to_string());

        if self.stop_should_fail.load(Ordering::SeqCst) {
            return Err(OperatorError::Unreachable(
                "mock operator unavailable".to_string(),
            ));
        }

        Ok(StopResponse {
            session_id: session_id.to_string(),
            duration: 120,
            fee: 5.0,
            status: "completed".to_string(),
        })
    }
}

/// Mock broker publisher with call tracking and configurable failure.
pub struct MockBrokerPublisher {
    /// Whether set_session_active should fail.
    pub should_fail: AtomicBool,
    /// Records of values passed to set_session_active.
    pub set_calls: Mutex<Vec<bool>>,
    /// Whether subscribe_lock_state was called.
    pub subscribe_called: AtomicBool,
}

impl MockBrokerPublisher {
    /// Create a new MockBrokerPublisher that succeeds by default.
    pub fn new() -> Self {
        Self {
            should_fail: AtomicBool::new(false),
            set_calls: Mutex::new(Vec::new()),
            subscribe_called: AtomicBool::new(false),
        }
    }

    /// Configure whether set_session_active should fail.
    pub fn set_should_fail(&self, fail: bool) {
        self.should_fail.store(fail, Ordering::SeqCst);
    }

    /// Get the list of values passed to set_session_active.
    pub fn get_set_calls(&self) -> Vec<bool> {
        self.set_calls.lock().unwrap().clone()
    }

    /// Check if set_session_active was called with a specific value.
    pub fn was_called_with(&self, active: bool) -> bool {
        self.set_calls.lock().unwrap().contains(&active)
    }

    /// Reset all call tracking.
    pub fn reset(&self) {
        self.set_calls.lock().unwrap().clear();
        self.subscribe_called.store(false, Ordering::SeqCst);
    }
}

impl Default for MockBrokerPublisher {
    fn default() -> Self {
        Self::new()
    }
}

#[tonic::async_trait]
impl SessionPublisher for MockBrokerPublisher {
    async fn set_session_active(&self, active: bool) -> Result<(), String> {
        self.set_calls.lock().unwrap().push(active);

        if self.should_fail.load(Ordering::SeqCst) {
            return Err("mock broker failure".to_string());
        }

        Ok(())
    }
}

/// A no-op publisher that does nothing (used when no broker is available).
pub struct NoopPublisher;

#[tonic::async_trait]
impl SessionPublisher for NoopPublisher {
    async fn set_session_active(&self, _active: bool) -> Result<(), String> {
        Ok(())
    }
}
