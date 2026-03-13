# Implementation Plan: MOCK_APPS

<!-- AGENT INSTRUCTIONS
- Implement exactly ONE top-level task group per session
- Task group 1 writes failing tests from test_spec.md — all subsequent groups
  implement code to make those tests pass
- Follow the git-flow: feature branch from main -> implement -> test -> merge to main -> push
- Update checkbox states as you go: [-] in progress, [x] complete
-->

## Overview

This plan implements six mock tools: three Rust CLI sensors (`rhivos/mock-sensors/`) and three Go CLI/server apps (`mock/`). Task group 1 writes failing tests. Group 2 implements mock sensors (Rust). Group 3 implements the mock PARKING_OPERATOR server (Go). Group 4 implements mock COMPANION_APP and PARKING_APP CLIs (Go). Group 5 runs integration tests against running services.

Ordering: tests first, then Rust sensors (simplest, no upstream deps), then PARKING_OPERATOR server (standalone), then CLI tools (depend on proto imports), then integration validation.

## Test Commands

- Spec tests (Rust sensors): `cd rhivos && cargo test -p mock-sensors`
- Spec tests (Go mocks): `cd mock && go test -v ./...`
- Property tests (Go): `cd mock && go test -v -run Property ./...`
- Integration tests: `cd tests/mock-apps && go test -v ./...`
- All tests: `make test`
- Linter (Rust): `cd rhivos && cargo clippy -p mock-sensors -- -D warnings`
- Linter (Go): `cd mock && go vet ./...`

## Tasks

- [ ] 1. Write failing spec tests
  - [ ] 1.1 Set up Rust sensor test infrastructure
    - Ensure `rhivos/mock-sensors/src/lib.rs` has module structure
    - Add dev-dependencies: tokio (test features)
    - Create unit test module for argument parsing and config defaults
    - _Test Spec: TS-09-1, TS-09-2, TS-09-3, TS-09-21, TS-09-E1, TS-09-E2_

  - [ ] 1.2 Write PARKING_OPERATOR unit tests (Go)
    - Create `mock/parking-operator/handler/handler_test.go`
    - Create `mock/parking-operator/store/store_test.go`
    - `TestStartSession` — TS-09-5
    - `TestStopSession` — TS-09-6
    - `TestGetStatus` — TS-09-7
    - `TestStopUnknownSession` — TS-09-E3
    - `TestStatusUnknownSession` — TS-09-E4
    - `TestInvalidJSON` — TS-09-E5
    - `TestConfigDefault` — TS-09-24
    - _Test Spec: TS-09-5, TS-09-6, TS-09-7, TS-09-24, TS-09-E3, TS-09-E4, TS-09-E5_

  - [ ] 1.3 Write COMPANION_APP CLI tests (Go)
    - Create `mock/companion-app-cli/main_test.go`
    - `TestLockCommand` — TS-09-9
    - `TestUnlockCommand` — TS-09-10
    - `TestStatusQuery` — TS-09-11
    - `TestMissingToken` — TS-09-E6
    - `TestGatewayUnreachable` — TS-09-E7
    - `TestConfigDefault` — TS-09-22
    - _Test Spec: TS-09-9, TS-09-10, TS-09-11, TS-09-22, TS-09-E6, TS-09-E7_

  - [ ] 1.4 Write PARKING_APP CLI tests (Go)
    - Create `mock/parking-app-cli/main_test.go` or per-package test files
    - `TestLookup` — TS-09-12
    - `TestAdapterInfo` — TS-09-13
    - `TestInstall` — TS-09-14
    - `TestWatch` — TS-09-15
    - `TestList` — TS-09-16
    - `TestRemove` — TS-09-17
    - `TestStatus` — TS-09-18
    - `TestStartSession` — TS-09-19
    - `TestStopSession` — TS-09-20
    - `TestConfigDefaults` — TS-09-23
    - `TestUnknownSubcommand` — TS-09-E8
    - `TestMissingRequiredFlag` — TS-09-E9
    - `TestUpstreamUnreachable` — TS-09-E10
    - _Test Spec: TS-09-12 through TS-09-20, TS-09-23, TS-09-E8, TS-09-E9, TS-09-E10_

  - [ ] 1.5 Write shared and property tests
    - `TestHelpFlag` — TS-09-25
    - `TestConnectionErrorMessage` — TS-09-26
    - `TestUpstreamErrorResponse` — TS-09-27
    - `TestPropertySensorSignalType` — TS-09-P1
    - `TestPropertySessionLifecycle` — TS-09-P2
    - `TestPropertySubcommandDispatch` — TS-09-P3
    - `TestPropertyConfigDefaults` — TS-09-P4
    - `TestPropertyErrorExitCode` — TS-09-P5
    - _Test Spec: TS-09-25, TS-09-26, TS-09-27, TS-09-P1 through TS-09-P5_

  - [ ] 1.V Verify task group 1
    - [ ] Rust tests compile: `cd rhivos && cargo test -p mock-sensors --no-run`
    - [ ] Go tests compile: `cd mock && go test -v ./... -run NONE`
    - [ ] All spec tests FAIL (red)
    - [ ] No linter warnings: `cd rhivos && cargo clippy -p mock-sensors -- -D warnings && cd mock && go vet ./...`

- [ ] 2. Mock sensors (Rust)
  - [ ] 2.1 Implement BrokerWriter shared library
    - Vendor kuksa.val.v1 proto files into `rhivos/mock-sensors/proto/`
    - Add tonic, prost, tonic-build dependencies
    - Create `build.rs` for proto code generation
    - Implement `BrokerWriter` in `src/lib.rs`: connect to DATA_BROKER, set_double, set_float, set_bool
    - Read `DATA_BROKER_ADDR` from env with default
    - _Requirements: 09-REQ-5.1_

  - [ ] 2.2 Implement location-sensor binary
    - Parse `--lat` and `--lon` arguments (clap or manual)
    - Write lat/lon to DATA_BROKER via BrokerWriter
    - Print usage on `--help`, error on missing/invalid args
    - _Requirements: 09-REQ-1.1, 09-REQ-1.E1, 09-REQ-1.E2, 09-REQ-6.1_

  - [ ] 2.3 Implement speed-sensor binary
    - Parse `--speed` argument
    - Write Vehicle.Speed to DATA_BROKER via BrokerWriter
    - _Requirements: 09-REQ-1.2, 09-REQ-1.E1, 09-REQ-1.E2_

  - [ ] 2.4 Implement door-sensor binary
    - Parse `--open` or `--closed` argument
    - Write Vehicle.Cabin.Door.Row1.DriverSide.IsOpen to DATA_BROKER
    - _Requirements: 09-REQ-1.3, 09-REQ-1.E1, 09-REQ-1.E2_

  - [ ] 2.V Verify task group 2
    - [ ] Sensor tests pass: `cd rhivos && cargo test -p mock-sensors`
    - [ ] All existing tests still pass: `cd rhivos && cargo test`
    - [ ] No linter warnings: `cd rhivos && cargo clippy -p mock-sensors -- -D warnings`
    - [ ] _Test Spec: TS-09-21, TS-09-E1, TS-09-E2, TS-09-P1_

- [x] 3. Mock PARKING_OPERATOR (Go)
  - [x] 3.1 Implement store package
    - `NewStore() *Store`: mutex-protected map
    - `Start(req StartRequest) StartResponse`: generate UUID, store session, return response
    - `Stop(req StopRequest) (StopResponse, error)`: find session, calculate duration and total, return response
    - `GetStatus(sessionID string) (*Session, error)`: look up session
    - _Requirements: 09-REQ-2.2, 09-REQ-2.3, 09-REQ-2.4, 09-REQ-2.E1, 09-REQ-2.E2_

  - [x] 3.2 Implement handler package
    - `StartHandler(store) http.HandlerFunc`: parse JSON body, call store.Start, return 200
    - `StopHandler(store) http.HandlerFunc`: parse JSON body, call store.Stop, return 200 or 404
    - `StatusHandler(store) http.HandlerFunc`: extract session_id from path, call store.GetStatus, return 200 or 404
    - Set Content-Type: application/json on all responses
    - Return `{"error": "..."}` for error responses
    - _Requirements: 09-REQ-2.2, 09-REQ-2.3, 09-REQ-2.4, 09-REQ-2.E1, 09-REQ-2.E2, 09-REQ-2.E3_

  - [x] 3.3 Implement main with serve subcommand
    - Subcommand dispatch: `serve` starts server, `--help` prints usage
    - Read port from `PORT` env or `--port` flag (default 8080)
    - Register routes using Go 1.22 ServeMux patterns
    - Handle SIGTERM/SIGINT graceful shutdown
    - Log ready message with port
    - _Requirements: 09-REQ-2.1, 09-REQ-2.5, 09-REQ-5.4, 09-REQ-6.1_

  - [x] 3.V Verify task group 3
    - [x] PARKING_OPERATOR tests pass: `cd mock && go test -v ./parking-operator/...`
    - [x] All existing tests still pass: `make test`
    - [x] No linter warnings: `cd mock && go vet ./parking-operator/...`
    - [x] _Test Spec: TS-09-5, TS-09-6, TS-09-7, TS-09-24, TS-09-E3, TS-09-E4, TS-09-E5, TS-09-P2_

- [x] 4. Mock CLI tools (Go)
  - [x] 4.1 Implement companion-app-cli
    - Subcommand dispatch: `lock`, `unlock`, `status`, `--help`
    - Read CLOUD_GATEWAY_URL, bearer token from env/flags
    - `lock`/`unlock`: POST to /vehicles/{vin}/commands with generated command_id
    - `status`: GET /vehicles/{vin}/commands/{command_id}
    - Add Authorization: Bearer header to all requests
    - Print JSON response to stdout, errors to stderr
    - _Requirements: 09-REQ-3.1, 09-REQ-3.2, 09-REQ-3.3, 09-REQ-3.E1, 09-REQ-3.E2, 09-REQ-5.2, 09-REQ-6.1_

  - [x] 4.2 Implement parking-app-cli REST subcommands
    - Subcommand dispatch for all 9 subcommands + `--help`
    - `lookup`: GET /operators?lat=&lon= to PARKING_FEE_SERVICE
    - `adapter-info`: GET /operators/{id}/adapter to PARKING_FEE_SERVICE
    - Read PARKING_FEE_SERVICE_URL from env/flag
    - _Requirements: 09-REQ-4.1, 09-REQ-4.2, 09-REQ-5.3_

  - [x] 4.3 Implement parking-app-cli gRPC subcommands
    - Import generated proto code for UpdateService and ParkingAdaptor
    - `install`, `watch`, `list`, `remove`, `status`: call UPDATE_SERVICE gRPC
    - `start-session`, `stop-session`: call PARKING_OPERATOR_ADAPTOR gRPC
    - Read UPDATE_SERVICE_ADDR, ADAPTOR_ADDR from env/flags
    - _Requirements: 09-REQ-4.3, 09-REQ-4.4, 09-REQ-4.5, 09-REQ-4.6, 09-REQ-4.7, 09-REQ-4.8, 09-REQ-4.9, 09-REQ-5.3_

  - [x] 4.4 Implement shared error handling
    - Unknown subcommand: print usage, exit 1
    - Missing required flags: print error, exit 1
    - Connection errors: print error with address, exit 1
    - Upstream error responses: print details, exit 1
    - _Requirements: 09-REQ-4.E1, 09-REQ-4.E2, 09-REQ-4.E3, 09-REQ-6.2, 09-REQ-6.3_

  - [x] 4.V Verify task group 4
    - [x] COMPANION_APP tests pass: `cd mock && go test -v ./companion-app-cli/...`
    - [x] PARKING_APP tests pass: `cd mock && go test -v ./parking-app-cli/...`
    - [x] All existing tests still pass: `make test`
    - [x] No linter warnings: `cd mock && go vet ./...`
    - [x] _Test Spec: TS-09-9 through TS-09-20, TS-09-22, TS-09-23, TS-09-25, TS-09-26, TS-09-27, TS-09-E6 through TS-09-E10, TS-09-P3, TS-09-P4, TS-09-P5_

- [x] 5. Integration test validation
  - [x] 5.1 Create integration test module
    - Create `tests/mock-apps/` Go module
    - Add `go.work` entry for `./tests/mock-apps`
    - Shared helpers: start/stop DATA_BROKER, start/stop PARKING_OPERATOR server, build sensor binaries
    - _Test Spec: TS-09-1, TS-09-2, TS-09-3, TS-09-4, TS-09-8_

  - [x] 5.2 Write and run integration tests
    - `TestLocationSensorWritesToBroker` — TS-09-1
    - `TestSpeedSensorWritesToBroker` — TS-09-2
    - `TestDoorSensorWritesToBroker` — TS-09-3
    - `TestParkingOperatorServeStartsServer` — TS-09-4
    - `TestParkingOperatorGracefulShutdown` — TS-09-8
    - _Test Spec: TS-09-1, TS-09-2, TS-09-3, TS-09-4, TS-09-8_

  - [x] 5.V Verify task group 5
    - [x] All integration tests pass: `cd tests/mock-apps && go test -v ./...`
    - [x] All unit tests still pass: `cd rhivos && cargo test -p mock-sensors && cd mock && go test -v ./...`
    - [x] All existing tests still pass: `make test`
    - [x] No linter warnings: `cd rhivos && cargo clippy -p mock-sensors -- -D warnings && cd mock && go vet ./...`
    - [x] All requirements 09-REQ-1 through 09-REQ-6 acceptance criteria met

- [x] 6. Checkpoint - All Tests Green
  - All unit, integration, and property tests pass
  - All 6 mock tools build, run, and produce correct output
  - Ask the user if questions arise

### Checkbox States

| Syntax   | Meaning                |
|----------|------------------------|
| `- [ ]`  | Not started (required) |
| `- [ ]*` | Not started (optional) |
| `- [x]`  | Completed              |
| `- [-]`  | In progress            |
| `- [~]`  | Queued                 |

## Traceability

| Requirement | Test Spec Entry | Implemented By Task | Verified By Test |
|-------------|-----------------|---------------------|------------------|
| 09-REQ-1.1 | TS-09-1 | 2.2 | tests/mock-apps::TestLocationSensorWritesToBroker |
| 09-REQ-1.2 | TS-09-2 | 2.3 | tests/mock-apps::TestSpeedSensorWritesToBroker |
| 09-REQ-1.3 | TS-09-3 | 2.4 | tests/mock-apps::TestDoorSensorWritesToBroker |
| 09-REQ-1.E1 | TS-09-E1 | 2.2, 2.3, 2.4 | mock-sensors::test_sensor_no_args |
| 09-REQ-1.E2 | TS-09-E2 | 2.1 | mock-sensors::test_sensor_broker_unreachable |
| 09-REQ-2.1 | TS-09-4 | 3.3 | tests/mock-apps::TestParkingOperatorServeStartsServer |
| 09-REQ-2.2 | TS-09-5 | 3.1, 3.2 | parking-operator/handler::TestStartSession |
| 09-REQ-2.3 | TS-09-6 | 3.1, 3.2 | parking-operator/handler::TestStopSession |
| 09-REQ-2.4 | TS-09-7 | 3.1, 3.2 | parking-operator/handler::TestGetStatus |
| 09-REQ-2.5 | TS-09-8 | 3.3 | tests/mock-apps::TestParkingOperatorGracefulShutdown |
| 09-REQ-2.E1 | TS-09-E3 | 3.1, 3.2 | parking-operator/handler::TestStopUnknownSession |
| 09-REQ-2.E2 | TS-09-E4 | 3.1, 3.2 | parking-operator/handler::TestStatusUnknownSession |
| 09-REQ-2.E3 | TS-09-E5 | 3.2 | parking-operator/handler::TestInvalidJSON |
| 09-REQ-3.1 | TS-09-9 | 4.1 | companion-app-cli::TestLockCommand |
| 09-REQ-3.2 | TS-09-10 | 4.1 | companion-app-cli::TestUnlockCommand |
| 09-REQ-3.3 | TS-09-11 | 4.1 | companion-app-cli::TestStatusQuery |
| 09-REQ-3.E1 | TS-09-E6 | 4.1 | companion-app-cli::TestMissingToken |
| 09-REQ-3.E2 | TS-09-E7 | 4.1 | companion-app-cli::TestGatewayUnreachable |
| 09-REQ-4.1 | TS-09-12 | 4.2 | parking-app-cli::TestLookup |
| 09-REQ-4.2 | TS-09-13 | 4.2 | parking-app-cli::TestAdapterInfo |
| 09-REQ-4.3 | TS-09-14 | 4.3 | parking-app-cli::TestInstall |
| 09-REQ-4.4 | TS-09-15 | 4.3 | parking-app-cli::TestWatch |
| 09-REQ-4.5 | TS-09-16 | 4.3 | parking-app-cli::TestList |
| 09-REQ-4.6 | TS-09-17 | 4.3 | parking-app-cli::TestRemove |
| 09-REQ-4.7 | TS-09-18 | 4.3 | parking-app-cli::TestStatus |
| 09-REQ-4.8 | TS-09-19 | 4.3 | parking-app-cli::TestStartSession |
| 09-REQ-4.9 | TS-09-20 | 4.3 | parking-app-cli::TestStopSession |
| 09-REQ-4.E1 | TS-09-E8 | 4.4 | parking-app-cli::TestUnknownSubcommand |
| 09-REQ-4.E2 | TS-09-E9 | 4.4 | parking-app-cli::TestMissingRequiredFlag |
| 09-REQ-4.E3 | TS-09-E10 | 4.4 | parking-app-cli::TestUpstreamUnreachable |
| 09-REQ-5.1 | TS-09-21 | 2.1 | mock-sensors::test_config_default |
| 09-REQ-5.2 | TS-09-22 | 4.1 | companion-app-cli::TestConfigDefault |
| 09-REQ-5.3 | TS-09-23 | 4.2, 4.3 | parking-app-cli::TestConfigDefaults |
| 09-REQ-5.4 | TS-09-24 | 3.3 | parking-operator::TestConfigDefault |
| 09-REQ-6.1 | TS-09-25 | 2.2, 3.3, 4.1, 4.2 | TestHelpFlag |
| 09-REQ-6.2 | TS-09-26 | 4.4 | TestConnectionErrorMessage |
| 09-REQ-6.3 | TS-09-27 | 4.4 | TestUpstreamErrorResponse |
| Property 1 | TS-09-P1 | 2.2, 2.3, 2.4 | TestPropertySensorSignalType |
| Property 2 | TS-09-P2 | 3.1 | TestPropertySessionLifecycle |
| Property 3 | TS-09-P3 | 4.2, 4.3, 4.4 | TestPropertySubcommandDispatch |
| Property 4 | TS-09-P4 | 2.1, 3.3, 4.1, 4.2 | TestPropertyConfigDefaults |
| Property 5 | TS-09-P5 | 2.2, 4.1, 4.4 | TestPropertyErrorExitCode |

## Notes

- Mock sensors are Rust binaries in `rhivos/mock-sensors/` with binary targets defined in Cargo.toml. They share a `BrokerWriter` utility in `src/lib.rs`.
- Mock Go apps are in `mock/` Go module. They use Go stdlib `net/http` for REST, `google.golang.org/grpc` for gRPC calls, and `flag` for CLI argument parsing.
- The parking-app-cli imports generated proto code from `gen/go/` for UPDATE_SERVICE and PARKING_OPERATOR_ADAPTOR gRPC calls.
- Integration tests requiring DATA_BROKER live in `tests/mock-apps/` and use the containerized Kuksa Databroker from `deployments/compose.yml`. Tests skip when DATA_BROKER is unavailable.
- Property tests for Go use table-driven tests with randomized inputs via `math/rand`.
- The mock PARKING_OPERATOR is the only long-lived server among the mock tools. It uses an in-memory store that resets on restart.
