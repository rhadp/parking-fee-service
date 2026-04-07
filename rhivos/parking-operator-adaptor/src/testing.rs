use crate::broker::{BrokerError, DataBrokerClient};
use crate::operator::{OperatorError, ParkingOperator, StartResponse, StopResponse, RateResponse};
use std::cell::RefCell;

/// Mock PARKING_OPERATOR client for unit testing.
///
/// Records calls and returns pre-configured responses.
/// Uses `RefCell` for interior mutability — not `Send`/`Sync`.
/// Use with single-threaded `#[tokio::test]`.
pub struct MockOperatorClient {
    start_response: RefCell<Option<StartResponse>>,
    stop_response: RefCell<Option<StopResponse>>,
    start_call_count: RefCell<usize>,
    stop_call_count: RefCell<usize>,
    fail_all: RefCell<bool>,
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
            start_call_count: RefCell::new(0),
            stop_call_count: RefCell::new(0),
            fail_all: RefCell::new(false),
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

    /// Make all subsequent calls fail.
    pub fn always_fail(&self) {
        *self.fail_all.borrow_mut() = true;
    }

    /// Return the number of times `start_session` was called.
    pub fn start_call_count(&self) -> usize {
        *self.start_call_count.borrow()
    }

    /// Return the number of times `stop_session` was called.
    pub fn stop_call_count(&self) -> usize {
        *self.stop_call_count.borrow()
    }

    /// Reset call counts and failure state.
    pub fn reset(&self) {
        *self.start_call_count.borrow_mut() = 0;
        *self.stop_call_count.borrow_mut() = 0;
        *self.fail_all.borrow_mut() = false;
    }
}

impl ParkingOperator for MockOperatorClient {
    async fn start_session(
        &self,
        _vehicle_id: &str,
        _zone_id: &str,
    ) -> Result<StartResponse, OperatorError> {
        *self.start_call_count.borrow_mut() += 1;
        if *self.fail_all.borrow() {
            return Err(OperatorError::RetriesExhausted("mock failure".to_string()));
        }
        self.start_response
            .borrow()
            .clone()
            .ok_or_else(|| OperatorError::HttpError("no mock response configured".to_string()))
    }

    async fn stop_session(&self, _session_id: &str) -> Result<StopResponse, OperatorError> {
        *self.stop_call_count.borrow_mut() += 1;
        if *self.fail_all.borrow() {
            return Err(OperatorError::RetriesExhausted("mock failure".to_string()));
        }
        self.stop_response
            .borrow()
            .clone()
            .ok_or_else(|| OperatorError::HttpError("no mock response configured".to_string()))
    }
}

/// Mock DATA_BROKER client for unit testing.
///
/// Records `set_bool` calls and optionally fails.
/// Uses `RefCell` — not `Send`/`Sync`. Use with single-threaded tests.
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

    /// Configure `set_bool` to fail.
    pub fn fail_set_bool(&self) {
        *self.fail_set_bool.borrow_mut() = true;
    }

    /// Return recorded `set_bool` calls.
    pub fn set_bool_calls(&self) -> Vec<(String, bool)> {
        self.set_bool_calls.borrow().clone()
    }

    /// Reset recorded calls.
    pub fn reset(&self) {
        self.set_bool_calls.borrow_mut().clear();
        *self.fail_set_bool.borrow_mut() = false;
    }
}

impl DataBrokerClient for MockBrokerClient {
    async fn set_bool(&self, signal: &str, value: bool) -> Result<(), BrokerError> {
        if *self.fail_set_bool.borrow() {
            return Err(BrokerError::OperationFailed(
                "simulated set_bool failure".to_string(),
            ));
        }
        self.set_bool_calls
            .borrow_mut()
            .push((signal.to_string(), value));
        Ok(())
    }
}

/// Helper to create a standard StartResponse for tests.
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

/// Helper to create a standard StopResponse for tests.
pub fn make_stop_response(session_id: &str) -> StopResponse {
    StopResponse {
        session_id: session_id.to_string(),
        status: "completed".to_string(),
        duration_seconds: 3600,
        total_amount: 2.50,
        currency: "EUR".to_string(),
    }
}
