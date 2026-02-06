//! Property-based tests for lock execution.
//!
//! These tests verify Properties 4 and 8 from the design document:
//! - Property 4: Valid Commands Return Correct Command_ID
//! - Property 8: Timeout Preserves State Consistency

use std::sync::Arc;
use std::time::Duration;

use locking_service::error::LockingError;
use locking_service::executor::LockExecutor;
use locking_service::proto::Door;
use locking_service::state::LockState;
use proptest::prelude::*;
use tokio::sync::RwLock;

/// Strategy to generate valid doors (excluding Unknown).
fn valid_door_strategy() -> impl Strategy<Value = Door> {
    prop_oneof![
        Just(Door::Driver),
        Just(Door::Passenger),
        Just(Door::RearLeft),
        Just(Door::RearRight),
        Just(Door::All),
    ]
}

/// Strategy to generate arbitrary command IDs.
fn command_id_strategy() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-zA-Z0-9_-]{1,64}").unwrap()
}

/// Helper to run async tests within proptest.
fn run_async<F, T>(f: F) -> T
where
    F: std::future::Future<Output = T>,
{
    tokio::runtime::Runtime::new().unwrap().block_on(f)
}

/// Creates a test executor with normal timeout.
fn create_test_executor() -> LockExecutor {
    LockExecutor::with_default_state(Duration::from_millis(100))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: locking-service, Property 4: Valid Commands Return Correct Command_ID
    ///
    /// For any Lock or Unlock command that passes authentication and safety validation,
    /// the response SHALL contain the same Command_ID that was provided in the request,
    /// and the success field SHALL be true.
    ///
    /// Validates: Requirements 1.2, 2.2
    #[test]
    fn property_lock_command_returns_correct_command_id(
        door in valid_door_strategy(),
        command_id in command_id_strategy()
    ) {
        let executor = create_test_executor();

        let result = run_async(executor.execute_lock(door, command_id.clone()));

        prop_assert!(result.is_ok(), "Lock command should succeed for valid door {:?}", door);

        let exec_result = result.unwrap();
        prop_assert_eq!(
            exec_result.command_id, command_id,
            "Response should contain the same Command_ID"
        );
        prop_assert!(exec_result.success, "Success field should be true");
        prop_assert!(exec_result.is_locked, "is_locked should be true after lock");
    }

    /// Feature: locking-service, Property 4: Valid Commands Return Correct Command_ID (unlock)
    ///
    /// Validates: Requirements 1.2, 2.2
    #[test]
    fn property_unlock_command_returns_correct_command_id(
        door in valid_door_strategy(),
        command_id in command_id_strategy()
    ) {
        let executor = create_test_executor();

        let result = run_async(executor.execute_unlock(door, command_id.clone()));

        prop_assert!(result.is_ok(), "Unlock command should succeed for valid door {:?}", door);

        let exec_result = result.unwrap();
        prop_assert_eq!(
            exec_result.command_id, command_id,
            "Response should contain the same Command_ID"
        );
        prop_assert!(exec_result.success, "Success field should be true");
        prop_assert!(!exec_result.is_locked, "is_locked should be false after unlock");
    }

    /// Feature: locking-service, Property 4: Door field matches request
    ///
    /// The response door field should match the requested door.
    #[test]
    fn property_response_door_matches_request(
        door in valid_door_strategy(),
        command_id in command_id_strategy()
    ) {
        let executor = create_test_executor();

        let result = run_async(executor.execute_lock(door, command_id));

        prop_assert!(result.is_ok());
        let exec_result = result.unwrap();
        prop_assert_eq!(exec_result.door, door, "Response door should match request");
    }

    /// Feature: locking-service, Property 8: State consistency after successful execution
    ///
    /// After a successful lock/unlock, the state should reflect the operation.
    #[test]
    fn property_state_consistent_after_execution(
        door in valid_door_strategy().prop_filter("exclude All for individual check", |d| *d != Door::All),
        command_id in command_id_strategy()
    ) {
        let executor = create_test_executor();

        // Execute lock
        let result = run_async(executor.execute_lock(door, command_id.clone()));
        prop_assert!(result.is_ok());

        // Verify state is locked
        let state = run_async(executor.get_door_state(door));
        prop_assert_eq!(state, Some(true), "Door should be locked after execute_lock");

        // Execute unlock
        let result = run_async(executor.execute_unlock(door, command_id));
        prop_assert!(result.is_ok());

        // Verify state is unlocked
        let state = run_async(executor.get_door_state(door));
        prop_assert_eq!(state, Some(false), "Door should be unlocked after execute_unlock");
    }
}

/// Tests for Property 8: Timeout Preserves State Consistency
mod timeout_tests {
    use super::*;

    /// Feature: locking-service, Property 8: Timeout Preserves State Consistency
    ///
    /// For any command that times out during execution, the lock state SHALL be
    /// left in a consistent state—either the operation completed fully or it
    /// did not occur at all. There SHALL be no partial state changes.
    ///
    /// Validates: Requirements 6.3
    #[tokio::test]
    async fn property_timeout_preserves_initial_state() {
        // Create executor with very short timeout
        let state = Arc::new(RwLock::new(LockState::default()));
        let executor = LockExecutor::new(Arc::clone(&state), Duration::from_nanos(1));

        // Capture initial state
        let initial_driver = state.read().await.driver.is_locked;
        let initial_passenger = state.read().await.passenger.is_locked;

        // This might timeout or might succeed depending on timing
        // Either way, state should be consistent
        let result = executor
            .execute_lock(Door::Driver, "cmd-timeout".to_string())
            .await;

        let final_state = state.read().await;

        match result {
            Ok(_) => {
                // If it succeeded, driver should be locked
                assert!(final_state.driver.is_locked);
            }
            Err(LockingError::TimeoutError(_)) => {
                // If it timed out, state should be unchanged
                assert_eq!(
                    final_state.driver.is_locked, initial_driver,
                    "Driver state should be unchanged after timeout"
                );
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        }

        // Passenger should always be unchanged
        assert_eq!(
            final_state.passenger.is_locked, initial_passenger,
            "Other doors should be unchanged"
        );
    }

    #[tokio::test]
    async fn property_timeout_no_partial_all_doors() {
        // Create executor with very short timeout
        let state = Arc::new(RwLock::new(LockState::default()));
        let executor = LockExecutor::new(Arc::clone(&state), Duration::from_nanos(1));

        // Try to lock all doors - might timeout
        let result = executor
            .execute_lock(Door::All, "cmd-all-timeout".to_string())
            .await;

        let final_state = state.read().await;

        match result {
            Ok(_) => {
                // If it succeeded, ALL doors should be locked
                assert!(final_state.driver.is_locked);
                assert!(final_state.passenger.is_locked);
                assert!(final_state.rear_left.is_locked);
                assert!(final_state.rear_right.is_locked);
            }
            Err(LockingError::TimeoutError(_)) => {
                // If it timed out, ALL doors should still be in initial state
                // (no partial updates)
                let all_unlocked = !final_state.driver.is_locked
                    && !final_state.passenger.is_locked
                    && !final_state.rear_left.is_locked
                    && !final_state.rear_right.is_locked;
                let all_locked = final_state.driver.is_locked
                    && final_state.passenger.is_locked
                    && final_state.rear_left.is_locked
                    && final_state.rear_right.is_locked;

                assert!(
                    all_unlocked || all_locked,
                    "State should be consistent - either all locked or all unlocked, not partial"
                );
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }

    #[tokio::test]
    async fn normal_execution_completes_within_timeout() {
        let executor = LockExecutor::with_default_state(Duration::from_secs(1));

        let result = executor
            .execute_lock(Door::Driver, "cmd-normal".to_string())
            .await;

        assert!(
            result.is_ok(),
            "Normal execution should complete within timeout"
        );
        assert_eq!(executor.get_door_state(Door::Driver).await, Some(true));
    }
}

/// Tests for invalid door handling.
mod invalid_door_tests {
    use super::*;

    #[tokio::test]
    async fn unknown_door_returns_error() {
        let executor = create_test_executor();

        let result = executor
            .execute_lock(Door::Unknown, "cmd-invalid".to_string())
            .await;

        assert!(matches!(
            result,
            Err(LockingError::InvalidDoor(Door::Unknown))
        ));
    }

    #[tokio::test]
    async fn invalid_door_maps_to_invalid_argument_status() {
        let executor = create_test_executor();

        let result = executor
            .execute_lock(Door::Unknown, "cmd-invalid".to_string())
            .await;

        if let Err(err) = result {
            let status: tonic::Status = err.into();
            assert_eq!(status.code(), tonic::Code::InvalidArgument);
        } else {
            panic!("Expected error for invalid door");
        }
    }
}
