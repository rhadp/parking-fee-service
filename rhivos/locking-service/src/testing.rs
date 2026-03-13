use std::cell::RefCell;

use crate::broker::{BrokerClient, BrokerError};

/// Mock DATA_BROKER client for unit testing.
/// Records calls and returns configurable values.
pub struct MockBrokerClient {
    speed: Option<f32>,
    door_open: Option<bool>,
    is_locked: RefCell<Option<bool>>,
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
            speed: None,
            door_open: None,
            is_locked: RefCell::new(None),
            set_bool_calls: RefCell::new(Vec::new()),
            set_string_calls: RefCell::new(Vec::new()),
            fail_next_set_string: RefCell::new(false),
        }
    }

    pub fn with_speed(mut self, speed: Option<f32>) -> Self {
        self.speed = speed;
        self
    }

    pub fn with_door_open(mut self, open: Option<bool>) -> Self {
        self.door_open = open;
        self
    }

    pub fn with_locked(self, locked: bool) -> Self {
        *self.is_locked.borrow_mut() = Some(locked);
        self
    }

    pub fn fail_next_set_string(&self) {
        *self.fail_next_set_string.borrow_mut() = true;
    }

    pub fn set_bool_calls(&self) -> Vec<(String, bool)> {
        self.set_bool_calls.borrow().clone()
    }

    pub fn set_string_calls(&self) -> Vec<(String, String)> {
        self.set_string_calls.borrow().clone()
    }

    pub fn reset(&self) {
        self.set_bool_calls.borrow_mut().clear();
        self.set_string_calls.borrow_mut().clear();
        *self.fail_next_set_string.borrow_mut() = false;
    }
}

impl BrokerClient for MockBrokerClient {
    async fn get_float(&self, signal: &str) -> Result<Option<f32>, BrokerError> {
        match signal {
            "Vehicle.Speed" => Ok(self.speed),
            _ => Ok(None),
        }
    }

    async fn get_bool(&self, signal: &str) -> Result<Option<bool>, BrokerError> {
        match signal {
            "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen" => Ok(self.door_open),
            "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked" => Ok(*self.is_locked.borrow()),
            _ => Ok(None),
        }
    }

    async fn set_bool(&self, signal: &str, value: bool) -> Result<(), BrokerError> {
        self.set_bool_calls
            .borrow_mut()
            .push((signal.to_string(), value));
        if signal == "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked" {
            *self.is_locked.borrow_mut() = Some(value);
        }
        Ok(())
    }

    async fn set_string(&self, signal: &str, value: &str) -> Result<(), BrokerError> {
        if *self.fail_next_set_string.borrow() {
            *self.fail_next_set_string.borrow_mut() = false;
            return Err(BrokerError("simulated publish failure".to_string()));
        }
        self.set_string_calls
            .borrow_mut()
            .push((signal.to_string(), value.to_string()));
        Ok(())
    }
}
