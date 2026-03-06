# Implementation Plan: Mock Apps (Spec 09)

<!-- AGENT INSTRUCTIONS
- Implement exactly ONE top-level task group per session
- Task group 1 writes failing tests from test_spec.md — all subsequent groups
  implement code to make those tests pass
- Follow the git-flow: feature branch from develop -> implement -> test -> merge to develop -> push
- Update checkbox states as you go: [-] in progress, [x] complete
-->

## Overview

This plan implements all mock/demo tools: three Rust sensor simulators (location, speed, door), a Go mock PARKING_OPERATOR server, a Go parking-app-cli with 9 subcommands, and a Go companion-app-cli with 3 subcommands. Task group 1 writes all failing spec tests. Groups 2-5 implement functionality. Group 6 is the final checkpoint.

## Dependencies

| Spec | From Group | To Group | Relationship |
|------|-----------|----------|--------------|
| 01_project_setup | 2 | 1 | Requires Go module structure, proto definitions, generated gRPC code, Go workspace (`go.work`), Rust workspace (`rhivos/Cargo.toml`) |
| 02_data_broker | 3 | 2 | Mock sensors write to DATA_BROKER via kuksa.val.v1 gRPC API |
| 05_parking_fee_service | 2 | 4 | Mock PARKING_APP CLI calls PARKING_FEE_SERVICE REST API |
| 06_cloud_gateway | 2 | 5 | Mock COMPANION_APP CLI calls CLOUD_GATEWAY REST API |
| 07_update_service | 2 | 4 | Mock PARKING_APP CLI calls UPDATE_SERVICE gRPC API |
| 08_parking_operator_adaptor | 2 | 4 | Mock PARKING_APP CLI calls PARKING_OPERATOR_ADAPTOR gRPC API; mock PARKING_OPERATOR receives calls from PARKING_OPERATOR_ADAPTOR |

## Test Commands

- Sensor tests: `cd rhivos && cargo test -p location-sensor -p speed-sensor -p door-sensor`
- Sensor lint: `cd rhivos && cargo clippy -p location-sensor -p speed-sensor -p door-sensor`
- Sensor build: `cd rhivos && cargo build -p location-sensor -p speed-sensor -p door-sensor`
- parking-operator tests: `cd mock/parking-operator && go test ./... -v`
- parking-operator lint: `cd mock/parking-operator && go vet ./...`
- parking-app-cli tests: `cd mock/parking-app-cli && go test ./... -v`
- parking-app-cli lint: `cd mock/parking-app-cli && go vet ./...`
- companion-app-cli tests: `cd mock/companion-app-cli && go test ./... -v`
- companion-app-cli lint: `cd mock/companion-app-cli && go vet ./...`

## Tasks

- [x] 1. Write failing spec tests
  - [x] 1.1 Create Rust sensor test scaffolding
    - Add three sensor crates to the Rust workspace (`rhivos/Cargo.toml`)
    - Create minimal `Cargo.toml` and `src/main.rs` for each crate with stub `main()` functions
    - Add inline test modules:
    - **location-sensor:** `test_missing_lat_lon_exits_with_error` (TS-09-E1), `test_missing_lon_exits_with_error` (TS-09-E1), `test_writes_correct_latitude_and_longitude` (TS-09-1)
    - **speed-sensor:** `test_missing_speed_exits_with_error` (TS-09-E1), `test_writes_correct_speed` (TS-09-2)
    - **door-sensor:** `test_missing_open_or_closed_exits_with_error` (TS-09-E1), `test_writes_open_true` (TS-09-3), `test_writes_closed_false` (TS-09-4)
    - Verify: `cd rhivos && cargo test -p location-sensor -p speed-sensor -p door-sensor` -- tests compile but fail
    - _Test Spec: TS-09-1, TS-09-2, TS-09-3, TS-09-4, TS-09-E1, TS-09-E2_

  - [x] 1.2 Create parking-operator test scaffolding
    - Create `mock/parking-operator/go.mod` and minimal stub files (`main.go`, `handler.go`, `session.go`, `models.go`)
    - Create `mock/parking-operator/handler_test.go` with: `TestStartSession_Valid` (TS-09-7), `TestStopSession_Valid` (TS-09-8), `TestGetStatus_ReturnsAllSessions` (TS-09-9), `TestGetStatus_EmptyWhenNoSessions` (TS-09-E9), `TestStartSession_MalformedBody` (TS-09-E7), `TestStopSession_UnknownSession` (TS-09-E8), `TestSessionStoreConsistency` (TS-09-P8)
    - Add `mock/parking-operator` to the root `go.work` file
    - Verify: `cd mock/parking-operator && go test ./... -v` -- tests compile but fail
    - _Test Spec: TS-09-7, TS-09-8, TS-09-9, TS-09-E7, TS-09-E8, TS-09-E9, TS-09-P8_

  - [x] 1.3 Create parking-app-cli test scaffolding
    - Create `mock/parking-app-cli/go.mod` and minimal stub files
    - Create test files with: `TestSubcommandDispatch_UnknownCommand` (TS-09-E4), `TestSubcommandDispatch_NoArguments` (TS-09-E4), `TestLookup_MissingFlags` (TS-09-E3), `TestAdapterInfo_MissingFlags` (TS-09-E3), `TestInstall_MissingFlags` (TS-09-E3), `TestRemove_MissingFlags` (TS-09-E3), `TestStatus_MissingFlags` (TS-09-E3), `TestStartSession_MissingFlags` (TS-09-E3), `TestStopSession_MissingFlags` (TS-09-E3), `TestLookup_CorrectRESTEndpoint` (TS-09-P1), `TestAdapterInfo_CorrectRESTEndpoint` (TS-09-P2), `TestInstall_CorrectGRPCMethod` (TS-09-P3), `TestServiceUnreachable_REST` (TS-09-E5), `TestServiceUnreachable_GRPC` (TS-09-E6)
    - Add `mock/parking-app-cli` to the root `go.work` file
    - Verify: `cd mock/parking-app-cli && go test ./... -v` -- tests compile but fail
    - _Test Spec: TS-09-5, TS-09-P1, TS-09-P2, TS-09-P3, TS-09-E3, TS-09-E4, TS-09-E5, TS-09-E6_

  - [x] 1.4 Create companion-app-cli test scaffolding
    - Create `mock/companion-app-cli/go.mod` and minimal stub files
    - Create test files with: `TestSubcommandDispatch_UnknownCommand` (TS-09-E4), `TestSubcommandDispatch_NoArguments` (TS-09-E4), `TestLock_MissingFlags` (TS-09-E3), `TestUnlock_MissingFlags` (TS-09-E3), `TestStatus_MissingFlags` (TS-09-E3), `TestLock_CorrectPayload` (TS-09-P4), `TestUnlock_CorrectPayload` (TS-09-P5), `TestStatus_CorrectEndpoint` (TS-09-P6), `TestBearerToken_IncludedInRequests` (TS-09-P4), `TestServiceUnreachable_REST` (TS-09-E5)
    - Add `mock/companion-app-cli` to the root `go.work` file
    - Verify: `cd mock/companion-app-cli && go test ./... -v` -- tests compile but fail
    - _Test Spec: TS-09-6, TS-09-P4, TS-09-P5, TS-09-P6, TS-09-E3, TS-09-E4, TS-09-E5_

  - [x] 1.V Verify task group 1
    - [x] All sensor tests compile but fail: `cd rhivos && cargo test -p location-sensor -p speed-sensor -p door-sensor`
    - [x] All parking-operator tests compile but fail: `cd mock/parking-operator && go test ./... -v`
    - [x] All parking-app-cli tests compile but fail: `cd mock/parking-app-cli && go test ./... -v`
    - [x] All companion-app-cli tests compile but fail: `cd mock/companion-app-cli && go test ./... -v`
    - [x] No linter warnings introduced

- [x] 2. Implement mock sensors
  - [x] 2.1 Implement location-sensor
    - Parse CLI arguments using `clap`: `--lat` (f64, required), `--lon` (f64, required), `--broker-addr` (string, default `http://localhost:55556`)
    - Connect to DATA_BROKER via gRPC (tonic client for kuksa.val.v1)
    - Send `SetRequest` for `Vehicle.CurrentLocation.Latitude` and `Vehicle.CurrentLocation.Longitude`
    - Print confirmation, exit with code 0 on success or code 1 on failure
    - _Requirements: 09-REQ-1.1, 09-REQ-7.1, 09-REQ-8.2_

  - [x] 2.2 Implement speed-sensor
    - Parse CLI arguments: `--speed` (f32, required), `--broker-addr` (string, default `http://localhost:55556`)
    - Connect to DATA_BROKER via gRPC
    - Send `SetRequest` for `Vehicle.Speed`
    - Print confirmation and exit
    - _Requirements: 09-REQ-2.1, 09-REQ-7.1, 09-REQ-8.2_

  - [x] 2.3 Implement door-sensor
    - Parse CLI arguments: `--open` (bool flag) and `--closed` (bool flag), mutually exclusive, one required; `--broker-addr` (string, default `http://localhost:55556`)
    - Connect to DATA_BROKER via gRPC
    - Send `SetRequest` for `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` with `true` (--open) or `false` (--closed)
    - Print confirmation and exit
    - _Requirements: 09-REQ-3.1, 09-REQ-7.1, 09-REQ-8.2_

  - [x] 2.V Verify task group 2
    - [x] Sensor tests pass: `cd rhivos && cargo test -p location-sensor -p speed-sensor -p door-sensor`
    - [x] No linter warnings: `cd rhivos && cargo clippy -p location-sensor -p speed-sensor -p door-sensor`

- [x] 3. Implement mock PARKING_OPERATOR
  - [x] 3.1 Implement data model and session store
    - Create `mock/parking-operator/models.go` with request/response types
    - Create `mock/parking-operator/session.go` with in-memory `SessionStore`: `NewSessionStore()`, `Create(vehicleID, zoneID)`, `Stop(sessionID)`, `List()`
    - Rate: 2.50 EUR/hr
    - _Requirements: 09-REQ-6.1, 09-REQ-6.2, 09-REQ-6.3_

  - [x] 3.2 Implement HTTP handlers
    - `HandleStartParking` -- parse JSON body, validate, create session, return 200 or 400
    - `HandleStopParking` -- parse JSON body, validate, stop session, return 200/404/400
    - `HandleParkingStatus` -- return all sessions as JSON array
    - `writeJSON` and `writeError` helpers
    - _Requirements: 09-REQ-6.1, 09-REQ-6.2, 09-REQ-6.3_

  - [x] 3.3 Implement server entry point
    - Read port from `PORT` env var or `-port` flag (default: 9090)
    - Register routes: `POST /parking/start`, `POST /parking/stop`, `GET /parking/status`
    - Start HTTP server and log startup message

  - [x] 3.V Verify task group 3
    - [x] All tests pass: `cd mock/parking-operator && go test ./... -v`
    - [x] No lint issues: `cd mock/parking-operator && go vet ./...`

- [ ] 4. Implement parking-app-cli
  - [ ] 4.1 Create shared internal packages
    - `internal/config/config.go` -- Read environment variables with defaults; flag-overrides-env precedence
    - `internal/output/output.go` -- `PrintJSON`, `PrintError` helpers
    - `internal/restclient/client.go` -- HTTP client wrapper with 10-second timeout
    - `internal/grpcclient/client.go` -- gRPC `Dial(addr)` helper with insecure credentials and 10-second timeout
    - _Requirements: 09-REQ-8.1_

  - [ ] 4.2 Implement subcommand dispatch
    - Parse `os.Args[1]` and route to appropriate handler
    - Print usage and exit code 1 for unknown subcommands or no arguments
    - List all 9 available subcommands in usage message
    - _Requirements: 09-REQ-4.1_

  - [ ] 4.3 Implement REST subcommands (lookup, adapter-info)
    - `lookup.go` -- Parse `--lat` and `--lon`, GET `{PARKING_FEE_SERVICE_URL}/operators?lat={lat}&lon={lon}`, print response
    - `adapter_info.go` -- Parse `--operator-id`, GET `{PARKING_FEE_SERVICE_URL}/operators/{id}/adapter`, print response
    - _Requirements: 09-REQ-4.2_

  - [ ] 4.4 Implement gRPC subcommands (install, watch, list, remove, status)
    - `install.go` -- Parse `--image-ref`, `--checksum`; call `InstallAdapter`; print response
    - `watch.go` -- Call `WatchAdapterStates` (streaming); print events; handle Ctrl+C
    - `list.go` -- Call `ListAdapters`; print response
    - `remove.go` -- Parse `--adapter-id`; call `RemoveAdapter`; print response
    - `status.go` -- Parse `--adapter-id`; call `GetAdapterStatus`; print response
    - _Requirements: 09-REQ-4.3_

  - [ ] 4.5 Implement session management subcommands (start-session, stop-session)
    - `start_session.go` -- Parse `--zone-id`; dial PARKING_OPERATOR_ADAPTOR; call `StartSession`; print response
    - `stop_session.go` -- Parse `--session-id`; dial PARKING_OPERATOR_ADAPTOR; call `StopSession`; print response
    - _Requirements: 09-REQ-4.3_

  - [ ] 4.V Verify task group 4
    - [ ] All tests pass: `cd mock/parking-app-cli && go test ./... -v`
    - [ ] No lint issues: `cd mock/parking-app-cli && go vet ./...`
    - [ ] Build succeeds: `go build ./mock/parking-app-cli/...`

- [ ] 5. Implement companion-app-cli
  - [ ] 5.1 Create shared internal packages
    - `internal/config/config.go` -- Read `CLOUD_GATEWAY_URL`, `BEARER_TOKEN` with defaults
    - `internal/output/output.go` -- Same pattern as parking-app-cli
    - `internal/restclient/client.go` -- HTTP client wrapper with bearer token support
    - _Requirements: 09-REQ-5.1_

  - [ ] 5.2 Implement subcommand dispatch and commands
    - Dispatch for `lock`, `unlock`, `status`
    - `lock.go` -- Parse `--vin`; generate UUID; POST to `/vehicles/{vin}/commands` with lock payload; include bearer token; warn if token missing
    - `unlock.go` -- Same as lock but `"type": "unlock"`
    - `status.go` -- Parse `--vin`; GET `/vehicles/{vin}/status`; include bearer token
    - _Requirements: 09-REQ-5.1_

  - [ ] 5.V Verify task group 5
    - [ ] All tests pass: `cd mock/companion-app-cli && go test ./... -v`
    - [ ] No lint issues: `cd mock/companion-app-cli && go vet ./...`
    - [ ] Build succeeds: `go build ./mock/companion-app-cli/...`

- [ ] 6. Checkpoint
  - [ ] 6.1 Run full test suite
    - `cd rhivos && cargo test -p location-sensor -p speed-sensor -p door-sensor` -- all pass
    - `cd mock/parking-operator && go test ./... -v` -- all pass
    - `cd mock/parking-app-cli && go test ./... -v` -- all pass
    - `cd mock/companion-app-cli && go test ./... -v` -- all pass

  - [ ] 6.2 Run linters
    - `cd rhivos && cargo clippy -p location-sensor -p speed-sensor -p door-sensor` -- no warnings
    - `cd mock/parking-operator && go vet ./...` -- no issues
    - `cd mock/parking-app-cli && go vet ./...` -- no issues
    - `cd mock/companion-app-cli && go vet ./...` -- no issues

  - [ ] 6.3 Build verification
    - All Rust sensor binaries build
    - All Go mock binaries build

  - [ ] 6.4 Review Definition of Done
    - Confirm all items in the design.md Definition of Done are satisfied

## Traceability

| Requirement | Test Spec Entry | Implemented By Task | Verified By Test |
|-------------|-----------------|---------------------|------------------|
| 09-REQ-1.1 | TS-09-1, TS-09-E1, TS-09-E2 | 2.1 | Sensor unit tests |
| 09-REQ-2.1 | TS-09-2, TS-09-E1, TS-09-E2 | 2.2 | Sensor unit tests |
| 09-REQ-3.1 | TS-09-3, TS-09-4, TS-09-E1, TS-09-E2 | 2.3 | Sensor unit tests |
| 09-REQ-4.1 | TS-09-5, TS-09-E3, TS-09-E4 | 4.2 | CLI dispatch tests |
| 09-REQ-4.2 | TS-09-P1, TS-09-P2, TS-09-E5 | 4.3 | REST subcommand tests |
| 09-REQ-4.3 | TS-09-P3, TS-09-E6 | 4.4, 4.5 | gRPC subcommand tests |
| 09-REQ-5.1 | TS-09-6, TS-09-P4, TS-09-P5, TS-09-P6, TS-09-E3, TS-09-E5 | 5.1, 5.2 | Companion CLI tests |
| 09-REQ-6.1 | TS-09-7, TS-09-E7 | 3.1, 3.2 | Handler tests |
| 09-REQ-6.2 | TS-09-8, TS-09-E8 | 3.1, 3.2 | Handler tests |
| 09-REQ-6.3 | TS-09-9, TS-09-E9, TS-09-P8 | 3.1, 3.2 | Handler + session store tests |
| 09-REQ-7.1 | TS-09-P7 | 2.1, 2.2, 2.3 | Sensor DATA_BROKER write tests |
| 09-REQ-8.1 | -- | 4.1 | Config loading verified at startup |
| 09-REQ-8.2 | -- | 2.1, 2.2, 2.3 | Sensor CLI argument tests |
