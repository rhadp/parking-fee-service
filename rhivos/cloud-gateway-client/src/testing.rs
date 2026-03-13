use std::cell::RefCell;

use crate::broker::{BrokerClient, BrokerError};
use crate::nats_client::{NatsError, NatsPublisher};

/// Mock DATA_BROKER client for unit testing.
/// Records `set_string` calls and can simulate failures.
pub struct MockBrokerClient {
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
            set_string_calls: RefCell::new(Vec::new()),
            fail_next_set_string: RefCell::new(false),
        }
    }

    /// Configure the mock to fail on the next `set_string` call.
    pub fn fail_next_set_string(&self) {
        *self.fail_next_set_string.borrow_mut() = true;
    }

    /// Return recorded `set_string` calls as (signal, value) pairs.
    pub fn set_string_calls(&self) -> Vec<(String, String)> {
        self.set_string_calls.borrow().clone()
    }

    /// Clear all recorded calls and reset failure flags.
    pub fn reset(&self) {
        self.set_string_calls.borrow_mut().clear();
        *self.fail_next_set_string.borrow_mut() = false;
    }
}

impl BrokerClient for MockBrokerClient {
    async fn set_string(&self, signal: &str, value: &str) -> Result<(), BrokerError> {
        if *self.fail_next_set_string.borrow() {
            *self.fail_next_set_string.borrow_mut() = false;
            return Err(BrokerError("simulated set_string failure".to_string()));
        }
        self.set_string_calls
            .borrow_mut()
            .push((signal.to_string(), value.to_string()));
        Ok(())
    }
}

/// Mock NATS publisher for unit testing.
/// Records publish calls and can simulate failures.
pub struct MockNatsPublisher {
    publishes: RefCell<Vec<(String, Vec<u8>)>>,
    fail_next: RefCell<bool>,
    failure_count: RefCell<u32>,
}

impl Default for MockNatsPublisher {
    fn default() -> Self {
        Self::new()
    }
}

impl MockNatsPublisher {
    pub fn new() -> Self {
        Self {
            publishes: RefCell::new(Vec::new()),
            fail_next: RefCell::new(false),
            failure_count: RefCell::new(0),
        }
    }

    /// Configure the mock to fail on the next `publish` call.
    pub fn fail_next_publish(&self) {
        *self.fail_next.borrow_mut() = true;
    }

    /// Return recorded publishes as (subject, payload) pairs.
    pub fn publishes(&self) -> Vec<(String, Vec<u8>)> {
        self.publishes.borrow().clone()
    }

    /// Return the most recent publish as (subject, payload), or None if none.
    pub fn last_publish(&self) -> Option<(String, Vec<u8>)> {
        self.publishes.borrow().last().cloned()
    }

    /// Return the number of publish failures that were handled gracefully.
    pub fn failure_count(&self) -> u32 {
        *self.failure_count.borrow()
    }

    /// Clear all recorded publishes and reset failure flags.
    pub fn reset(&self) {
        self.publishes.borrow_mut().clear();
        *self.fail_next.borrow_mut() = false;
        *self.failure_count.borrow_mut() = 0;
    }
}

impl NatsPublisher for MockNatsPublisher {
    async fn publish(&self, subject: &str, payload: &[u8]) -> Result<(), NatsError> {
        if *self.fail_next.borrow() {
            *self.fail_next.borrow_mut() = false;
            *self.failure_count.borrow_mut() += 1;
            return Err(NatsError("simulated publish failure".to_string()));
        }
        self.publishes
            .borrow_mut()
            .push((subject.to_string(), payload.to_vec()));
        Ok(())
    }
}
