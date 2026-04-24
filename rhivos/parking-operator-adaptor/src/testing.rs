use std::cell::RefCell;

use crate::broker::{BrokerError, DataBrokerClient};
use crate::operator::{
    OperatorError, ParkingOperator, RateResponse, StartResponse, StopResponse,
};

/// Mock implementation of ParkingOperator for unit testing.
///
/// Records all start/stop calls and returns configurable responses.
/// Uses `RefCell` for interior mutability — not `Send`/`Sync` but
/// compatible with `#[tokio::test]` (current-thread runtime).
pub struct MockOperatorClient {
    start_response: RefCell<Option<StartResponse>>,
    stop_response: RefCell<Option<StopResponse>>,
    start_calls: RefCell<Vec<(String, String)>>,
    stop_calls: RefCell<Vec<String>>,
    fail_always: RefCell<bool>,
}

impl Default for MockOperatorClient {
    fn default() -> Self {
        Self::new()
    }
}

impl MockOperatorClient {
    pub fn new() -> Self {
        Self {
            start_response: RefCell::new(None),
            stop_response: RefCell::new(None),
            start_calls: RefCell::new(Vec::new()),
            stop_calls: RefCell::new(Vec::new()),
            fail_always: RefCell::new(false),
        }
    }

    /// Configure the response returned by `start_session`.
    pub fn on_start_return(&self, resp: StartResponse) {
        *self.start_response.borrow_mut() = Some(resp);
    }

    /// Configure the response returned by `stop_session`.
    pub fn on_stop_return(&self, resp: StopResponse) {
        *self.stop_response.borrow_mut() = Some(resp);
    }

    /// Configure the mock to always fail.
    pub fn always_fail(&self) {
        *self.fail_always.borrow_mut() = true;
    }

    /// Returns the number of `start_session` calls made.
    pub fn start_call_count(&self) -> usize {
        self.start_calls.borrow().len()
    }

    /// Returns the number of `stop_session` calls made.
    pub fn stop_call_count(&self) -> usize {
        self.stop_calls.borrow().len()
    }

    /// Returns the list of `(vehicle_id, zone_id)` pairs from start calls.
    pub fn start_calls(&self) -> Vec<(String, String)> {
        self.start_calls.borrow().clone()
    }

    /// Returns the list of `session_id` values from stop calls.
    pub fn stop_calls(&self) -> Vec<String> {
        self.stop_calls.borrow().clone()
    }
}

impl ParkingOperator for MockOperatorClient {
    async fn start_session(
        &self,
        vehicle_id: &str,
        zone_id: &str,
    ) -> Result<StartResponse, OperatorError> {
        self.start_calls
            .borrow_mut()
            .push((vehicle_id.to_string(), zone_id.to_string()));

        if *self.fail_always.borrow() {
            return Err(OperatorError::Http("mock failure".to_string()));
        }

        self.start_response
            .borrow()
            .clone()
            .ok_or_else(|| OperatorError::Http("no mock response configured".to_string()))
    }

    async fn stop_session(&self, session_id: &str) -> Result<StopResponse, OperatorError> {
        self.stop_calls
            .borrow_mut()
            .push(session_id.to_string());

        if *self.fail_always.borrow() {
            return Err(OperatorError::Http("mock failure".to_string()));
        }

        self.stop_response
            .borrow()
            .clone()
            .ok_or_else(|| OperatorError::Http("no mock response configured".to_string()))
    }
}

/// Mock implementation of DataBrokerClient for unit testing.
///
/// Records all `set_bool` calls and supports configurable failure.
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
    pub fn new() -> Self {
        Self {
            set_bool_calls: RefCell::new(Vec::new()),
            fail_set_bool: RefCell::new(false),
        }
    }

    /// Configure the mock to fail all `set_bool` calls.
    pub fn fail_set_bool(&self) {
        *self.fail_set_bool.borrow_mut() = true;
    }

    /// Returns the list of `(signal, value)` pairs from `set_bool` calls.
    pub fn set_bool_calls(&self) -> Vec<(String, bool)> {
        self.set_bool_calls.borrow().clone()
    }

    /// Returns the last `set_bool` call as `(signal, value)`, or `None`.
    pub fn last_set_bool(&self) -> Option<(String, bool)> {
        self.set_bool_calls.borrow().last().cloned()
    }
}

impl DataBrokerClient for MockBrokerClient {
    async fn set_bool(&self, signal: &str, value: bool) -> Result<(), BrokerError> {
        if *self.fail_set_bool.borrow() {
            return Err(BrokerError::Rpc("simulated failure".to_string()));
        }
        self.set_bool_calls
            .borrow_mut()
            .push((signal.to_string(), value));
        Ok(())
    }
}

/// Helper to create a standard start response for testing.
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

/// Helper to create a standard stop response for testing.
pub fn make_stop_response(session_id: &str) -> StopResponse {
    StopResponse {
        session_id: session_id.to_string(),
        status: "completed".to_string(),
        duration_seconds: 3600,
        total_amount: 2.50,
        currency: "EUR".to_string(),
    }
}
