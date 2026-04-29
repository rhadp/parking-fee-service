use crate::broker::{BrokerClient, BrokerError};
use std::cell::RefCell;

/// A mock implementation of `BrokerClient` for unit testing.
///
/// Supports configurable return values and call recording. Uses `RefCell`
/// for interior mutability — not `Send`/`Sync`, but works with
/// `#[tokio::test]` in single-threaded mode.
pub struct MockBrokerClient {
    speed: Option<f32>,
    door_open: Option<bool>,
    locked: Option<bool>,
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
    /// Create a new `MockBrokerClient` with default values.
    pub fn new() -> Self {
        Self {
            speed: None,
            door_open: None,
            locked: None,
            set_bool_calls: RefCell::new(Vec::new()),
            set_string_calls: RefCell::new(Vec::new()),
            fail_next_set_string: RefCell::new(false),
        }
    }

    /// Set the speed value to return from `get_float`.
    pub fn with_speed(mut self, speed: Option<f32>) -> Self {
        self.speed = speed;
        self
    }

    /// Set the door-open value to return from `get_bool`.
    pub fn with_door_open(mut self, door_open: Option<bool>) -> Self {
        self.door_open = door_open;
        self
    }

    /// Set the locked value to return from `get_bool`.
    pub fn with_locked(mut self, locked: Option<bool>) -> Self {
        self.locked = locked;
        self
    }

    /// Configure the next `set_string` call to fail.
    pub fn fail_next_set_string(&self) {
        *self.fail_next_set_string.borrow_mut() = true;
    }

    /// Returns a copy of all `set_bool` calls recorded.
    pub fn set_bool_calls(&self) -> Vec<(String, bool)> {
        self.set_bool_calls.borrow().clone()
    }

    /// Returns a copy of all `set_string` calls recorded.
    pub fn set_string_calls(&self) -> Vec<(String, String)> {
        self.set_string_calls.borrow().clone()
    }
}

impl BrokerClient for MockBrokerClient {
    async fn get_float(
        &self,
        signal: &str,
    ) -> Result<Option<f32>, BrokerError> {
        match signal {
            crate::broker::SIGNAL_SPEED => Ok(self.speed),
            _ => Ok(None),
        }
    }

    async fn get_bool(
        &self,
        signal: &str,
    ) -> Result<Option<bool>, BrokerError> {
        match signal {
            crate::broker::SIGNAL_DOOR_OPEN => Ok(self.door_open),
            crate::broker::SIGNAL_IS_LOCKED => Ok(self.locked),
            _ => Ok(None),
        }
    }

    async fn set_bool(
        &self,
        signal: &str,
        value: bool,
    ) -> Result<(), BrokerError> {
        self.set_bool_calls
            .borrow_mut()
            .push((signal.to_string(), value));
        Ok(())
    }

    async fn set_string(
        &self,
        signal: &str,
        value: &str,
    ) -> Result<(), BrokerError> {
        if *self.fail_next_set_string.borrow() {
            *self.fail_next_set_string.borrow_mut() = false;
            return Err(BrokerError::RpcError("simulated failure".to_string()));
        }
        self.set_string_calls
            .borrow_mut()
            .push((signal.to_string(), value.to_string()));
        Ok(())
    }
}
