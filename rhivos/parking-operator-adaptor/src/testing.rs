use std::cell::RefCell;

use crate::broker::{BrokerClient, BrokerError, SIGNAL_SESSION_ACTIVE};
use crate::event_loop::OperatorApi;
use crate::operator::{OperatorError, StartResponse, StopResponse, RateResponse};

/// Mock implementation of BrokerClient for unit testing.
///
/// Supports configurable return values and records calls for assertion.
/// Uses RefCell for interior mutability (not Send/Sync).
/// Must be used with single-threaded tokio runtime (`#[tokio::test]`).
pub struct MockBrokerClient {
    set_bool_calls: RefCell<Vec<(String, bool)>>,
    fail_set_bool: RefCell<bool>,
}

impl Default for MockBrokerClient {
    fn default() -> Self {
        Self::new()
    }
}

impl MockBrokerClient {
    /// Create a new MockBrokerClient.
    pub fn new() -> Self {
        Self {
            set_bool_calls: RefCell::new(Vec::new()),
            fail_set_bool: RefCell::new(false),
        }
    }

    /// Configure set_bool to fail on subsequent calls.
    pub fn fail_set_bool(&self) {
        *self.fail_set_bool.borrow_mut() = true;
    }

    /// Get recorded set_bool calls as (signal, value) pairs.
    pub fn set_bool_calls(&self) -> Vec<(String, bool)> {
        self.set_bool_calls.borrow().clone()
    }

    /// Check if the last set_bool call for SessionActive matches the expected value.
    pub fn last_session_active_value(&self) -> Option<bool> {
        self.set_bool_calls
            .borrow()
            .iter()
            .rev()
            .find(|(signal, _)| signal == SIGNAL_SESSION_ACTIVE)
            .map(|(_, value)| *value)
    }
}

impl BrokerClient for MockBrokerClient {
    async fn set_bool(&self, signal: &str, value: bool) -> Result<(), BrokerError> {
        if *self.fail_set_bool.borrow() {
            return Err(BrokerError::RpcFailed("simulated failure".to_string()));
        }
        self.set_bool_calls
            .borrow_mut()
            .push((signal.to_string(), value));
        Ok(())
    }
}

/// Mock implementation of the PARKING_OPERATOR client for unit testing.
///
/// Records calls and returns configurable responses.
pub struct MockOperatorClient {
    start_responses: RefCell<Vec<Result<StartResponse, OperatorError>>>,
    stop_responses: RefCell<Vec<Result<StopResponse, OperatorError>>>,
    start_call_count: RefCell<usize>,
    stop_call_count: RefCell<usize>,
    start_calls: RefCell<Vec<(String, String)>>,
    stop_calls: RefCell<Vec<String>>,
}

impl Default for MockOperatorClient {
    fn default() -> Self {
        Self::new()
    }
}

impl MockOperatorClient {
    /// Create a new MockOperatorClient with no configured responses.
    pub fn new() -> Self {
        Self {
            start_responses: RefCell::new(Vec::new()),
            stop_responses: RefCell::new(Vec::new()),
            start_call_count: RefCell::new(0),
            stop_call_count: RefCell::new(0),
            start_calls: RefCell::new(Vec::new()),
            stop_calls: RefCell::new(Vec::new()),
        }
    }

    /// Configure the next start_session response.
    pub fn on_start_return(&self, resp: Result<StartResponse, OperatorError>) {
        self.start_responses.borrow_mut().push(resp);
    }

    /// Configure the next stop_session response.
    pub fn on_stop_return(&self, resp: Result<StopResponse, OperatorError>) {
        self.stop_responses.borrow_mut().push(resp);
    }

    /// Get the number of start_session calls made.
    pub fn start_call_count(&self) -> usize {
        *self.start_call_count.borrow()
    }

    /// Get the number of stop_session calls made.
    pub fn stop_call_count(&self) -> usize {
        *self.stop_call_count.borrow()
    }

    /// Get the recorded start_session calls as (vehicle_id, zone_id) pairs.
    pub fn start_calls(&self) -> Vec<(String, String)> {
        self.start_calls.borrow().clone()
    }

    /// Get the recorded stop_session calls as session_id strings.
    pub fn stop_calls(&self) -> Vec<String> {
        self.stop_calls.borrow().clone()
    }

    /// Simulate start_session: record the call and return the next response.
    pub async fn start_session(
        &self,
        vehicle_id: &str,
        zone_id: &str,
    ) -> Result<StartResponse, OperatorError> {
        *self.start_call_count.borrow_mut() += 1;
        self.start_calls
            .borrow_mut()
            .push((vehicle_id.to_string(), zone_id.to_string()));
        self.start_responses
            .borrow_mut()
            .pop()
            .unwrap_or(Err(OperatorError::RequestFailed(
                "no response configured".to_string(),
            )))
    }

    /// Simulate stop_session: record the call and return the next response.
    pub async fn stop_session(&self, session_id: &str) -> Result<StopResponse, OperatorError> {
        *self.stop_call_count.borrow_mut() += 1;
        self.stop_calls
            .borrow_mut()
            .push(session_id.to_string());
        self.stop_responses
            .borrow_mut()
            .pop()
            .unwrap_or(Err(OperatorError::RequestFailed(
                "no response configured".to_string(),
            )))
    }

    /// Reset all call counts and recorded calls.
    pub fn reset(&self) {
        *self.start_call_count.borrow_mut() = 0;
        *self.stop_call_count.borrow_mut() = 0;
        self.start_calls.borrow_mut().clear();
        self.stop_calls.borrow_mut().clear();
        self.start_responses.borrow_mut().clear();
        self.stop_responses.borrow_mut().clear();
    }
}

impl OperatorApi for MockOperatorClient {
    async fn start_session(
        &self,
        vehicle_id: &str,
        zone_id: &str,
    ) -> Result<StartResponse, OperatorError> {
        MockOperatorClient::start_session(self, vehicle_id, zone_id).await
    }

    async fn stop_session(&self, session_id: &str) -> Result<StopResponse, OperatorError> {
        MockOperatorClient::stop_session(self, session_id).await
    }
}

/// Helper to create a default successful StartResponse for tests.
pub fn make_start_response(session_id: &str) -> StartResponse {
    StartResponse {
        session_id: session_id.to_string(),
        status: "active".to_string(),
        rate: RateResponse {
            rate_type: "per_hour".to_string(),
            amount: 2.5,
            currency: "EUR".to_string(),
        },
    }
}

/// Helper to create a default successful StopResponse for tests.
pub fn make_stop_response(session_id: &str) -> StopResponse {
    StopResponse {
        session_id: session_id.to_string(),
        status: "completed".to_string(),
        duration_seconds: 3600,
        total_amount: 2.50,
        currency: "EUR".to_string(),
    }
}
