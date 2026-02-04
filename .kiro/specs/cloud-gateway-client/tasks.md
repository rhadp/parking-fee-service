# Implementation Plan: CLOUD_GATEWAY_CLIENT

## Overview

This implementation plan covers the CLOUD_GATEWAY_CLIENT component for the SDV Parking Demo System. The service is implemented in Rust, runs in the RHIVOS safety partition (ASIL-B), and bridges cloud commands to local vehicle services via MQTT over TLS and gRPC over UDS.

Tasks are organized to build incrementally: project setup, data models, MQTT client with certificate hot-reload, command processing, telemetry publishing with offline buffering, and integration testing.

## Tasks

- [ ] 1. Set up cloud-gateway-client project structure
  - [ ] 1.1 Create Rust crate structure for cloud-gateway-client
    - Create `rhivos/cloud-gateway-client/Cargo.toml` with dependencies (rumqttc, tonic, tokio, serde, serde_json, thiserror, tracing, proptest, notify, chrono, x509-parser)
    - Create `rhivos/cloud-gateway-client/src/lib.rs` as library root
    - Create `rhivos/cloud-gateway-client/src/main.rs` as binary entry point
    - Add crate to `rhivos/Cargo.toml` workspace members
    - _Requirements: 8.1, 8.2_

  - [ ] 1.2 Create proto client bindings for LOCKING_SERVICE
    - Create `rhivos/cloud-gateway-client/build.rs` with tonic-build configuration
    - Configure proto path to `proto/services/locking_service.proto`
    - Generate client code for LockingService
    - _Requirements: 4.1, 4.2_

- [ ] 2. Implement core data models and configuration
  - [ ] 2.1 Implement Command and CommandResponse structs
    - Create `rhivos/cloud-gateway-client/src/command.rs`
    - Implement Command struct with command_id, command_type, doors, auth_token
    - Implement CommandType enum (Lock, Unlock) with serde rename
    - Implement Door enum with serde rename
    - Implement CommandResponse struct with status, error_code, error_message
    - Implement ResponseStatus enum (Success, Failed)
    - _Requirements: 2.2, 5.2, 5.3_

  - [ ] 2.2 Write property test for Command JSON round-trip
    - **Property 2: Command JSON Round-Trip**
    - Generate random valid Commands and verify serialize/deserialize produces equivalent struct
    - **Validates: Requirements 2.2**

  - [ ] 2.3 Implement Telemetry struct
    - Create `rhivos/cloud-gateway-client/src/telemetry.rs`
    - Implement Telemetry struct with timestamp, location, door_locked, door_open, parking_session_active
    - Implement Location struct with latitude, longitude
    - Implement TelemetryState for internal state tracking
    - _Requirements: 7.2_

  - [ ] 2.4 Implement ServiceConfig and MqttConfig structs
    - Create `rhivos/cloud-gateway-client/src/config.rs`
    - Implement ServiceConfig with vin, mqtt, socket paths, timeouts
    - Implement MqttConfig with broker_url, client_id, TLS paths, keepalive, reconnect params
    - Implement Default traits with values from design document
    - Implement from_env() to load from environment variables
    - _Requirements: 8.1, 8.2, 8.4_

  - [ ] 2.5 Write property test for configuration validation
    - **Property 15: Configuration Validation**
    - Generate invalid configurations and verify validation fails with descriptive errors
    - **Validates: Requirements 8.3**

  - [ ] 2.6 Implement error types
    - Create `rhivos/cloud-gateway-client/src/error.rs`
    - Implement CloudGatewayError enum
    - Implement ValidationError enum (MalformedJson, MissingField, AuthFailed, InvalidCommandType, InvalidDoor)
    - Implement ForwardError enum (ServiceUnavailable, ExecutionFailed, Timeout)
    - Implement MqttError enum (ConnectionFailed, TlsError, SubscribeFailed, PublishFailed)
    - Implement CertWatcherError enum (WatcherInitFailed, WatchPathFailed)
    - Implement CertLoadError enum (FileNotFound, PermissionDenied, InvalidFormat, ParseFailed)
    - Implement From<ValidationError> for CommandResponse
    - Implement From<ForwardError> for CommandResponse
    - _Requirements: 2.3, 2.4, 3.2, 3.3, 3.4, 4.5, 5.4, 1.7_

- [ ] 3. Checkpoint - Verify data models compile
  - Run `cargo check` in cloud-gateway-client directory
  - Ensure all types are properly defined with serde derives
  - Ask the user if questions arise

- [ ] 4. Implement command validation
  - [ ] 4.1 Implement CommandValidator struct
    - Create `rhivos/cloud-gateway-client/src/validator.rs`
    - Implement CommandValidator with valid_tokens configuration
    - Implement validate() that parses JSON and validates all fields
    - Implement validate_auth_token() that checks against valid_tokens
    - Implement validate_command_type() that accepts only "lock" or "unlock"
    - Implement validate_doors() that validates door identifiers
    - _Requirements: 2.2, 2.3, 2.4, 3.1, 3.2, 3.3, 3.4_

  - [ ] 4.2 Write property test for malformed JSON rejection
    - **Property 3: Malformed JSON Rejection**
    - Generate invalid JSON byte sequences and verify MalformedJson error
    - **Validates: Requirements 2.3**

  - [ ] 4.3 Write property test for missing required fields
    - **Property 4: Missing Required Fields Rejection**
    - Generate JSON objects missing command_id, type, or auth_token and verify MissingField error
    - **Validates: Requirements 2.4**

  - [ ] 4.4 Write property test for invalid auth token rejection
    - **Property 5: Invalid Auth Token Rejection**
    - Generate commands with tokens not in valid_tokens and verify AuthFailed error
    - **Validates: Requirements 3.2**

  - [ ] 4.5 Write property test for invalid command type rejection
    - **Property 6: Invalid Command Type Rejection**
    - Generate commands with type not "lock" or "unlock" and verify InvalidCommandType error
    - **Validates: Requirements 3.3**

  - [ ] 4.6 Write property test for invalid door rejection
    - **Property 7: Invalid Door Rejection**
    - Generate commands with invalid door identifiers and verify InvalidDoor error
    - **Validates: Requirements 3.4**

- [ ] 5. Checkpoint - Verify validation logic
  - Run `cargo test` for validation unit and property tests
  - Ensure all validation error paths work correctly
  - Ask the user if questions arise

- [ ] 6. Implement certificate watcher for hot-reload
  - [ ] 6.1 Implement CertificateWatcher struct
    - Create `rhivos/cloud-gateway-client/src/cert_watcher.rs`
    - Implement CertificatePaths struct with ca_cert_path, client_cert_path, client_key_path
    - Implement LoadedCertificates struct with cert data, expiry_date, loaded_at
    - Implement CertReloadEvent struct with timestamp, status, cert_path, expiry_date, error_message
    - Implement CertReloadStatus enum (Success, Failed)
    - Implement CertificateWatcher with notify::RecommendedWatcher
    - Implement new() that initializes file watcher for certificate paths
    - Implement start() that begins watching certificate files
    - _Requirements: 1.6, 1.7, 1.8_

  - [ ] 6.2 Implement certificate loading and validation
    - Implement load_certificate() that reads and validates certificate file
    - Implement extract_expiry_date() using x509-parser to get certificate expiry
    - Implement handle_cert_change() that processes file system notifications
    - On valid cert: update current_certs, log success with expiry date
    - On invalid cert: keep existing cert, log error
    - _Requirements: 1.6, 1.7, 1.8_

  - [ ] 6.3 Write property test for certificate hot-reload on file change
    - **Property 17: Certificate Hot-Reload on File Change**
    - Generate valid certificate updates and verify reload succeeds without restart
    - **Validates: Requirements 1.6**

  - [ ] 6.4 Write property test for certificate reload failure resilience
    - **Property 18: Certificate Reload Failure Resilience**
    - Generate invalid certificates and verify existing cert is retained
    - **Validates: Requirements 1.7**

  - [ ] 6.5 Write property test for certificate reload event logging
    - **Property 19: Certificate Reload Event Logging**
    - Generate reload events and verify log contains timestamp, status, path, expiry date
    - **Validates: Requirements 1.8**

- [ ] 7. Implement MQTT client with certificate hot-reload
  - [ ] 7.1 Implement MqttClient struct with TLS connection
    - Create `rhivos/cloud-gateway-client/src/mqtt.rs`
    - Implement MqttClient with rumqttc AsyncClient, EventLoop, and CertificateWatcher
    - Implement ConnectionState enum (Disconnected, Connecting, Connected, Reconnecting)
    - Implement new() that configures TLS from MqttConfig and initializes CertificateWatcher
    - Implement connect() that establishes MQTT connection
    - Implement subscribe() and publish() methods
    - Implement disconnect() for clean shutdown
    - Implement is_connected() for connection state checking
    - _Requirements: 1.1, 1.2, 1.5, 1.6_

  - [ ] 7.2 Implement exponential backoff reconnection
    - Implement reconnect_with_backoff() in MqttClient
    - Calculate delay as min(initial_delay * 2^attempt, max_delay)
    - Resubscribe to all topics after successful reconnection
    - _Requirements: 1.3, 1.4_

  - [ ] 7.3 Write property test for exponential backoff calculation
    - **Property 1: Exponential Backoff Calculation**
    - Generate attempt numbers and verify delay calculation matches formula
    - **Validates: Requirements 1.3**

- [ ] 8. Checkpoint - Verify MQTT client and certificate watcher
  - Run `cargo test` for MQTT and certificate watcher tests
  - Ensure TLS connection and certificate hot-reload work correctly
  - Ask the user if questions arise

- [ ] 9. Implement command forwarding
  - [ ] 9.1 Implement CommandForwarder struct
    - Create `rhivos/cloud-gateway-client/src/forwarder.rs`
    - Implement CommandForwarder with LockingServiceClient and timeout
    - Implement forward_lock() that calls LOCKING_SERVICE Lock RPC
    - Implement forward_unlock() that calls LOCKING_SERVICE Unlock RPC
    - Handle gRPC errors and map to ForwardError
    - Implement timeout handling with tokio::time::timeout
    - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5_

  - [ ] 9.2 Write property test for command forwarding by type
    - **Property 8: Command Forwarding by Type**
    - Generate valid lock/unlock commands and verify correct RPC is called
    - **Validates: Requirements 4.1, 4.2**

  - [ ] 9.3 Write property test for response status mapping
    - **Property 9: Response Status Matches LOCKING_SERVICE Result**
    - Generate LOCKING_SERVICE success/error responses and verify correct status mapping
    - **Validates: Requirements 4.3, 4.4**

- [ ] 10. Implement response publishing
  - [ ] 10.1 Implement ResponsePublisher struct
    - Create `rhivos/cloud-gateway-client/src/response.rs`
    - Implement ResponsePublisher with mqtt_client and vin
    - Implement publish_success() that publishes success response
    - Implement publish_failure() that publishes failure response with error details
    - Serialize CommandResponse to JSON and publish to vehicles/{VIN}/command_responses
    - _Requirements: 5.1, 5.2, 5.3, 5.4_

  - [ ] 10.2 Write property test for response command_id correlation
    - **Property 10: Response Command ID Correlation**
    - Generate commands and verify response contains same command_id
    - **Validates: Requirements 5.2**

  - [ ] 10.3 Write property test for response structure completeness
    - **Property 11: Response Structure Completeness**
    - Generate success/failure responses and verify required fields present
    - **Validates: Requirements 5.3, 5.4**

- [ ] 11. Implement command handler
  - [ ] 11.1 Implement CommandHandler struct
    - Create `rhivos/cloud-gateway-client/src/handler.rs`
    - Implement CommandHandler with validator, forwarder, response_publisher
    - Implement handle_message() that orchestrates command processing
    - Parse topic to extract VIN, validate command, forward to LOCKING_SERVICE, publish response
    - Implement overall command timeout (5 seconds)
    - _Requirements: 2.1, 2.2, 3.1, 4.1, 4.2, 5.1, 5.5_

  - [ ] 11.2 Write property test for command timeout enforcement
    - **Property 12: Command Timeout Enforcement**
    - Simulate slow LOCKING_SERVICE and verify timeout response published
    - **Validates: Requirements 5.5**

- [ ] 12. Checkpoint - Verify command processing
  - Run `cargo test` for command handler tests
  - Ensure end-to-end command flow works with mocks
  - Ask the user if questions arise

- [ ] 13. Implement offline telemetry buffer
  - [ ] 13.1 Implement OfflineTelemetryBuffer struct
    - Create `rhivos/cloud-gateway-client/src/offline_buffer.rs`
    - Implement BufferedTelemetry struct with telemetry and buffered_at timestamp
    - Implement OfflineTelemetryBuffer with VecDeque, max_messages (100), max_age (60s)
    - Implement new() with configurable limits
    - Implement Default trait with 100 messages / 60 seconds limits
    - _Requirements: 7.6, 7.7_

  - [ ] 13.2 Implement buffer operations with FIFO eviction
    - Implement push() that adds telemetry, evicts expired, evicts oldest if full (FIFO)
    - Implement drain() that returns all messages in chronological order and clears buffer
    - Implement evict_expired() that removes messages older than max_age
    - Implement len() and is_empty() helper methods
    - _Requirements: 7.6, 7.7, 7.8_

  - [ ] 13.3 Write property test for offline buffer limits
    - **Property 20: Offline Telemetry Buffer Limits**
    - Generate sequences of messages and verify buffer never exceeds 100 messages or 60 seconds
    - **Validates: Requirements 7.6**

  - [ ] 13.4 Write property test for FIFO eviction
    - **Property 21: Offline Buffer FIFO Eviction**
    - Generate overflow scenarios and verify oldest messages are evicted first
    - **Validates: Requirements 7.7**

  - [ ] 13.5 Write property test for chronological publishing
    - **Property 22: Buffered Message Chronological Publishing**
    - Generate buffered messages and verify drain returns them in chronological order
    - **Validates: Requirements 7.8**

- [ ] 14. Implement telemetry subscription and publishing
  - [ ] 14.1 Implement SignalSubscriber struct
    - Create `rhivos/cloud-gateway-client/src/subscriber.rs`
    - Implement SignalSubscriber with DataBrokerClient and signal channel
    - Implement subscribe_all() that subscribes to all required VSS signals
    - Implement run() that receives signal updates and sends to channel
    - Handle DATA_BROKER disconnection gracefully
    - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 6.6_

  - [ ] 14.2 Implement TelemetryPublisher struct with offline buffering
    - Update `rhivos/cloud-gateway-client/src/telemetry.rs`
    - Implement TelemetryPublisher with mqtt_client, vin, signal_rx, current_state, offline_buffer
    - Implement run() that batches signal updates and publishes at configured interval
    - Implement publish_or_buffer() that publishes if connected, buffers if offline
    - Implement drain_offline_buffer() that publishes buffered messages in chronological order
    - Stop publishing when DATA_BROKER disconnected, resume when reconnected
    - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5, 7.6, 7.7, 7.8_

  - [ ] 14.3 Write property test for telemetry field completeness
    - **Property 13: Telemetry Contains All Required Fields**
    - Generate telemetry messages and verify all required fields present
    - **Validates: Requirements 7.2**

  - [ ] 14.4 Write property test for telemetry rate limiting
    - **Property 14: Telemetry Rate Limiting**
    - Simulate rapid signal updates and verify publish rate is bounded
    - **Validates: Requirements 7.3**

- [ ] 15. Checkpoint - Verify telemetry with offline buffering
  - Run `cargo test` for telemetry and offline buffer tests
  - Ensure buffering and draining work correctly
  - Ask the user if questions arise

- [ ] 16. Implement logging
  - [ ] 16.1 Implement structured logging
    - Create `rhivos/cloud-gateway-client/src/logging.rs`
    - Implement LogEntry struct with timestamp, level, command_id, correlation_id, event_type, details
    - Implement EventType enum for all loggable events including CertReloadSuccess, CertReloadFailed, TelemetryBuffered, TelemetryBufferDrained
    - Configure tracing subscriber for structured JSON output
    - _Requirements: 10.1, 10.2, 10.3, 10.4, 10.5, 1.8_

- [ ] 17. Implement main service and graceful shutdown
  - [ ] 17.1 Implement service startup and main loop
    - Update `rhivos/cloud-gateway-client/src/main.rs`
    - Load configuration from environment
    - Initialize MQTT client with CertificateWatcher and connect with TLS
    - Initialize LOCKING_SERVICE gRPC client
    - Initialize DATA_BROKER gRPC client
    - Subscribe to command topic
    - Spawn command handler task
    - Spawn telemetry publisher task with offline buffer
    - _Requirements: 1.1, 1.6, 2.1, 6.1, 8.1, 8.3_

  - [ ] 17.2 Implement graceful shutdown handler
    - Register SIGTERM signal handler
    - Track in-flight command count
    - Wait for in-flight operations to complete (up to 10 seconds)
    - Disconnect MQTT cleanly
    - _Requirements: 9.1, 9.2, 9.3, 9.4_

  - [ ] 17.3 Write property test for shutdown timeout enforcement
    - **Property 16: Shutdown Timeout Enforcement**
    - Simulate slow in-flight operations and verify shutdown completes within 10 seconds
    - **Validates: Requirements 9.4**

- [ ] 18. Checkpoint - Verify service startup and shutdown
  - Run `cargo test` for all tests
  - Verify service starts, connects, and shuts down cleanly
  - Ask the user if questions arise

- [ ] 19. Integration testing
  - [ ] 19.1 Create mock LOCKING_SERVICE for testing
    - Create `rhivos/cloud-gateway-client/src/test_utils.rs`
    - Implement MockLockingService that simulates Lock/Unlock responses
    - Support configurable success/failure responses
    - Support delay injection for timeout testing
    - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5_

  - [ ] 19.2 Create mock DATA_BROKER for testing
    - Add MockDataBrokerClient to test_utils.rs
    - Implement signal subscription simulation
    - Support configurable signal updates
    - Support disconnection simulation
    - _Requirements: 6.1, 7.4, 7.5_

  - [ ] 19.3 Create mock certificate files for testing
    - Add certificate generation utilities to test_utils.rs
    - Support creating valid and invalid test certificates
    - Support simulating file system changes for hot-reload testing
    - _Requirements: 1.6, 1.7, 1.8_

  - [ ] 19.4 Write integration tests for command flow
    - Test complete flow: MQTT receive → validate → forward → respond
    - Test validation error responses
    - Test LOCKING_SERVICE error propagation
    - Test timeout handling
    - _Requirements: 2.1-2.4, 3.1-3.4, 4.1-4.5, 5.1-5.5_

  - [ ] 19.5 Write integration tests for telemetry flow with offline buffering
    - Test signal subscription and telemetry publishing
    - Test rate limiting behavior
    - Test DATA_BROKER disconnection handling
    - Test offline buffering when MQTT disconnected
    - Test buffer draining on MQTT reconnection
    - Test chronological order of buffered messages
    - _Requirements: 6.1-6.6, 7.1-7.8_

  - [ ] 19.6 Write integration tests for certificate hot-reload
    - Test certificate reload on file change
    - Test continued operation with invalid certificate update
    - Test logging of reload events
    - _Requirements: 1.6, 1.7, 1.8_

- [ ] 20. Final checkpoint - Verify complete implementation
  - Run `cargo test` for all unit and property tests
  - Run `cargo clippy` for linting
  - Ensure all 22 properties pass
  - Ask the user if questions arise

## Notes

- All tasks including property tests are required for comprehensive implementation
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation
- Property tests validate universal correctness properties from the design document
- The service uses `proptest` crate for property-based testing with minimum 100 iterations per test
- MQTT client uses `rumqttc` crate with TLS support
- gRPC clients use `tonic` crate with UDS transport
- Certificate watching uses `notify` crate for file system notifications
- Certificate parsing uses `x509-parser` crate for expiry date extraction
- Offline buffer uses `VecDeque` for efficient FIFO operations
