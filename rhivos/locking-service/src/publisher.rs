//! State publication for LOCKING_SERVICE.
//!
//! This module provides the StatePublisher that publishes lock state changes
//! to the DATA_BROKER with exponential backoff retry logic.

use std::time::Duration;

use crate::error::LockingError;
use crate::proto::Door;
use crate::validator::vss_paths;

/// Error type for state publication failures.
#[derive(Debug, Clone)]
pub enum PublishError {
    /// All retry attempts failed.
    AllRetriesFailed { attempts: u32, last_error: String },
    /// Invalid door specified.
    InvalidDoor(Door),
}

impl std::fmt::Display for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PublishError::AllRetriesFailed {
                attempts,
                last_error,
            } => {
                write!(
                    f,
                    "Failed to publish after {} attempts: {}",
                    attempts, last_error
                )
            }
            PublishError::InvalidDoor(door) => {
                write!(f, "Invalid door for publication: {:?}", door)
            }
        }
    }
}

impl std::error::Error for PublishError {}

/// Trait for writing signals to DATA_BROKER.
#[async_trait::async_trait]
pub trait SignalWriter: Send + Sync {
    /// Writes a boolean signal value.
    async fn write_bool(&self, path: &str, value: bool) -> Result<(), LockingError>;
}

/// Publishes lock state changes to DATA_BROKER with retry logic.
pub struct StatePublisher<W: SignalWriter> {
    signal_writer: W,
    max_retries: u32,
    base_delay: Duration,
}

impl<W: SignalWriter> StatePublisher<W> {
    /// Creates a new StatePublisher with the given writer and retry settings.
    pub fn new(signal_writer: W, max_retries: u32, base_delay: Duration) -> Self {
        Self {
            signal_writer,
            max_retries,
            base_delay,
        }
    }

    /// Publishes the lock state for a door with exponential backoff retry.
    ///
    /// # Arguments
    ///
    /// * `door` - The door whose state to publish
    /// * `is_locked` - The new lock state (true = locked)
    ///
    /// # Returns
    ///
    /// * `Ok(())` if publication succeeds
    /// * `Err(PublishError::AllRetriesFailed)` if all retries fail
    /// * `Err(PublishError::InvalidDoor)` if door is invalid
    pub async fn publish_lock_state(
        &self,
        door: Door,
        is_locked: bool,
    ) -> Result<(), PublishError> {
        // Handle Door::All by publishing to all doors
        if door == Door::All {
            // Publish to all doors - if any fails, return error
            for d in [
                Door::Driver,
                Door::Passenger,
                Door::RearLeft,
                Door::RearRight,
            ] {
                self.publish_single_door(d, is_locked).await?;
            }
            return Ok(());
        }

        self.publish_single_door(door, is_locked).await
    }

    /// Publishes state for a single door with retry logic.
    async fn publish_single_door(&self, door: Door, is_locked: bool) -> Result<(), PublishError> {
        let door_path = vss_paths::door_to_path(door).ok_or(PublishError::InvalidDoor(door))?;
        let signal_path = vss_paths::door_is_locked(door_path);

        let mut delay = self.base_delay;
        let mut last_error = String::new();

        for attempt in 0..self.max_retries {
            match self.signal_writer.write_bool(&signal_path, is_locked).await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    last_error = e.to_string();
                    if attempt < self.max_retries - 1 {
                        tokio::time::sleep(delay).await;
                        delay *= 2; // Exponential backoff
                    }
                }
            }
        }

        Err(PublishError::AllRetriesFailed {
            attempts: self.max_retries,
            last_error,
        })
    }
}

/// Test utilities for mocking the signal writer.
pub mod test_utils {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, RwLock};

    /// Record of a write operation.
    #[derive(Debug, Clone)]
    pub struct WriteRecord {
        pub path: String,
        pub value: bool,
    }

    /// Mock signal writer for testing.
    #[derive(Clone, Default)]
    pub struct MockSignalWriter {
        writes: Arc<RwLock<Vec<WriteRecord>>>,
        errors: Arc<RwLock<HashMap<String, String>>>,
        fail_count: Arc<RwLock<HashMap<String, u32>>>,
    }

    impl MockSignalWriter {
        /// Creates a new mock signal writer.
        pub fn new() -> Self {
            Self::default()
        }

        /// Gets all write records.
        pub fn get_writes(&self) -> Vec<WriteRecord> {
            self.writes.read().unwrap().clone()
        }

        /// Sets a path to fail with an error.
        pub fn set_error(&self, path: &str, error: &str) {
            self.errors
                .write()
                .unwrap()
                .insert(path.to_string(), error.to_string());
        }

        /// Sets a path to fail a specific number of times before succeeding.
        pub fn set_fail_count(&self, path: &str, count: u32) {
            self.fail_count
                .write()
                .unwrap()
                .insert(path.to_string(), count);
        }

        /// Clears all writes.
        pub fn clear_writes(&self) {
            self.writes.write().unwrap().clear();
        }
    }

    #[async_trait::async_trait]
    impl SignalWriter for MockSignalWriter {
        async fn write_bool(&self, path: &str, value: bool) -> Result<(), LockingError> {
            // Check if we should fail this attempt
            {
                let mut fail_counts = self.fail_count.write().unwrap();
                if let Some(count) = fail_counts.get_mut(path) {
                    if *count > 0 {
                        *count -= 1;
                        return Err(LockingError::DataBrokerError(format!(
                            "Simulated failure, {} retries remaining",
                            count
                        )));
                    }
                }
            }

            // Check for permanent errors
            if let Some(error) = self.errors.read().unwrap().get(path) {
                return Err(LockingError::DataBrokerError(error.clone()));
            }

            // Record the write
            self.writes.write().unwrap().push(WriteRecord {
                path: path.to_string(),
                value,
            });

            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_utils::MockSignalWriter;

    fn create_test_publisher() -> (StatePublisher<MockSignalWriter>, MockSignalWriter) {
        let writer = MockSignalWriter::new();
        let publisher = StatePublisher::new(writer.clone(), 3, Duration::from_millis(1));
        (publisher, writer)
    }

    #[tokio::test]
    async fn test_publish_success() {
        let (publisher, writer) = create_test_publisher();

        let result = publisher.publish_lock_state(Door::Driver, true).await;

        assert!(result.is_ok());
        let writes = writer.get_writes();
        assert_eq!(writes.len(), 1);
        assert!(writes[0].path.contains("IsLocked"));
        assert!(writes[0].value);
    }

    #[tokio::test]
    async fn test_publish_unlock() {
        let (publisher, writer) = create_test_publisher();

        let result = publisher.publish_lock_state(Door::Driver, false).await;

        assert!(result.is_ok());
        let writes = writer.get_writes();
        assert_eq!(writes.len(), 1);
        assert!(!writes[0].value);
    }

    #[tokio::test]
    async fn test_publish_all_doors() {
        let (publisher, writer) = create_test_publisher();

        let result = publisher.publish_lock_state(Door::All, true).await;

        assert!(result.is_ok());
        let writes = writer.get_writes();
        assert_eq!(writes.len(), 4);
    }

    #[tokio::test]
    async fn test_publish_retry_success() {
        let (publisher, writer) = create_test_publisher();

        // Fail twice, then succeed
        writer.set_fail_count("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", 2);

        let result = publisher.publish_lock_state(Door::Driver, true).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_publish_all_retries_failed() {
        let (publisher, writer) = create_test_publisher();

        // Permanent failure
        writer.set_error(
            "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked",
            "connection refused",
        );

        let result = publisher.publish_lock_state(Door::Driver, true).await;

        assert!(result.is_err());
        match result {
            Err(PublishError::AllRetriesFailed { attempts, .. }) => {
                assert_eq!(attempts, 3);
            }
            _ => panic!("Expected AllRetriesFailed"),
        }
    }

    #[tokio::test]
    async fn test_publish_invalid_door() {
        let (publisher, _writer) = create_test_publisher();

        let result = publisher.publish_lock_state(Door::Unknown, true).await;

        assert!(matches!(result, Err(PublishError::InvalidDoor(_))));
    }
}
