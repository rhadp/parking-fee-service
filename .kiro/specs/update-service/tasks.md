# Implementation Plan: UPDATE_SERVICE

## Overview

This implementation plan covers the UPDATE_SERVICE component for the SDV Parking Demo System. The service is implemented in Rust, runs in the RHIVOS QM partition, and manages containerized parking operator adapter lifecycle via gRPC over TCP/TLS.

Tasks are organized to build incrementally: project setup, core data models, operation logging, registry authentication, image downloading, attestation validation, container management, state tracking, watcher streaming, gRPC service, offload scheduling, and integration testing.

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
    - Implement UpdateError enum with all variants from design (DownloadError, ValidationError, ContainerError, AdapterNotFound, AdapterAlreadyExists, RegistryUnavailable, InvalidRegistryUrl, AttestationDigestMismatch, InvalidAttestationSignature, MissingAttestationField, AttestationNotFound, AuthenticationFailed, TokenEndpointUnreachable, InvalidCredentials)
    - Implement From<UpdateError> for tonic::Status with proper gRPC status codes per design
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
    - Define ContainerOperation, OperationOutcome, AuthEvent, and LogEntry structs per design
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
    - Implement exponential backoff retry logic (max 3 retries, base delay from config, max delay 30s)
    - Return DownloadedImage with manifest_path, layers_dir, config_path
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5_

  - [ ] 7.2 Write property test for download retry behavior
    - **Property 5: Download Retry and Failure Handling**
    - Inject network failures and verify retry count (up to 3) and final ERROR state with failure message
    - **Validates: Requirements 2.4, 2.5**

- [ ] 8. Implement attestation validation
  - [ ] 8.1 Implement AttestationValidator struct
    - Create `rhivos/update-service/src/attestation.rs`
    - Implement Attestation, AttestationPayload, AttestationSubject, AttestationSignature structs per design
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
    - Generate attestations with missing fields and verify ERROR state with message indicating missing field
    - **Validates: Requirements 3.3, 3.4**

- [ ] 9. Implement container management
  - [ ] 9.1 Implement ContainerManager struct
    - Create `rhivos/update-service/src/container.rs`
    - Implement ContainerManager with storage_path (/var/lib/containers/adapters/) and data_broker_socket
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
    - Implement transition() to change state, update timestamp, and notify watchers
    - Implement list_all() to return all adapter info
    - Implement remove() to delete adapter from tracking
    - Implement restore_from_containers() to recover state on startup by querying podman
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 7.1, 7.2, 7.3_

  - [ ] 11.2 Write property test for state timestamp updates
    - **Property 10: State Timestamp Updates**
    - Verify last_updated is updated on every state change with adapter_id, current_state, error_message, and last_updated
    - **Validates: Requirements 5.2, 5.3**

  - [ ] 11.3 Write property test for list adapters completeness
    - **Property 14: List Adapters Returns Complete Info**
    - Install multiple adapters and verify list contains all with adapter_id, state, error_message, and last_updated fields
    - **Validates: Requirements 7.1, 7.2**

- [ ] 12. Implement watcher management
  - [ ] 12.1 Implement WatcherManager struct
    - Create `rhivos/update-service/src/watcher.rs`
    - Implement WatcherManager with watchers Vec of mpsc::Sender
    - Implement register() to add new watcher and return receiver
    - Implement broadcast() to send AdapterStateEvent to all active watchers
    - Implement cleanup_disconnected() to remove closed channels
    - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5_

  - [ ] 12.2 Write property test for watcher event reception
    - **Property 11: Watcher Receives State Events**
    - Register watcher, trigger state change, verify event received with adapter_id, previous_state, new_state, timestamp, and error_message
    - **Validates: Requirements 6.1, 6.2, 6.3**

  - [ ] 12.3 Write property test for watcher cleanup
    - **Property 12: Watcher Cleanup on Disconnect**
    - Disconnect one watcher, verify others still receive events without interruption
    - **Validates: Requirements 6.4**

  - [ ] 12.4 Write property test for initial state emission
    - **Property 13: New Watcher Receives Initial State**
    - Install adapters, connect new watcher, verify initial state events received for all tracked adapters
    - **Validates: Requirements 6.5**

- [ ] 13. Implement gRPC service
  - [ ] 13.1 Implement UpdateServiceImpl struct
    - Create `rhivos/update-service/src/service.rs`
    - Implement UpdateServiceImpl with all components wired together (state_tracker, image_downloader, attestation_validator, container_manager, watcher_manager, config, logger)
    - Implement generate_correlation_id() helper for request tracing using UUID
    - Implement helper methods for install workflow orchestration
    - _Requirements: 1.1, 1.2, 10.1, 10.2, 10.3, 10.4, 12.4_

  - [ ] 13.2 Implement InstallAdapter RPC handler
    - Generate correlation ID and log request
    - Check for existing adapter (return success for RUNNING, return current state for DOWNLOADING/INSTALLING)
    - Validate registry_url format (return error for malformed URLs)
    - Initialize state as DOWNLOADING with state transition logging
    - Spawn async task for download → validate attestation → install → start workflow
    - Log all container operations with outcomes
    - Return InstallAdapterResponse with initial state
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 2.2, 2.3, 4.2, 4.3, 12.1, 12.2, 12.3_

  - [ ] 13.3 Write property test for install state initialization
    - **Property 1: Install Initiates Download State**
    - Verify valid install request returns DOWNLOADING state and adapter is tracked
    - **Validates: Requirements 1.1, 1.2**

  - [ ] 13.4 Write property test for install idempotence (running)
    - **Property 2: Install Idempotence for Running Adapters**
    - Install already-running adapter, verify success returned without re-download, state remains RUNNING
    - **Validates: Requirements 1.3**

  - [ ] 13.5 Write property test for install idempotence (in-progress)
    - **Property 3: Install Idempotence for In-Progress Adapters**
    - Install while DOWNLOADING/INSTALLING, verify current state returned without duplicate installation
    - **Validates: Requirements 1.4**

  - [ ] 13.6 Write property test for invalid registry URL
    - **Property 4: Invalid Registry URL Returns Error**
    - Provide malformed URLs and verify error response, no adapter entry created
    - **Validates: Requirements 1.5**

  - [ ] 13.7 Write property test for state progression
    - **Property 6: State Progression Through Installation**
    - Verify DOWNLOADING → INSTALLING → RUNNING progression with each transition recorded
    - **Validates: Requirements 2.3, 4.2, 4.3**

  - [ ] 13.8 Implement UninstallAdapter RPC handler
    - Generate correlation ID and log request
    - Check adapter exists (return NOT_FOUND error if not)
    - Stop container if running, log operation
    - Remove container and storage, log operation
    - Remove from state tracker with state transition logging
    - Emit state change event to all watchers
    - Return UninstallAdapterResponse
    - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5, 12.1, 12.2, 12.3_

  - [ ] 13.9 Write property test for uninstall completeness
    - **Property 15: Uninstall Removes Adapter Completely**
    - Verify container stopped, removed, state cleared, event emitted to watchers
    - **Validates: Requirements 8.1, 8.2, 8.3, 8.5**

  - [ ] 13.10 Write property test for uninstall non-existent
    - **Property 16: Uninstall Non-Existent Returns Error**
    - Uninstall unknown adapter_id, verify NOT_FOUND error returned
    - **Validates: Requirements 8.4**

  - [ ] 13.11 Implement ListAdapters RPC handler
    - Generate correlation ID and log request
    - Return all adapters from state tracker with adapter_id, state, error_message, and last_updated
    - Return empty list when no adapters installed
    - _Requirements: 7.1, 7.2, 7.3, 12.1_

  - [ ] 13.12 Implement WatchAdapterStates RPC handler
    - Generate correlation ID and log request
    - Register watcher with WatcherManager
    - Emit initial state for all adapters as AdapterStateEvent messages
    - Stream AdapterStateEvent messages on state changes
    - Handle client disconnect gracefully with cleanup
    - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 12.1_

- [ ] 14. Checkpoint - Verify gRPC service
  - Run `cargo test` for all tests
  - Verify service compiles and handlers are wired correctly
  - Ask the user if questions arise

- [ ] 15. Implement gRPC server startup with TLS
  - [ ] 15.1 Implement TLS server listener
    - Update `rhivos/update-service/src/main.rs`
    - Load TLS certificate and key from configured paths
    - Create TCP listener on configured address (default 0.0.0.0:50052)
    - Initialize all components (authenticator, downloader, attestation_validator, container_manager, state_tracker, watcher_manager, logger)
    - Restore state from running containers on startup via restore_from_containers()
    - Start tonic gRPC server with TLS and UpdateServiceImpl
    - Handle graceful shutdown on SIGTERM
    - _Requirements: 5.4, 10.5, 10.6_

- [ ] 16. Implement offload scheduler
  - [ ] 16.1 Implement OffloadScheduler struct
    - Create `rhivos/update-service/src/offload.rs`
    - Implement OffloadScheduler with state_tracker, container_manager, offload_threshold (24 hours), check_interval (1 hour), logger
    - Implement start() to spawn background task
    - Implement check_and_offload() to find adapters in STOPPED state for more than 24 hours and remove them
    - Track last_activity timestamp for each adapter
    - Emit state change event indicating automatic removal
    - Log offload operations
    - _Requirements: 9.1, 9.2, 9.3, 9.4, 12.3_

  - [ ] 16.2 Write property test for automatic offload
    - **Property 17: Automatic Offload After Inactivity**
    - Simulate 24+ hours of STOPPED state, verify automatic uninstall and state change event emitted
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

- All tasks are required for comprehensive implementation per steering rules
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation
- Property tests validate universal correctness properties from the design document (24 total)
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
