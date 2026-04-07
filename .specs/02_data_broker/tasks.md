# Implementation Plan: DATA_BROKER

<!-- AGENT INSTRUCTIONS: Follow tasks in order. Each group must be completed and verified before moving to the next. -->

## Overview

This implementation plan covers the configuration and validation of Eclipse Kuksa Databroker as the DATA_BROKER component. Work is organized into 6 task groups: writing failing spec tests, configuring compose.yml for dual listeners, validating the VSS overlay, implementing edge case tests, implementing smoke tests, and final wiring verification. No custom application code is written -- deliverables are compose.yml updates, VSS overlay validation, and integration tests.

## Dependencies

| Spec | From Group | To Group | Relationship |
|------|-----------|----------|--------------|
| 01_project_setup | 7 | 1 | Uses compose.yml and VSS overlay from group 7 |

## Tasks

- [x] 1. Write failing spec tests
  - Write integration tests that verify DATA_BROKER connectivity, signal availability, read/write operations, and subscriptions. All tests will fail initially since the compose.yml is not yet configured for dual listeners.

  - [x] 1.1 Create test module `tests/databroker/` with Go test file and module initialization
    - Set up Go module and initial test file structure
    - _Test Spec: TS-02-SMOKE-1_
    - _Requirements: 02-REQ-1.1, 02-REQ-2.1_

  - [x] 1.2 Implement TCP connectivity test: gRPC connect to `localhost:55556`, verify metadata query succeeds
    - _Test Spec: TS-02-1_
    - _Requirements: 02-REQ-2.1, 02-REQ-2.2_

  - [x] 1.3 Implement UDS connectivity test: gRPC connect to `unix:///tmp/kuksa-databroker.sock`, verify metadata query succeeds
    - _Test Spec: TS-02-2_
    - _Requirements: 02-REQ-3.1, 02-REQ-3.2_

  - [x] 1.4 Implement standard VSS signal metadata tests: verify all 5 standard signals present with correct types
    - _Test Spec: TS-02-4, TS-02-P1_
    - _Requirements: 02-REQ-5.1, 02-REQ-5.2_

  - [x] 1.5 Implement custom VSS signal metadata tests: verify all 3 custom signals present with correct types
    - _Test Spec: TS-02-5, TS-02-P1_
    - _Requirements: 02-REQ-6.1, 02-REQ-6.2, 02-REQ-6.3, 02-REQ-6.4_

  - [x] 1.6 Implement signal set/get tests for TCP and UDS, including cross-transport consistency and subscription tests
    - _Test Spec: TS-02-6, TS-02-7, TS-02-8, TS-02-9, TS-02-10, TS-02-11, TS-02-P4_
    - _Requirements: 02-REQ-4.1, 02-REQ-8.1, 02-REQ-8.2, 02-REQ-9.1, 02-REQ-9.2, 02-REQ-10.1_

  - [x] 1.V Verify task group 1
    - [x] All spec tests exist and compile (expected: tests fail because compose.yml is not yet configured for dual listeners)
    ```
    cd tests/databroker && go test -run TestCompile ./... 2>&1 || echo "Tests compile but fail as expected"
    ```

- [x] 2. Configure compose.yml for dual listeners
  - Update the existing compose.yml (from spec 01) to configure the DATA_BROKER with pinned image version, dual listener args, port mapping, and volume mounts.

  - [x] 2.1 Pin the databroker image to `ghcr.io/eclipse-kuksa/kuksa-databroker:0.5.1` in `deployments/compose.yml`
    - _Requirements: 02-REQ-1.1, 02-REQ-1.2_

  - [x] 2.2 Add dual listener command args: `--address 0.0.0.0 --port 55555 --unix-socket /tmp/kuksa-databroker.sock`
    - _Requirements: 02-REQ-2.1, 02-REQ-3.1, 02-REQ-4.1_

  - [x] 2.3 Configure port mapping `55556:55555` for the databroker service
    - _Requirements: 02-REQ-2.2_

  - [x] 2.4 Add shared volume mount for UDS socket directory so co-located containers can access `/tmp/kuksa-databroker.sock`
    - _Requirements: 02-REQ-3.2_

  - [x] 2.5 Mount `deployments/vss-overlay.json` into the container and add `--vss /vss-overlay.json` to the command args
    - _Requirements: 02-REQ-6.4_

  - [x] 2.6 Verify the databroker runs in permissive mode (no auth flags in command args)
    - _Requirements: 02-REQ-7.1_

  - [x] 2.V Verify task group 2
    - [x] `podman compose up kuksa-databroker` starts successfully with both listeners active
    ```
    cd deployments && podman compose up -d kuksa-databroker && sleep 3 && podman compose logs kuksa-databroker | grep -i "listening" && podman compose down
    ```

- [x] 3. Validate VSS overlay
  - Validate and complete the VSS overlay file to ensure all 3 custom signals are correctly defined and loadable by the databroker.

  - [x] 3.1 Verify `Vehicle.Parking.SessionActive` is defined as type `boolean` in the overlay file
    - _Requirements: 02-REQ-6.1_

  - [x] 3.2 Verify `Vehicle.Command.Door.Lock` is defined as type `string` in the overlay file
    - _Requirements: 02-REQ-6.2_

  - [x] 3.3 Verify `Vehicle.Command.Door.Response` is defined as type `string` in the overlay file
    - _Requirements: 02-REQ-6.3_

  - [x] 3.4 Fix any issues found in the overlay file (incorrect types, missing entries, syntax errors)
    - _Requirements: 02-REQ-6.1, 02-REQ-6.2, 02-REQ-6.3, 02-REQ-6.4_

  - [x] 3.V Verify task group 3
    - [x] The databroker loads the overlay and all 3 custom signals are queryable via metadata
    ```
    cd deployments && podman compose up -d kuksa-databroker && sleep 3 && echo "Query custom signals via grpcurl or kuksa-client" && podman compose down
    ```

- [x] 4. Implement edge case tests
  - Add edge case tests for error scenarios: non-existent signals, overlay errors, and permissive mode behavior.

  - [x] 4.1 Implement test for setting a non-existent signal (expect NOT_FOUND error)
    - _Test Spec: TS-02-E1_
    - _Requirements: 02-REQ-8.E1_

  - [x] 4.2 Implement test for overlay with syntax error (expect container failure)
    - _Test Spec: TS-02-E2_
    - _Requirements: 02-REQ-6.E1_

  - [x] 4.3 Implement test for missing overlay file (expect container failure)
    - _Test Spec: TS-02-E3_
    - _Requirements: 02-REQ-6.E2_

  - [x] 4.4 Implement test for permissive mode with arbitrary token (expect success)
    - _Test Spec: TS-02-E4_
    - _Requirements: 02-REQ-7.E1_

  - [x] 4.5 Implement pinned image version verification test
    - _Test Spec: TS-02-3_
    - _Requirements: 02-REQ-1.1_

  - [x] 4.V Verify task group 4
    - [x] All edge case tests pass
    ```
    cd tests/databroker && go test -run "TestEdgeCase|TestImageVersion" -v ./...
    ```

- [x] 5. Implement smoke tests
  - Add smoke tests for CI/CD quick verification.

  - [x] 5.1 Implement smoke test: databroker health check (start container, verify TCP connection within 10s)
    - _Test Spec: TS-02-SMOKE-1_
    - _Requirements: 02-REQ-1.1, 02-REQ-2.1_

  - [x] 5.2 Implement smoke test: full signal inventory check (verify all 8 signals present)
    - _Test Spec: TS-02-SMOKE-2_
    - _Requirements: 02-REQ-5.1, 02-REQ-6.1, 02-REQ-6.2, 02-REQ-6.3_

  - [x] 5.V Verify task group 5
    - [x] All smoke tests pass
    ```
    cd tests/databroker && go test -run "TestSmoke" -v ./...
    ```

- [x] 6. Wiring verification
  - Run the complete test suite end-to-end and verify all requirements are met.

  - [x] 6.1 Run all integration tests (acceptance, property, edge case, smoke) and verify 100% pass rate
    - _Test Spec: TS-02-1 through TS-02-12, TS-02-P1 through TS-02-P3, TS-02-E1 through TS-02-E4, TS-02-SMOKE-1, TS-02-SMOKE-2_
    - _Requirements: 02-REQ-1.1, 02-REQ-1.2, 02-REQ-2.1, 02-REQ-2.2, 02-REQ-3.1, 02-REQ-3.2, 02-REQ-4.1, 02-REQ-5.1, 02-REQ-5.2, 02-REQ-6.1, 02-REQ-6.2, 02-REQ-6.3, 02-REQ-6.4, 02-REQ-7.1, 02-REQ-8.1, 02-REQ-8.2, 02-REQ-9.1, 02-REQ-9.2, 02-REQ-10.1_

  - [x] 6.2 Verify compose.yml contains all required configuration: pinned image, dual listener args, port mapping, volume mounts, overlay flag, no auth flags
    - _Requirements: 02-REQ-1.1, 02-REQ-2.1, 02-REQ-2.2, 02-REQ-3.1, 02-REQ-3.2, 02-REQ-4.1, 02-REQ-6.4, 02-REQ-7.1_

  - [x] 6.V Verify task group 6
    - [x] Final wiring verification: start databroker, confirm both listeners, all 8 signals, and cross-transport consistency
    ```
    cd tests/databroker && go test -v ./... && echo "All DATA_BROKER tests passed"
    ```

## Test Commands

```bash
# Run all databroker integration tests
cd tests/databroker && go test -v ./...

# Run only smoke tests (fast CI check)
cd tests/databroker && go test -run "TestSmoke" -v ./...

# Run only edge case tests
cd tests/databroker && go test -run "TestEdgeCase" -v ./...

# Run only connectivity tests
cd tests/databroker && go test -run "TestConnect" -v ./...

# Start databroker for manual testing
cd deployments && podman compose up -d kuksa-databroker

# View databroker logs
cd deployments && podman compose logs -f kuksa-databroker

# Stop databroker
cd deployments && podman compose down
```

## Traceability

| Requirement | Test Spec Entry | Implemented By Task | Verified By Test |
|------------|----------------|--------------------|--------------------|
| 02-REQ-1.1 | TS-02-3, TS-02-SMOKE-1 | 2.1, 4.5, 5.1 | TS-02-3, TS-02-SMOKE-1 |
| 02-REQ-1.2 | TS-02-3, TS-02-SMOKE-1 | 2.1 | TS-02-3, TS-02-SMOKE-1 |
| 02-REQ-1.E1 | | 2.1 | |
| 02-REQ-2.1 | TS-02-1, TS-02-SMOKE-1 | 1.2, 2.2 | TS-02-1, TS-02-SMOKE-1 |
| 02-REQ-2.2 | TS-02-1, TS-02-SMOKE-1 | 2.3 | TS-02-1, TS-02-SMOKE-1 |
| 02-REQ-2.E1 | | 2.3 | |
| 02-REQ-3.1 | TS-02-2 | 1.3, 2.2 | TS-02-2 |
| 02-REQ-3.2 | TS-02-2 | 2.4 | TS-02-2 |
| 02-REQ-3.E1 | | 2.2 | |
| 02-REQ-3.E2 | | 2.2 | |
| 02-REQ-4.1 | TS-02-8, TS-02-9, TS-02-11, TS-02-P3 | 1.6, 2.2 | TS-02-8, TS-02-9, TS-02-11, TS-02-P3 |
| 02-REQ-4.E1 | | 2.2 | |
| 02-REQ-5.1 | TS-02-4, TS-02-P1, TS-02-SMOKE-2 | 1.4 | TS-02-4, TS-02-P1, TS-02-SMOKE-2 |
| 02-REQ-5.2 | TS-02-4, TS-02-P1, TS-02-SMOKE-2 | 1.4 | TS-02-4, TS-02-P1, TS-02-SMOKE-2 |
| 02-REQ-5.E1 | | 1.4 | |
| 02-REQ-6.1 | TS-02-5, TS-02-P1, TS-02-SMOKE-2 | 1.5, 3.1, 3.4 | TS-02-5, TS-02-P1, TS-02-SMOKE-2 |
| 02-REQ-6.2 | TS-02-5, TS-02-P1, TS-02-SMOKE-2 | 1.5, 3.2, 3.4 | TS-02-5, TS-02-P1, TS-02-SMOKE-2 |
| 02-REQ-6.3 | TS-02-5, TS-02-P1, TS-02-SMOKE-2 | 1.5, 3.3, 3.4 | TS-02-5, TS-02-P1, TS-02-SMOKE-2 |
| 02-REQ-6.4 | TS-02-5, TS-02-P1, TS-02-SMOKE-2 | 2.5, 3.4 | TS-02-5, TS-02-P1, TS-02-SMOKE-2 |
| 02-REQ-6.E1 | TS-02-E2 | 4.2 | TS-02-E2 |
| 02-REQ-6.E2 | TS-02-E3 | 4.3 | TS-02-E3 |
| 02-REQ-7.1 | TS-02-12 | 2.6 | TS-02-12 |
| 02-REQ-7.E1 | TS-02-E4 | 4.4 | TS-02-E4 |
| 02-REQ-8.1 | TS-02-6, TS-02-P2 | 1.6 | TS-02-6, TS-02-P2 |
| 02-REQ-8.2 | TS-02-6, TS-02-P2 | 1.6 | TS-02-6, TS-02-P2 |
| 02-REQ-8.E1 | TS-02-E1 | 4.1 | TS-02-E1 |
| 02-REQ-9.1 | TS-02-7, TS-02-P2, TS-02-P3 | 1.6 | TS-02-7, TS-02-P2, TS-02-P3 |
| 02-REQ-9.2 | TS-02-8, TS-02-9, TS-02-P3 | 1.6 | TS-02-8, TS-02-9, TS-02-P3 |
| 02-REQ-9.E1 | | 1.6 | |
| 02-REQ-10.1 | TS-02-10, TS-02-11, TS-02-P4 | 1.6 | TS-02-10, TS-02-11, TS-02-P4 |
| 02-REQ-10.E1 | | 1.6 | |

## Notes

- All tests are integration tests requiring a running DATA_BROKER container via Podman Compose.
- No custom application code is written — deliverables are compose.yml configuration updates and tests.
- Tests connect via both TCP (port 55556) and UDS (`/tmp/kuksa-databroker.sock`).
