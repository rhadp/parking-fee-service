use std::cell::RefCell;

use crate::broker::{BrokerClient, BrokerError, SIGNAL_DOOR_OPEN, SIGNAL_IS_LOCKED, SIGNAL_SPEED};

/// Mock implementation of BrokerClient for unit testing.
///
/// Supports configurable return values and records calls for assertion.
/// Uses RefCell for interior mutability (not Send/Sync).
/// Must be used with single-threaded tokio runtime (`#[tokio::test]`).
pub struct MockBrokerClient {
    speed: Option<f32>,
    door_open: Option<bool>,
    locked: Option<bool>,
    set_bool_calls: RefCell<Vec<(String, bool)>>,
    set_string_calls: RefCell<Vec<(String, String)>>,
    fail_next_set_string: RefCell<bool>,
}

impl MockBrokerClient {
    /// Create a new MockBrokerClient with no configured values.
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

    /// Configure the speed value returned by get_float for Vehicle.Speed.
    pub fn with_speed(mut self, speed: Option<f32>) -> Self {
        self.speed = speed;
        self
    }

    /// Configure the door open value returned by get_bool for IsOpen.
    pub fn with_door_open(mut self, door_open: Option<bool>) -> Self {
        self.door_open = door_open;
        self
    }

    /// Configure the locked value returned by get_bool for IsLocked.
    pub fn with_locked(mut self, locked: Option<bool>) -> Self {
        self.locked = locked;
        self
    }

    /// Make the next set_string call fail with a simulated RPC error.
    pub fn fail_next_set_string(&self) {
        *self.fail_next_set_string.borrow_mut() = true;
    }

    /// Get recorded set_bool calls as (signal, value) pairs.
    pub fn set_bool_calls(&self) -> Vec<(String, bool)> {
        self.set_bool_calls.borrow().clone()
    }

    /// Get recorded set_string calls as (signal, value) pairs.
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
        if signal == SIGNAL_DOOR_OPEN {
            Ok(self.door_open)
        } else if signal == SIGNAL_IS_LOCKED {
            Ok(self.locked)
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
            return Err(BrokerError::RpcFailed("simulated failure".to_string()));
        }
        self.set_string_calls
            .borrow_mut()
            .push((signal.to_string(), value.to_string()));
        Ok(())
    }
}
