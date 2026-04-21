//! Test helpers: MockBrokerClient for unit tests.
use crate::broker::{BrokerClient, BrokerError, SIGNAL_IS_LOCKED, SIGNAL_IS_OPEN, SIGNAL_SPEED};
use async_trait::async_trait;
use std::cell::RefCell;

/// Mock DATA_BROKER client for unit tests.
/// Uses `RefCell` for interior mutability; not `Send`/`Sync`.
/// Use `#[tokio::test]` (single task — no cross-thread requirement).
pub struct MockBrokerClient {
    speed: Option<f32>,
    door_open: Option<bool>,
    is_locked: Option<bool>,
    set_bool_calls: RefCell<Vec<(String, bool)>>,
    set_string_calls: RefCell<Vec<(String, String)>>,
    fail_next_set_string: RefCell<bool>,
}

impl MockBrokerClient {
    pub fn new() -> Self {
        Self {
            speed: Some(0.0),
            door_open: Some(false),
            is_locked: Some(false),
            set_bool_calls: RefCell::new(Vec::new()),
            set_string_calls: RefCell::new(Vec::new()),
            fail_next_set_string: RefCell::new(false),
        }
    }

    pub fn with_speed(mut self, speed: f32) -> Self {
        self.speed = Some(speed);
        self
    }

    pub fn with_speed_none(mut self) -> Self {
        self.speed = None;
        self
    }

    pub fn with_door_open(mut self, open: bool) -> Self {
        self.door_open = Some(open);
        self
    }

    pub fn with_door_open_none(mut self) -> Self {
        self.door_open = None;
        self
    }

    pub fn with_locked(mut self, locked: bool) -> Self {
        self.is_locked = Some(locked);
        self
    }

    /// Make the next `set_string` call return an error.
    pub fn fail_next_set_string(&self) {
        *self.fail_next_set_string.borrow_mut() = true;
    }

    pub fn set_bool_calls(&self) -> Vec<(String, bool)> {
        self.set_bool_calls.borrow().clone()
    }

    pub fn set_string_calls(&self) -> Vec<(String, String)> {
        self.set_string_calls.borrow().clone()
    }
}

#[async_trait(?Send)]
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
        } else if signal == SIGNAL_IS_LOCKED {
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
            return Err(BrokerError::Transport("simulated failure".to_string()));
        }
        self.set_string_calls
            .borrow_mut()
            .push((signal.to_string(), value.to_string()));
        Ok(())
    }
}
