use std::cell::RefCell;

use crate::broker::{BrokerClient, BrokerError, SIGNAL_IS_LOCKED, SIGNAL_IS_OPEN};

/// Mock implementation of BrokerClient for unit testing.
///
/// Supports configurable return values and records all `set_bool` / `set_string`
/// calls for assertion. Uses `RefCell` for interior mutability -- not `Send`/`Sync`,
/// but works with `#[tokio::test]` (current-thread runtime).
pub struct MockBrokerClient {
    speed: RefCell<Option<f32>>,
    door_open: RefCell<Option<bool>>,
    locked: RefCell<Option<bool>>,
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
    pub fn new() -> Self {
        Self {
            speed: RefCell::new(None),
            door_open: RefCell::new(None),
            locked: RefCell::new(None),
            set_bool_calls: RefCell::new(Vec::new()),
            set_string_calls: RefCell::new(Vec::new()),
            fail_next_set_string: RefCell::new(false),
        }
    }

    pub fn with_speed(self, speed: impl Into<Option<f32>>) -> Self {
        *self.speed.borrow_mut() = speed.into();
        self
    }

    pub fn with_door_open(self, door_open: impl Into<Option<bool>>) -> Self {
        *self.door_open.borrow_mut() = door_open.into();
        self
    }

    pub fn with_locked(self, locked: impl Into<Option<bool>>) -> Self {
        *self.locked.borrow_mut() = locked.into();
        self
    }

    pub fn set_bool_calls(&self) -> Vec<(String, bool)> {
        self.set_bool_calls.borrow().clone()
    }

    pub fn set_string_calls(&self) -> Vec<(String, String)> {
        self.set_string_calls.borrow().clone()
    }

    /// Configure the mock to fail the next `set_string` call with a simulated RPC error.
    pub fn fail_next_set_string(&self) {
        *self.fail_next_set_string.borrow_mut() = true;
    }
}

impl BrokerClient for MockBrokerClient {
    async fn get_float(&self, _signal: &str) -> Result<Option<f32>, BrokerError> {
        Ok(*self.speed.borrow())
    }

    async fn get_bool(&self, signal: &str) -> Result<Option<bool>, BrokerError> {
        match signal {
            s if s == SIGNAL_IS_OPEN => Ok(*self.door_open.borrow()),
            s if s == SIGNAL_IS_LOCKED => Ok(*self.locked.borrow()),
            _ => Ok(None),
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
            return Err(BrokerError::Rpc("simulated failure".to_string()));
        }
        self.set_string_calls
            .borrow_mut()
            .push((signal.to_string(), value.to_string()));
        Ok(())
    }
}
