# Implementation Plan: PARKING_OPERATOR_ADAPTOR

## Overview

This implementation plan covers the PARKING_OPERATOR_ADAPTOR component for the SDV Parking Demo System. The service is implemented in Rust, runs as a container in the RHIVOS QM partition, and manages parking sessions automatically based on vehicle lock state.

Tasks are organized to build incrementally: project setup, core data models, DATA_BROKER integration, PARKING_OPERATOR API client, session management, gRPC server, and integration testing.

## Tasks

- [ ] 1. Set up parking-operator-adaptor project structure
  - [ ] 1.1 Create Rust crate structure for parking-operator-adaptor
    - Create `rhivos/parking-operator-adaptor/Cargo.toml` with dependencies (tonic, tokio, reqwest, serde, thiserror, tracing, proptest)
    - Create `rhivos/parking-operator-adaptor/src/lib.rs` as library root
    - Create `rhivos/parking-operator-adaptor/src/main.rs` as binary entry point
    - Add crate to `rhivos/Cargo.toml` workspace members
    - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5_

  - [ ] 1.2 Generate Rust bindings from parking_adaptor.proto
    - Create `rhivos/parking-operator-adaptor/build.rs` with tonic-build configuration
    - Create `proto/services/parking_adaptor.proto` with service definition from design
    - Configure proto path and generate server/client code
    - _Requirements: 8.1, 8.2, 8.3_

- [ ] 2. Implement core data models and configuration
  - [ ] 2.1 Implement Session and SessionState types
    - Create `rhivos/parking-operator-adaptor/src/session.rs`
    - Implement SessionState enum (None, Starting, Active, Stopping, Stopped, Error)
    - Implement Session struct with all fields from design
    - Implement duration() and state helper methods
    - Add Serialize/Deserialize derives for persistence
    - _Requirements: 7.1, 7.2_

  - [ ] 2.2 Implement ServiceConfig struct
    - Create `rhivos/parking-operator-adaptor/src/config.rs`
    - Define all configuration fields from design (listen_addr, TLS paths, DATA_BROKER socket, operator URL, etc.)
    - Implement Default trait with values from design document
    - Add environment variable loading support
    - _Requirements: 10.1, 10.2, 10.3, 10.4_

  - [ ] 2.3 Implement error types
    - Create `rhivos/parking-operator-adaptor/src/error.rs`
    - Implement ParkingError enum with all variants from design
    - Implement ApiError enum for PARKING_OPERATOR API errors
    - Implement From<ParkingError> for tonic::Status with proper gRPC status codes
    - _Requirements: 6.3, 6.4, 2.3, 3.6, 4.6_

- [ ] 3. Checkpoint - Verify data models compile
  - Run `cargo check` in parking-operator-adaptor directory
  - Ensure all types are properly defined
  - Ask the user if questions arise

- [ ] 4. Implement session persistence
  - [ ] 4.1 Implement SessionStore struct
    - Create `rhivos/parking-operator-adaptor/src/store.rs`
    - Implement SessionStore with storage_path
    - Implement save() to serialize session to JSON file
    - Implement load() to deserialize session from JSON file
    - Implement clear() to remove stored session
    - _Requirements: 7.3, 7.4_

  - [ ] 4.2 Write property test for session persistence round-trip
    - **Property 13: Session Persistence Round-Trip**
    - Generate random sessions, save and load, verify equivalence
    - **Validates: Requirements 7.3**

- [ ] 5. Implement DATA_BROKER integration
  - [ ] 5.1 Implement LocationReader struct
    - Create `rhivos/parking-operator-adaptor/src/location.rs`
    - Implement LocationReader with data_broker_client
    - Implement read_location() to fetch latitude and longitude
    - Return LocationError if signals unavailable
    - _Requirements: 2.1, 2.2, 2.3, 2.4_

  - [ ] 5.2 Write property test for location reading
    - **Property 3: Location Reading During Session Start**
    - Test with available and unavailable location signals
    - **Validates: Requirements 2.1, 2.2, 2.3**

  - [ ] 5.3 Implement StatePublisher struct
    - Create `rhivos/parking-operator-adaptor/src/publisher.rs`
    - Implement StatePublisher with data_broker_client
    - Implement publish_session_active() to write Vehicle.Parking.SessionActive
    - _Requirements: 3.4, 4.4_

  - [ ] 5.4 Implement SignalSubscriber struct
    - Create `rhivos/parking-operator-adaptor/src/subscriber.rs`
    - Implement SignalSubscriber with data_broker_client and session_manager reference
    - Implement start() to subscribe to IsLocked signal
    - Implement on_signal_change() to detect lock/unlock transitions
    - Implement reconnect() with exponential backoff
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5_

  - [ ] 5.5 Write property test for lock event triggers session start
    - **Property 1: Lock Event Triggers Session Start**
    - Simulate lock events and verify session start initiation
    - **Validates: Requirements 1.2**

  - [ ] 5.6 Write property test for unlock event triggers session stop
    - **Property 2: Unlock Event Triggers Session Stop**
    - Simulate unlock events and verify session stop initiation
    - **Validates: Requirements 1.3**

  - [ ] 5.7 Write property test for DATA_BROKER reconnection
    - **Property 15: DATA_BROKER Reconnection with Backoff**
    - Simulate connection loss and verify reconnection attempts
    - **Validates: Requirements 1.4, 1.5**

- [ ] 6. Checkpoint - Verify DATA_BROKER integration
  - Run `cargo test` for unit tests
  - Ensure location reading and state publishing work correctly
  - Ask the user if questions arise

- [ ] 7. Implement PARKING_OPERATOR API client
  - [ ] 7.1 Implement OperatorApiClient struct
    - Create `rhivos/parking-operator-adaptor/src/operator.rs`
    - Implement OperatorApiClient with http_client, base_url, vehicle_id, retry settings
    - Implement start_session() to call POST /parking/start with retry
    - Implement stop_session() to call POST /parking/stop with retry
    - Implement get_status() to call GET /parking/status/{session_id}
    - Implement call_with_retry() helper with exponential backoff
    - _Requirements: 3.1, 3.2, 3.5, 4.1, 4.2, 4.5, 5.4, 9.1, 9.2, 9.3, 9.4, 9.5_

  - [ ] 7.2 Write property test for API request completeness (start)
    - **Property 4: Session Start API Request Completeness**
    - Verify start request includes all required fields
    - **Validates: Requirements 3.1, 3.2, 3.3**

  - [ ] 7.3 Write property test for API request completeness (stop)
    - **Property 5: Session Stop API Request Completeness**
    - Verify stop request includes session_id and timestamp
    - **Validates: Requirements 4.1, 4.2, 4.3**

  - [ ] 7.4 Write property test for API retry behavior
    - **Property 7: API Retry with Exponential Backoff**
    - Inject failures and verify retry count and delays
    - **Validates: Requirements 3.5, 4.5**

  - [ ] 7.5 Write property test for error state after retries
    - **Property 8: Error State After Retry Exhaustion**
    - Verify ERROR state and error message after all retries fail
    - **Validates: Requirements 3.6, 4.6**

- [ ] 8. Implement session management
  - [ ] 8.1 Implement SessionManager struct
    - Create `rhivos/parking-operator-adaptor/src/manager.rs`
    - Implement SessionManager with all dependencies (location_reader, operator_client, state_publisher, session_store)
    - Implement operation_lock mutex for concurrency control
    - Implement start_session() with full workflow (location → API → publish → persist)
    - Implement stop_session() with full workflow (API → publish → persist)
    - Implement get_status() to return current session info
    - Implement on_lock_state_change() for automatic session control
    - _Requirements: 1.2, 1.3, 3.1, 3.3, 3.4, 4.1, 4.3, 4.4, 5.1, 6.1, 6.2, 7.1, 7.2, 7.5_

  - [ ] 8.2 Write property test for session state publication
    - **Property 6: Session State Publication Consistency**
    - Verify SessionActive matches session state
    - **Validates: Requirements 3.4, 4.4**

  - [ ] 8.3 Write property test for state timestamp recording
    - **Property 12: State Transition Timestamp Recording**
    - Verify last_updated is updated on state changes
    - **Validates: Requirements 7.1, 7.2**

  - [ ] 8.4 Write property test for concurrent operation prevention
    - **Property 14: Concurrent Operation Prevention**
    - Verify concurrent operations are rejected
    - **Validates: Requirements 7.5**

- [ ] 9. Implement status polling
  - [ ] 9.1 Implement StatusPoller struct
    - Create `rhivos/parking-operator-adaptor/src/poller.rs`
    - Implement StatusPoller with operator_client and session_store
    - Implement start() to spawn background polling task
    - Poll GET /parking/status/{session_id} at configured interval
    - Update session current_cost from poll response
    - _Requirements: 5.4_

- [ ] 10. Checkpoint - Verify session management
  - Run `cargo test` for all tests
  - Verify session start/stop workflows work correctly
  - Ask the user if questions arise

- [ ] 11. Implement gRPC service
  - [ ] 11.1 Implement ParkingAdaptorImpl struct
    - Create `rhivos/parking-operator-adaptor/src/service.rs`
    - Implement ParkingAdaptorImpl with session_manager reference
    - Wire up gRPC trait implementation
    - _Requirements: 8.1, 8.2, 8.3_

  - [ ] 11.2 Implement StartSession RPC handler
    - Delegate to session_manager.start_session()
    - Return StartSessionResponse with session details or error
    - _Requirements: 6.1, 6.3, 8.1_

  - [ ] 11.3 Implement StopSession RPC handler
    - Delegate to session_manager.stop_session()
    - Return StopSessionResponse with final cost and duration or error
    - _Requirements: 6.2, 6.4, 8.2_

  - [ ] 11.4 Implement GetSessionStatus RPC handler
    - Delegate to session_manager.get_status()
    - Return GetSessionStatusResponse with all session fields
    - _Requirements: 5.1, 5.2, 5.3, 8.3_

  - [ ] 11.5 Write property test for status response completeness
    - **Property 9: Status Response Completeness**
    - Verify all required fields are present in response
    - **Validates: Requirements 5.1, 5.2**

  - [ ] 11.6 Write property test for manual session control
    - **Property 10: Manual Session Control Independence**
    - Verify sessions start/stop regardless of lock state
    - **Validates: Requirements 6.1, 6.2**

  - [ ] 11.7 Write property test for invalid operation rejection
    - **Property 11: Invalid Operation Rejection**
    - Verify errors for duplicate start and stop without session
    - **Validates: Requirements 6.3, 6.4**

- [ ] 12. Implement gRPC server startup with TLS
  - [ ] 12.1 Implement TLS server listener
    - Update `rhivos/parking-operator-adaptor/src/main.rs`
    - Load TLS certificate and key from configured paths
    - Create TCP listener on configured address
    - Initialize all components (location_reader, operator_client, state_publisher, session_store, session_manager)
    - Restore session from storage on startup
    - Start SignalSubscriber for lock event subscription
    - Start StatusPoller for active session polling
    - Start tonic gRPC server with TLS and ParkingAdaptorImpl
    - Handle graceful shutdown on SIGTERM
    - _Requirements: 7.4, 8.4, 8.5, 10.1, 10.2, 10.3_

- [ ] 13. Checkpoint - Verify gRPC service
  - Run `cargo test` for all tests
  - Verify service compiles and handlers are wired correctly
  - Ask the user if questions arise

- [ ] 14. Implement logging
  - [ ] 14.1 Implement structured logging
    - Create `rhivos/parking-operator-adaptor/src/logging.rs`
    - Configure tracing subscriber for structured JSON output
    - Add logging to all request handlers, state transitions, and API calls
    - Include correlation identifiers for end-to-end tracing
    - _Requirements: 11.1, 11.2, 11.3, 11.4, 11.5_

- [ ] 15. Checkpoint - Verify complete service
  - Run `cargo test` for all unit and property tests
  - Run `cargo clippy` for linting
  - Verify service starts with TLS and accepts connections
  - Ask the user if questions arise

- [ ] 16. Integration testing
  - [ ] 16.1 Create mock PARKING_OPERATOR for testing
    - Create `rhivos/parking-operator-adaptor/src/test_utils.rs`
    - Implement MockOperator that serves REST API responses
    - Support configurable responses for success/failure scenarios
    - Support configurable delays for timeout testing
    - _Requirements: 3.1, 4.1, 9.1_

  - [ ] 16.2 Create mock DATA_BROKER for testing
    - Implement MockDataBroker that simulates signal subscription and publication
    - Support signal injection for lock/unlock events
    - Support failure injection for testing error paths
    - _Requirements: 1.1, 2.1, 2.2, 3.4, 4.4_

  - [ ] 16.3 Write integration tests for end-to-end flows
    - Test complete session start flow: lock event → location read → API call → publish → persist
    - Test complete session stop flow: unlock event → API call → publish → persist
    - Test manual session control via gRPC
    - Test session recovery after container restart
    - Test status polling during active sessions
    - Test error handling and retry behavior
    - _Requirements: 1.1-1.5, 2.1-2.4, 3.1-3.6, 4.1-4.6, 5.1-5.4, 6.1-6.4, 7.1-7.5_

- [ ] 17. Final checkpoint - Verify complete implementation
  - Run `cargo test` for all unit, property, and integration tests
  - Run `cargo clippy` for linting
  - Ensure all 15 properties pass
  - Ask the user if questions arise

## Notes

- All tasks including property tests are required for comprehensive implementation
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation
- Property tests validate universal correctness properties from the design document
- The service uses `proptest` crate for property-based testing with minimum 100 iterations per test
- TLS certificates should be generated for development using `infra/certs/` tooling
- The service is containerized and managed by UPDATE_SERVICE
