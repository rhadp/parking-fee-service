//! Test helpers: `MockBrokerClient` that implements `BrokerClient` with
//! configurable return values and call recording.
//!
//! Uses `RefCell` for interior mutability; not `Send`/`Sync`, but works with
//! `#[tokio::test(flavor = "current_thread")]`.

use std::cell::RefCell;

use crate::broker::{BrokerClient, BrokerError, SIGNAL_IS_OPEN, SIGNAL_SPEED};

/// A test double for DATA_BROKER that records calls and returns configured values.
pub struct MockBrokerClient {
    /// Configured return value for `Vehicle.Speed` (`None` = signal not set).
    speed: Option<f32>,
    /// Configured return value for `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen`
    /// (`None` = signal not set).
    door_open: Option<bool>,
    /// Recorded `set_bool` calls: (signal, value).
    set_bool_calls: RefCell<Vec<(String, bool)>>,
    /// Recorded `set_string` calls: (signal, value).
    set_string_calls: RefCell<Vec<(String, String)>>,
    /// When `true`, the next `set_string` call returns an error and clears this flag.
    fail_next_set_string: RefCell<bool>,
}

impl MockBrokerClient {
    /// Create a new mock with no signals set and no failure injection.
    pub fn new() -> Self {
        Self {
            speed: None,
            door_open: None,
            set_bool_calls: RefCell::new(Vec::new()),
            set_string_calls: RefCell::new(Vec::new()),
            fail_next_set_string: RefCell::new(false),
        }
    }

    /// Set the configured speed value (simulates `Vehicle.Speed` having a value).
    pub fn with_speed(mut self, speed: f32) -> Self {
        self.speed = Some(speed);
        self
    }

    /// Set the configured door-open value (simulates `IsOpen` having a value).
    pub fn with_door_open(mut self, open: bool) -> Self {
        self.door_open = Some(open);
        self
    }

    /// Inject a failure for the next `set_string` call. The call after that succeeds.
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
            .push((signal.to_owned(), value));
        Ok(())
    }

    async fn set_string(&self, signal: &str, value: &str) -> Result<(), BrokerError> {
        let should_fail = {
            let mut flag = self.fail_next_set_string.borrow_mut();
            let current = *flag;
            if current {
                *flag = false;
            }
            current
        };

        if should_fail {
            return Err(BrokerError::PublishFailed(
                "injected failure for test".to_owned(),
            ));
        }

        self.set_string_calls
            .borrow_mut()
            .push((signal.to_owned(), value.to_owned()));
        Ok(())
    }
}
