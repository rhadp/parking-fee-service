# Implementation Plan: LOCKING_SERVICE

## Overview

This implementation plan covers the ASIL-B LOCKING_SERVICE component for the SDV Parking Demo System. The service is implemented in Rust, runs in the RHIVOS safety partition, and communicates via gRPC over Unix Domain Sockets.

Tasks are organized to build incrementally: project setup, core data models, safety validation, command execution, state publication, gRPC server, and finally integration testing.

## Tasks

- [x] 1. Set up locking-service project structure
  - [x] 1.1 Create Rust crate structure for locking-service
    - Create `rhivos/locking-service/Cargo.toml` with dependencies (tonic, tokio, thiserror, tracing, proptest)
    - Create `rhivos/locking-service/src/lib.rs` as library root
    - Create `rhivos/locking-service/src/main.rs` as binary entry point
    - Add crate to `rhivos/Cargo.toml` workspace members
    - _Requirements: 8.1, 8.2, 8.3, 8.4_

  - [x] 1.2 Generate Rust bindings from locking_service.proto
    - Proto bindings are generated in `shared` crate via `rhivos/shared/build.rs`
    - Bindings available at `shared::sdv::services::locking`
    - Re-exported from `locking_service::proto` for convenience
    - _Requirements: 8.1, 8.2, 8.3_

- [x] 2. Implement core data models and configuration
  - [x] 2.1 Implement LockState and DoorState structs
    - Create `rhivos/locking-service/src/state.rs`
    - Implement LockState with driver, passenger, rear_left, rear_right fields
    - Implement DoorState with is_locked, is_open, last_updated fields
    - Implement get_door() and set_locked() methods
    - _Requirements: 3.1, 3.2_

  - [x] 2.2 Implement ServiceConfig struct
    - Create `rhivos/locking-service/src/config.rs`
    - Define socket_path, data_broker_socket, timeouts, retry settings
    - Implement Default trait with values from design document
    - Add environment variable loading support
    - _Requirements: 5.2, 6.1_

  - [x] 2.3 Implement error types
    - Create `rhivos/locking-service/src/error.rs`
    - Implement LockingError enum with AuthError, SafetyError, ExecutionError, DataBrokerError, TimeoutError, InvalidDoor variants
    - Implement SafetyViolation enum with DoorAjar and VehicleMoving variants
    - Implement From<LockingError> for tonic::Status
    - _Requirements: 1.3, 1.4, 2.3, 2.4, 3.3, 4.3, 6.2_

- [x] 3. Checkpoint - Verify data models compile
  - Run `cargo check` in locking-service directory
  - Ensure all types are properly defined
  - Ask the user if questions arise

- [x] 4. Implement authentication validation
  - [x] 4.1 Implement auth token validation
    - Create `rhivos/locking-service/src/auth.rs`
    - Implement validate_auth_token() function that checks token against config.valid_tokens
    - Return AuthError for invalid or missing tokens
    - _Requirements: 1.4, 2.4_

  - [x] 4.2 Write property test for auth token rejection
    - **Property 1: Invalid Auth Token Rejection**
    - Generate random invalid tokens and verify rejection with UNAUTHENTICATED status
    - **Validates: Requirements 1.4, 2.4**

- [ ] 5. Implement safety validation
  - [ ] 5.1 Implement SafetyValidator struct
    - Create `rhivos/locking-service/src/validator.rs`
    - Implement SafetyValidator with data_broker_client and validation_timeout
    - Implement validate_lock() that reads IsOpen signal and rejects if door is open
    - Implement validate_unlock() that reads Vehicle.Speed and rejects if speed > 0
    - Handle DATA_BROKER unavailability with appropriate error
    - _Requirements: 1.3, 2.3, 4.1, 4.2, 4.3, 4.4_

  - [ ] 5.2 Write property test for lock safety constraint
    - **Property 2: Lock Fails When Door Is Open**
    - Generate lock commands with door open state and verify rejection
    - Verify lock state remains unchanged after rejection
    - **Validates: Requirements 1.3**

  - [ ] 5.3 Write property test for unlock safety constraint
    - **Property 3: Unlock Fails When Vehicle Is Moving**
    - Generate unlock commands with non-zero speed and verify rejection
    - Verify lock state remains unchanged after rejection
    - **Validates: Requirements 2.3**

- [ ] 6. Implement lock execution
  - [ ] 6.1 Implement LockExecutor struct
    - Create `rhivos/locking-service/src/executor.rs`
    - Implement LockExecutor with lock_state (Arc<RwLock<LockState>>) and execution_timeout
    - Implement execute_lock() that sets is_locked=true for specified door
    - Implement execute_unlock() that sets is_locked=false for specified door
    - Implement timeout handling with tokio::time::timeout
    - _Requirements: 1.2, 2.2, 6.1, 6.2, 6.3_

  - [ ] 6.2 Write property test for command ID consistency
    - **Property 4: Valid Commands Return Correct Command_ID**
    - Generate valid commands and verify response contains same Command_ID
    - **Validates: Requirements 1.2, 2.2**

  - [ ] 6.3 Write property test for timeout state consistency
    - **Property 8: Timeout Preserves State Consistency**
    - Simulate timeout scenarios and verify no partial state changes
    - **Validates: Requirements 6.3**

- [ ] 7. Checkpoint - Verify core logic
  - Run `cargo test` for unit tests
  - Ensure safety validation and execution work correctly
  - Ask the user if questions arise

- [ ] 8. Implement state publication
  - [ ] 8.1 Implement StatePublisher struct
    - Create `rhivos/locking-service/src/publisher.rs`
    - Implement StatePublisher with data_broker_client, max_retries, base_delay
    - Implement publish_lock_state() with exponential backoff retry logic
    - Return PublishError::AllRetriesFailed after max retries exhausted
    - _Requirements: 5.1, 5.3, 5.4_

  - [ ] 8.2 Write property test for state publication consistency
    - **Property 5: State Publication Consistency**
    - Verify successful lock publishes IsLocked=true, unlock publishes IsLocked=false
    - **Validates: Requirements 1.5, 2.5, 5.1**

- [ ] 9. Implement logging
  - [ ] 9.1 Implement structured logging
    - Create `rhivos/locking-service/src/logging.rs`
    - Implement LogEntry struct with timestamp, level, command_id, correlation_id, event_type, door, details
    - Implement EventType enum (CommandReceived, AuthValidation, SafetyValidation, Execution, StatePublish, CommandComplete)
    - Configure tracing subscriber for structured JSON output
    - _Requirements: 7.1, 7.2, 7.3, 7.4_

- [ ] 10. Implement gRPC service
  - [ ] 10.1 Implement LockingServiceImpl struct
    - Create `rhivos/locking-service/src/service.rs`
    - Implement LockingServiceImpl with data_broker_client, lock_state, config, logger
    - Wire together auth validation, safety validation, execution, and state publication
    - Implement overall command timeout using tokio::time::timeout
    - _Requirements: 1.1, 1.2, 2.1, 2.2, 6.1, 6.2_

  - [ ] 10.2 Implement Lock RPC handler
    - Implement tonic::LockingService::lock() method
    - Validate auth token, then safety constraints (door not open)
    - Execute lock operation and publish state
    - Return LockResponse with success, error_message, command_id, state_published
    - Log all steps with correlation ID
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5_

  - [ ] 10.3 Implement Unlock RPC handler
    - Implement tonic::LockingService::unlock() method
    - Validate auth token, then safety constraints (vehicle stationary)
    - Execute unlock operation and publish state
    - Return UnlockResponse with success, error_message, command_id, state_published
    - Log all steps with correlation ID
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5_

  - [ ] 10.4 Implement GetLockState RPC handler
    - Implement tonic::LockingService::get_lock_state() method
    - Return current is_locked and is_open for specified door
    - Return error for invalid door identifier
    - _Requirements: 3.1, 3.2, 3.3_

  - [ ] 10.5 Write property test for GetLockState completeness
    - **Property 6: GetLockState Returns Complete State**
    - Verify response contains both is_locked and is_open for valid doors
    - **Validates: Requirements 3.1, 3.2**

  - [ ] 10.6 Write property test for invalid door handling
    - **Property 7: Invalid Door Returns Error**
    - Generate invalid door identifiers and verify INVALID_ARGUMENT error
    - **Validates: Requirements 3.3**

- [ ] 11. Implement gRPC server startup
  - [ ] 11.1 Implement UDS server listener
    - Update `rhivos/locking-service/src/main.rs`
    - Create Unix Domain Socket at configured path
    - Initialize DATA_BROKER client connection
    - Start tonic gRPC server with LockingServiceImpl
    - Handle graceful shutdown on SIGTERM
    - _Requirements: 8.4, 5.2_

- [ ] 12. Checkpoint - Verify gRPC service
  - Run `cargo test` for all tests
  - Verify service starts and listens on UDS
  - Ask the user if questions arise

- [ ] 13. Integration testing
  - [ ] 13.1 Create mock DATA_BROKER client for testing
    - Create `rhivos/locking-service/src/test_utils.rs`
    - Implement MockDataBrokerClient that simulates signal reads/writes
    - Support configurable responses for speed and door state signals
    - Support failure injection for testing error paths
    - _Requirements: 4.3, 5.3, 5.4_

  - [ ] 13.2 Write integration tests for end-to-end flows
    - Test complete lock flow: auth → safety → execute → publish → response
    - Test complete unlock flow with stationary vehicle
    - Test partial success when publish fails
    - Test DATA_BROKER unavailability handling
    - _Requirements: 1.1-1.5, 2.1-2.5, 4.3, 5.4_

- [ ] 14. Final checkpoint - Verify complete implementation
  - Run `cargo test` for all unit and property tests
  - Run `cargo clippy` for linting
  - Ensure all 8 properties pass
  - Ask the user if questions arise

## Notes

- All tasks including property tests are required for comprehensive implementation
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation
- Property tests validate universal correctness properties from the design document
- The service uses `proptest` crate for property-based testing with minimum 100 iterations per test
