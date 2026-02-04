# Implementation Plan: UPDATE_SERVICE

## Overview

This implementation plan covers the UPDATE_SERVICE component for the SDV Parking Demo System. The service is implemented in Rust, runs in the RHIVOS QM partition, and manages containerized parking operator adapter lifecycle via gRPC over TCP/TLS.

Tasks are organized to build incrementally: project setup, core data models, registry authentication, image downloading, attestation validation, container management, state tracking, watcher streaming, gRPC server, offload scheduling, operation logging, and integration testing.

## Tasks

- [ ] 1. Set up update-service project structure
  - [ ] 1.1 Create Rust crate structure for update-service
    - Create `rhivos/update-service/Cargo.toml` with dependencies (tonic, tokio, reqwest, sha2, serde, thiserror, tracing, tracing-subscriber, proptest, uuid)
    - Create `rhivos/update-service/src/lib.rs` as library root
    - Create `rhivos/update-service/src/main.rs` as binary entry point
    - Add crate to `rhivos/Cargo.toml` workspace members
    - _Requirements: 10.1, 10.2, 10.3, 10.4_

  - [ ] 1.2 Generate Rust bindings from update_service.proto
    - Create `rhivos/update-service/build.rs` with tonic-build configuration
    - Create `proto/services/update_service.proto` with service definition from design
    - Configure proto path and generate server/client code
    - _Requirements: 10.1, 10.2, 10.3, 10.4_

- [ ] 2. Implement core data models and configuration
  - [ ] 2.1 Implement AdapterState enum and AdapterEntry struct
    - Create `rhivos/update-service/src/state.rs`
    - Implement AdapterState enum (Unknown, Downloading, Installing, Running, Stopped, Error)
    - Implement AdapterEntry with adapter_id, state, error_message, last_updated, last_activity
    - _Requirements: 5.1, 5.2, 5.3_

  - [ ] 2.2 Implement ServiceConfig struct
    - Create `rhivos/update-service/src/config.rs`
    - Define listen_addr, TLS paths, storage_path, data_broker_socket, retry settings, offload settings
    - Add registry_username, registry_password, token_refresh_buffer_secs, log_level fields
    - Implement Default trait with values from design document
    - Add environment variable loading support (REGISTRY_USERNAME, REGISTRY_PASSWORD)
    - _Requirements: 4.5, 9.4, 10.5, 11.4_

  - [ ] 2.3 Implement error types
    - Create `rhivos/update-service/src/error.rs`
    - Implement UpdateError enum with all variants from design (including AuthenticationFailed, TokenEndpointUnreachable, InvalidCredentials)
    - Implement From<UpdateError> for tonic::Status with proper gRPC status codes
    - _Requirements: 1.5, 2.5, 3.2, 3.4, 4.6, 8.4, 11.6_

- [ ] 3. Checkpoint - Verify data models compile
  - Run `cargo check` in update-service directory
  - Ensure all types are properly defined
  - Ask the user if questions arise

- [ ] 4. Implement operation logging
  - [ ] 4.1 Implement OperationLogger struct
    - Create `rhivos/update-service/src/logger.rs`
    - Implement OperationLogger with service_name field
    - Implement log_request() for incoming requests with correlation ID
    - Implement log_state_transition() for state changes with previous/new state and reason
    - Implement log_container_operation() for container ops (pull, install, start, stop, remove) with outcome
    - Implement log_auth_event() for authentication events
    - Define ContainerOperation, OperationOutcome, AuthEvent, and LogEntry structs
    - Configure tracing subscriber for structured JSON output
    - _Requirements: 12.1, 12.2, 12.3, 12.4_

  - [ ] 4.2 Write property test for request logging
    - **Property 22: Request Logging with Correlation ID**
    - Verify all requests are logged with timestamp, request type, adapter_id, and correlation ID
    - **Validates: Requirements 12.1, 12.4**

  - [ ] 4.3 Write property test for state transition logging
    - **Property 23: State Transition Logging**
    - Verify state transitions are logged with previous state, new state, reason, and correlation ID
    - **Validates: Requirements 12.2, 12.4**

  - [ ] 4.4 Write property test for container operation logging
    - **Property 24: Container Operation Logging**
    - Verify container operations are logged with outcome and correlation ID
    - **Validates: Requirements 12.3, 12.4**

- [ ] 5. Implement registry authentication
  - [ ] 5.1 Implement RegistryAuthenticator struct
    - Create `rhivos/update-service/src/authenticator.rs`
    - Implement RegistryAuthenticator with http_client, credentials, token_cache
    - Implement RegistryCredentials and CachedToken structs
    - Implement from_env() to load credentials from REGISTRY_USERNAME/REGISTRY_PASSWORD
    - Implement get_token() to return cached token or fetch new one
    - Implement fetch_token() to handle 401 challenge and obtain Bearer token from /v2/token endpoint
    - Implement is_token_valid() to check token expiration
    - Implement AuthError enum (AuthenticationFailed, TokenEndpointUnreachable, InvalidCredentials)
    - _Requirements: 11.1, 11.2, 11.3, 11.4, 11.5, 11.6, 11.7_

  - [ ] 5.2 Write property test for Bearer token authentication
    - **Property 18: Bearer Token Authentication on 401 Challenge**
    - Verify 401 response triggers token fetch from /v2/token and retry with Authorization header
    - **Validates: Requirements 11.1, 11.2, 11.3**

  - [ ] 5.3 Write property test for token caching
    - **Property 19: Token Caching and Refresh**
    - Verify tokens are cached and reused, refreshed before expiration
    - **Validates: Requirements 11.5**

  - [ ] 5.4 Write property test for authentication failure
    - **Property 20: Authentication Failure Transitions to Error**
    - Verify auth failure transitions adapter state to ERROR with appropriate message
    - **Validates: Requirements 11.6**

  - [ ] 5.5 Write property test for anonymous access
    - **Property 21: Anonymous Access for Public Registries**
    - Verify anonymous access works when no credentials configured
    - **Validates: Requirements 11.7**

- [ ] 6. Checkpoint - Verify logging and authentication
  - Run `cargo test` for unit tests
  - Ensure logger and authenticator work correctly
  - Ask the user if questions arise


- [ ] 7. Implement image downloading
  - [ ] 7.1 Implement ImageDownloader struct
    - Create `rhivos/update-service/src/downloader.rs`
    - Implement ImageDownloader with http_client, authenticator, max_retries, base_delay, logger
    - Implement download() method with OCI registry protocol support
    - Implement authenticated_request() to handle 401 challenges via authenticator
    - Implement exponential backoff retry logic
    - Return DownloadedImage with manifest_path, layers_dir, config_path
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5_

  - [ ] 7.2 Write property test for download retry behavior
    - **Property 5: Download Retry and Failure Handling**
    - Inject network failures and verify retry count and final ERROR state
    - **Validates: Requirements 2.4, 2.5**

- [ ] 8. Implement attestation validation
  - [ ] 8.1 Implement AttestationValidator struct
    - Create `rhivos/update-service/src/attestation.rs`
    - Implement Attestation, AttestationPayload, AttestationSubject, AttestationSignature structs
    - Implement AttestationValidator with http_client and authenticator
    - Implement fetch_attestation() to retrieve attestation from registry for image digest
    - Implement validate() to verify attestation signature and subject digest matches image
    - Implement validate_structure() to check required fields (subject digest, predicate type, signature)
    - _Requirements: 3.1, 3.2, 3.3, 3.4_

  - [ ] 8.2 Write property test for attestation verification
    - **Property 7: Attestation Verification**
    - Generate mismatched digests and verify rejection with ERROR state and content deletion
    - **Validates: Requirements 3.1, 3.2**

  - [ ] 8.3 Write property test for attestation structure validation
    - **Property 8: Attestation Structure Validation**
    - Generate attestations with missing fields and verify ERROR state
    - **Validates: Requirements 3.3, 3.4**

- [ ] 9. Implement container management
  - [ ] 9.1 Implement ContainerManager struct
    - Create `rhivos/update-service/src/container.rs`
    - Implement ContainerManager with storage_path and data_broker_socket
    - Implement install() to load image into podman
    - Implement start() to run container with network access to DATA_BROKER
    - Implement stop() to stop running container
    - Implement remove() to delete container and storage
    - Implement list_running() to query podman for running containers
    - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 5.4, 8.1, 8.2_

  - [ ] 9.2 Write property test for container startup failure handling
    - **Property 9: Container Startup Failure Handling**
    - Inject startup failures and verify ERROR state with failure reason
    - **Validates: Requirements 4.6**

- [ ] 10. Checkpoint - Verify core components
  - Run `cargo test` for unit tests
  - Ensure downloader, attestation validator, and container manager work correctly
  - Ask the user if questions arise

- [ ] 11. Implement state tracking
  - [ ] 11.1 Implement StateTracker struct
    - Create `rhivos/update-service/src/tracker.rs`
    - Implement StateTracker with adapters HashMap and watcher_manager reference
    - Implement get_state() to retrieve current adapter state
    - Implement transition() to change state and notify watchers
    - Implement list_all() to return all adapter info
    - Implement remove() to delete adapter from tracking
    - Implement restore_from_containers() to recover state on startup
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 7.1, 7.2, 7.3_

  - [ ] 11.2 Write property test for state timestamp updates
    - **Property 10: State Timestamp Updates**
    - Verify last_updated is updated on every state change
    - **Validates: Requirements 5.2, 5.3**

  - [ ] 11.3 Write property test for list adapters completeness
    - **Property 14: List Adapters Returns Complete Info**
    - Install multiple adapters and verify list contains all with complete fields
    - **Validates: Requirements 7.1, 7.2**

- [ ] 12. Implement watcher management
  - [ ] 12.1 Implement WatcherManager struct
    - Create `rhivos/update-service/src/watcher.rs`
    - Implement WatcherManager with watchers Vec of mpsc::Sender
    - Implement register() to add new watcher and return receiver
    - Implement broadcast() to send event to all active watchers
    - Implement cleanup_disconnected() to remove closed channels
    - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5_

  - [ ] 12.2 Write property test for watcher event reception
    - **Property 11: Watcher Receives State Events**
    - Register watcher, trigger state change, verify event received with all fields
    - **Validates: Requirements 6.1, 6.2, 6.3**

  - [ ] 12.3 Write property test for watcher cleanup
    - **Property 12: Watcher Cleanup on Disconnect**
    - Disconnect one watcher, verify others still receive events
    - **Validates: Requirements 6.4**

  - [ ] 12.4 Write property test for initial state emission
    - **Property 13: New Watcher Receives Initial State**
    - Install adapters, connect new watcher, verify initial state events received
    - **Validates: Requirements 6.5**

- [ ] 13. Implement gRPC service
  - [ ] 13.1 Implement UpdateServiceImpl struct
    - Create `rhivos/update-service/src/service.rs`
    - Implement UpdateServiceImpl with all components wired together
    - Implement generate_correlation_id() helper for request tracing
    - Implement helper methods for install workflow orchestration
    - _Requirements: 1.1, 1.2, 10.1, 10.2, 10.3, 10.4, 12.4_

  - [ ] 13.2 Implement InstallAdapter RPC handler
    - Generate correlation ID and log request
    - Check for existing adapter (idempotence for RUNNING/in-progress)
    - Validate registry_url format
    - Initialize state as DOWNLOADING with state transition logging
    - Spawn async task for download → validate attestation → install → start workflow
    - Log all container operations with outcomes
    - Return InstallAdapterResponse with initial state
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 2.2, 2.3, 4.2, 4.3, 12.1, 12.2, 12.3_

  - [ ] 13.3 Write property test for install state initialization
    - **Property 1: Install Initiates Download State**
    - Verify valid install request returns DOWNLOADING state
    - **Validates: Requirements 1.1, 1.2**

  - [ ] 13.4 Write property test for install idempotence (running)
    - **Property 2: Install Idempotence for Running Adapters**
    - Install already-running adapter, verify no re-download
    - **Validates: Requirements 1.3**

  - [ ] 13.5 Write property test for install idempotence (in-progress)
    - **Property 3: Install Idempotence for In-Progress Adapters**
    - Install while DOWNLOADING/INSTALLING, verify current state returned
    - **Validates: Requirements 1.4**

  - [ ] 13.6 Write property test for invalid registry URL
    - **Property 4: Invalid Registry URL Returns Error**
    - Provide malformed URLs and verify error response
    - **Validates: Requirements 1.5**

  - [ ] 13.7 Implement UninstallAdapter RPC handler
    - Generate correlation ID and log request
    - Check adapter exists
    - Stop container if running, log operation
    - Remove container and storage, log operation
    - Remove from state tracker with state transition logging
    - Emit state change event
    - Return UninstallAdapterResponse
    - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5, 12.1, 12.2, 12.3_

  - [ ] 13.8 Write property test for uninstall completeness
    - **Property 15: Uninstall Removes Adapter Completely**
    - Verify container stopped, removed, state cleared, event emitted
    - **Validates: Requirements 8.1, 8.2, 8.3, 8.5**

  - [ ] 13.9 Write property test for uninstall non-existent
    - **Property 16: Uninstall Non-Existent Returns Error**
    - Uninstall unknown adapter_id, verify NOT_FOUND error
    - **Validates: Requirements 8.4**

  - [ ] 13.10 Implement ListAdapters RPC handler
    - Generate correlation ID and log request
    - Return all adapters from state tracker with complete info
    - _Requirements: 7.1, 7.2, 7.3, 12.1_

  - [ ] 13.11 Implement WatchAdapterStates RPC handler
    - Generate correlation ID and log request
    - Register watcher with WatcherManager
    - Emit initial state for all adapters
    - Stream AdapterStateEvent messages
    - Handle client disconnect gracefully
    - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 12.1_

  - [ ] 13.12 Write property test for state progression
    - **Property 6: State Progression Through Installation**
    - Verify DOWNLOADING → INSTALLING → RUNNING progression
    - **Validates: Requirements 2.3, 4.2, 4.3**

- [ ] 14. Checkpoint - Verify gRPC service
  - Run `cargo test` for all tests
  - Verify service compiles and handlers are wired correctly
  - Ask the user if questions arise

- [ ] 15. Implement gRPC server startup with TLS
  - [ ] 15.1 Implement TLS server listener
    - Update `rhivos/update-service/src/main.rs`
    - Load TLS certificate and key from configured paths
    - Create TCP listener on configured address
    - Initialize all components (authenticator, downloader, attestation_validator, container_manager, state_tracker, watcher_manager, logger)
    - Restore state from running containers on startup
    - Start tonic gRPC server with TLS and UpdateServiceImpl
    - Handle graceful shutdown on SIGTERM
    - _Requirements: 5.4, 10.5_

- [ ] 16. Implement offload scheduler
  - [ ] 16.1 Implement OffloadScheduler struct
    - Create `rhivos/update-service/src/offload.rs`
    - Implement OffloadScheduler with state_tracker, container_manager, offload_threshold, check_interval, logger
    - Implement start() to spawn background task
    - Implement check_and_offload() to find and remove inactive adapters
    - Track last_activity timestamp for each adapter
    - Log offload operations
    - _Requirements: 9.1, 9.2, 9.3, 9.4, 12.3_

  - [ ] 16.2 Write property test for automatic offload
    - **Property 17: Automatic Offload After Inactivity**
    - Simulate 24+ hours of STOPPED state, verify automatic uninstall and event
    - **Validates: Requirements 9.1, 9.2, 9.3**

- [ ] 17. Checkpoint - Verify complete service
  - Run `cargo test` for all unit and property tests
  - Run `cargo clippy` for linting
  - Verify service starts with TLS and accepts connections
  - Ask the user if questions arise

- [ ] 18. Integration testing
  - [ ] 18.1 Create mock registry for testing
    - Create `rhivos/update-service/src/test_utils.rs`
    - Implement MockRegistry that serves OCI manifests, layers, and attestations
    - Support configurable responses for success/failure scenarios
    - Support 401 challenge responses for auth testing
    - _Requirements: 2.1, 3.1, 11.1, 11.2_

  - [ ] 18.2 Create mock container manager for testing
    - Implement MockContainerManager that simulates podman operations
    - Support failure injection for testing error paths
    - _Requirements: 4.1, 4.6_

  - [ ] 18.3 Write integration tests for end-to-end flows
    - Test complete install flow: request → authenticate → download → validate attestation → install → start → RUNNING
    - Test uninstall flow with running container
    - Test watcher streaming over multiple state changes
    - Test state restoration on service restart
    - Test offload scheduler timing
    - Test authentication flow with 401 challenge
    - Test anonymous access for public registries
    - Verify correlation IDs flow through all log entries
    - _Requirements: 1.1-1.5, 2.1-2.5, 3.1-3.4, 4.1-4.6, 5.4, 6.1-6.5, 8.1-8.5, 9.1-9.3, 11.1-11.7, 12.1-12.4_

- [ ] 19. Final checkpoint - Verify complete implementation
  - Run `cargo test` for all unit, property, and integration tests
  - Run `cargo clippy` for linting
  - Ensure all 24 properties pass
  - Ask the user if questions arise

## Notes

- All tasks including property tests are required for comprehensive implementation
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation
- Property tests validate universal correctness properties from the design document
- The service uses `proptest` crate for property-based testing with minimum 100 iterations per test
- TLS certificates should be generated for development using `infra/certs/` tooling
- Test organization follows design document structure:
  - `tests/property/install_properties.rs` - Properties 1-4
  - `tests/property/download_properties.rs` - Properties 5-6
  - `tests/property/attestation_properties.rs` - Properties 7-8
  - `tests/property/container_properties.rs` - Property 9
  - `tests/property/state_properties.rs` - Properties 10, 14
  - `tests/property/watcher_properties.rs` - Properties 11-13
  - `tests/property/uninstall_properties.rs` - Properties 15-16
  - `tests/property/offload_properties.rs` - Property 17
  - `tests/property/auth_properties.rs` - Properties 18-21
  - `tests/property/logging_properties.rs` - Properties 22-24
