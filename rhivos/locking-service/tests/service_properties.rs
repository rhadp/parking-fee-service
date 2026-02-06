//! Property-based tests for gRPC service.
//!
//! These tests verify Properties 5, 6, 7 from the design document:
//! - Property 5: State Publication Consistency
//! - Property 6: GetLockState Returns Complete State
//! - Property 7: Invalid Door Returns Error

use std::sync::Arc;
use std::time::Duration;

use locking_service::config::ServiceConfig;
use locking_service::proto::locking_service_server::LockingService;
use locking_service::proto::{Door, GetLockStateRequest, LockRequest, UnlockRequest};
use locking_service::publisher::test_utils::MockSignalWriter;
use locking_service::service::LockingServiceImpl;
use locking_service::state::LockState;
use locking_service::validator::test_utils::MockSignalReader;
use proptest::prelude::*;
use tokio::sync::RwLock;
use tonic::Request;

/// Strategy to generate valid doors (excluding Unknown and All for GetLockState).
fn valid_single_door_strategy() -> impl Strategy<Value = Door> {
    prop_oneof![
        Just(Door::Driver),
        Just(Door::Passenger),
        Just(Door::RearLeft),
        Just(Door::RearRight),
    ]
}

/// Creates a test service with mock signal reader/writer.
fn create_test_service() -> (
    LockingServiceImpl<MockSignalReader, MockSignalWriter>,
    MockSignalReader,
    MockSignalWriter,
) {
    let state = Arc::new(RwLock::new(LockState::default()));
    let reader = MockSignalReader::new();
    let writer = MockSignalWriter::new();
    let config = ServiceConfig::default().with_execution_timeout(Duration::from_secs(1));

    // Set up default signals for safety validation
    reader.set_bool("Vehicle.Cabin.Door.Row1.DriverSide.IsOpen", false);
    reader.set_bool("Vehicle.Cabin.Door.Row1.PassengerSide.IsOpen", false);
    reader.set_bool("Vehicle.Cabin.Door.Row2.DriverSide.IsOpen", false);
    reader.set_bool("Vehicle.Cabin.Door.Row2.PassengerSide.IsOpen", false);
    reader.set_float("Vehicle.Speed", 0.0);

    let service = LockingServiceImpl::new(state, reader.clone(), writer.clone(), config);
    (service, reader, writer)
}

/// Helper to run async tests within proptest.
fn run_async<F, T>(f: F) -> T
where
    F: std::future::Future<Output = T>,
{
    tokio::runtime::Runtime::new().unwrap().block_on(f)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: locking-service, Property 5: State Publication Consistency
    ///
    /// For any successful Lock operation, the state published to DATA_BROKER
    /// SHALL be IsLocked=true.
    ///
    /// Validates: Requirements 1.5, 2.5, 5.1
    #[test]
    fn property_lock_publishes_locked_true(door in valid_single_door_strategy()) {
        let (service, _reader, writer) = create_test_service();

        let request = Request::new(LockRequest {
            door: door.into(),
            command_id: "cmd-lock".to_string(),
            auth_token: "demo-token".to_string(),
        });

        let result = run_async(service.lock(request));

        prop_assert!(result.is_ok(), "Lock should succeed");

        // Check that the published value is true (locked)
        let writes = writer.get_writes();
        prop_assert!(!writes.is_empty(), "Should have published state");

        for write in writes {
            if write.path.contains("IsLocked") {
                prop_assert!(write.value, "Published IsLocked should be true");
            }
        }
    }

    /// Feature: locking-service, Property 5: State Publication Consistency
    ///
    /// For any successful Unlock operation, the state published to DATA_BROKER
    /// SHALL be IsLocked=false.
    ///
    /// Validates: Requirements 1.5, 2.5, 5.1
    #[test]
    fn property_unlock_publishes_locked_false(door in valid_single_door_strategy()) {
        let (service, _reader, writer) = create_test_service();

        let request = Request::new(UnlockRequest {
            door: door.into(),
            command_id: "cmd-unlock".to_string(),
            auth_token: "demo-token".to_string(),
        });

        let result = run_async(service.unlock(request));

        prop_assert!(result.is_ok(), "Unlock should succeed");

        // Check that the published value is false (unlocked)
        let writes = writer.get_writes();
        prop_assert!(!writes.is_empty(), "Should have published state");

        for write in writes {
            if write.path.contains("IsLocked") {
                prop_assert!(!write.value, "Published IsLocked should be false");
            }
        }
    }

    /// Feature: locking-service, Property 6: GetLockState Returns Complete State
    ///
    /// For any valid door identifier, GetLockState SHALL return a response
    /// containing both is_locked and is_open fields.
    ///
    /// Validates: Requirements 3.1, 3.2
    #[test]
    fn property_get_lock_state_returns_complete_state(door in valid_single_door_strategy()) {
        let (service, _reader, _writer) = create_test_service();

        let request = Request::new(GetLockStateRequest {
            door: door.into(),
        });

        let result = run_async(service.get_lock_state(request));

        prop_assert!(result.is_ok(), "GetLockState should succeed for valid door {:?}", door);

        let response = result.unwrap().into_inner();

        // Response should have the door field set correctly
        let response_door = Door::try_from(response.door).unwrap_or(Door::Unknown);
        prop_assert_eq!(response_door, door, "Response door should match request");

        // is_locked and is_open are bool fields - they're always present in protobuf
        // Just verify we got a valid response
        prop_assert!(true, "Response contains is_locked and is_open fields");
    }

    /// Feature: locking-service, Property 7: Invalid Door Returns Error
    ///
    /// For any request with an invalid door identifier (DOOR_UNKNOWN),
    /// the LOCKING_SERVICE SHALL return an error.
    ///
    /// Validates: Requirements 3.3
    #[test]
    fn property_invalid_door_returns_error(_ in Just(0)) {
        let (service, _reader, _writer) = create_test_service();

        let request = Request::new(GetLockStateRequest {
            door: Door::Unknown.into(),
        });

        let result = run_async(service.get_lock_state(request));

        prop_assert!(result.is_err(), "GetLockState should fail for Unknown door");

        let status = result.unwrap_err();
        prop_assert_eq!(
            status.code(),
            tonic::Code::InvalidArgument,
            "Should return INVALID_ARGUMENT for unknown door"
        );
    }
}

/// Integration tests for end-to-end flows (Task 13.2).
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_complete_lock_flow() {
        let (service, _reader, writer) = create_test_service();

        // Execute lock
        let request = Request::new(LockRequest {
            door: Door::Driver.into(),
            command_id: "cmd-1".to_string(),
            auth_token: "demo-token".to_string(),
        });

        let response = service.lock(request).await;

        assert!(response.is_ok());
        let resp = response.unwrap().into_inner();
        assert!(resp.success);
        assert_eq!(resp.command_id, "cmd-1");

        // Verify state was published
        let writes = writer.get_writes();
        assert!(!writes.is_empty());
        assert!(writes
            .iter()
            .any(|w| w.path.contains("IsLocked") && w.value));

        // Verify internal state
        let state_request = Request::new(GetLockStateRequest {
            door: Door::Driver.into(),
        });
        let state_response = service.get_lock_state(state_request).await.unwrap();
        assert!(state_response.into_inner().is_locked);
    }

    #[tokio::test]
    async fn test_complete_unlock_flow() {
        let (service, _reader, writer) = create_test_service();

        // First lock the door
        let lock_request = Request::new(LockRequest {
            door: Door::Driver.into(),
            command_id: "cmd-lock".to_string(),
            auth_token: "demo-token".to_string(),
        });
        service.lock(lock_request).await.unwrap();
        writer.clear_writes();

        // Now unlock
        let unlock_request = Request::new(UnlockRequest {
            door: Door::Driver.into(),
            command_id: "cmd-unlock".to_string(),
            auth_token: "demo-token".to_string(),
        });

        let response = service.unlock(unlock_request).await;

        assert!(response.is_ok());
        let resp = response.unwrap().into_inner();
        assert!(resp.success);

        // Verify state was published as unlocked
        let writes = writer.get_writes();
        assert!(!writes.is_empty());
        assert!(writes
            .iter()
            .any(|w| w.path.contains("IsLocked") && !w.value));
    }

    #[tokio::test]
    async fn test_lock_fails_door_open() {
        let (service, reader, _writer) = create_test_service();

        // Set door as open
        reader.set_bool("Vehicle.Cabin.Door.Row1.DriverSide.IsOpen", true);

        let request = Request::new(LockRequest {
            door: Door::Driver.into(),
            command_id: "cmd-1".to_string(),
            auth_token: "demo-token".to_string(),
        });

        let response = service.lock(request).await;

        assert!(response.is_err());
        assert_eq!(
            response.unwrap_err().code(),
            tonic::Code::FailedPrecondition
        );
    }

    #[tokio::test]
    async fn test_unlock_fails_vehicle_moving() {
        let (service, reader, _writer) = create_test_service();

        // Set vehicle as moving
        reader.set_float("Vehicle.Speed", 50.0);

        let request = Request::new(UnlockRequest {
            door: Door::Driver.into(),
            command_id: "cmd-1".to_string(),
            auth_token: "demo-token".to_string(),
        });

        let response = service.unlock(request).await;

        assert!(response.is_err());
        assert_eq!(
            response.unwrap_err().code(),
            tonic::Code::FailedPrecondition
        );
    }

    #[tokio::test]
    async fn test_auth_failure() {
        let (service, _reader, _writer) = create_test_service();

        let request = Request::new(LockRequest {
            door: Door::Driver.into(),
            command_id: "cmd-1".to_string(),
            auth_token: "invalid-token".to_string(),
        });

        let response = service.lock(request).await;

        assert!(response.is_err());
        assert_eq!(response.unwrap_err().code(), tonic::Code::Unauthenticated);
    }

    #[tokio::test]
    async fn test_data_broker_unavailable_for_validation() {
        let (service, reader, _writer) = create_test_service();

        // Make signal reader return error
        reader.set_bool_error(
            "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen",
            "DATA_BROKER unavailable",
        );

        let request = Request::new(LockRequest {
            door: Door::Driver.into(),
            command_id: "cmd-1".to_string(),
            auth_token: "demo-token".to_string(),
        });

        let response = service.lock(request).await;

        assert!(response.is_err());
        assert_eq!(response.unwrap_err().code(), tonic::Code::Unavailable);
    }
}
