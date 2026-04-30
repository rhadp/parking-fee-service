use crate::event_loop::{BrokerOps, OperatorOps};
use crate::operator::{OperatorError, StartResponse, StopResponse, RateResponse};
use std::cell::RefCell;

/// A mock implementation of `OperatorOps` for unit testing.
///
/// Supports configurable return values and call recording. Uses `RefCell`
/// for interior mutability — not `Send`/`Sync`, but works with
/// `#[tokio::test]` in single-threaded mode.
pub struct MockOperatorClient {
    start_response: RefCell<Option<Result<StartResponse, OperatorError>>>,
    stop_response: RefCell<Option<Result<StopResponse, OperatorError>>>,
    start_calls: RefCell<Vec<(String, String)>>,
    stop_calls: RefCell<Vec<String>>,
}

impl Default for MockOperatorClient {
    fn default() -> Self {
        Self::new()
    }
}

impl MockOperatorClient {
    /// Create a new mock operator client.
    pub fn new() -> Self {
        Self {
            start_response: RefCell::new(None),
            stop_response: RefCell::new(None),
            start_calls: RefCell::new(Vec::new()),
            stop_calls: RefCell::new(Vec::new()),
        }
    }

    /// Configure the response for the next `start_session` call.
    pub fn on_start_return(&self, resp: Result<StartResponse, OperatorError>) {
        *self.start_response.borrow_mut() = Some(resp);
    }

    /// Configure a successful start response with the given parameters.
    pub fn on_start_success(&self, session_id: &str, rate_type: &str, amount: f64, currency: &str) {
        self.on_start_return(Ok(StartResponse {
            session_id: session_id.to_string(),
            status: "active".to_string(),
            rate: RateResponse {
                rate_type: rate_type.to_string(),
                amount,
                currency: currency.to_string(),
            },
        }));
    }

    /// Configure the response for the next `stop_session` call.
    pub fn on_stop_return(&self, resp: Result<StopResponse, OperatorError>) {
        *self.stop_response.borrow_mut() = Some(resp);
    }

    /// Configure a successful stop response with the given parameters.
    pub fn on_stop_success(
        &self,
        session_id: &str,
        duration_seconds: u64,
        total_amount: f64,
        currency: &str,
    ) {
        self.on_stop_return(Ok(StopResponse {
            session_id: session_id.to_string(),
            status: "completed".to_string(),
            duration_seconds,
            total_amount,
            currency: currency.to_string(),
        }));
    }

    /// Returns a copy of all `start_session` calls recorded.
    pub fn start_calls(&self) -> Vec<(String, String)> {
        self.start_calls.borrow().clone()
    }

    /// Returns the number of `start_session` calls.
    pub fn start_call_count(&self) -> usize {
        self.start_calls.borrow().len()
    }

    /// Returns a copy of all `stop_session` calls recorded.
    pub fn stop_calls(&self) -> Vec<String> {
        self.stop_calls.borrow().clone()
    }

    /// Returns the number of `stop_session` calls.
    pub fn stop_call_count(&self) -> usize {
        self.stop_calls.borrow().len()
    }
}

impl OperatorOps for MockOperatorClient {
    async fn start_session(
        &self,
        vehicle_id: &str,
        zone_id: &str,
    ) -> Result<StartResponse, OperatorError> {
        self.start_calls
            .borrow_mut()
            .push((vehicle_id.to_string(), zone_id.to_string()));
        self.start_response
            .borrow_mut()
            .take()
            .unwrap_or(Err(OperatorError::RequestFailed(
                "no mock response configured".to_string(),
            )))
    }

    async fn stop_session(&self, session_id: &str) -> Result<StopResponse, OperatorError> {
        self.stop_calls
            .borrow_mut()
            .push(session_id.to_string());
        self.stop_response
            .borrow_mut()
            .take()
            .unwrap_or(Err(OperatorError::RequestFailed(
                "no mock response configured".to_string(),
            )))
    }
}

/// A mock implementation of `BrokerOps` for unit testing.
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
    /// Create a new mock broker client.
    pub fn new() -> Self {
        Self {
            set_bool_calls: RefCell::new(Vec::new()),
            fail_set_bool: RefCell::new(false),
        }
    }

    /// Configure the next `set_bool` call to fail.
    pub fn fail_set_bool(&self) {
        *self.fail_set_bool.borrow_mut() = true;
    }

    /// Returns a copy of all `set_bool` calls recorded.
    pub fn set_bool_calls(&self) -> Vec<(String, bool)> {
        self.set_bool_calls.borrow().clone()
    }

    /// Returns the last `set_bool` call, if any.
    pub fn last_set_bool(&self) -> Option<(String, bool)> {
        self.set_bool_calls.borrow().last().cloned()
    }
}

impl BrokerOps for MockBrokerClient {
    async fn set_bool(&self, signal: &str, value: bool) -> Result<(), String> {
        if *self.fail_set_bool.borrow() {
            *self.fail_set_bool.borrow_mut() = false;
            return Err("simulated broker failure".to_string());
        }
        self.set_bool_calls
            .borrow_mut()
            .push((signal.to_string(), value));
        Ok(())
    }
}
