# Implementation Plan: CLI Simulators

## Overview

This plan implements two Go CLI applications for local development testing:
- COMPANION_CLI: Simulates the Flutter COMPANION_APP for remote vehicle control
- PARKING_CLI: Simulates the Kotlin PARKING_APP for parking session management

Both CLIs use REPL interfaces and connect to the same backend services as the real mobile apps.

## Tasks

- [ ] 1. Set up project structure and shared REPL framework
  - [ ] 1.1 Create directory structure for both CLIs
    - Create `backend/companion-cli/cmd/companion-cli/main.go`
    - Create `backend/companion-cli/internal/config/config.go`
    - Create `backend/companion-cli/internal/repl/repl.go`
    - Create `backend/parking-cli/cmd/parking-cli/main.go`
    - Create `backend/parking-cli/internal/config/config.go`
    - Create `backend/parking-cli/internal/repl/repl.go`
    - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5, 8.6_

  - [ ]* 1.2 Write property tests for configuration loading
    - **Property 3: Configuration with Defaults**
    - **Validates: Requirements 9.1, 9.2, 9.3, 9.4, 9.5, 9.6, 9.7**

  - [ ]* 1.3 Write property tests for REPL command handling
    - **Property 5: Unknown Command Handling**
    - **Property 6: Help Command Completeness**
    - **Validates: Requirements 8.4, 8.6**

- [ ] 2. Checkpoint - Ensure project structure compiles
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 3. Implement COMPANION_CLI HTTP client and commands
  - [ ] 3.1 Implement GatewayClient for CLOUD_GATEWAY REST API
    - Create `backend/companion-cli/internal/client/gateway.go`
    - Implement SendLockCommand, SendUnlockCommand, GetCommandStatus, Ping
    - _Requirements: 1.1, 2.1, 3.1, 10.5_

  - [ ] 3.2 Implement COMPANION_CLI commands (lock, unlock, status, ping, help, quit)
    - Wire commands to GatewayClient
    - Implement output formatting for responses and errors
    - _Requirements: 1.2, 1.3, 1.4, 2.2, 2.3, 2.4, 3.2, 3.3, 3.4, 10.1, 10.2, 10.3_

  - [ ]* 3.3 Write property tests for response/error propagation
    - **Property 1: Response Field Propagation** (COMPANION_CLI portion)
    - **Property 2: Error Message Propagation** (COMPANION_CLI portion)
    - **Validates: Requirements 1.2, 1.4, 2.2, 2.4, 3.2, 3.4**

- [ ] 4. Checkpoint - COMPANION_CLI functional
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 5. Implement PARKING_CLI gRPC clients
  - [ ] 5.1 Implement DataBrokerClient for DATA_BROKER gRPC service
    - Create `backend/parking-cli/internal/client/databroker.go`
    - Implement SetLocation, GetLocation, Close
    - _Requirements: 4.1, 4.4_

  - [ ] 5.2 Implement ParkingFeeClient for PARKING_FEE_SERVICE REST API
    - Create `backend/parking-cli/internal/client/parking.go`
    - Implement GetZone
    - _Requirements: 5.1_

  - [ ] 5.3 Implement UpdateServiceClient for UPDATE_SERVICE gRPC service
    - Create `backend/parking-cli/internal/client/update.go`
    - Implement ListAdapters, InstallAdapter, UninstallAdapter, Close
    - _Requirements: 6.1, 6.3, 6.5_

  - [ ] 5.4 Implement ParkingAdaptorClient for PARKING_OPERATOR_ADAPTOR gRPC service
    - Create `backend/parking-cli/internal/client/adaptor.go`
    - Implement StartSession, StopSession, GetSessionStatus, Close
    - _Requirements: 7.1, 7.3, 7.5_

  - [ ] 5.5 Implement LockingServiceClient for LOCKING_SERVICE gRPC service
    - Create `backend/parking-cli/internal/client/locking.go`
    - Implement GetLockState, GetAllLockStates, Close
    - _Requirements: 11.1_

- [ ] 6. Implement PARKING_CLI commands
  - [ ] 6.1 Implement location command (set and get)
    - Wire to DataBrokerClient
    - Implement output formatting
    - _Requirements: 4.1, 4.2, 4.3, 4.4_

  - [ ] 6.2 Implement zone command
    - Wire to ParkingFeeClient
    - Implement output formatting for zone info and not-found case
    - _Requirements: 5.1, 5.2, 5.3, 5.4_

  - [ ] 6.3 Implement adapter commands (adapters, install, uninstall)
    - Wire to UpdateServiceClient
    - Implement output formatting for adapter list and operations
    - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 6.6_

  - [ ] 6.4 Implement session commands (start, stop, session)
    - Wire to ParkingAdaptorClient
    - Implement output formatting for session info
    - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5, 7.6, 7.7_

  - [ ] 6.5 Implement locks command
    - Wire to LockingServiceClient
    - Implement output formatting for all door states
    - _Requirements: 11.1, 11.2, 11.3_

  - [ ] 6.6 Implement ping command for all services
    - Test connectivity to DATA_BROKER, PARKING_FEE_SERVICE, UPDATE_SERVICE, PARKING_ADAPTOR, LOCKING_SERVICE
    - Display status for each service
    - _Requirements: 10.5_

- [ ] 7. Checkpoint - PARKING_CLI functional
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 8. Write property tests for PARKING_CLI
  - [ ]* 8.1 Write property tests for response field propagation
    - **Property 1: Response Field Propagation** (PARKING_CLI portion)
    - **Validates: Requirements 5.2, 6.2, 6.4, 7.2, 7.4, 7.6, 11.2**

  - [ ]* 8.2 Write property tests for error message propagation
    - **Property 2: Error Message Propagation** (PARKING_CLI portion)
    - **Validates: Requirements 4.3, 5.4, 6.6, 7.7, 10.1, 10.2, 11.3**

  - [ ]* 8.3 Write property tests for command argument parsing
    - **Property 4: Command Argument Parsing**
    - **Validates: Requirements 3.1, 4.1, 6.3, 6.5, 7.1**

  - [ ]* 8.4 Write property tests for timeout and ping
    - **Property 7: Timeout Message Format**
    - **Property 8: Ping Command Coverage**
    - **Validates: Requirements 10.3, 10.5**

- [ ] 9. Update backend README and Makefile
  - [ ] 9.1 Add CLI build targets to Makefile
    - Add `build-companion-cli` and `build-parking-cli` targets
    - Add `build-cli` target to build both
    - Update `build-backend` to include CLIs

  - [ ] 9.2 Update backend/README.md with CLI documentation
    - Document CLI usage and commands
    - Document environment variables
    - Add examples

- [ ] 10. Final checkpoint - All tests pass
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation
- Property tests validate universal correctness properties
- Unit tests validate specific examples and edge cases
- The CLIs use the existing generated protobuf code from `backend/gen/`
