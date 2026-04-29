# Implementation Plan: DATA_BROKER

<!-- AGENT INSTRUCTIONS: Follow tasks in order. Each group must be completed and verified before moving to the next. -->

## Overview

This implementation plan covers the configuration and validation of Eclipse Kuksa Databroker as the DATA_BROKER component. Work is organized into 6 task groups: writing failing spec tests, configuring compose.yml for dual listeners, validating the VSS overlay, implementing edge case tests, implementing smoke tests, and final wiring verification. No custom application code is written -- deliverables are compose.yml updates, VSS overlay validation, and integration tests.

## Dependencies

| Spec | From Group | To Group | Relationship |
|------|-----------|----------|--------------|
| 01_project_setup | 7 | 1 | Uses compose.yml and VSS overlay from group 7 |

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
cd deployments && podman compose up -d databroker

# View databroker logs
cd deployments && podman compose logs -f databroker

# Stop databroker
cd deployments && podman compose down
```

## Tasks

- [ ] 1. Write failing spec tests
  - Write integration tests that verify DATA_BROKER connectivity, signal availability, read/write operations, and subscriptions. All tests will fail initially since the compose.yml is not yet configured for dual listeners.

  - [ ] 1.1 Create test module `tests/databroker/` with Go test file and module initialization
    - Set up Go module and initial test file structure
    - Added tests/databroker/go.mod (module github.com/rhadp/parking-fee-service/tests/databroker, go 1.22)
    - Added ./tests/databroker to go.work workspace
    - Added cd tests/databroker to Makefile lint and check targets
    - _Test Spec: TS-02-SMOKE-1_
    - _Requirements: 02-REQ-1.1, 02-REQ-2.1_

  - [ ] 1.2 Implement TCP connectivity test: gRPC connect to `localhost:55556`, verify metadata query succeeds
    - TestTCPConnectivity in tests/databroker/signal_test.go; skips when container not running
    - _Test Spec: TS-02-1_
    - _Requirements: 02-REQ-2.1, 02-REQ-2.2_

  - [ ] 1.3 Implement UDS connectivity test: gRPC connect to `unix:///tmp/kuksa-databroker.sock`, verify metadata query succeeds
    - TestUDSConnectivity in tests/databroker/signal_test.go; effectiveUDSSocket() checks both /tmp/kuksa/ and /tmp/ paths
    - _Test Spec: TS-02-2_
    - _Requirements: 02-REQ-3.1, 02-REQ-3.2_

  - [ ] 1.4 Implement standard VSS signal metadata tests: verify all 5 standard signals present with correct types
    - TestStandardVSSSignalMetadata, TestPropertySignalCompleteness in tests/databroker/signal_test.go and property_test.go
    - _Test Spec: TS-02-4, TS-02-P1_
    - _Requirements: 02-REQ-5.1, 02-REQ-5.2_

  - [ ] 1.5 Implement custom VSS signal metadata tests: verify all 3 custom signals present with correct types
    - TestCustomVSSSignalMetadata in tests/databroker/signal_test.go; also covered by TestPropertySignalCompleteness
    - _Test Spec: TS-02-5, TS-02-P1_
    - _Requirements: 02-REQ-6.1, 02-REQ-6.2, 02-REQ-6.3, 02-REQ-6.4_

  - [ ] 1.6 Implement signal set/get tests for TCP and UDS, including cross-transport consistency and subscription tests
    - TCP: TestSignalSetGetViaTCP, TestPermissiveMode
    - UDS: TestSignalSetGetViaUDS
    - Cross-transport: TestCrossTransportTCPWriteUDSRead, TestCrossTransportUDSWriteTCPRead
    - Subscriptions: TestSubscriptionViaTCP, TestSubscriptionCrossTransport (pubsub_test.go)
    - Properties: TestPropertyWriteReadRoundtrip, TestPropertyCrossTransportEquivalence (property_test.go)
    - _Test Spec: TS-02-6, TS-02-7, TS-02-8, TS-02-9, TS-02-10, TS-02-11, TS-02-P4_
    - _Requirements: 02-REQ-4.1, 02-REQ-8.1, 02-REQ-8.2, 02-REQ-9.1, 02-REQ-9.2, 02-REQ-10.1_

  - [ ] 1.V Verify task group 1
    - [ ] All spec tests exist and compile (expected: tests fail because compose.yml is not yet configured for dual listeners)
    - Result: go test -c -o /dev/null ./... → PASS; TestComposeTCPListener, TestComposeUDSSocket, TestComposeUDSVolume FAIL (expected); all live gRPC tests SKIP (container not running); make check → PASS
    ```
    cd tests/databroker && go test -run TestCompile ./... 2>&1 || echo "Tests compile but fail as expected"
    ```

- [ ] 2. Configure compose.yml for dual listeners
  - Update the existing compose.yml (from spec 01) to configure the DATA_BROKER with pinned image version, dual listener args, port mapping, and volume mounts.

  - [ ] 2.1 Pin the databroker image to `ghcr.io/eclipse-kuksa/kuksa-databroker:0.6.1` in `deployments/compose.yml`
    - _Requirements: 02-REQ-1.1, 02-REQ-1.2_
    - Note: pinned to :0.6 per 02-REQ-1.1; container reports package version 0.6.1; see errata 02_databroker_cli_and_image.md §E02-1

  - [ ] 2.2 Add dual listener command args: `--address 0.0.0.0:55555 --uds-path /tmp/kuksa-databroker.sock`
    - _Requirements: 02-REQ-2.1, 02-REQ-3.1, 02-REQ-4.1_
    - Note: per errata, uses `--address 0.0.0.0 --port 55555` and `--unix-socket /tmp/kuksa-databroker.sock` (combined host:port and --uds-path are invalid for this binary)

  - [ ] 2.3 Configure port mapping `55556:55555` for the databroker service
    - _Requirements: 02-REQ-2.2_

  - [ ] 2.4 Add shared volume mount for UDS socket directory so co-located containers can access `/tmp/kuksa-databroker.sock`
    - _Requirements: 02-REQ-3.2_
    - Bind mount /tmp/kuksa (host) to /tmp (container); socket accessible at /tmp/kuksa/kuksa-databroker.sock on host

  - [ ] 2.5 Mount the VSS overlay file into the container and add the overlay flag to the command args
    - _Requirements: 02-REQ-6.4_
    - Uses `--vss vss_release_5.1.json,/vss-overlay.json` to load both the bundled VSS 5.1 tree and custom overlay; overlay file volume-mounted at /vss-overlay.json
    - Note: kuksa-databroker 0.6.1 uses `--vss` flag; comma-separated list loads multiple files; when `--vss` is explicit, the default tree must be included; bundled file is vss_release_5.1.json (not 4.0)

  - [ ] 2.6 Verify the databroker runs in permissive mode (no auth flags in command args)
    - _Requirements: 02-REQ-7.1_
    - Verified: no --token, --auth, --jwt, or --tls-server-cert flags in command; TestComposePermissiveMode PASS

  - [ ] 2.V Verify task group 2
    - [ ] All TestCompose* static tests pass (`go test -run TestCompose ./...` in tests/databroker — 7/7 pass)
    - [ ] tests/databroker added to GO_TEST_MODULES_RECURSIVE in Makefile; `make test` passes with databroker tests included
    - [ ] Errata created: docs/errata/02_databroker_cli_and_image.md documenting version discrepancy, CLI flag differences, VSS file format
    ```
    cd deployments && podman compose up -d databroker && sleep 3 && podman compose logs databroker | grep -i "listening" && podman compose down
    ```

- [ ] 3. Validate VSS overlay
  - Validate and complete the VSS overlay file to ensure all 3 custom signals are correctly defined and loadable by the databroker.

  - [ ] 3.1 Verify `Vehicle.Parking.SessionActive` is defined as type `boolean` in the overlay file
    - _Requirements: 02-REQ-6.1_
    - Verified: `deployments/vss-overlay.json` has `"datatype": "boolean"` for this signal

  - [ ] 3.2 Verify `Vehicle.Command.Door.Lock` is defined as type `string` in the overlay file
    - _Requirements: 02-REQ-6.2_
    - Verified: `deployments/vss-overlay.json` has `"datatype": "string"` for this signal

  - [ ] 3.3 Verify `Vehicle.Command.Door.Response` is defined as type `string` in the overlay file
    - _Requirements: 02-REQ-6.3_
    - Verified: `deployments/vss-overlay.json` has `"datatype": "string"` for this signal

  - [ ] 3.4 Fix any issues found in the overlay file (incorrect types, missing entries, syntax errors)
    - _Requirements: 02-REQ-6.1, 02-REQ-6.2, 02-REQ-6.3, 02-REQ-6.4_
    - Overlay file verified clean: no duplicate JSON keys, correct nested structure with branch nodes, all 3 custom signals with correct datatypes
    - Created `TestVSSOverlayFormat` in tests/databroker/overlay_test.go: navigates JSON tree structure, verifies signal datatypes at exact tree paths, branch node types, children fields, and leaf signal types (sensor/actuator/attribute)
    - Created `TestVSSOverlayNoDuplicateKeys` to detect duplicate JSON keys at any nesting depth (addresses critical review finding about silently overwritten duplicate keys)
    - Enhanced `TestStandardVSSSignalMetadata` and `TestCustomVSSSignalMetadata` with data-type validation via set/get roundtrip (addresses major review findings about missing type assertions)
    - Enhanced `TestPropertySignalCompleteness` with data-type validation via set/get roundtrip (addresses major review finding about missing type-correct checks)

  - [ ] 3.V Verify task group 3
    - [ ] Static tests pass: `TestVSSOverlayFormat` verifies all 3 custom signals present with correct types, branch nodes defined, and JSON valid (20 subtests pass including leaf_type, datatype, not_branch, description checks)
    - [ ] `TestVSSOverlayNoDuplicateKeys` passes: no duplicate JSON keys found at any nesting level
    - [ ] `make check` passes: all quality gates green
    ```
    cd deployments && podman compose up -d databroker && sleep 3 && echo "Query custom signals via grpcurl or kuksa-client" && podman compose down
    ```

- [ ] 4. Implement edge case tests
  - Add edge case tests for error scenarios: non-existent signals, overlay errors, and permissive mode behavior.

  - [ ] 4.1 Implement test for setting a non-existent signal (expect NOT_FOUND error)
    - _Test Spec: TS-02-E1_
    - _Requirements: 02-REQ-8.E1_
    - `TestEdgeCaseNonExistentSignal` in tests/databroker/edge_test.go; sends gRPC Set for `Vehicle.NonExistent.Signal`, asserts NOT_FOUND or INVALID_ARGUMENT error code; skips when TCP unreachable

  - [ ] 4.2 Implement test for overlay with syntax error (expect container failure)
    - _Test Spec: TS-02-E2_
    - _Requirements: 02-REQ-6.E1_
    - `TestEdgeCaseOverlaySyntaxError` in tests/databroker/edge_test.go; writes invalid JSON to overlay, runs `podman compose up -d kuksa-databroker` with 30s timeout; uses `assertContainerNotRunning` to positively verify container is not in "Up" state; cleanup restores overlay and calls `composeDown` with socket cleanup

  - [ ] 4.3 Implement test for missing overlay file (expect container failure)
    - _Test Spec: TS-02-E3_
    - _Requirements: 02-REQ-6.E2_
    - `TestEdgeCaseMissingOverlay` in tests/databroker/edge_test.go; renames overlay to .bak, runs `podman compose up -d kuksa-databroker` with 30s timeout; uses `assertContainerNotRunning` to positively verify container failed; cleanup calls `composeDown` with socket cleanup, removes any podman-created directory at overlay path, restores original file

  - [ ] 4.4 Implement test for permissive mode with arbitrary token (expect success)
    - _Test Spec: TS-02-E4_
    - _Requirements: 02-REQ-7.E1_
    - `TestPermissiveModeWithArbitraryToken` in tests/databroker/pubsub_test.go; dials TCP with insecure credentials, adds `Authorization: Bearer invalid-token-12345` metadata, calls Get(Vehicle.Speed), asserts no error; skips when TCP unreachable

  - [ ] 4.5 Implement pinned image version verification test
    - _Test Spec: TS-02-3_
    - _Requirements: 02-REQ-1.1_
    - Static: `TestComposePinnedImageVersion` in tests/databroker/edge_test.go; verifies compose.yml references `ghcr.io/eclipse-kuksa/kuksa-databroker:0.6` per 02-REQ-1.1
    - Live: `TestImageVersion` in tests/databroker/edge_test.go; runs `podman ps`, verifies running image is not :latest and contains kuksa-databroker; skips when TCP unreachable

  - [ ] 4.V Verify task group 4
    - [ ] All edge case tests compile and pass (SKIP when databroker unavailable; PASS for static checks and Podman-based tests)
    - Result: 12 static/Podman tests PASS; TestEdgeCaseNonExistentSignal, TestImageVersion, TestPermissiveModeWithArbitraryToken SKIP (TCP unreachable); TestEdgeCaseOverlaySyntaxError, TestEdgeCaseMissingOverlay PASS (Podman available); 0 FAIL; make check PASS
    - Added `assertContainerNotRunning` helper to address critical review finding (assertion gap when compose up returns nil error but container failed)
    - Added `composeDown` helper with UDS socket cleanup to prevent stale sockets from causing subsequent test failures
    ```
    cd tests/databroker && go test -run "TestEdgeCase|TestImageVersion" -v ./...
    ```

- [ ] 5. Implement smoke tests
  - Add smoke tests for CI/CD quick verification.

  - [ ] 5.1 Implement smoke test: databroker health check (start container, verify TCP connection within 10s)
    - _Test Spec: TS-02-SMOKE-1_
    - _Requirements: 02-REQ-1.1, 02-REQ-2.1_
    - `TestSmokeHealthCheck` in tests/databroker/smoke_test.go; uses `ensureDatabrokerRunning` helper to start container if not running, verifies gRPC metadata query returns populated entries for Vehicle.Speed, tears down via t.Cleanup
    - Fixed: compose.yml was using invalid `--metadata` flag; changed to `--vss vss_release_4.0.json,/vss-overlay.json` per kuksa-databroker 0.5.0 CLI (see errata §4)
    - Fixed: strengthened assertions to verify response content (non-empty entries) instead of only checking transport connectivity

  - [ ] 5.2 Implement smoke test: full signal inventory check (verify all 8 signals present)
    - _Test Spec: TS-02-SMOKE-2_
    - _Requirements: 02-REQ-5.1, 02-REQ-6.1, 02-REQ-6.2, 02-REQ-6.3_
    - `TestSmokeFullSignalInventory` in tests/databroker/smoke_test.go; uses `ensureDatabrokerRunning` to bootstrap container if needed, queries metadata for all 8 signals, reports missing signals with foundCount assertion
    - Fixed: now uses `ensureDatabrokerRunning` instead of `skipIfTCPUnreachable` so the test can actually run and verify signals (previously always SKIP)

  - [ ] 5.V Verify task group 5
    - [ ] All smoke tests compile and PASS with live container; skip gracefully when Podman unavailable
    - Result: TestSmokeHealthCheck PASS (started container, verified gRPC metadata with populated entries); TestSmokeFullSignalInventory PASS (all 8/8 signals found — 5 standard + 3 custom); 0 FAIL; make check EXIT_CODE: 0
    ```
    cd tests/databroker && go test -run "TestSmoke" -v ./...
    ```

- [ ] 6. Wiring verification

  - [ ] 6.1 Trace every execution path from design.md end-to-end
    - For each path, verify the entry point actually calls the next function
      in the chain (read the configuration, do not assume)
    - Confirm no configuration step is a placeholder (empty volume mount,
      commented-out flag) that was never completed
    - Every path must be live in the compose.yml -- errata or deferrals do not
      satisfy this check
    - _Requirements: all_

  - [ ] 6.2 Verify return values propagate correctly
    - For every compose.yml configuration that produces an observable effect
      consumed by another spec (port mapping, UDS socket path, VSS signals),
      confirm the downstream consumer references the correct value
    - _Requirements: all_

  - [ ] 6.3 Run the integration smoke tests
    - All `TS-02-SMOKE-*` tests pass using real components (no stub bypass)
    - _Test Spec: TS-02-SMOKE-1, TS-02-SMOKE-2_

  - [ ] 6.4 Stub / dead-code audit
    - Search all files touched by this spec for: commented-out configuration,
      `// TODO`, placeholder values, unused volume mounts
    - Each hit must be either: (a) justified with a comment explaining why it
      is intentional, or (b) replaced with a real configuration
    - Document any intentional placeholders here with rationale

  - [ ] 6.5 Cross-spec entry point verification
    - For each execution path whose entry point is owned by another spec
      (e.g., LOCKING_SERVICE connecting via UDS, CLOUD_GATEWAY_CLIENT
      connecting via TCP), grep the codebase to confirm the entry point is
      actually called from production code -- not just from tests
    - If the upstream caller does not exist, either implement it within this
      spec or file an issue and remove the path from design.md
    - _Requirements: all_

  - [ ] 6.V Verify wiring group
    - [ ] All smoke tests pass
    - [ ] No unjustified stubs remain in touched files
    - [ ] All execution paths from design.md are live (traceable in code)
    - [ ] All cross-spec entry points are called from production code
    - [ ] All existing tests still pass: `cd tests/databroker && go test -v ./...`

## Traceability

| Requirement | Test Spec Entry | Implemented By Task | Verified By Test |
|------------|----------------|--------------------|--------------------|
| 02-REQ-1.1 | TS-02-3, TS-02-SMOKE-1 | 2.1, 4.5, 5.1 | TS-02-3, TS-02-SMOKE-1 |
| 02-REQ-1.2 | TS-02-3, TS-02-SMOKE-1 | 2.1 | TS-02-3, TS-02-SMOKE-1 |
| 02-REQ-1.E1 | TS-02-E5 | 2.1 | TS-02-E5 |
| 02-REQ-2.1 | TS-02-1, TS-02-SMOKE-1 | 1.2, 2.2 | TS-02-1, TS-02-SMOKE-1 |
| 02-REQ-2.2 | TS-02-1, TS-02-SMOKE-1 | 2.3 | TS-02-1, TS-02-SMOKE-1 |
| 02-REQ-2.E1 | TS-02-E6 | 2.3 | TS-02-E6 |
| 02-REQ-3.1 | TS-02-2 | 1.3, 2.2 | TS-02-2 |
| 02-REQ-3.2 | TS-02-2 | 2.4 | TS-02-2 |
| 02-REQ-3.E1 | TS-02-E7 | 2.2 | TS-02-E7 |
| 02-REQ-3.E2 | TS-02-E8 | 2.2 | TS-02-E8 |
| 02-REQ-4.1 | TS-02-8, TS-02-9, TS-02-11, TS-02-P3 | 1.6, 2.2 | TS-02-8, TS-02-9, TS-02-11, TS-02-P3 |
| 02-REQ-4.E1 | TS-02-E9 | 2.2 | TS-02-E9 |
| 02-REQ-5.1 | TS-02-4, TS-02-P1, TS-02-SMOKE-2 | 1.4 | TS-02-4, TS-02-P1, TS-02-SMOKE-2 |
| 02-REQ-5.2 | TS-02-4, TS-02-P1, TS-02-SMOKE-2 | 1.4 | TS-02-4, TS-02-P1, TS-02-SMOKE-2 |
| 02-REQ-5.E1 | TS-02-E10 | 1.4 | TS-02-E10 |
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
| 02-REQ-9.E1 | TS-02-E11 | 1.6 | TS-02-E11 |
| 02-REQ-10.1 | TS-02-10, TS-02-11, TS-02-P4 | 1.6 | TS-02-10, TS-02-11, TS-02-P4 |
| 02-REQ-10.E1 | TS-02-E12 | 1.6 | TS-02-E12 |

## Notes

- All tests are integration tests requiring a running DATA_BROKER container via Podman Compose.
- No custom application code is written — deliverables are compose.yml configuration updates and tests.
- Tests connect via both TCP (port 55556) and UDS (`/tmp/kuksa-databroker.sock`).
