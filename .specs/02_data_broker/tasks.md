# Implementation Plan: DATA_BROKER

<!-- AGENT INSTRUCTIONS
- Implement exactly ONE top-level task group per session
- Task group 1 writes failing tests from test_spec.md — all subsequent groups
  implement code to make those tests pass
- Follow the git-flow: feature branch from main -> implement -> test -> merge to main -> push
- Update checkbox states as you go: [-] in progress, [x] complete
-->

## Overview

This plan configures Eclipse Kuksa Databroker for the SDV Parking Demo. No application code is written — deliverables are compose.yml updates and integration tests. Task group 1 writes failing integration tests. Task group 2 updates the compose.yml configuration. Task group 3 runs integration tests against the live databroker to validate all signals and pub/sub behavior.

The ordering ensures tests are written first (TDD), then configuration changes are made to pass them. All integration tests require a running Podman and databroker container; they skip gracefully when Podman is unavailable.

## Test Commands

- Spec tests: `cd tests/databroker && go test -v ./...`
- Config-only tests (no Podman): `cd tests/databroker && go test -v -run 'TestCompose' ./...`
- Integration tests (requires Podman): `cd tests/databroker && go test -v -run 'TestLive|TestSignal|TestPubSub|TestEdge|TestProperty' ./...`
- All tests: `make test`
- Linter: `cd tests/databroker && go vet ./...`

## Tasks

- [ ] 1. Write failing spec tests
  - [ ] 1.1 Create tests/databroker Go module
    - Create `tests/databroker/go.mod` with module path `github.com/rhadp/parking-fee-service/tests/databroker`
    - Add `go.work` entry for `./tests/databroker`
    - Create shared test helper `tests/databroker/helpers_test.go` with: Podman skip check, databroker start/stop, gRPC connect (TCP + UDS), compose.yml parser
    - _Test Spec: TS-02-1 through TS-02-17_

  - [ ] 1.2 Write compose configuration tests
    - Create `tests/databroker/compose_test.go` with tests for compose.yml parsing
    - `TestComposeTCPListener` — TS-02-1: verify port mapping and --address flag
    - `TestComposeUDSListener` — TS-02-2: verify --uds-path flag
    - `TestComposeUDSVolume` — TS-02-3: verify UDS volume mount
    - `TestComposeImageVersion` — TS-02-5: verify pinned image version
    - _Test Spec: TS-02-1, TS-02-2, TS-02-3, TS-02-5_

  - [ ] 1.3 Write live connectivity and signal metadata tests
    - Create `tests/databroker/signal_test.go` with tests requiring running databroker
    - `TestLiveDualListener` — TS-02-4: TCP + UDS connectivity
    - `TestSignalCustomSessionActive` — TS-02-6: custom signal metadata
    - `TestSignalCustomDoorLock` — TS-02-7: custom signal metadata
    - `TestSignalCustomDoorResponse` — TS-02-8: custom signal metadata
    - `TestSignalStandardIsLocked` — TS-02-10: standard signal metadata
    - `TestSignalStandardIsOpen` — TS-02-11: standard signal metadata
    - _Test Spec: TS-02-4, TS-02-6, TS-02-7, TS-02-8, TS-02-10, TS-02-11_

  - [ ] 1.4 Write signal set/get and pub/sub tests
    - Create `tests/databroker/pubsub_test.go`
    - `TestSignalStandardLatitude` — TS-02-12: standard signal metadata
    - `TestSignalStandardLongitude` — TS-02-13: standard signal metadata
    - `TestSignalStandardSpeed` — TS-02-14: standard signal metadata
    - `TestSignalCustomSetGet` — TS-02-9: set/get roundtrip
    - `TestPubSubNotification` — TS-02-15: subscription notification
    - `TestBooleanRoundtrip` — TS-02-16: boolean set/get
    - `TestStringJsonRoundtrip` — TS-02-17: JSON string set/get
    - _Test Spec: TS-02-9, TS-02-12, TS-02-13, TS-02-14, TS-02-15, TS-02-16, TS-02-17_

  - [ ] 1.5 Write edge case and property tests
    - Create `tests/databroker/edge_test.go`
    - `TestEdgeUDSSocketRestart` — TS-02-E1: restart with existing socket
    - `TestEdgeConcurrentTCPUDS` — TS-02-E2: simultaneous TCP + UDS clients
    - `TestEdgeMalformedOverlay` — TS-02-E3: bad JSON overlay
    - `TestEdgeGetUnsetSignal` — TS-02-E4: get unset custom signal
    - `TestEdgeNonExistentSignal` — TS-02-E5: query non-existent path
    - `TestEdgeSubscriberReconnect` — TS-02-E6: disconnect/reconnect
    - Create `tests/databroker/property_test.go`
    - `TestPropertyDualListenerAvailability` — TS-02-P1
    - `TestPropertyCustomSignalCompleteness` — TS-02-P2
    - `TestPropertyStandardSignalAvailability` — TS-02-P3
    - `TestPropertySetGetRoundtrip` — TS-02-P4
    - `TestPropertyPubSubDelivery` — TS-02-P5
    - _Test Spec: TS-02-E1 through TS-02-E6, TS-02-P1 through TS-02-P5_

  - [ ] 1.V Verify task group 1
    - [ ] All spec tests exist and are syntactically valid: `cd tests/databroker && go vet ./...`
    - [ ] Compose config tests FAIL (compose.yml not yet updated): `cd tests/databroker && go test -v -run 'TestCompose' ./...`
    - [ ] No linter warnings: `cd tests/databroker && go vet ./...`

- [ ] 2. Update compose.yml for dual listeners and version pinning
  - [ ] 2.1 Pin Kuksa Databroker image version
    - Update `deployments/compose.yml` databroker image from `latest` to `ghcr.io/eclipse-kuksa/kuksa-databroker:0.5.1`
    - _Requirements: 02-REQ-2.1_

  - [ ] 2.2 Configure dual listeners
    - Update databroker service `command` to include `--address 0.0.0.0:55555` and `--uds-path /tmp/kuksa-databroker.sock`
    - _Requirements: 02-REQ-1.1, 02-REQ-1.2_

  - [ ] 2.3 Add UDS volume mount
    - Add a named volume `kuksa-uds` with bind mount to `/tmp/kuksa` on the host
    - Mount the volume to `/tmp` in the databroker container
    - Create `/tmp/kuksa` host directory in Makefile `infra-up` target if needed
    - _Requirements: 02-REQ-1.3_

  - [ ] 2.V Verify task group 2
    - [ ] Compose config tests pass: `cd tests/databroker && go test -v -run 'TestCompose' ./...`
    - [ ] All existing tests still pass: `cd tests/setup && go test -v ./...`
    - [ ] No linter warnings: `cd tests/databroker && go vet ./...`
    - [ ] Requirements 02-REQ-1.1, 02-REQ-1.2, 02-REQ-1.3, 02-REQ-2.1 acceptance criteria met

- [ ] 3. Checkpoint - Configuration Complete
  - Verify compose.yml is valid: `podman compose -f deployments/compose.yml config`
  - Ensure all compose-parsing tests pass
  - Ask the user if questions arise

- [ ] 4. Integration test validation (live databroker)
  - [ ] 4.1 Verify dual listener connectivity
    - Start databroker: `make infra-up`
    - Run connectivity tests: `cd tests/databroker && go test -v -run 'TestLiveDualListener' ./...`
    - Fix any connection issues (socket path, port mapping)
    - _Requirements: 02-REQ-1.4_

  - [ ] 4.2 Verify signal metadata (custom + standard)
    - Run signal metadata tests: `cd tests/databroker && go test -v -run 'TestSignal' ./...`
    - Verify all 8 signals are accessible with correct datatypes
    - _Requirements: 02-REQ-3.1, 02-REQ-3.2, 02-REQ-3.3, 02-REQ-4.1, 02-REQ-4.2, 02-REQ-4.3, 02-REQ-4.4, 02-REQ-4.5_

  - [ ] 4.3 Verify set/get and pub/sub
    - Run pub/sub tests: `cd tests/databroker && go test -v -run 'TestPubSub|TestBoolean|TestString' ./...`
    - _Requirements: 02-REQ-3.4, 02-REQ-5.1, 02-REQ-5.2, 02-REQ-5.3_

  - [ ] 4.4 Verify edge cases and properties
    - Run edge case tests: `cd tests/databroker && go test -v -run 'TestEdge' ./...`
    - Run property tests: `cd tests/databroker && go test -v -run 'TestProperty' ./...`
    - Fix any failures
    - _Requirements: 02-REQ-1.E1, 02-REQ-1.E2, 02-REQ-3.E1, 02-REQ-3.E2, 02-REQ-4.E1, 02-REQ-5.E1_

  - [ ] 4.V Verify task group 4
    - [ ] All integration tests pass: `cd tests/databroker && go test -v ./...`
    - [ ] All existing tests still pass: `make test`
    - [ ] No linter warnings: `cd tests/databroker && go vet ./...`
    - [ ] All requirements 02-REQ-1 through 02-REQ-5 acceptance criteria met
    - [ ] `make infra-down` cleans up containers

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
| 02-REQ-1.1 | TS-02-1 | 2.2 | tests/databroker/compose_test.go::TestComposeTCPListener |
| 02-REQ-1.2 | TS-02-2 | 2.2 | tests/databroker/compose_test.go::TestComposeUDSListener |
| 02-REQ-1.3 | TS-02-3 | 2.3 | tests/databroker/compose_test.go::TestComposeUDSVolume |
| 02-REQ-1.4 | TS-02-4 | 4.1 | tests/databroker/signal_test.go::TestLiveDualListener |
| 02-REQ-1.E1 | TS-02-E1 | 4.4 | tests/databroker/edge_test.go::TestEdgeUDSSocketRestart |
| 02-REQ-1.E2 | TS-02-E2 | 4.4 | tests/databroker/edge_test.go::TestEdgeConcurrentTCPUDS |
| 02-REQ-2.1 | TS-02-5 | 2.1 | tests/databroker/compose_test.go::TestComposeImageVersion |
| 02-REQ-3.1 | TS-02-6 | 4.2 | tests/databroker/signal_test.go::TestSignalCustomSessionActive |
| 02-REQ-3.2 | TS-02-7 | 4.2 | tests/databroker/signal_test.go::TestSignalCustomDoorLock |
| 02-REQ-3.3 | TS-02-8 | 4.2 | tests/databroker/signal_test.go::TestSignalCustomDoorResponse |
| 02-REQ-3.4 | TS-02-9 | 4.3 | tests/databroker/pubsub_test.go::TestSignalCustomSetGet |
| 02-REQ-3.E1 | TS-02-E3 | 4.4 | tests/databroker/edge_test.go::TestEdgeMalformedOverlay |
| 02-REQ-3.E2 | TS-02-E4 | 4.4 | tests/databroker/edge_test.go::TestEdgeGetUnsetSignal |
| 02-REQ-4.1 | TS-02-10 | 4.2 | tests/databroker/signal_test.go::TestSignalStandardIsLocked |
| 02-REQ-4.2 | TS-02-11 | 4.2 | tests/databroker/signal_test.go::TestSignalStandardIsOpen |
| 02-REQ-4.3 | TS-02-12 | 4.2 | tests/databroker/pubsub_test.go::TestSignalStandardLatitude |
| 02-REQ-4.4 | TS-02-13 | 4.2 | tests/databroker/pubsub_test.go::TestSignalStandardLongitude |
| 02-REQ-4.5 | TS-02-14 | 4.2 | tests/databroker/pubsub_test.go::TestSignalStandardSpeed |
| 02-REQ-4.E1 | TS-02-E5 | 4.4 | tests/databroker/edge_test.go::TestEdgeNonExistentSignal |
| 02-REQ-5.1 | TS-02-15 | 4.3 | tests/databroker/pubsub_test.go::TestPubSubNotification |
| 02-REQ-5.2 | TS-02-16 | 4.3 | tests/databroker/pubsub_test.go::TestBooleanRoundtrip |
| 02-REQ-5.3 | TS-02-17 | 4.3 | tests/databroker/pubsub_test.go::TestStringJsonRoundtrip |
| 02-REQ-5.E1 | TS-02-E6 | 4.4 | tests/databroker/edge_test.go::TestEdgeSubscriberReconnect |
| Property 1 | TS-02-P1 | 4.4 | tests/databroker/property_test.go::TestPropertyDualListenerAvailability |
| Property 2 | TS-02-P2 | 4.4 | tests/databroker/property_test.go::TestPropertyCustomSignalCompleteness |
| Property 3 | TS-02-P3 | 4.4 | tests/databroker/property_test.go::TestPropertyStandardSignalAvailability |
| Property 4 | TS-02-P4 | 4.4 | tests/databroker/property_test.go::TestPropertySetGetRoundtrip |
| Property 5 | TS-02-P5 | 4.4 | tests/databroker/property_test.go::TestPropertyPubSubDelivery |

## Notes

- All tests live in `tests/databroker/` as a standalone Go module. Tests that require a running databroker container use a shared `TestMain` that handles `podman compose up/down`.
- Tests skip gracefully when Podman is not available (runtime detection via `exec.LookPath("podman")`).
- Compose config tests (TS-02-1, TS-02-2, TS-02-3, TS-02-5) parse the YAML file directly and do NOT require Podman. These are the only tests that can run without infrastructure.
- gRPC communication uses the `kuksa.val.v1` API. Tests may use `grpcurl` CLI (shelling out) or a generated Go gRPC client. The simpler `grpcurl` approach is preferred for this spec since no application code is being written.
- The UDS socket is accessible on the host at `/tmp/kuksa/kuksa-databroker.sock` via the bind-mounted volume.
- Edge case TS-02-E3 (malformed overlay) needs a temporary compose override or a separate compose file to avoid corrupting the real overlay. Use `t.Cleanup()` to restore state.
