//! Lock execution for LOCKING_SERVICE.
//!
//! This module provides the LockExecutor that executes lock/unlock operations
//! on the vehicle doors. For this demo, operations are simulated by updating
//! in-memory state.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;

use crate::error::LockingError;
use crate::proto::Door;
use crate::state::LockState;

/// Result of a lock/unlock execution.
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// The command ID that was executed.
    pub command_id: String,
    /// Whether the operation succeeded.
    pub success: bool,
    /// The door that was operated on.
    pub door: Door,
    /// The new lock state (true = locked, false = unlocked).
    pub is_locked: bool,
}

/// Executes lock/unlock operations on vehicle doors.
///
/// The executor manages the lock state and ensures operations complete
/// within the configured timeout. For this demo, operations are simulated
/// by updating in-memory state rather than controlling actual hardware.
pub struct LockExecutor {
    /// Shared lock state for all doors.
    lock_state: Arc<RwLock<LockState>>,
    /// Maximum time allowed for execution.
    execution_timeout: Duration,
}

impl LockExecutor {
    /// Creates a new LockExecutor with the given state and timeout.
    pub fn new(lock_state: Arc<RwLock<LockState>>, execution_timeout: Duration) -> Self {
        Self {
            lock_state,
            execution_timeout,
        }
    }

    /// Creates a new LockExecutor with default state.
    pub fn with_default_state(execution_timeout: Duration) -> Self {
        Self {
            lock_state: Arc::new(RwLock::new(LockState::default())),
            execution_timeout,
        }
    }

    /// Returns a clone of the shared lock state.
    pub fn lock_state(&self) -> Arc<RwLock<LockState>> {
        Arc::clone(&self.lock_state)
    }

    /// Returns the configured execution timeout.
    pub fn execution_timeout(&self) -> Duration {
        self.execution_timeout
    }

    /// Executes a lock operation on the specified door.
    ///
    /// Sets the door's `is_locked` state to `true`.
    ///
    /// # Arguments
    ///
    /// * `door` - The door to lock
    /// * `command_id` - The command ID for tracking
    ///
    /// # Returns
    ///
    /// * `Ok(ExecutionResult)` with the command ID and new state
    /// * `Err(LockingError::InvalidDoor)` if the door is invalid
    /// * `Err(LockingError::TimeoutError)` if execution exceeds timeout
    pub async fn execute_lock(
        &self,
        door: Door,
        command_id: String,
    ) -> Result<ExecutionResult, LockingError> {
        self.execute_with_timeout(door, command_id, true).await
    }

    /// Executes an unlock operation on the specified door.
    ///
    /// Sets the door's `is_locked` state to `false`.
    ///
    /// # Arguments
    ///
    /// * `door` - The door to unlock
    /// * `command_id` - The command ID for tracking
    ///
    /// # Returns
    ///
    /// * `Ok(ExecutionResult)` with the command ID and new state
    /// * `Err(LockingError::InvalidDoor)` if the door is invalid
    /// * `Err(LockingError::TimeoutError)` if execution exceeds timeout
    pub async fn execute_unlock(
        &self,
        door: Door,
        command_id: String,
    ) -> Result<ExecutionResult, LockingError> {
        self.execute_with_timeout(door, command_id, false).await
    }

    /// Internal method that executes a lock/unlock with timeout.
    async fn execute_with_timeout(
        &self,
        door: Door,
        command_id: String,
        lock: bool,
    ) -> Result<ExecutionResult, LockingError> {
        // Validate door before attempting execution
        if door == Door::Unknown {
            return Err(LockingError::InvalidDoor(door));
        }

        // Execute with timeout
        let result =
            tokio::time::timeout(self.execution_timeout, self.do_execute(door, lock)).await;

        match result {
            Ok(Ok(())) => Ok(ExecutionResult {
                command_id,
                success: true,
                door,
                is_locked: lock,
            }),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(LockingError::TimeoutError(
                self.execution_timeout.as_millis() as u64,
            )),
        }
    }

    /// Internal method that performs the actual state update.
    async fn do_execute(&self, door: Door, lock: bool) -> Result<(), LockingError> {
        let mut state = self.lock_state.write().await;

        // For Door::All, we update all doors atomically
        if door == Door::All {
            state.set_locked(Door::Driver, lock);
            state.set_locked(Door::Passenger, lock);
            state.set_locked(Door::RearLeft, lock);
            state.set_locked(Door::RearRight, lock);
            return Ok(());
        }

        // For individual doors, update just that door
        if state.set_locked(door, lock) {
            Ok(())
        } else {
            Err(LockingError::InvalidDoor(door))
        }
    }

    /// Gets the current lock state for a door.
    pub async fn get_door_state(&self, door: Door) -> Option<bool> {
        let state = self.lock_state.read().await;
        state.get_door(door).map(|d| d.is_locked)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_executor() -> LockExecutor {
        LockExecutor::with_default_state(Duration::from_millis(100))
    }

    #[tokio::test]
    async fn test_execute_lock_single_door() {
        let executor = create_test_executor();

        let result = executor
            .execute_lock(Door::Driver, "cmd-1".to_string())
            .await;

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.command_id, "cmd-1");
        assert!(result.success);
        assert_eq!(result.door, Door::Driver);
        assert!(result.is_locked);

        // Verify state was updated
        assert_eq!(executor.get_door_state(Door::Driver).await, Some(true));
        assert_eq!(executor.get_door_state(Door::Passenger).await, Some(false));
    }

    #[tokio::test]
    async fn test_execute_unlock_single_door() {
        let executor = create_test_executor();

        // First lock the door
        executor
            .execute_lock(Door::Driver, "cmd-1".to_string())
            .await
            .unwrap();
        assert_eq!(executor.get_door_state(Door::Driver).await, Some(true));

        // Now unlock it
        let result = executor
            .execute_unlock(Door::Driver, "cmd-2".to_string())
            .await;

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.command_id, "cmd-2");
        assert!(result.success);
        assert!(!result.is_locked);

        // Verify state was updated
        assert_eq!(executor.get_door_state(Door::Driver).await, Some(false));
    }

    #[tokio::test]
    async fn test_execute_lock_all_doors() {
        let executor = create_test_executor();

        let result = executor
            .execute_lock(Door::All, "cmd-all".to_string())
            .await;

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.command_id, "cmd-all");
        assert!(result.is_locked);

        // Verify all doors are locked
        assert_eq!(executor.get_door_state(Door::Driver).await, Some(true));
        assert_eq!(executor.get_door_state(Door::Passenger).await, Some(true));
        assert_eq!(executor.get_door_state(Door::RearLeft).await, Some(true));
        assert_eq!(executor.get_door_state(Door::RearRight).await, Some(true));
    }

    #[tokio::test]
    async fn test_execute_invalid_door() {
        let executor = create_test_executor();

        let result = executor
            .execute_lock(Door::Unknown, "cmd-1".to_string())
            .await;

        assert!(matches!(result, Err(LockingError::InvalidDoor(_))));
    }

    #[tokio::test]
    async fn test_command_id_preserved() {
        let executor = create_test_executor();
        let command_id = "unique-command-id-12345";

        let result = executor
            .execute_lock(Door::Driver, command_id.to_string())
            .await
            .unwrap();

        assert_eq!(result.command_id, command_id);
    }

    #[tokio::test]
    async fn test_concurrent_execution() {
        let executor = create_test_executor();
        let state = executor.lock_state();

        // Execute locks concurrently
        let (r1, r2, r3, r4) = tokio::join!(
            executor.execute_lock(Door::Driver, "cmd-1".to_string()),
            executor.execute_lock(Door::Passenger, "cmd-2".to_string()),
            executor.execute_lock(Door::RearLeft, "cmd-3".to_string()),
            executor.execute_lock(Door::RearRight, "cmd-4".to_string()),
        );

        assert!(r1.is_ok());
        assert!(r2.is_ok());
        assert!(r3.is_ok());
        assert!(r4.is_ok());

        // All doors should be locked
        let final_state = state.read().await;
        assert!(final_state.driver.is_locked);
        assert!(final_state.passenger.is_locked);
        assert!(final_state.rear_left.is_locked);
        assert!(final_state.rear_right.is_locked);
    }

    #[tokio::test]
    async fn test_shared_state() {
        let state = Arc::new(RwLock::new(LockState::default()));
        let executor1 = LockExecutor::new(Arc::clone(&state), Duration::from_millis(100));
        let executor2 = LockExecutor::new(Arc::clone(&state), Duration::from_millis(100));

        // Lock via executor1
        executor1
            .execute_lock(Door::Driver, "cmd-1".to_string())
            .await
            .unwrap();

        // State should be visible from executor2
        assert_eq!(executor2.get_door_state(Door::Driver).await, Some(true));
    }
}
