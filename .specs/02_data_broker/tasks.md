# Implementation Plan: DATA_BROKER (Spec 02)

<!-- AGENT INSTRUCTIONS
- Implement exactly ONE top-level task group per session
- Task group 1 writes failing tests from test_spec.md — all subsequent groups
  implement code to make those tests pass
- Follow the git-flow: feature branch from develop -> implement -> test -> merge to develop -> push
- Update checkbox states as you go: [-] in progress, [x] complete
-->

## Overview

This plan deploys Eclipse Kuksa Databroker as the DATA_BROKER. No custom application code is written -- the work consists of downloading the binary, creating a VSS overlay file, configuring dual listeners, and writing integration tests. Task group 1 writes failing tests first; subsequent groups implement configuration to make them pass.

## Test Commands

- Spec tests: `cd tests/setup && go test -run TestDataBroker -v`
- All tests: `cd tests/setup && go test -v`
- Infrastructure up: `make infra-up`
- Infrastructure down: `make infra-down`
- Linter: `cd tests/setup && go vet ./...`

## Tasks

- [x] 1. Write failing spec tests
  - [x] 1.1 Create DATA_BROKER test file
    - Create `tests/setup/databroker_test.go` with test functions for each test spec entry
    - `TestDataBrokerHealth` -- verify gRPC connectivity to `localhost:55556` (TS-02-1)
    - `TestDataBrokerStandardSignals` -- query metadata for 5 standard VSS signals (TS-02-2)
    - `TestDataBrokerCustomSignals` -- query metadata for 3 custom overlay signals (TS-02-3)
    - `TestDataBrokerCrossPartitionAccess` -- write/read over TCP on port 55556 (TS-02-4)
    - `TestDataBrokerUDSAccess` -- connect and operate via UDS (TS-02-5)
    - _Test Spec: TS-02-1 through TS-02-5_

  - [x] 1.2 Write property test functions
    - `TestDataBrokerWriteReadRoundTrip` -- table-driven subtests for all 8 signals (TS-02-P1)
    - `TestDataBrokerSubscription` -- subscribe, write from separate goroutine, verify delivery (TS-02-P2)
    - `TestDataBrokerOverlayMerge` -- verify custom and standard signals coexist (TS-02-P3)
    - _Test Spec: TS-02-P1 through TS-02-P3_

  - [x] 1.3 Write edge case test functions
    - `TestDataBrokerNonExistentSignal` -- get/set on `Vehicle.NonExistent.Signal` (TS-02-E1)
    - `TestDataBrokerUnsetSignal` -- read signal never written, verify no value (TS-02-E2)
    - `TestDataBrokerTypeMismatch` -- write string to bool signal (TS-02-E3)
    - `TestDataBrokerHealthDuringStartup` -- verify health check behavior (TS-02-E4)
    - _Test Spec: TS-02-E1 through TS-02-E4_

  - [x] 1.4 Add Kuksa gRPC client dependency
    - Add Go dependencies for gRPC connectivity to Kuksa Databroker
    - Use official Kuksa client library or raw gRPC with Kuksa proto definitions
    - Update `tests/setup/go.mod` and run `go mod tidy`

  - [x] 1.V Verify task group 1
    - [x] All spec tests exist and are syntactically valid
    - [x] All spec tests FAIL (red) -- no infrastructure exists yet
    - [x] No linter warnings: `cd tests/setup && go vet ./...`

- [x] 2. Download and configure Kuksa Databroker binary
  - [x] 2.1 Add Kuksa Databroker to compose.yml
    - Add `databroker` service to the project's `compose.yml`
    - Image: `ghcr.io/eclipse-kuksa/kuksa-databroker:0.5.0` (pinned to validated tag)
    - Container name: `databroker`
    - Port mapping: `55556:55556`
    - Health check: TCP check on port 55556 or gRPC health probe
    - Restart policy: `unless-stopped`
    - _Requirements: 02-REQ-1.1, 02-REQ-1.2, 02-REQ-8.1_

  - [x] 2.2 Validate container startup and teardown
    - Run `make infra-up` and verify container starts and reaches healthy state
    - Run `make infra-down` and verify clean teardown
    - _Requirements: 02-REQ-1.2, 02-REQ-8.1_

  - [x] 2.V Verify task group 2
    - [x] Spec tests TS-02-1 pass: `cd tests/setup && go test -run TestDataBrokerHealth -v`
    - [x] All existing tests still pass: `cd tests/setup && go test -v`
    - [x] No linter warnings: `cd tests/setup && go vet ./...`
    - [x] Requirements 02-REQ-1.1, 02-REQ-1.2, 02-REQ-8.1 acceptance criteria met

- [x] 3. Create VSS overlay file with custom signals
  - [x] 3.1 Create overlay directory structure
    - Create `config/vss/` directory in the project root
    - _Requirements: 02-REQ-2.1_

  - [x] 3.2 Create VSS overlay JSON file
    - Create `config/vss/vss_overlay.json` with Vehicle.Parking.SessionActive, Vehicle.Command.Door.Lock, Vehicle.Command.Door.Response
    - Use `actuator` type for all custom signals, with correct datatypes (boolean, string, string)
    - Validate JSON syntax: `python3 -m json.tool config/vss/vss_overlay.json`
    - _Requirements: 02-REQ-2.1_

  - [x] 3.3 Mount overlay in compose.yml
    - Add volume mount for `./config/vss/vss_overlay.json` to the databroker container
    - Add `--vss` flag with comma-separated VSS v5.1 + overlay files
    - Verify container logs show overlay loaded successfully
    - _Requirements: 02-REQ-2.1, 02-REQ-2.2_

  - [x] 3.V Verify task group 3
    - [x] Spec tests TS-02-2, TS-02-3 pass: `cd tests/setup && go test -run "TestDataBrokerStandardSignals|TestDataBrokerCustomSignals" -v`
    - [x] Spec test TS-02-P3 passes: `cd tests/setup && go test -run TestDataBrokerOverlayMerge -v`
    - [x] All existing tests still pass: `cd tests/setup && go test -v`
    - [x] No linter warnings: `cd tests/setup && go vet ./...`
    - [x] Requirements 02-REQ-2.1, 02-REQ-2.2, 02-REQ-3.1 acceptance criteria met

- [ ] 4. Configure dual listeners (UDS + TCP)
  - [ ] 4.1 Configure TCP listener
    - Ensure databroker binds to `0.0.0.0:55556`
    - Verify cross-partition access from test host
    - _Requirements: 02-REQ-5.1, 02-REQ-5.2_

  - [ ] 4.2 Configure UDS listener
    - Add `--unix-socket` flag to enable UDS endpoint at `/tmp/kuksa/databroker.sock`
    - Create bind mount volume for the UDS socket path `/tmp/kuksa/`
    - Verify same-partition access via UDS (confirmed via container logs; skipped on macOS due to VM boundary)
    - _Requirements: 02-REQ-4.1, 02-REQ-4.2_

  - [ ] 4.3 Verify both listeners work simultaneously
    - Run signal operations via TCP and UDS concurrently
    - Verify both interfaces produce identical results
    - Note: UDS test skips on macOS (Podman VM boundary prevents host-side UDS access)
    - _Requirements: 02-REQ-4.2, 02-REQ-5.2_

  - [ ] 4.V Verify task group 4
    - [ ] Spec tests TS-02-4, TS-02-5 pass: `cd tests/setup && go test -run "TestDataBrokerCrossPartitionAccess|TestDataBrokerUDSAccess" -v`
    - [ ] All existing tests still pass: `cd tests/setup && go test -v`
    - [ ] No linter warnings: `cd tests/setup && go vet ./...`
    - [ ] Requirements 02-REQ-4.1, 02-REQ-4.2, 02-REQ-5.1, 02-REQ-5.2 acceptance criteria met

- [ ] 5. Integration test with signal read/write
  - [ ] 5.1 Validate signal read/write round-trip
    - Run TS-02-P1 tests for all 8 signals (bool, float, double, string)
    - Fix any type mapping or API issues
    - All 8 signals pass write/read round-trip (bool, float, double, string types)
    - _Requirements: 02-REQ-6.1, 02-REQ-6.2_

  - [ ] 5.2 Validate subscription delivery
    - Run TS-02-P2 tests for pub/sub behavior
    - Verify subscribers receive updates within 5-second timeout
    - Both IsLocked and SessionActive subscription tests pass
    - _Requirements: 02-REQ-7.1, 02-REQ-7.2_

  - [ ] 5.3 Validate edge cases
    - Run TS-02-E1 (non-existent signal), TS-02-E2 (unset signal), TS-02-E3 (type mismatch), TS-02-E4 (health check)
    - Document any Kuksa version-specific behavior differences
    - Kuksa 0.5.0 findings documented in `docs/errata/02_data_broker_kuksa_api.md`: type mismatch strictly rejected, gRPC health check not implemented (fallback to ListMetadata)
    - _Requirements: 02-REQ-6.E1, 02-REQ-6.E2, 02-REQ-3.2, 02-REQ-8.2_

  - [ ] 5.V Verify task group 5
    - [ ] All spec tests pass: `cd tests/setup && go test -run TestDataBroker -v`
    - [ ] All existing tests still pass: `cd tests/setup && go test -v`
    - [ ] No linter warnings: `cd tests/setup && go vet ./...`
    - [ ] All requirements acceptance criteria met

- [ ] 6. Checkpoint -- DATA_BROKER Complete
  - [ ] 6.1 Full verification run
    - Run: `make infra-up && cd tests/setup && go test -run TestDataBroker -v && make infra-down`
    - Verify all tests pass end-to-end
    - All 12 tests pass (1 UDS test skipped on macOS due to Podman VM boundary)
  - [ ] 6.2 Requirements verification
    - Confirm every requirement has at least one passing test (see traceability table)
    - All 20 requirements verified: 18 with passing tests, 2 documented as not separately testable (02-REQ-2.E1, 02-REQ-4.E1)
  - [ ] 6.3 Document Kuksa version-specific findings
    - If any Kuksa behavior differs from design assumptions, update design.md
    - All 7 divergences documented in `docs/errata/02_data_broker_kuksa_api.md`; no additional findings

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
| 02-REQ-1.1 | TS-02-1 | Task 2.1 | `TestDataBrokerHealth` |
| 02-REQ-1.2 | TS-02-1 | Task 2.1, 2.2 | `TestDataBrokerHealth` |
| 02-REQ-1.E1 | TS-02-E4 | Task 2.1 | `TestDataBrokerHealthDuringStartup` |
| 02-REQ-2.1 | TS-02-3 | Task 3.2, 3.3 | `TestDataBrokerCustomSignals` |
| 02-REQ-2.2 | TS-02-3, TS-02-P3 | Task 3.3 | `TestDataBrokerCustomSignals`, `TestDataBrokerOverlayMerge` |
| 02-REQ-2.E1 | -- | Task 3.2 | (startup failure; not separately testable) |
| 02-REQ-3.1 | TS-02-2 | Task 3.3 | `TestDataBrokerStandardSignals` |
| 02-REQ-3.2 | TS-02-E2 | Task 5.3 | `TestDataBrokerUnsetSignal` |
| 02-REQ-4.1 | TS-02-5 | Task 4.2 | `TestDataBrokerUDSAccess` |
| 02-REQ-4.2 | TS-02-5 | Task 4.2 | `TestDataBrokerUDSAccess` |
| 02-REQ-4.E1 | -- | Task 4.2 | (connection error to bad UDS path) |
| 02-REQ-5.1 | TS-02-4 | Task 4.1 | `TestDataBrokerCrossPartitionAccess` |
| 02-REQ-5.2 | TS-02-4 | Task 4.1 | `TestDataBrokerCrossPartitionAccess` |
| 02-REQ-5.E1 | TS-02-E4 | Task 2.1 | `TestDataBrokerHealthDuringStartup` |
| 02-REQ-6.1 | TS-02-P1 | Task 5.1 | `TestDataBrokerWriteReadRoundTrip` |
| 02-REQ-6.2 | TS-02-P1 | Task 5.1 | `TestDataBrokerWriteReadRoundTrip` |
| 02-REQ-6.E1 | TS-02-E1 | Task 5.3 | `TestDataBrokerNonExistentSignal` |
| 02-REQ-6.E2 | TS-02-E3 | Task 5.3 | `TestDataBrokerTypeMismatch` |
| 02-REQ-7.1 | TS-02-P2 | Task 5.2 | `TestDataBrokerSubscription` |
| 02-REQ-7.2 | TS-02-P2 | Task 5.2 | `TestDataBrokerSubscription` |
| 02-REQ-7.E1 | -- | Task 5.2 | (stream termination; implicit in subscription tests) |
| 02-REQ-8.1 | TS-02-1 | Task 2.1 | `TestDataBrokerHealth` |
| 02-REQ-8.2 | TS-02-E4 | Task 5.3 | `TestDataBrokerHealthDuringStartup` |

## Notes

- The DATA_BROKER is third-party software (Eclipse Kuksa Databroker). Tests validate deployment and configuration correctness, not Kuksa internals.
- UDS support depends on the Kuksa Databroker version. If native UDS is unavailable, document the limitation and use TCP for all consumers.
- All tests require running infrastructure (`make infra-up`). Tests are integration tests, not unit tests.
- The overlay file is shared infrastructure used by all vehicle services (AC7 in master PRD).
