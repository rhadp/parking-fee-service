# Implementation Plan: UPDATE_SERVICE

## Overview

This implementation plan covers the UPDATE_SERVICE component for the SDV Parking Demo System. The service is implemented in Rust, runs in the RHIVOS QM partition, and manages containerized parking operator adapter lifecycle via gRPC over TCP/TLS.

Tasks are organized to build incrementally: project setup, core data models, image downloading, manifest validation, container management, state tracking, watcher streaming, gRPC server, offload scheduling, and integration testing.

## Tasks

- [ ] 1. Set up update-service project structure
  - [ ] 1.1 Create Rust crate structure for update-service
    - Create `rhivos/update-service/Cargo.toml` with dependencies (tonic, tokio, reqwest, sha2, serde, thiserror, tracing, proptest)
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
    - Implement Default trait with values from design document
    - Add environment variable loading support
    - _Requirements: 4.5, 9.4, 10.5_

  - [ ] 2.3 Implement error types
    - Create `rhivos/update-service/src/error.rs`
    - Implement UpdateError enum with all variants from design
    - Implement From<UpdateError> for tonic::Status with proper gRPC status codes
    - _Requirements: 1.5, 2.5, 3.2, 3.4, 4.6, 8.4_

- [ ] 3. Checkpoint - Verify data models compile
  - Run `cargo check` in update-service directory
  - Ensure all types are properly defined
  - Ask the user if questions arise

- [ ] 4. Implement image downloading
  - [ ] 4.1 Implement ImageDownloader struct
    - Create `rhivos/update-service/src/downloader.rs`
    - Implement ImageDownloader with http_client (reqwest), max_retries, base_delay
    - Implement download() method with OCI registry protocol support
    - Implement exponential backoff retry logic
    - Return DownloadedImage with manifest_path, layers_dir, config_path
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5_

  - [ ] 4.2 Write property test for download retry behavior
    - **Property 5: Download Retry and Failure Handling**
    - Inject network failures and verify retry count and final ERROR state
    - **Validates: Requirements 2.4, 2.5**

- [ ] 5. Implement manifest validation
  - [ ] 5.1 Implement ManifestValidator struct
    - Create `rhivos/update-service/src/validator.rs`
    - Implement OciManifest and OciDescriptor structs for deserialization
    - Implement validate_checksum() using sha2 crate for SHA256 verification
    - Implement validate_structure() to check required fields (config, layers, mediaType)
    - _Requirements: 3.1, 3.2, 3.3, 3.4_

  - [ ] 5.2 Write property test for checksum validation
    - **Property 7: Checksum Validation**
    - Generate mismatched checksums and verify rejection with ERROR state
    - **Validates: Requirements 3.1, 3.2**

  - [ ] 5.3 Write property test for manifest structure validation
    - **Property 8: Manifest Structure Validation**
    - Generate manifests with missing fields and verify ERROR state
    - **Validates: Requirements 3.3, 3.4**

- [ ] 6. Implement container management
  - [ ] 6.1 Implement ContainerManager struct
    - Create `rhivos/update-service/src/container.rs`
    - Implement ContainerManager with storage_path and data_broker_socket
    - Implement install() to load image into podman
    - Implement start() to run container with network access to DATA_BROKER
    - Implement stop() to stop running container
    - Implement remove() to delete container and storage
    - Implement list_running() to query podman for running containers
    - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 5.4, 8.1, 8.2_

  - [ ] 6.2 Write property test for container startup failure handling
    - **Property 9: Container Startup Failure Handling**
    - Inject startup failures and verify ERROR state with failure reason
    - **Validates: Requirements 4.6**

- [ ] 7. Checkpoint - Verify core components
  - Run `cargo test` for unit tests
  - Ensure downloader, validator, and container manager work correctly
  - Ask the user if questions arise

- [ ] 8. Implement state tracking
  - [ ] 8.1 Implement StateTracker struct
    - Create `rhivos/update-service/src/tracker.rs`
    - Implement StateTracker with adapters HashMap and watcher_manager reference
    - Implement get_state() to retrieve current adapter state
    - Implement transition() to change state and notify watchers
    - Implement list_all() to return all adapter info
    - Implement remove() to delete adapter from tracking
    - Implement restore_from_containers() to recover state on startup
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 7.1, 7.2, 7.3_

  - [ ] 8.2 Write property test for state timestamp updates
    - **Property 10: State Timestamp Updates**
    - Verify last_updated is updated on every state change
    - **Validates: Requirements 5.2, 5.3**

  - [ ] 8.3 Write property test for list adapters completeness
    - **Property 14: List Adapters Returns Complete Info**
    - Install multiple adapters and verify list contains all with complete fields
    - **Validates: Requirements 7.1, 7.2**

- [ ] 9. Implement watcher management
  - [ ] 9.1 Implement WatcherManager struct
    - Create `rhivos/update-service/src/watcher.rs`
    - Implement WatcherManager with watchers Vec of mpsc::Sender
    - Implement register() to add new watcher and return receiver
    - Implement broadcast() to send event to all active watchers
    - Implement cleanup_disconnected() to remove closed channels
    - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5_

  - [ ] 9.2 Write property test for watcher event reception
    - **Property 11: Watcher Receives State Events**
    - Register watcher, trigger state change, verify event received with all fields
    - **Validates: Requirements 6.1, 6.2, 6.3**

  - [ ] 9.3 Write property test for watcher cleanup
    - **Property 12: Watcher Cleanup on Disconnect**
    - Disconnect one watcher, verify others still receive events
    - **Validates: Requirements 6.4**

  - [ ] 9.4 Write property test for initial state emission
    - **Property 13: New Watcher Receives Initial State**
    - Install adapters, connect new watcher, verify initial state events received
    - **Validates: Requirements 6.5**

- [ ] 10. Implement gRPC service
  - [ ] 10.1 Implement UpdateServiceImpl struct
    - Create `rhivos/update-service/src/service.rs`
    - Implement UpdateServiceImpl with all components wired together
    - Implement helper methods for install workflow orchestration
    - _Requirements: 1.1, 1.2, 10.1, 10.2, 10.3, 10.4_

  - [ ] 10.2 Implement InstallAdapter RPC handler
    - Check for existing adapter (idempotence for RUNNING/in-progress)
    - Validate registry_url format
    - Initialize state as DOWNLOADING
    - Spawn async task for download → validate → install → start workflow
    - Return InstallAdapterResponse with initial state
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 2.2, 2.3, 4.2, 4.3_

  - [ ] 10.3 Write property test for install state initialization
    - **Property 1: Install Initiates Download State**
    - Verify valid install request returns DOWNLOADING state
    - **Validates: Requirements 1.1, 1.2**

  - [ ] 10.4 Write property test for install idempotence (running)
    - **Property 2: Install Idempotence for Running Adapters**
    - Install already-running adapter, verify no re-download
    - **Validates: Requirements 1.3**

  - [ ] 10.5 Write property test for install idempotence (in-progress)
    - **Property 3: Install Idempotence for In-Progress Adapters**
    - Install while DOWNLOADING/INSTALLING, verify current state returned
    - **Validates: Requirements 1.4**

  - [ ] 10.6 Write property test for invalid registry URL
    - **Property 4: Invalid Registry URL Returns Error**
    - Provide malformed URLs and verify error response
    - **Validates: Requirements 1.5**

  - [ ] 10.7 Implement UninstallAdapter RPC handler
    - Check adapter exists
    - Stop container if running
    - Remove container and storage
    - Remove from state tracker
    - Emit state change event
    - Return UninstallAdapterResponse
    - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5_

  - [ ] 10.8 Write property test for uninstall completeness
    - **Property 15: Uninstall Removes Adapter Completely**
    - Verify container stopped, removed, state cleared, event emitted
    - **Validates: Requirements 8.1, 8.2, 8.3, 8.5**

  - [ ] 10.9 Write property test for uninstall non-existent
    - **Property 16: Uninstall Non-Existent Returns Error**
    - Uninstall unknown adapter_id, verify NOT_FOUND error
    - **Validates: Requirements 8.4**

  - [ ] 10.10 Implement ListAdapters RPC handler
    - Return all adapters from state tracker with complete info
    - _Requirements: 7.1, 7.2, 7.3_

  - [ ] 10.11 Implement WatchAdapterStates RPC handler
    - Register watcher with WatcherManager
    - Emit initial state for all adapters
    - Stream AdapterStateEvent messages
    - Handle client disconnect gracefully
    - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5_

  - [ ] 10.12 Write property test for state progression
    - **Property 6: State Progression Through Installation**
    - Verify DOWNLOADING → INSTALLING → RUNNING progression
    - **Validates: Requirements 2.3, 4.2, 4.3**

- [ ] 11. Checkpoint - Verify gRPC service
  - Run `cargo test` for all tests
  - Verify service compiles and handlers are wired correctly
  - Ask the user if questions arise

- [ ] 12. Implement gRPC server startup with TLS
  - [ ] 12.1 Implement TLS server listener
    - Update `rhivos/update-service/src/main.rs`
    - Load TLS certificate and key from configured paths
    - Create TCP listener on configured address
    - Initialize all components (downloader, validator, container_manager, state_tracker, watcher_manager)
    - Restore state from running containers on startup
    - Start tonic gRPC server with TLS and UpdateServiceImpl
    - Handle graceful shutdown on SIGTERM
    - _Requirements: 5.4, 10.5_

- [ ] 13. Implement offload scheduler
  - [ ] 13.1 Implement OffloadScheduler struct
    - Create `rhivos/update-service/src/offload.rs`
    - Implement OffloadScheduler with state_tracker, container_manager, offload_threshold, check_interval
    - Implement start() to spawn background task
    - Implement check_and_offload() to find and remove inactive adapters
    - Track last_activity timestamp for each adapter
    - _Requirements: 9.1, 9.2, 9.3, 9.4_

  - [ ] 13.2 Write property test for automatic offload
    - **Property 17: Automatic Offload After Inactivity**
    - Simulate 24+ hours of STOPPED state, verify automatic uninstall and event
    - **Validates: Requirements 9.1, 9.2, 9.3**

- [ ] 14. Implement logging
  - [ ] 14.1 Implement structured logging
    - Create `rhivos/update-service/src/logging.rs`
    - Configure tracing subscriber for structured JSON output
    - Add logging to all request handlers, state transitions, and container operations
    - Include correlation identifiers for end-to-end tracing
    - _Requirements: 11.1, 11.2, 11.3, 11.4_

- [ ] 15. Checkpoint - Verify complete service
  - Run `cargo test` for all unit and property tests
  - Run `cargo clippy` for linting
  - Verify service starts with TLS and accepts connections
  - Ask the user if questions arise

- [ ] 16. Integration testing
  - [ ] 16.1 Create mock registry for testing
    - Create `rhivos/update-service/src/test_utils.rs`
    - Implement MockRegistry that serves OCI manifests and layers
    - Support configurable responses for success/failure scenarios
    - _Requirements: 2.1, 3.1_

  - [ ] 16.2 Create mock container manager for testing
    - Implement MockContainerManager that simulates podman operations
    - Support failure injection for testing error paths
    - _Requirements: 4.1, 4.6_

  - [ ] 16.3 Write integration tests for end-to-end flows
    - Test complete install flow: request → download → validate → install → start → RUNNING
    - Test uninstall flow with running container
    - Test watcher streaming over multiple state changes
    - Test state restoration on service restart
    - Test offload scheduler timing
    - _Requirements: 1.1-1.5, 2.1-2.5, 3.1-3.4, 4.1-4.6, 5.4, 6.1-6.5, 8.1-8.5, 9.1-9.3_

- [ ] 17. Final checkpoint - Verify complete implementation
  - Run `cargo test` for all unit, property, and integration tests
  - Run `cargo clippy` for linting
  - Ensure all 17 properties pass
  - Ask the user if questions arise

## Notes

- All tasks including property tests are required for comprehensive implementation
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation
- Property tests validate universal correctness properties from the design document
- The service uses `proptest` crate for property-based testing with minimum 100 iterations per test
- TLS certificates should be generated for development using `infra/certs/` tooling
