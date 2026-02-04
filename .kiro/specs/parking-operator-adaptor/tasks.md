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

  - [ ] 1.2 Create parking_adaptor.proto service definition
    - Create `proto/services/parking_adaptor.proto` with ParkingAdaptor service
    - Define SessionState enum (NONE, STARTING, ACTIVE, STOPPING, STOPPED, ERROR)
    - Define StartSessionRequest/Response, StopSessionRequest/Response, GetSessionStatusRequest/Response
    - _Requirements: 8.1, 8.2, 8.3_

  - [ ] 1.3 Generate Rust bindings from parking_adaptor.proto
    - Create `rhivos/parking-operator-adaptor/build.rs` with tonic-build configuration
    - Configure proto path and generate server/client code
    - _Requirements: 8.1, 8.2, 8.3_

- [ ] 2. Implement core data models and configuration
  - [ ] 2.1 Implement Session and SessionState types
    - Create `rhivos/parking-operator-adaptor/src/session.rs`
    - Implement SessionState enum (None, Starting, Active, Stopping, Stopped, Error)
    - Implement Session struct with all fields (session_id, state, start_time, end_time, location, zone_id, hourly_rate, current_cost, final_cost, error_message, last_updated)
    - Implement duration() and is_active() and is_in_progress() helper methods
    - Add Serialize/Deserialize derives for persistence
    - _Requirements: 7.1, 7.2_

  - [ ] 2.2 Implement Location struct
    - Create `rhivos/parking-operator-adaptor/src/location.rs`
    - Define Location struct with latitude and longitude fields
    - _Requirements: 2.1, 2.2_

  - [ ] 2.3 Implement ServiceConfig struct
    - Create `rhivos/parking-operator-adaptor/src/config.rs`
    - Define all configuration fields (listen_addr, tls_cert_path, tls_key_path, data_broker_socket, operator_base_url, vehicle_id, hourly_rate, api_max_retries, api_base_delay_ms, api_timeout_ms, reconnect_max_attempts, reconnect_base_delay_ms, poll_interval_seconds, storage_path)
    - Implement Default trait with values from design document
    - Add environment variable loading support using std::env
    - _Requirements: 10.1, 10.2, 10.3, 10.4_

  - [ ] 2.4 Implement error types
    - Create `rhivos/parking-operator-adaptor/src/error.rs`
    - Implement ParkingError enum (SessionAlreadyActive, NoActiveSession, OperationInProgress, LocationUnavailable, OperatorApiError, DataBrokerError, DataBrokerConnectionLost, StorageError, ApiTimeout)
    - Implement ApiError enum (HttpError, NetworkError, Timeout, InvalidResponse)
    - Implement From<ParkingError> for tonic::Status with proper gRPC status codes per design
    - _Requirements: 6.3, 6.4, 2.3, 3.6, 4.6, 8.5_

- [ ] 3. Checkpoint - Verify data models compile
  - Run `cargo check` in parking-operator-adaptor directory
  - Ensure all types are properly defined
  - Ask the user if questions arise

- [ ] 4. Implement session persistence
  - [ ] 4.1 Implement SessionStore struct
    - Create `rhivos/parking-operator-adaptor/src/store.rs`
    - Implement SessionStore with storage_path field
    - Implement save() to serialize session to JSON file
    - Implement load() to deserialize session from JSON file
    - Implement clear() to remove stored session
    - _Requirements: 7.3, 7.4_

  - [ ] 4.2 Write property test for session persistence round-trip
    - **Property 13: Session Persistence Round-Trip**
    - *For any* session saved to persistent storage, loading that session SHALL produce an equivalent session object with all fields preserved
    - Generate random sessions with proptest, save and load, verify equivalence
    - **Validates: Requirements 7.3**

- [ ] 5. Implement DATA_BROKER integration
  - [ ] 5.1 Implement LocationReader struct
    - Add LocationReader to `rhivos/parking-operator-adaptor/src/location.rs`
    - Implement LocationReader with data_broker_client field
    - Implement read_location() to fetch Vehicle.CurrentLocation.Latitude and Vehicle.CurrentLocation.Longitude
    - Return LocationError if signals unavailable
    - _Requirements: 2.1, 2.2, 2.3_

  - [ ] 5.2 Write property test for location reading
    - **Property 3: Location Reading During Session Start**
    - *For any* session start operation, the PARKING_OPERATOR_ADAPTOR SHALL read both latitude and longitude. If either is unavailable, session start SHALL be rejected
    - Test with available and unavailable location signals
    - **Validates: Requirements 2.1, 2.2, 2.3**

  - [ ] 5.3 Implement StatePublisher struct
    - Create `rhivos/parking-operator-adaptor/src/publisher.rs`
    - Implement StatePublisher with data_broker_client field
    - Implement publish_session_active(active: bool) to write Vehicle.Parking.SessionActive signal
    - _Requirements: 3.4, 4.4_

  - [ ] 5.4 Implement SignalSubscriber struct
    - Create `rhivos/parking-operator-adaptor/src/subscriber.rs`
    - Implement SignalSubscriber with data_broker_client, session_manager, reconnect_attempts, reconnect_base_delay fields
    - Implement start() to subscribe to Vehicle.Cabin.Door.Row1.DriverSide.IsLocked signal
    - Implement on_signal_change() to detect false→true (lock) and true→false (unlock) transitions
    - Implement reconnect() with exponential backoff (5 attempts max)
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5_

  - [ ] 5.5 Write property test for lock event triggers session start
    - **Property 1: Lock Event Triggers Session Start**
    - *For any* vehicle in unlocked state with no active session, when IsLocked transitions false→true, session start SHALL be initiated
    - Simulate lock events and verify session start initiation
    - **Validates: Requirements 1.2**

  - [ ] 5.6 Write property test for unlock event triggers session stop
    - **Property 2: Unlock Event Triggers Session Stop**
    - *For any* vehicle with active session, when IsLocked transitions true→false, session stop SHALL be initiated
    - Simulate unlock events and verify session stop initiation
    - **Validates: Requirements 1.3**

  - [ ] 5.7 Write property test for DATA_BROKER reconnection
    - **Property 15: DATA_BROKER Reconnection with Backoff**
    - *For any* DATA_BROKER connection loss, reconnection SHALL be attempted with exponential backoff, entering degraded state after 5 failures
    - Simulate connection loss and verify reconnection attempts with increasing delays
    - **Validates: Requirements 1.4, 1.5**

- [ ] 6. Checkpoint - Verify DATA_BROKER integration
  - Run `cargo test` for unit tests
  - Ensure location reading and state publishing work correctly
  - Ask the user if questions arise

- [ ] 7. Implement PARKING_OPERATOR API client
  - [ ] 7.1 Implement API request/response models
    - Create `rhivos/parking-operator-adaptor/src/operator.rs`
    - Implement StartRequest struct (vehicle_id, latitude, longitude, zone_id, timestamp)
    - Implement StartResponse struct (session_id, zone_id, hourly_rate, start_time)
    - Implement StopRequest struct (session_id, timestamp)
    - Implement StopResponse struct (session_id, start_time, end_time, duration_seconds, total_cost, payment_status)
    - Implement StatusResponse struct (session_id, state, start_time, duration_seconds, current_cost, zone_id)
    - _Requirements: 3.2, 4.2, 9.1_

  - [ ] 7.2 Implement OperatorApiClient struct
    - Add OperatorApiClient to `rhivos/parking-operator-adaptor/src/operator.rs`
    - Implement with http_client (reqwest), base_url, vehicle_id, max_retries, base_delay, request_timeout fields
    - Implement call_with_retry() helper with exponential backoff
    - Implement start_session(location, zone_id) to call POST /parking/start with retry
    - Implement stop_session(session_id) to call POST /parking/stop with retry
    - Implement get_status(session_id) to call GET /parking/status/{session_id}
    - Handle HTTP error responses (4xx, 5xx) and map to ApiError
    - Implement 10 second request timeout
    - _Requirements: 3.1, 3.2, 3.5, 4.1, 4.2, 4.5, 5.4, 9.1, 9.2, 9.3, 9.4, 9.5_

  - [ ] 7.3 Write property test for API request completeness (start)
    - **Property 4: Session Start API Request Completeness**
    - *For any* session start with valid location, POST /parking/start SHALL include vehicle_id, latitude, longitude, zone_id, timestamp
    - Verify start request includes all required fields
    - **Validates: Requirements 3.1, 3.2, 3.3**

  - [ ] 7.4 Write property test for API request completeness (stop)
    - **Property 5: Session Stop API Request Completeness**
    - *For any* session stop with active session, POST /parking/stop SHALL include session_id and timestamp
    - Verify stop request includes session_id and timestamp
    - **Validates: Requirements 4.1, 4.2, 4.3**

  - [ ] 7.5 Write property test for API retry behavior
    - **Property 7: API Retry with Exponential Backoff**
    - *For any* failed API call, retry up to 3 times with exponential backoff (delay doubles after each attempt)
    - Inject failures and verify retry count and delays
    - **Validates: Requirements 3.5, 4.5**

  - [ ] 7.6 Write property test for error state after retries
    - **Property 8: Error State After Retry Exhaustion**
    - *For any* operation where all retries fail, session state SHALL transition to ERROR with error_message
    - Verify ERROR state and error message after all retries fail
    - **Validates: Requirements 3.6, 4.6**

- [ ] 8. Checkpoint - Verify PARKING_OPERATOR API client
  - Run `cargo test` for API client tests
  - Verify retry logic and error handling work correctly
  - Ask the user if questions arise

- [ ] 9. Implement session management
  - [ ] 9.1 Implement SessionManager struct
    - Create `rhivos/parking-operator-adaptor/src/manager.rs`
    - Implement SessionManager with current_session (RwLock), location_reader, operator_client, state_publisher, session_store, operation_lock (Mutex) fields
    - Implement start_session(zone_id) with full workflow: check no active session → read location → call API → store session_id → publish SessionActive=true → persist
    - Implement stop_session() with full workflow: check active session → call API → update final_cost/duration → publish SessionActive=false → persist
    - Implement get_status() to return current session info
    - Implement on_lock_state_change(old, new) for automatic session control
    - Implement is_operation_in_progress() to check STARTING/STOPPING states
    - _Requirements: 1.2, 1.3, 3.1, 3.3, 3.4, 4.1, 4.3, 4.4, 5.1, 6.1, 6.2, 7.1, 7.2, 7.5_

  - [ ] 9.2 Write property test for session state publication
    - **Property 6: Session State Publication Consistency**
    - *For any* successful session start, SessionActive=true SHALL be published. *For any* successful stop, SessionActive=false SHALL be published
    - Verify SessionActive matches session state
    - **Validates: Requirements 3.4, 4.4**

  - [ ] 9.3 Write property test for state timestamp recording
    - **Property 12: State Transition Timestamp Recording**
    - *For any* session state change, last_updated timestamp SHALL be updated to reflect the time of transition
    - Verify last_updated is updated on state changes
    - **Validates: Requirements 7.1, 7.2**

  - [ ] 9.4 Write property test for concurrent operation prevention
    - **Property 14: Concurrent Operation Prevention**
    - *For any* operation in progress (STARTING/STOPPING), concurrent operations SHALL be rejected with OperationInProgress error
    - Verify concurrent operations are rejected
    - **Validates: Requirements 7.5**

- [ ] 10. Implement status polling
  - [ ] 10.1 Implement StatusPoller struct
    - Create `rhivos/parking-operator-adaptor/src/poller.rs`
    - Implement StatusPoller with operator_client, session_store, poll_interval fields
    - Implement start() to spawn background polling task
    - Poll GET /parking/status/{session_id} at configured interval during active sessions
    - Update session current_cost from poll response
    - _Requirements: 5.4_

- [ ] 11. Checkpoint - Verify session management
  - Run `cargo test` for all tests
  - Verify session start/stop workflows work correctly
  - Ask the user if questions arise

- [ ] 12. Implement gRPC service
  - [ ] 12.1 Implement ParkingAdaptorImpl struct
    - Create `rhivos/parking-operator-adaptor/src/service.rs`
    - Implement ParkingAdaptorImpl with session_manager (Arc) and config fields
    - Implement ParkingAdaptor trait from generated proto code
    - _Requirements: 8.1, 8.2, 8.3_

  - [ ] 12.2 Implement StartSession RPC handler
    - Implement start_session() RPC method
    - Extract zone_id from StartSessionRequest
    - Delegate to session_manager.start_session(zone_id)
    - Return StartSessionResponse with success, session_id, state, or error_message
    - _Requirements: 6.1, 6.3, 8.1_

  - [ ] 12.3 Implement StopSession RPC handler
    - Implement stop_session() RPC method
    - Delegate to session_manager.stop_session()
    - Return StopSessionResponse with success, session_id, state, final_cost, duration_seconds, or error_message
    - _Requirements: 6.2, 6.4, 8.2_

  - [ ] 12.4 Implement GetSessionStatus RPC handler
    - Implement get_session_status() RPC method
    - Delegate to session_manager.get_status()
    - Return GetSessionStatusResponse with has_active_session, session_id, state, start_time_unix, duration_seconds, current_cost, zone_id, latitude, longitude, error_message
    - _Requirements: 5.1, 5.2, 5.3, 8.3_

  - [ ] 12.5 Write property test for status response completeness
    - **Property 9: Status Response Completeness**
    - *For any* GetSessionStatus request when session exists, response SHALL include session_id, state, start_time, duration, current_cost, zone_id, error_message (if ERROR)
    - Verify all required fields are present in response
    - **Validates: Requirements 5.1, 5.2**

  - [ ] 12.6 Write property test for manual session control
    - **Property 10: Manual Session Control Independence**
    - *For any* StartSession/StopSession gRPC request, session operation SHALL proceed regardless of current lock state
    - Verify sessions start/stop regardless of lock state
    - **Validates: Requirements 6.1, 6.2**

  - [ ] 12.7 Write property test for invalid operation rejection
    - **Property 11: Invalid Operation Rejection**
    - *For any* StartSession when session active, return error. *For any* StopSession when no session, return error
    - Verify errors for duplicate start and stop without session
    - **Validates: Requirements 6.3, 6.4**

- [ ] 13. Implement gRPC server startup with TLS
  - [ ] 13.1 Implement main entry point with TLS server
    - Update `rhivos/parking-operator-adaptor/src/main.rs`
    - Load ServiceConfig from environment variables
    - Validate required configuration, fail with clear error if missing
    - Load TLS certificate and key from configured paths
    - Initialize all components (LocationReader, OperatorApiClient, StatePublisher, SessionStore, SessionManager)
    - Restore session from storage on startup (for container restart recovery)
    - Start SignalSubscriber for lock event subscription
    - Start StatusPoller for active session polling
    - Create TCP listener on configured address
    - Start tonic gRPC server with TLS and ParkingAdaptorImpl
    - Handle graceful shutdown on SIGTERM
    - _Requirements: 7.4, 8.4, 8.5, 10.1, 10.2, 10.3, 10.4_

- [ ] 14. Checkpoint - Verify gRPC service
  - Run `cargo test` for all tests
  - Verify service compiles and handlers are wired correctly
  - Ask the user if questions arise

- [ ] 15. Implement logging
  - [ ] 15.1 Implement structured logging
    - Create `rhivos/parking-operator-adaptor/src/logging.rs`
    - Configure tracing subscriber for structured JSON output
    - Add logging to all gRPC request handlers with timestamp, request type, parameters
    - Add logging to all session state transitions with previous state, new state, reason
    - Add logging to all PARKING_OPERATOR API calls with request details and response status
    - Add logging to all DATA_BROKER signal subscriptions and publications
    - Include correlation identifiers (request_id) for end-to-end tracing
    - _Requirements: 11.1, 11.2, 11.3, 11.4, 11.5_

- [ ] 16. Checkpoint - Verify complete service
  - Run `cargo test` for all unit and property tests
  - Run `cargo clippy` for linting
  - Verify service starts with TLS and accepts connections
  - Ask the user if questions arise

- [ ] 17. Integration testing
  - [ ] 17.1 Create mock PARKING_OPERATOR for testing
    - Create `rhivos/parking-operator-adaptor/src/test_utils.rs`
    - Implement MockOperator that serves REST API responses
    - Support configurable responses for success/failure scenarios
    - Support configurable delays for timeout testing
    - _Requirements: 3.1, 4.1, 9.1_

  - [ ] 17.2 Create mock DATA_BROKER for testing
    - Add MockDataBroker to test_utils.rs
    - Implement mock that simulates signal subscription and publication
    - Support signal injection for lock/unlock events
    - Support failure injection for testing error paths
    - _Requirements: 1.1, 2.1, 2.2, 3.4, 4.4_

  - [ ] 17.3 Write integration tests for end-to-end flows
    - Create `rhivos/parking-operator-adaptor/tests/integration_test.rs`
    - Test complete session start flow: lock event → location read → API call → publish → persist
    - Test complete session stop flow: unlock event → API call → publish → persist
    - Test manual session control via gRPC StartSession/StopSession
    - Test session recovery after container restart (load from storage)
    - Test status polling during active sessions
    - Test error handling and retry behavior
    - _Requirements: 1.1-1.5, 2.1-2.4, 3.1-3.6, 4.1-4.6, 5.1-5.4, 6.1-6.4, 7.1-7.5_

- [ ] 18. Final checkpoint - Verify complete implementation
  - Run `cargo test` for all unit, property, and integration tests
  - Run `cargo clippy` for linting
  - Ensure all 15 correctness properties pass
  - Ask the user if questions arise

## Notes

- All tasks including property tests are required for comprehensive implementation
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation
- Property tests validate universal correctness properties from the design document
- The service uses `proptest` crate for property-based testing with minimum 100 iterations per test
- TLS certificates should be generated for development using `infra/certs/` tooling
- The service is containerized and managed by UPDATE_SERVICE
- Follow gitflow workflow: create feature branches from `develop` for each task
