use crate::broker::{BrokerClient, BrokerError};
use std::cell::RefCell;

/// Mock DATA_BROKER client for unit testing.
///
/// Uses `RefCell` for interior mutability to record calls.
/// Not `Send`/`Sync` — use with `#[tokio::test]` (single-threaded).
pub struct MockBrokerClient {
    speed: Option<f32>,
    door_open: Option<bool>,
    is_locked: Option<bool>,
    set_bool_calls: RefCell<Vec<(String, bool)>>,
    set_string_calls: RefCell<Vec<(String, String)>>,
    fail_next_set_string: RefCell<bool>,
}

impl Default for MockBrokerClient {
    fn default() -> Self {
        Self::new()
    }
}

impl MockBrokerClient {
    /// Create a new mock with default values (all None).
    pub fn new() -> Self {
        Self {
            speed: None,
            door_open: None,
            is_locked: None,
            set_bool_calls: RefCell::new(Vec::new()),
            set_string_calls: RefCell::new(Vec::new()),
            fail_next_set_string: RefCell::new(false),
        }
    }

    /// Set the speed signal return value.
    pub fn with_speed(mut self, speed: Option<f32>) -> Self {
        self.speed = speed;
        self
    }

    /// Set the door open signal return value.
    pub fn with_door_open(mut self, door_open: Option<bool>) -> Self {
        self.door_open = door_open;
        self
    }

    /// Set the is_locked signal return value.
    pub fn with_locked(mut self, locked: Option<bool>) -> Self {
        self.is_locked = locked;
        self
    }

    /// Configure the next `set_string` call to fail.
    pub fn fail_next_set_string(&self) {
        *self.fail_next_set_string.borrow_mut() = true;
    }

    /// Return recorded `set_bool` calls.
    pub fn set_bool_calls(&self) -> Vec<(String, bool)> {
        self.set_bool_calls.borrow().clone()
    }

    /// Return recorded `set_string` calls.
    pub fn set_string_calls(&self) -> Vec<(String, String)> {
        self.set_string_calls.borrow().clone()
    }
}

impl BrokerClient for MockBrokerClient {
    async fn get_float(&self, signal: &str) -> Result<Option<f32>, BrokerError> {
        if signal.contains("Speed") {
            Ok(self.speed)
        } else {
            Ok(None)
        }
    }

    async fn get_bool(&self, signal: &str) -> Result<Option<bool>, BrokerError> {
        if signal.contains("IsOpen") {
            Ok(self.door_open)
        } else if signal.contains("IsLocked") {
            Ok(self.is_locked)
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
        if *self.fail_next_set_string.borrow() {
            *self.fail_next_set_string.borrow_mut() = false;
            return Err(BrokerError::OperationFailed(
                "simulated set_string failure".to_string(),
            ));
        }
        self.set_string_calls
            .borrow_mut()
            .push((signal.to_string(), value.to_string()));
        Ok(())
    }
}
