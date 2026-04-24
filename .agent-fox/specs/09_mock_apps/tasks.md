# Implementation Plan: Mock Apps

<!-- AGENT INSTRUCTIONS
- Implement exactly ONE top-level task group per session
- Task group 1 writes failing tests from test_spec.md — all subsequent groups
  implement code to make those tests pass
- Follow the git-flow: feature branch from main -> implement -> test -> merge to main -> push
- Update checkbox states as you go: [-] in progress, [x] complete
-->

## Overview

This plan implements the six mock tools: three Rust mock sensor binaries (`rhivos/mock-sensors/`) and three Go mock CLI apps (`mock/`). Task group 1 writes failing tests. Group 2 implements the Rust mock sensors (shared lib + three binaries). Group 3 implements the Go mock parking-operator server. Group 4 implements parking-app-cli and companion-app-cli. Group 5 runs integration smoke tests and wiring verification.

Ordering: tests first, then Rust sensors (simplest, no dependencies on other mock tools), then parking-operator (standalone server), then CLI apps (depend on proto definitions and service interfaces), then wiring verification.

## Test Commands

- Mock sensor unit tests: `cd rhivos && cargo test -p mock-sensors`
- Mock sensor binary tests: `cd rhivos && cargo test -p mock-sensors --test '*'`
- Go mock app unit tests: `cd mock && go test -v ./...`
- Integration tests: `cd tests/mock-apps && go test -v ./...`
- Rust linter: `cd rhivos && cargo clippy -p mock-sensors -- -D warnings`
- Go linter: `cd mock && go vet ./...`
- All tests: `make test`

## Tasks

- [x] 1. Write failing spec tests
  - [x] 1.1 Set up Rust test infrastructure for mock sensors
    - Ensure `rhivos/mock-sensors/` has `src/lib.rs` with module declarations and stub `publish_datapoint` function
    - Add dev-dependencies: `tokio` (test features)
    - Create argument validation tests for each sensor binary (exit code checks)
    - Include door-sensor mutual exclusion test (both --open and --closed)
    - _Test Spec: TS-09-E1, TS-09-E2, TS-09-E3, TS-09-E4, TS-09-E12_

  - [x] 1.2 Set up Go test infrastructure for mock apps
    - Create `tests/mock-apps/` Go module with test helpers (mock HTTP server, mock gRPC server, process runner)
    - Add `go.work` entry for `./tests/mock-apps`
    - _Test Spec: TS-09-5 through TS-09-17_

  - [x] 1.3 Write parking-operator unit tests
    - `TestStartSession` — TS-09-14: POST /parking/start returns session with UUID and rate
    - `TestStopSession` — TS-09-15: POST /parking/stop returns duration and total_amount
    - `TestSessionStatus` — TS-09-16: GET /parking/status returns session state
    - `TestStopUnknownSession` — TS-09-E7: POST /parking/stop with unknown session returns 404
    - `TestStatusUnknownSession` — TS-09-E8: GET /parking/status unknown returns 404
    - `TestMalformedRequest` — TS-09-E9: malformed body returns 400
    - _Test Spec: TS-09-14, TS-09-15, TS-09-16, TS-09-E7, TS-09-E8, TS-09-E9, TS-09-P5_

  - [x] 1.4 Write companion-app-cli tests
    - `TestLockCommand` — TS-09-11: lock sends correct POST with auth header
    - `TestUnlockCommand` — TS-09-12: unlock sends correct POST
    - `TestStatusCommand` — TS-09-13: status sends correct GET
    - `TestMissingToken` — TS-09-E5: exits 1 without token
    - `TestMissingVIN` — TS-09-E6: exits 1 without VIN
    - _Test Spec: TS-09-11, TS-09-12, TS-09-13, TS-09-E5, TS-09-E6, TS-09-P6_

  - [x] 1.5 Write parking-app-cli tests
    - `TestLookup` — TS-09-5: lookup queries PARKING_FEE_SERVICE
    - `TestAdapterInfo` — TS-09-6: adapter-info queries metadata
    - `TestInstall` — TS-09-7: install calls InstallAdapter RPC
    - `TestList` — TS-09-8: list calls ListAdapters RPC
    - `TestAdapterStatus` — TS-09-18: status calls GetAdapterStatus RPC
    - `TestRemove` — TS-09-19: remove calls RemoveAdapter RPC
    - `TestStartSession` — TS-09-9: start-session calls StartSession RPC
    - `TestStopSession` — TS-09-10: stop-session calls StopSession RPC
    - _Test Spec: TS-09-5, TS-09-6, TS-09-7, TS-09-8, TS-09-9, TS-09-10, TS-09-18, TS-09-19_

  - [x] 1.V Verify task group 1
    - [x] Rust test files compile: `cd rhivos && cargo test -p mock-sensors --no-run`
    - [x] Go test files compile: `cd tests/mock-apps && go test -c ./...`
    - [x] All spec tests FAIL (red phase)
    - [x] No linter warnings: `cd rhivos && cargo clippy -p mock-sensors -- -D warnings`

- [ ] 2. Rust mock sensors
  - [ ] 2.1 Vendor kuksa.val.v1 proto files
    - Copy kuksa.val.v1 proto files into `rhivos/mock-sensors/proto/`
    - Create `build.rs` for tonic-build code generation
    - Add dependencies: `tonic`, `prost`, `tokio`, `clap`
    - _Requirements: 09-REQ-10.1_

  - [ ] 2.2 Implement shared publish_datapoint helper
    - Implement `publish_datapoint(broker_addr, path, value)` in `src/lib.rs`
    - Connect to DATA_BROKER via tonic gRPC
    - Call kuksa.val.v1 `Set` RPC with target VSS path and value
    - Support `DatapointValue` enum: Double, Float, Bool
    - _Requirements: 09-REQ-10.2_

  - [ ] 2.3 Implement location-sensor binary
    - Parse `--lat`, `--lon`, `--broker-addr` with clap
    - Call `publish_datapoint` for Latitude and Longitude
    - Exit 0 on success, stderr + exit 1 on failure
    - _Requirements: 09-REQ-1.1, 09-REQ-1.2, 09-REQ-1.E1, 09-REQ-1.E2_

  - [ ] 2.4 Implement speed-sensor binary
    - Parse `--speed`, `--broker-addr` with clap
    - Call `publish_datapoint` for Vehicle.Speed
    - Exit 0 on success, stderr + exit 1 on failure
    - _Requirements: 09-REQ-2.1, 09-REQ-2.2, 09-REQ-2.E1, 09-REQ-2.E2_

  - [ ] 2.5 Implement door-sensor binary
    - Parse `--open`/`--closed`, `--broker-addr` with clap
    - Call `publish_datapoint` for IsOpen
    - Exit 0 on success, stderr + exit 1 on failure
    - _Requirements: 09-REQ-3.1, 09-REQ-3.2, 09-REQ-3.E1, 09-REQ-3.E2_

  - [ ] 2.V Verify task group 2
    - [ ] All sensor binaries build: `cd rhivos && cargo build -p mock-sensors`
    - [ ] Argument validation tests pass: `cd rhivos && cargo test -p mock-sensors`
    - [ ] All existing tests still pass: `cd rhivos && cargo test`
    - [ ] No linter warnings: `cd rhivos && cargo clippy -p mock-sensors -- -D warnings`
    - [ ] _Test Spec: TS-09-E1, TS-09-E2, TS-09-E3, TS-09-E4_

- [ ] 3. Mock parking-operator server
  - [ ] 3.1 Implement session store
    - In-memory `map[string]*Session` with mutex
    - UUID generation for session_id
    - Start: store session with status "active", hardcoded rate {per_hour, 2.50, EUR}
    - Stop: calculate duration = stop_timestamp - start_timestamp, total_amount = 2.50 * (duration / 3600.0)
    - _Requirements: 09-REQ-8.5, 09-REQ-8.2, 09-REQ-8.3_

  - [ ] 3.2 Implement HTTP handlers
    - `POST /parking/start` — parse JSON body, create session, return JSON
    - `POST /parking/stop` — parse JSON body, stop session, return JSON
    - `GET /parking/status/{session_id}` — lookup session, return JSON
    - Error responses: 404 for unknown sessions, 400 for malformed requests
    - _Requirements: 09-REQ-8.2, 09-REQ-8.3, 09-REQ-8.4, 09-REQ-8.E1, 09-REQ-8.E2, 09-REQ-8.E3_

  - [ ] 3.3 Implement serve subcommand
    - Parse `--port` flag with default 9090
    - Start HTTP server with graceful shutdown on SIGTERM/SIGINT
    - Log listening address to stderr
    - _Requirements: 09-REQ-8.1_

  - [ ] 3.V Verify task group 3
    - [ ] parking-operator builds: `cd mock && go build ./parking-operator/...`
    - [ ] Session management tests pass: `cd mock && go test -v ./parking-operator/...`
    - [ ] All existing tests still pass: `make test`
    - [ ] No linter warnings: `cd mock && go vet ./...`
    - [ ] _Test Spec: TS-09-14, TS-09-15, TS-09-16, TS-09-17, TS-09-E7, TS-09-E8, TS-09-E9_

- [ ] 4. Go mock CLI apps (parking-app-cli, companion-app-cli)
  - [ ] 4.1 Implement companion-app-cli
    - Subcommands: `lock`, `unlock`, `status`
    - HTTP client with bearer token from `--token` or `CLOUD_GATEWAY_TOKEN`
    - Target address from `--gateway-addr` or `CLOUD_GATEWAY_ADDR` (default: `http://localhost:8081`)
    - Print JSON response to stdout, errors to stderr
    - _Requirements: 09-REQ-7.1, 09-REQ-7.2, 09-REQ-7.3, 09-REQ-7.4, 09-REQ-7.5, 09-REQ-7.E1, 09-REQ-7.E2, 09-REQ-7.E3_

  - [ ] 4.2 Implement parking-app-cli REST subcommands
    - Subcommands: `lookup`, `adapter-info`
    - HTTP client targeting PARKING_FEE_SERVICE from `--service-addr` or `PARKING_FEE_SERVICE_ADDR` (default: `http://localhost:8080`)
    - Print JSON response to stdout, errors to stderr
    - _Requirements: 09-REQ-4.1, 09-REQ-4.2, 09-REQ-4.3, 09-REQ-4.E1, 09-REQ-4.E2_

  - [ ] 4.3 Implement parking-app-cli gRPC subcommands (UPDATE_SERVICE)
    - Subcommands: `install`, `watch`, `list`, `remove`, `status`
    - gRPC client targeting UPDATE_SERVICE from `--update-addr` or `UPDATE_SERVICE_ADDR` (default: `localhost:50052`)
    - Proto source: `proto/update_service.proto` (package `update_service.v1`, service `UpdateService`; see design.md § gRPC Proto Dependencies)
    - Generate Go stubs via `make proto` before building
    - `watch` streams events until EOF or SIGINT
    - Print responses to stdout, errors to stderr
    - _Requirements: 09-REQ-5.1, 09-REQ-5.2, 09-REQ-5.3, 09-REQ-5.4, 09-REQ-5.5, 09-REQ-5.6, 09-REQ-5.E1, 09-REQ-5.E2_

  - [ ] 4.4 Implement parking-app-cli session override subcommands
    - Subcommands: `start-session`, `stop-session`
    - gRPC client targeting PARKING_OPERATOR_ADAPTOR from `--adaptor-addr` or `ADAPTOR_ADDR` (default: `localhost:50053`)
    - Proto source: `proto/parking_adaptor.proto` (package `parking_adaptor.v1`, service `ParkingAdaptor`; see design.md § gRPC Proto Dependencies)
    - Generate Go stubs via `make proto` before building
    - Print responses to stdout, errors to stderr
    - _Requirements: 09-REQ-6.1, 09-REQ-6.2, 09-REQ-6.3, 09-REQ-6.E1_

  - [ ] 4.V Verify task group 4
    - [ ] Both CLIs build: `cd mock && go build ./parking-app-cli/... && go build ./companion-app-cli/...`
    - [ ] CLI tests pass: `cd mock && go test -v ./...`
    - [ ] All existing tests still pass: `make check` clean; integration tests in tests/mock-apps all pass
    - [ ] No linter warnings: `cd mock && go vet ./...`
    - [ ] _Test Spec: TS-09-5, TS-09-6, TS-09-7, TS-09-8, TS-09-9, TS-09-10, TS-09-11, TS-09-12, TS-09-13, TS-09-18, TS-09-19, TS-09-E5, TS-09-E6, TS-09-E10, TS-09-E11_

- [ ] 5. Wiring verification
  - [ ] 5.1 Run mock sensor integration tests against DATA_BROKER
    - Added TestLocationSensor, TestSpeedSensor, TestDoorSensorOpen, TestDoorSensorClosed
      to tests/mock-apps/sensor_test.go. Tests use a stub kuksa.val.v1 gRPC server
      (see docs/errata/09_mock_apps_sensor_proto_compat.md).
    - Also added TestSensorsUnreachableBroker and TestSensorSmoke.
    - _Test Spec: TS-09-1, TS-09-2, TS-09-3, TS-09-4, TS-09-SMOKE-1_

  - [ ] 5.2 Run parking-operator smoke test
    - Added TestParkingOperatorSmoke to tests/mock-apps/smoke_test.go:
      starts server binary, runs full start→stop lifecycle via HTTP, sends SIGTERM.
    - _Test Spec: TS-09-SMOKE-2_

  - [ ] 5.3 Run companion-app-cli smoke test against CLOUD_GATEWAY
    - Added TestCompanionAppSmoke to tests/mock-apps/smoke_test.go:
      lock → get command_id → status sequence against a mock CLOUD_GATEWAY.
    - _Test Spec: TS-09-SMOKE-3_

  - [ ] 5.4 Run property tests
    - Sensor publish-and-exit (TS-09-P1): Added TestSensorPublishProperty to
      tests/mock-apps/sensor_test.go with 10 lat/lon combos, 8 speed values,
      and 2 door states, verifying correct VSS paths and values across the
      input domain via stub kuksa.val.v1 gRPC server.
    - Sensor argument validation (TS-09-P2): Added TestSensorArgumentValidationProperty
      to tests/mock-apps/sensor_test.go systematically enumerating all missing-arg
      subsets for each sensor. Added TestGoCliArgumentValidationProperty to
      tests/mock-apps/parking_app_test.go covering all subcommand × missing-flag
      combinations for parking-app-cli and companion-app-cli.
    - Parking operator session integrity (TS-09-P3): Added TestSessionIntegrityProperty
      to mock/parking-operator/server_test.go with 10 timestamp/duration combinations.
    - Parking operator session uniqueness (TS-09-P4/P5): Already covered by
      TestSessionIDUniqueness and TestConcurrentSessionUniqueness in
      mock/parking-operator/server_test.go.
    - Bearer token enforcement (TS-09-P6): Already covered by TestBearerTokenProperty
      in tests/mock-apps/companion_test.go with 12 diverse token variations.
    - _Test Spec: TS-09-P1, TS-09-P2, TS-09-P3, TS-09-P4, TS-09-P5, TS-09-P6_

  - [ ] 5.V Verify task group 5
    - [ ] All integration tests pass: `cd tests/mock-apps && go test -v ./...`
    - [ ] All unit tests still pass: `cd rhivos && cargo test -p mock-sensors && cd mock && go test -v ./...`
    - [ ] All existing tests still pass: `make test`
    - [ ] All requirements 09-REQ-1 through 09-REQ-10 acceptance criteria met
          (sensor tests use stub v1 gRPC server; all tests pass)

- [ ] 6. Checkpoint - All Tests Green
  - All unit, integration, property, and smoke tests pass
  - All six binaries build and run correctly
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
| 09-REQ-1.1 | TS-09-1 | 2.3 | tests/mock-apps::TestLocationSensor |
| 09-REQ-1.2 | TS-09-1 | 2.3 | tests/mock-apps::TestLocationSensor |
| 09-REQ-1.E1 | TS-09-E1 | 2.3 | mock-sensors::test_location_sensor_missing_args |
| 09-REQ-1.E2 | TS-09-E4 | 2.3 | mock-sensors::test_sensor_unreachable_broker |
| 09-REQ-2.1 | TS-09-2 | 2.4 | tests/mock-apps::TestSpeedSensor |
| 09-REQ-2.2 | TS-09-2 | 2.4 | tests/mock-apps::TestSpeedSensor |
| 09-REQ-2.E1 | TS-09-E2 | 2.4 | mock-sensors::test_speed_sensor_missing_args |
| 09-REQ-2.E2 | TS-09-E4 | 2.4 | mock-sensors::test_sensor_unreachable_broker |
| 09-REQ-3.1 | TS-09-3, TS-09-4 | 2.5 | tests/mock-apps::TestDoorSensor |
| 09-REQ-3.2 | TS-09-3 | 2.5 | tests/mock-apps::TestDoorSensor |
| 09-REQ-3.E1 | TS-09-E3 | 2.5 | mock-sensors::test_door_sensor_missing_args |
| 09-REQ-3.E2 | TS-09-E4 | 2.5 | mock-sensors::test_sensor_unreachable_broker |
| 09-REQ-3.E3 | TS-09-E12 | 2.5 | mock-sensors::test_door_sensor_mutual_exclusion |
| 09-REQ-4.1 | TS-09-5 | 4.2 | tests/mock-apps::TestLookup |
| 09-REQ-4.2 | TS-09-6 | 4.2 | tests/mock-apps::TestAdapterInfo |
| 09-REQ-4.3 | TS-09-5 | 4.2 | tests/mock-apps::TestLookup |
| 09-REQ-4.E1 | TS-09-E1 | 4.2 | tests/mock-apps::TestLookupMissingArgs |
| 09-REQ-4.E2 | TS-09-E11 | 4.2 | tests/mock-apps::TestLookupHTTPError |
| 09-REQ-5.1 | TS-09-7 | 4.3 | tests/mock-apps::TestInstall |
| 09-REQ-5.2 | TS-09-8 | 4.3 | tests/mock-apps::TestList |
| 09-REQ-5.3 | TS-09-WATCH | 4.3 | tests/mock-apps::TestWatch |
| 09-REQ-5.4 | TS-09-18 | 4.3 | tests/mock-apps::TestAdapterStatus |
| 09-REQ-5.5 | TS-09-19 | 4.3 | tests/mock-apps::TestRemove |
| 09-REQ-5.6 | TS-09-7 | 4.3 | tests/mock-apps::TestInstall |
| 09-REQ-5.E1 | TS-09-E10 | 4.3 | tests/mock-apps::TestInstallMissingArgs |
| 09-REQ-5.E2 | TS-09-E10 | 4.3 | tests/mock-apps::TestInstallGRPCError |
| 09-REQ-6.1 | TS-09-9 | 4.4 | tests/mock-apps::TestStartSession |
| 09-REQ-6.2 | TS-09-10 | 4.4 | tests/mock-apps::TestStopSession |
| 09-REQ-6.3 | TS-09-9 | 4.4 | tests/mock-apps::TestStartSession |
| 09-REQ-6.E1 | TS-09-E10 | 4.4 | tests/mock-apps::TestSessionGRPCError |
| 09-REQ-7.1 | TS-09-11 | 4.1 | tests/mock-apps::TestLockCommand |
| 09-REQ-7.2 | TS-09-12 | 4.1 | tests/mock-apps::TestUnlockCommand |
| 09-REQ-7.3 | TS-09-13 | 4.1 | tests/mock-apps::TestStatusCommand |
| 09-REQ-7.4 | TS-09-11 | 4.1 | tests/mock-apps::TestLockCommand |
| 09-REQ-7.5 | TS-09-11 | 4.1 | tests/mock-apps::TestLockCommand |
| 09-REQ-7.E1 | TS-09-E6 | 4.1 | tests/mock-apps::TestMissingVIN |
| 09-REQ-7.E2 | TS-09-E5 | 4.1 | tests/mock-apps::TestMissingToken |
| 09-REQ-7.E3 | TS-09-E11 | 4.1 | tests/mock-apps::TestHTTPError |
| 09-REQ-8.1 | TS-09-17 | 3.3 | tests/mock-apps::TestGracefulShutdown |
| 09-REQ-8.2 | TS-09-14 | 3.1, 3.2 | mock/parking-operator::TestStartSession |
| 09-REQ-8.3 | TS-09-15 | 3.1, 3.2 | mock/parking-operator::TestStopSession |
| 09-REQ-8.4 | TS-09-16 | 3.2 | mock/parking-operator::TestSessionStatus |
| 09-REQ-8.5 | TS-09-14, TS-09-15 | 3.1 | mock/parking-operator::TestStartSession |
| 09-REQ-8.E1 | TS-09-E7 | 3.2 | mock/parking-operator::TestStopUnknownSession |
| 09-REQ-8.E2 | TS-09-E8 | 3.2 | mock/parking-operator::TestStatusUnknownSession |
| 09-REQ-8.E3 | TS-09-E9 | 3.2 | mock/parking-operator::TestMalformedRequest |
| 09-REQ-9.1 | TS-09-E1 through TS-09-E11 | all | error output tests |
| 09-REQ-9.2 | TS-09-E1 through TS-09-E11 | all | exit code tests |
| 09-REQ-9.3 | TS-09-1 through TS-09-17 | all | success exit code tests |
| 09-REQ-10.1 | TS-09-1 | 2.1 | mock-sensors build verification |
| 09-REQ-10.2 | TS-09-1, TS-09-2, TS-09-3 | 2.2 | sensor integration tests |
| Property 1 | TS-09-P1 | 5.4 | tests/mock-apps::TestSensorPublishProperty |
| Property 2 | TS-09-P2 | 5.4 | tests/mock-apps::TestSensorArgumentValidationProperty, tests/mock-apps::TestGoCliArgumentValidationProperty |
| Property 4 | TS-09-P3 | 5.4 | mock/parking-operator::TestSessionIntegrityProperty |
| Property 5 | TS-09-P4, TS-09-P5 | 5.4 | mock/parking-operator::TestSessionIDUniqueness, mock/parking-operator::TestConcurrentSessionUniqueness |
| Property 6 | TS-09-P6 | 5.4 | tests/mock-apps::TestBearerTokenProperty |

## Notes

- Mock sensors follow the per-crate proto vendoring pattern: kuksa.val.v1 protos are copied into `rhivos/mock-sensors/proto/` and compiled via `build.rs`.
- The `publish_datapoint` helper in `lib.rs` is shared across all three sensor binaries, keeping each `bin/*.rs` file minimal (arg parsing + one or two publish calls).
- The mock parking-operator uses Go stdlib `net/http` and `encoding/json` -- no external web framework needed for three endpoints.
- parking-app-cli connects to three different services (PARKING_FEE_SERVICE via REST, UPDATE_SERVICE via gRPC, PARKING_OPERATOR_ADAPTOR via gRPC). Each subcommand connects only to its target service.
- companion-app-cli uses bearer token auth. The token source priority is: `--token` flag > `CLOUD_GATEWAY_TOKEN` env var.
- Integration tests in `tests/mock-apps/` test each mock tool as a black box via subprocess execution, verifying stdout/stderr/exit-code contracts.
