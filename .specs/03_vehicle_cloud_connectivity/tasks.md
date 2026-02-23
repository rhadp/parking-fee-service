# Implementation Plan: Vehicle-to-Cloud Connectivity (Phase 2.2)

<!-- AGENT INSTRUCTIONS
- Implement exactly ONE top-level task group per session
- Task group 1 writes failing tests from test_spec.md — all subsequent groups
  implement code to make those tests pass
- Follow the git-flow: feature branch from develop -> implement -> test -> merge to develop -> push
- Update checkbox states as you go: [-] in progress, [x] complete
-->

## Overview

This plan implements the vehicle-to-cloud connectivity layer: the CLOUD_GATEWAY
backend service, enhanced mock COMPANION_APP CLI, and integration tests. The
approach is test-first: task group 1 creates failing tests from `test_spec.md`,
then subsequent groups implement code to make those tests pass.

Ordering rationale:
1. Tests first (red) — establishes the verification baseline
2. CLOUD_GATEWAY core (config, MQTT client, command tracker) — foundation
3. CLOUD_GATEWAY REST API — depends on core modules
4. CLOUD_GATEWAY MQTT bridge — connects REST to MQTT
5. Mock COMPANION_APP CLI — depends on CLOUD_GATEWAY being functional
6. Integration testing — depends on all components being functional

## Test Commands

- Spec tests (all): `cd tests/cloud_connectivity && go test -v -count=1 ./...`
- Spec tests (unit only): `cd tests/cloud_connectivity && go test -v -count=1 -run "TestUnit" ./...`
- Spec tests (integration only): `cd tests/cloud_connectivity && go test -v -count=1 -run "TestIntegration" ./...`
- Spec tests (property only): `cd tests/cloud_connectivity && go test -v -count=1 -run "TestProperty" ./...`
- Spec tests (edge only): `cd tests/cloud_connectivity && go test -v -count=1 -run "TestEdge" ./...`
- CLOUD_GATEWAY unit tests: `cd backend/cloud-gateway && go test -v -count=1 ./...`
- CLOUD_GATEWAY integration tests: `cd backend/cloud-gateway && go test -v -count=1 -tags integration ./...`
- Mock CLI unit tests: `cd mock/companion-app-cli && go test -v -count=1 ./...`
- Linter (Go): `go vet ./backend/cloud-gateway/... ./mock/companion-app-cli/...`
- All tests: `make check`

## Tasks

- [x] 1. Write failing spec tests
  - [x] 1.1 Set up test module structure
    - Create `tests/cloud_connectivity/go.mod` as a standalone Go module
      (module path: `github.com/rhadp/parking-fee-service/tests/cloud_connectivity`)
    - Create a `helpers_test.go` with test helpers: `repoRoot(t)`,
      `execCommand`, `waitForPort`, `portIsOpen`, `httpGet`, `httpPost`,
      `startProcess`, `newMQTTTestClient`
    - _Test Spec: all (shared infrastructure)_

  - [x] 1.2 Write CLOUD_GATEWAY REST API tests
    - Translate TS-03-1 through TS-03-5 (REST endpoints) into Go tests
    - Group under `TestUnit_REST_*` naming convention
    - _Test Spec: TS-03-1, TS-03-2, TS-03-3, TS-03-4, TS-03-5_

  - [x] 1.3 Write CLOUD_GATEWAY MQTT bridge tests
    - Translate TS-03-6 through TS-03-10 (MQTT bridge) into Go tests
    - Translate TS-03-11 through TS-03-14 (command lifecycle) into Go tests
    - Group under `TestUnit_MQTT_*`, `TestUnit_Bridge_*`,
      `TestIntegration_MQTT_*` naming conventions
    - _Test Spec: TS-03-6 through TS-03-14_

  - [x] 1.4 Write mock COMPANION_APP CLI tests
    - Translate TS-03-15 through TS-03-21 (CLI commands) into Go tests
    - Group under `TestUnit_CLI_*` naming convention
    - _Test Spec: TS-03-15 through TS-03-21_

  - [x] 1.5 Write multi-vehicle and integration tests
    - Translate TS-03-22 through TS-03-26 (multi-vehicle, integration) into
      Go tests
    - Group under `TestIntegration_*`, `TestUnit_MultiVehicle_*` naming
      conventions
    - _Test Spec: TS-03-22 through TS-03-26_

  - [x] 1.6 Write edge case and property tests
    - Translate TS-03-E1 through TS-03-E10 into Go tests
    - Translate TS-03-P1 through TS-03-P7 into Go tests
    - Group under `TestEdge_*` and `TestProperty_*` naming conventions
    - _Test Spec: TS-03-E1 through TS-03-E10, TS-03-P1 through TS-03-P7_

  - [x] 1.V Verify task group 1
    - [x] All spec tests exist and are syntactically valid:
      `cd tests/cloud_connectivity && go vet ./...`
    - [x] All spec tests FAIL (red) — no implementation yet:
      `cd tests/cloud_connectivity && go test -count=1 ./... 2>&1 | grep -c FAIL`
    - [x] No linter warnings introduced:
      `cd tests/cloud_connectivity && go vet ./...`

- [x] 2. CLOUD_GATEWAY core modules
  - [x] 2.1 Create configuration module
    - Create `backend/cloud-gateway/internal/config/config.go`
    - Parse environment variables: `PORT`, `MQTT_BROKER_URL`, `MQTT_CLIENT_ID`,
      `COMMAND_TIMEOUT`, `AUTH_TOKEN`
    - Provide defaults as specified in design.md
    - Create `config_test.go` with tests for default values and env overrides
    - _Requirements: 03-REQ-2.1 (broker URL config)_

  - [x] 2.2 Create MQTT client module
    - Create `backend/cloud-gateway/internal/mqtt/client.go`
    - Implement MQTT client wrapper using `eclipse/paho.mqtt.golang`:
      `Connect()`, `Publish()`, `Subscribe()`, `Disconnect()`
    - Implement configurable reconnection with exponential backoff
    - Create `topics.go` with topic construction helpers:
      `CommandTopic(vin)`, `ResponseTopic(vin)`, `TelemetryTopic(vin)`
    - Create unit tests in `client_test.go` and `topics_test.go`
    - _Requirements: 03-REQ-2.1, 03-REQ-2.E1, 03-REQ-2.E2_

  - [x] 2.3 Create command tracker module
    - Create `backend/cloud-gateway/internal/bridge/tracker.go`
    - Implement in-memory pending command tracker:
      `Register(commandID) chan`, `Resolve(commandID, response) bool`,
      `HasPending(commandID) bool`
    - Implement configurable timeout per pending command
    - Thread-safe using `sync.Mutex`
    - Create unit tests in `tracker_test.go`
    - _Requirements: 03-REQ-2.5, 03-REQ-3.1, 03-REQ-3.2, 03-REQ-2.E3,
      03-REQ-3.E1, 03-REQ-3.E2, 03-REQ-5.2_

  - [x] 2.V Verify task group 2
    - [x] Core module unit tests pass:
      `cd backend/cloud-gateway && go test -v -count=1 ./internal/config/... ./internal/mqtt/... ./internal/bridge/...`
    - [x] No linter warnings:
      `cd backend/cloud-gateway && go vet ./...`
    - [x] Spec tests for tracker/bridge pass:
      `cd tests/cloud_connectivity && go test -v -count=1 -run "TestUnit_Bridge|TestProperty_P2|TestProperty_P5|TestProperty_P6|TestEdge_E6|TestEdge_E7" ./...`

- [x] 3. CLOUD_GATEWAY REST API
  - [x] 3.1 Create auth middleware
    - Create `backend/cloud-gateway/internal/api/middleware.go`
    - Implement bearer token validation middleware
    - Compare token against configured `AUTH_TOKEN`
    - Return 401 for missing, malformed, or invalid tokens
    - Exempt `/health` endpoint from auth
    - Create unit tests in `middleware_test.go`
    - _Requirements: 03-REQ-1.4_

  - [x] 3.2 Create command handler
    - Create `backend/cloud-gateway/internal/api/commands.go`
    - Implement `POST /vehicles/{vin}/commands` handler:
      - Parse and validate JSON body (command_id, type, doors)
      - Extract VIN from URL path
      - Map REST `type` to MQTT `action`
      - Register pending command in tracker
      - Publish command to MQTT via bridge
      - Wait for response (with timeout)
      - Return appropriate HTTP response
    - Create unit tests in `commands_test.go`
    - _Requirements: 03-REQ-1.1, 03-REQ-1.5, 03-REQ-1.E1, 03-REQ-1.E2_

  - [x] 3.3 Create status handler
    - Create `backend/cloud-gateway/internal/api/status.go`
    - Implement `GET /vehicles/{vin}/status` handler:
      - Extract VIN from URL path
      - Read telemetry cache
      - Return 404 if no telemetry available for VIN
      - Return 200 with cached telemetry
    - Create unit tests in `status_test.go`
    - _Requirements: 03-REQ-1.2_

  - [x] 3.4 Create HTTP router
    - Create `backend/cloud-gateway/internal/api/router.go`
    - Wire up all routes: `/health`, `/vehicles/{vin}/commands`,
      `/vehicles/{vin}/status`
    - Apply auth middleware to protected routes
    - Create unit tests in `router_test.go`
    - _Requirements: 03-REQ-1.3_

  - [x] 3.V Verify task group 3
    - [x] REST API unit tests pass:
      `cd backend/cloud-gateway && go test -v -count=1 ./internal/api/...`
    - [x] No linter warnings:
      `cd backend/cloud-gateway && go vet ./...`
    - [x] Spec REST tests pass:
      `cd tests/cloud_connectivity && go test -v -count=1 -run "TestUnit_REST|TestProperty_P3|TestEdge_E1|TestEdge_E2" ./...`

- [x] 4. CLOUD_GATEWAY MQTT bridge and main entry point
  - [x] 4.1 Create bridge module
    - Create `backend/cloud-gateway/internal/bridge/bridge.go`
    - Implement REST-to-MQTT bridge:
      - `SendCommand(vin, command)`: publish to MQTT, register pending
      - `HandleResponse(payload)`: parse MQTT response, resolve pending
      - `HandleTelemetry(payload)`: parse telemetry, update cache
    - Wire MQTT subscriptions to handler functions
    - Subscribe to wildcard topics: `vehicles/+/command_responses`,
      `vehicles/+/telemetry`
    - Create unit tests in `bridge_test.go`
    - _Requirements: 03-REQ-2.2, 03-REQ-2.3, 03-REQ-2.4, 03-REQ-3.3,
      03-REQ-3.4, 03-REQ-5.1_

  - [x] 4.2 Update main.go entry point
    - Update `backend/cloud-gateway/main.go`:
      - Load configuration
      - Initialize MQTT client with retry/backoff
      - Initialize bridge (tracker + MQTT + telemetry cache)
      - Initialize HTTP router with bridge dependency injection
      - Start HTTP server
      - Graceful shutdown on SIGINT/SIGTERM
    - Replace stub implementation from spec 01
    - _Requirements: 03-REQ-2.1, 03-REQ-2.E1_

  - [x] 4.V Verify task group 4
    - [x] All CLOUD_GATEWAY unit tests pass:
      `cd backend/cloud-gateway && go test -v -count=1 ./...`
    - [x] No linter warnings:
      `cd backend/cloud-gateway && go vet ./...`
    - [x] Spec bridge/MQTT tests pass:
      `cd tests/cloud_connectivity && go test -v -count=1 -run "TestUnit_Bridge|TestUnit_MQTT|TestProperty_P1|TestProperty_P4" ./...`
    - [x] CLOUD_GATEWAY starts and responds to health check:
      manually start and `curl http://localhost:8081/health`

- [ ] 5. Mock COMPANION_APP CLI enhancement
  - [ ] 5.1 Implement lock command
    - Update `mock/companion-app-cli/cmd/lock.go`
    - Generate UUID for command_id
    - Build JSON body: `{"command_id":"...","type":"lock","doors":["driver"]}`
    - Send POST to `<gateway-url>/vehicles/<vin>/commands`
    - Include `Authorization: Bearer <token>` header
    - Print response JSON to stdout on success
    - Print error to stderr and exit non-zero on failure
    - Create unit tests in `lock_test.go` using `httptest.Server`
    - _Requirements: 03-REQ-4.1, 03-REQ-4.4, 03-REQ-4.5, 03-REQ-4.6,
      03-REQ-4.7_

  - [ ] 5.2 Implement unlock command
    - Update `mock/companion-app-cli/cmd/unlock.go`
    - Same as lock but with `type: "unlock"`
    - Create unit tests in `unlock_test.go`
    - _Requirements: 03-REQ-4.2_

  - [ ] 5.3 Implement status command
    - Update `mock/companion-app-cli/cmd/status.go`
    - Send GET to `<gateway-url>/vehicles/<vin>/status`
    - Include `Authorization: Bearer <token>` header
    - Print response JSON to stdout on success
    - Print error to stderr and exit non-zero on failure
    - Create unit tests in `status_test.go`
    - _Requirements: 03-REQ-4.3_

  - [ ] 5.4 Add token validation
    - Update `mock/companion-app-cli/cmd/root.go`
    - Validate that `--token` flag is provided before executing any command
    - Print clear error message if missing
    - _Requirements: 03-REQ-4.E1, 03-REQ-4.E2_

  - [ ] 5.V Verify task group 5
    - [ ] CLI unit tests pass:
      `cd mock/companion-app-cli && go test -v -count=1 ./...`
    - [ ] No linter warnings:
      `cd mock/companion-app-cli && go vet ./...`
    - [ ] Spec CLI tests pass:
      `cd tests/cloud_connectivity && go test -v -count=1 -run "TestUnit_CLI|TestEdge_E8|TestEdge_E9" ./...`
    - [ ] CLI help still works:
      `go run ./mock/companion-app-cli --help`

- [ ] 6. Integration testing and final verification
  - [ ] 6.1 Write CLOUD_GATEWAY integration test
    - Create `backend/cloud-gateway/integration_test.go`
    - Test the full cycle: HTTP POST -> MQTT publish -> simulated subscriber
      -> MQTT response -> HTTP response
    - Gate on Mosquitto availability (skip if not running)
    - Test command correlation (command_id matching)
    - Test multi-vehicle routing
    - _Requirements: 03-REQ-6.1, 03-REQ-6.2, 03-REQ-6.3_

  - [ ] 6.2 Write end-to-end integration test with CLI
    - Create `tests/cloud_connectivity/e2e_test.go`
    - Start CLOUD_GATEWAY as a subprocess
    - Start simulated CLOUD_GATEWAY_CLIENT (MQTT subscriber)
    - Execute companion-app-cli commands and verify results
    - Gate on Mosquitto availability
    - _Requirements: 03-REQ-6.1, 03-REQ-6.3_

  - [ ] 6.3 Run full spec test suite and fix failures
    - Run all spec tests:
      `cd tests/cloud_connectivity && go test -v -count=1 ./...`
    - Run edge case tests:
      `cd tests/cloud_connectivity && go test -v -count=1 -run TestEdge ./...`
    - Run property tests:
      `cd tests/cloud_connectivity && go test -v -count=1 -run TestProperty ./...`
    - Fix any remaining failures
    - _Test Spec: all_

  - [ ] 6.4 Update Makefile for new test targets
    - Ensure `make test` includes CLOUD_GATEWAY tests
    - Ensure `make lint` includes CLOUD_GATEWAY and mock CLI
    - Add integration test target if not already covered
    - Verify `make check` passes
    - _Requirements: build system integration_

  - [ ] 6.V Verify task group 6
    - [ ] All integration tests pass (with Mosquitto running):
      `cd backend/cloud-gateway && go test -v -count=1 -tags integration ./...`
    - [ ] All spec tests pass:
      `cd tests/cloud_connectivity && go test -v -count=1 ./...`
    - [ ] All previous spec tests still pass (no regressions):
      `cd tests/setup && go test -v -count=1 ./...`
    - [ ] `make check` exits 0
    - [ ] No linter warnings
    - [ ] All changes committed and pushed

- [ ] 7. Checkpoint — connectivity layer complete
  - All CLOUD_GATEWAY REST endpoints are functional.
  - CLOUD_GATEWAY bridges REST to MQTT and back.
  - Mock COMPANION_APP CLI sends real commands.
  - Integration tests verify end-to-end flow.
  - Verify end-to-end:
    - `make infra-up`
    - Start CLOUD_GATEWAY: `go run ./backend/cloud-gateway`
    - In another terminal, subscribe to MQTT:
      `mosquitto_sub -t 'vehicles/+/commands'`
    - In another terminal, send lock command:
      `go run ./mock/companion-app-cli lock --vin VIN12345 --token demo-token`
    - Verify MQTT message appears in subscriber terminal
  - Ask the user if questions arise before ending the session.

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
| 03-REQ-1.1 | TS-03-1 | 3.2 | `TestUnit_REST_CommandEndpoint` |
| 03-REQ-1.2 | TS-03-2 | 3.3 | `TestUnit_REST_StatusEndpoint` |
| 03-REQ-1.3 | TS-03-3 | 3.4 | `TestUnit_REST_HealthEndpoint` |
| 03-REQ-1.4 | TS-03-4 | 3.1 | `TestUnit_REST_Unauthorized` |
| 03-REQ-1.5 | TS-03-5 | 3.2 | `TestUnit_REST_ValidCommandAccepted` |
| 03-REQ-1.E1 | TS-03-E1 | 3.2 | `TestEdge_MissingFields` |
| 03-REQ-1.E2 | TS-03-E2 | 3.2 | `TestEdge_InvalidJSON` |
| 03-REQ-2.1 | TS-03-6 | 2.2, 4.2 | `TestIntegration_MQTT_Connect` |
| 03-REQ-2.2 | TS-03-7 | 4.1 | `TestIntegration_MQTT_PublishCommand` |
| 03-REQ-2.3 | TS-03-8 | 4.1 | `TestIntegration_MQTT_SubscribeResponses` |
| 03-REQ-2.4 | TS-03-9 | 4.1 | `TestIntegration_MQTT_SubscribeTelemetry` |
| 03-REQ-2.5 | TS-03-10 | 2.3 | `TestUnit_Bridge_ResolvesPending` |
| 03-REQ-2.E1 | TS-03-E3 | 2.2, 4.2 | `TestEdge_MQTTUnreachableStartup` |
| 03-REQ-2.E2 | TS-03-E4 | 2.2 | `TestEdge_MQTTDisconnected` |
| 03-REQ-2.E3 | TS-03-E5 | 2.3 | `TestEdge_CommandTimeout` |
| 03-REQ-3.1 | TS-03-11 | 4.1 | `TestUnit_Bridge_CommandIDPreserved` |
| 03-REQ-3.2 | TS-03-12 | 2.3 | `TestUnit_Bridge_ResponseMatchedByID` |
| 03-REQ-3.3 | TS-03-13 | 4.1 | `TestUnit_Bridge_MQTTCommandSchema` |
| 03-REQ-3.4 | TS-03-14 | 4.1 | `TestUnit_Bridge_MQTTResponseSchema` |
| 03-REQ-3.E1 | TS-03-E6 | 2.3 | `TestEdge_UnknownCommandID` |
| 03-REQ-3.E2 | TS-03-E7 | 2.3 | `TestEdge_DuplicateCommandID` |
| 03-REQ-4.1 | TS-03-15 | 5.1 | `TestUnit_CLI_LockCommand` |
| 03-REQ-4.2 | TS-03-16 | 5.2 | `TestUnit_CLI_UnlockCommand` |
| 03-REQ-4.3 | TS-03-17 | 5.3 | `TestUnit_CLI_StatusCommand` |
| 03-REQ-4.4 | TS-03-18 | 5.1, 5.2, 5.3 | `TestUnit_CLI_BearerToken` |
| 03-REQ-4.5 | TS-03-19 | 5.1, 5.2, 5.3 | `TestUnit_CLI_VINInURL` |
| 03-REQ-4.6 | TS-03-20 | 5.1, 5.2, 5.3 | `TestUnit_CLI_SuccessOutput` |
| 03-REQ-4.7 | TS-03-21 | 5.1, 5.2, 5.3 | `TestUnit_CLI_ErrorOutput` |
| 03-REQ-4.E1 | TS-03-E8 | 5.4 | `TestEdge_MissingToken` |
| 03-REQ-4.E2 | TS-03-E9 | 5.4 | `TestEdge_GatewayUnreachable` |
| 03-REQ-5.1 | TS-03-22 | 4.1 | `TestIntegration_MultiVehicleRouting` |
| 03-REQ-5.2 | TS-03-23 | 2.3 | `TestUnit_MultiVehicle_ResponseIsolation` |
| 03-REQ-6.1 | TS-03-24 | 6.1, 6.2 | `TestIntegration_EndToEnd` |
| 03-REQ-6.2 | TS-03-25 | 6.1 | `TestIntegration_RunsWithGoTest` |
| 03-REQ-6.3 | TS-03-26 | 6.1 | `TestIntegration_CommandCorrelation` |
| 03-REQ-6.E1 | TS-03-E10 | 6.1 | `TestEdge_IntegrationSkipsWithoutMosquitto` |
| Property 1 | TS-03-P1 | 4.1 | `TestProperty_CommandIDPreservation` |
| Property 2 | TS-03-P2 | 2.3 | `TestProperty_ResponseCorrelation` |
| Property 3 | TS-03-P3 | 3.1 | `TestProperty_AuthEnforcement` |
| Property 4 | TS-03-P4 | 4.1 | `TestProperty_TopicRouting` |
| Property 5 | TS-03-P5 | 2.3 | `TestProperty_TimeoutGuarantee` |
| Property 6 | TS-03-P6 | 2.3 | `TestProperty_MultiVehicleIsolation` |
| Property 7 | TS-03-P7 | 2.2, 4.2 | `TestProperty_GracefulDegradation` |

## Notes

- **Test implementation language:** All spec tests are Go tests. Unit tests
  live within the component packages (`backend/cloud-gateway/internal/...`
  and `mock/companion-app-cli/cmd/...`). Integration and spec verification
  tests live in `tests/cloud_connectivity/`.
- **Integration tests require Mosquitto:** Tests that exercise the full
  REST-MQTT-REST cycle require Mosquitto running on localhost:1883 (`make
  infra-up`). They should skip gracefully if Mosquitto is not available.
- **MQTT client library:** Use `eclipse/paho.mqtt.golang` v1.4+ for the
  CLOUD_GATEWAY MQTT client. This is a well-maintained, production-grade Go
  MQTT client.
- **Dependency on spec 01:** This spec assumes the Go module skeleton
  (`backend/cloud-gateway/`) and mock CLI skeleton
  (`mock/companion-app-cli/`) from spec 01 already exist. If they do not,
  task group 2 should create the necessary directory structure.
- **Dependency on spec 02:** The integration test simulates
  CLOUD_GATEWAY_CLIENT behavior (subscribing to MQTT commands and publishing
  responses) without depending on the actual spec 02 Rust implementation.
- **Session sizing:** Each task group is scoped for one coding session.
  Groups 2 and 5 are the smallest; group 6 (integration testing) may require
  more time due to timing-sensitive MQTT tests.
