//! Test-only mock broker client.
//!
//! `MockBrokerClient` implements `BrokerClient` with configurable return values
//! and call recording. It is not `Send`/`Sync` (uses `RefCell`) but works
//! correctly under `#[tokio::test]` which runs on a single-threaded executor.

use std::cell::RefCell;

use crate::broker::{BrokerClient, BrokerError, SIGNAL_IS_OPEN, SIGNAL_SPEED};

pub struct MockBrokerClient {
    speed: Option<f32>,
    door_open: Option<bool>,
    set_bool_calls: RefCell<Vec<(String, bool)>>,
    set_string_calls: RefCell<Vec<(String, String)>>,
    fail_next_set_string: RefCell<bool>,
}

impl MockBrokerClient {
    /// Create a new mock with safe defaults: speed=0.0, door_open=false.
    pub fn new() -> Self {
        MockBrokerClient {
            speed: Some(0.0),
            door_open: Some(false),
            set_bool_calls: RefCell::new(Vec::new()),
            set_string_calls: RefCell::new(Vec::new()),
            fail_next_set_string: RefCell::new(false),
        }
    }

    /// Configure the value returned for `Vehicle.Speed`.
    /// Pass `None` to simulate an unset signal (treated as 0.0 per 03-REQ-3.E1).
    pub fn with_speed(mut self, speed: Option<f32>) -> Self {
        self.speed = speed;
        self
    }

    /// Configure the value returned for `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen`.
    /// Pass `None` to simulate an unset signal (treated as false per 03-REQ-3.E2).
    pub fn with_door_open(mut self, door_open: Option<bool>) -> Self {
        self.door_open = door_open;
        self
    }

    /// Cause the next `set_string` call to return an error (simulates 03-REQ-5.E1).
    pub fn fail_next_set_string(&self) {
        *self.fail_next_set_string.borrow_mut() = true;
    }

    /// Return a snapshot of all `set_bool` calls recorded so far.
    pub fn set_bool_calls(&self) -> Vec<(String, bool)> {
        self.set_bool_calls.borrow().clone()
    }

    /// Return a snapshot of all `set_string` calls recorded so far.
    pub fn set_string_calls(&self) -> Vec<(String, String)> {
        self.set_string_calls.borrow().clone()
    }
}

impl BrokerClient for MockBrokerClient {
    async fn get_float(&self, signal: &str) -> Result<Option<f32>, BrokerError> {
        if signal == SIGNAL_SPEED {
            Ok(self.speed)
        } else {
            Ok(None)
        }
    }

    async fn get_bool(&self, signal: &str) -> Result<Option<bool>, BrokerError> {
        if signal == SIGNAL_IS_OPEN {
            Ok(self.door_open)
        } else {
            Ok(None)
        }
    }

    async fn set_bool(&self, signal: &str, value: bool) -> Result<(), BrokerError> {
        self.set_bool_calls
            .borrow_mut()
            .push((signal.to_string(), value));
        Ok(())
    }

    async fn set_string(&self, signal: &str, value: &str) -> Result<(), BrokerError> {
        // Consume the fail-next flag atomically.
        let should_fail = {
            let mut flag = self.fail_next_set_string.borrow_mut();
            let v = *flag;
            if v {
                *flag = false;
            }
            v
        };
        if should_fail {
            return Err(BrokerError::Other("simulated set_string failure".to_string()));
        }
        self.set_string_calls
            .borrow_mut()
            .push((signal.to_string(), value.to_string()));
        Ok(())
    }
}
