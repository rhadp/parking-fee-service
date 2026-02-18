# Implementation Plan: CLOUD_GATEWAY + CLOUD_GATEWAY_CLIENT + Mock COMPANION_APP

<!-- AGENT INSTRUCTIONS
- Implement exactly ONE top-level task group per session
- Follow the git-flow: feature branch from develop -> implement -> test -> merge to develop -> push
- Update checkbox states as you go: [-] in progress, [x] complete
- CLOUD_GATEWAY (Go) and CLOUD_GATEWAY_CLIENT (Rust) can be developed in parallel
  after shared message schemas are defined (task group 1)
- Integration tests require ALL components running: Kuksa, Mosquitto,
  LOCKING_SERVICE, CLOUD_GATEWAY, CLOUD_GATEWAY_CLIENT
-->

## Overview

This plan implements the vehicle-to-cloud connectivity layer in dependency
order:

1. Shared message schemas and MQTT topics (foundation for both services).
2. CLOUD_GATEWAY REST API and state management (Go).
3. CLOUD_GATEWAY MQTT client integration (Go).
4. CLOUD_GATEWAY_CLIENT VIN management and registration (Rust).
5. CLOUD_GATEWAY_CLIENT command processing and result forwarding (Rust).
6. CLOUD_GATEWAY_CLIENT telemetry and status handling (Rust).
7. Vehicle pairing flow and mock COMPANION_APP CLI (Go).
8. End-to-end integration tests.

## Test Commands

- Go unit tests (cloud-gateway): `cd backend/cloud-gateway && go test ./...`
- Go unit tests (companion-app-cli): `cd mock/companion-app-cli && go test ./...`
- Rust unit tests: `cd rhivos && cargo test --workspace`
- Rust unit tests (cloud-gateway-client): `cd rhivos && cargo test -p cloud-gateway-client`
- All tests: `make test`
- Go linter: `cd backend/cloud-gateway && go vet ./...`
- Rust linter: `cd rhivos && cargo clippy --workspace -- -D warnings`
- All linters: `make lint`
- Build all: `make build`
- Integration tests: requires `make infra-up` + running services (see group 9)

## Tasks

- [x] 1. Shared Message Schemas
  - [x] 1.1 Define Go message types
    - Create `backend/cloud-gateway/messages/types.go`
    - Define structs for: `CommandMessage`, `CommandResponse`,
      `StatusRequest`, `StatusResponse`, `TelemetryMessage`,
      `RegistrationMessage`
    - Add JSON tags matching the design document schemas
    - Define MQTT topic pattern constants
    - _Requirements: 03-REQ-2.2 (prerequisite)_

  - [x] 1.2 Define Rust message types
    - Create `rhivos/cloud-gateway-client/src/messages.rs`
    - Define structs with `serde` derive for the same message types
    - Define MQTT topic pattern constants
    - _Requirements: 03-REQ-3.1 (prerequisite)_

  - [x] 1.3 Write schema compatibility tests
    - Go test: serialize a sample message to JSON, verify expected output
    - Rust test: serialize the same sample, verify identical JSON output
    - Ensures both sides agree on wire format
    - _Requirements: 03-REQ-2.2, 03-REQ-3.2 (prerequisite)_

  - [x] 1.V Verify task group 1
    - [x] `go test ./...` in cloud-gateway passes
    - [x] `cargo test -p cloud-gateway-client` passes
    - [x] Both sides produce identical JSON for the same logical message

- [ ] 2. CLOUD_GATEWAY Vehicle State and REST API
  - [ ] 2.1 Create vehicle state store
    - Create `backend/cloud-gateway/state/store.go`
    - Implement `Store` with thread-safe `VehicleEntry` map
    - Methods: `RegisterVehicle`, `GetVehicle`, `UpdateState`,
      `AddCommand`, `UpdateCommandResult`, `PairVehicle`, `ValidateToken`
    - Write unit tests for all methods including concurrent access
    - _Requirements: 03-REQ-5.3, 03-REQ-5.4, 03-REQ-5.5_

  - [ ] 2.2 Create auth middleware
    - Create `backend/cloud-gateway/api/middleware.go`
    - Extract `Authorization: Bearer` header
    - Validate token against state store for the target VIN
    - Return 401 for missing/invalid tokens
    - Write unit tests
    - _Requirements: 03-REQ-5.5, 03-REQ-1.E2_

  - [ ] 2.3 Create REST handlers
    - Create `backend/cloud-gateway/api/handlers.go`
    - Implement: `GET /healthz`, `POST /api/v1/pair`,
      `POST /api/v1/vehicles/{vin}/lock`,
      `POST /api/v1/vehicles/{vin}/unlock`,
      `GET /api/v1/vehicles/{vin}/status`
    - Lock/unlock: create command entry, return 202 with command_id
    - Status: return cached vehicle state
    - Pair: validate VIN + PIN, generate token, return token
    - Write unit tests using `httptest.Server`
    - **Property 3: Async Command Pattern**
    - **Property 4: Pairing Authorization**
    - _Requirements: 03-REQ-1.1, 03-REQ-1.2, 03-REQ-1.3, 03-REQ-1.4,
      03-REQ-1.5, 03-REQ-1.6, 03-REQ-5.4_

  - [ ] 2.4 Wire up main.go
    - Replace spec 01 skeleton with real implementation
    - Parse config (listen-addr, mqtt-addr)
    - Initialize state store, register routes, start HTTP server
    - Graceful shutdown on SIGINT/SIGTERM
    - _Requirements: 03-REQ-1.6_

  - [ ] 2.V Verify task group 2
    - [ ] `cd backend/cloud-gateway && go test ./...` passes
    - [ ] `cd backend/cloud-gateway && go vet ./...` clean
    - [ ] `go build` produces binary
    - [ ] REST endpoints return correct responses (tested via httptest)
    - [ ] Requirements 03-REQ-1.1–1.6, 03-REQ-5.3–5.5 acceptance criteria met

- [ ] 3. CLOUD_GATEWAY MQTT Client
  - [ ] 3.1 Create MQTT client module
    - Create `backend/cloud-gateway/mqtt/client.go`
    - Connect to Mosquitto with auto-reconnect
    - Subscribe to: `vehicles/+/command_responses` (QoS 2),
      `vehicles/+/telemetry` (QoS 0), `vehicles/+/registration` (QoS 2),
      `vehicles/+/status_response` (QoS 2)
    - Publish method for commands and status requests (QoS 2)
    - _Requirements: 03-REQ-2.1, 03-REQ-2.5_

  - [ ] 3.2 Create MQTT message handlers
    - Create `backend/cloud-gateway/mqtt/handlers.go`
    - On `command_responses`: parse JSON, find command in state store by
      command_id, update result
    - On `telemetry`: parse JSON, update vehicle cached state
    - On `registration`: parse JSON, register vehicle in state store
    - On `status_response`: parse JSON, update vehicle cached state
    - Write unit tests for each handler
    - _Requirements: 03-REQ-2.3, 03-REQ-2.4, 03-REQ-5.3_

  - [ ] 3.3 Integrate MQTT with REST handlers
    - Lock/unlock handlers: after creating command entry, publish
      `CommandMessage` to MQTT
    - Status handler: return cached state (updated by telemetry/status
      response MQTT handlers)
    - Error handling: if MQTT publish fails, return 503
    - _Requirements: 03-REQ-2.2, 03-REQ-1.E3_

  - [ ] 3.4 Write MQTT integration tests
    - Connect to real Mosquitto (`make infra-up`)
    - Verify publish/subscribe round-trip
    - Verify QoS levels
    - Tests are skipped if Mosquitto is unavailable
    - **Property 6: QoS Compliance**
    - _Requirements: 03-REQ-2.1, 03-REQ-2.2_

  - [ ] 3.V Verify task group 3
    - [ ] `cd backend/cloud-gateway && go test ./...` passes (unit tests)
    - [ ] MQTT integration tests pass with `make infra-up`
    - [ ] `go vet ./...` clean
    - [ ] Requirements 03-REQ-2.1–2.5 acceptance criteria met

- [ ] 4. Checkpoint — CLOUD_GATEWAY Complete
  - REST API and MQTT client working
  - Commit and verify clean state

- [ ] 5. CLOUD_GATEWAY_CLIENT VIN and Registration
  - [ ] 5.1 Create VIN management module
    - Create `rhivos/cloud-gateway-client/src/vin.rs`
    - Implement `generate_vin()` → 17-char VIN starting with "DEMO"
    - Implement `generate_pairing_pin()` → 6-digit numeric string
    - Implement `load_or_create(data_dir)` → reads existing or generates new
    - Persist to `{data_dir}/vin.json`
    - Write unit tests for generation and persistence
    - **Property 7: VIN Persistence**
    - _Requirements: 03-REQ-5.1, 03-REQ-5.E3_

  - [ ] 5.2 Create config module
    - Create `rhivos/cloud-gateway-client/src/config.rs`
    - Parse CLI flags and env vars: `--mqtt-addr`, `--databroker-addr`,
      `--data-dir`, `--telemetry-interval`
    - _Requirements: 03-REQ-3.6, 03-REQ-4.3_

  - [ ] 5.3 Create MQTT client wrapper
    - Create `rhivos/cloud-gateway-client/src/mqtt.rs`
    - Connect to Mosquitto using `rumqttc` with auto-reconnect
    - Subscribe to `vehicles/{vin}/commands` (QoS 2) and
      `vehicles/{vin}/status_request` (QoS 2)
    - Publish registration message on startup
    - _Requirements: 03-REQ-3.1, 03-REQ-5.2_

  - [ ] 5.4 Write VIN and MQTT tests
    - Unit tests: VIN generation uniqueness, persistence round-trip,
      PIN format
    - MQTT integration test (`#[ignore]`): connect to Mosquitto, publish
      registration, verify message received
    - _Requirements: 03-REQ-5.1, 03-REQ-5.2_

  - [ ] 5.V Verify task group 5
    - [ ] `cargo test -p cloud-gateway-client` passes
    - [ ] VIN file is created and reused across restarts
    - [ ] Registration message published to MQTT
    - [ ] `cargo clippy -p cloud-gateway-client -- -D warnings` clean

- [ ] 6. CLOUD_GATEWAY_CLIENT Command Processing
  - [ ] 6.1 Create command handler
    - Create `rhivos/cloud-gateway-client/src/command_handler.rs`
    - Parse incoming MQTT `CommandMessage` JSON
    - Write `Vehicle.Command.Door.Lock` to DATA_BROKER via Kuksa client
    - Store current `command_id` for response correlation
    - Handle invalid JSON gracefully (log and discard)
    - _Requirements: 03-REQ-3.2, 03-REQ-3.3, 03-REQ-3.E3_

  - [ ] 6.2 Create result forwarder
    - Create `rhivos/cloud-gateway-client/src/result_forwarder.rs`
    - Subscribe to `Vehicle.Command.Door.LockResult` on DATA_BROKER
    - On result change: construct `CommandResponse` JSON with the stored
      `command_id` and publish to MQTT `command_responses` topic (QoS 2)
    - _Requirements: 03-REQ-3.4_

  - [ ] 6.3 Create status handler
    - Create `rhivos/cloud-gateway-client/src/status_handler.rs`
    - On receiving MQTT `StatusRequest`: read all signals from DATA_BROKER,
      construct `StatusResponse` JSON, publish to MQTT `status_response`
      topic (QoS 2)
    - Handle missing signals gracefully (null/default)
    - _Requirements: 03-REQ-3.5_

  - [ ] 6.4 Write command processing tests
    - Unit tests with mock MQTT and Kuksa clients:
      - Lock command → Kuksa write with correct value
      - Unlock command → Kuksa write with correct value
      - LockResult change → MQTT response published with correct command_id
      - Status request → DATA_BROKER read → MQTT response
      - Invalid command JSON → discarded, no Kuksa write
    - **Property 1: Command Delivery**
    - **Property 2: Result Propagation**
    - _Requirements: 03-REQ-3.2, 03-REQ-3.3, 03-REQ-3.4, 03-REQ-3.5_

  - [ ] 6.V Verify task group 6
    - [ ] `cargo test -p cloud-gateway-client` passes all tests
    - [ ] `cargo clippy -p cloud-gateway-client -- -D warnings` clean
    - [ ] Requirements 03-REQ-3.1–3.6 acceptance criteria met

- [ ] 7. CLOUD_GATEWAY_CLIENT Telemetry
  - [ ] 7.1 Create telemetry publisher
    - Create `rhivos/cloud-gateway-client/src/telemetry.rs`
    - Background task on configurable timer (default 5s)
    - Read vehicle signals from DATA_BROKER: `IsLocked`, `IsOpen`, `Speed`,
      `Latitude`, `Longitude`, `ParkingSessionActive`
    - Construct `TelemetryMessage` JSON
    - Publish to `vehicles/{vin}/telemetry` (QoS 0)
    - Handle missing signals (omit or null)
    - _Requirements: 03-REQ-4.1, 03-REQ-4.2, 03-REQ-4.3_

  - [ ] 7.2 Update main.rs
    - Replace spec 01 skeleton with real implementation
    - Initialize: config → VIN → MQTT → Kuksa → spawn command handler,
      result forwarder, status handler, telemetry publisher
    - Graceful shutdown on SIGINT/SIGTERM
    - Connection retry with exponential backoff for both MQTT and Kuksa
    - _Requirements: 03-REQ-3.E1, 03-REQ-3.E2_

  - [ ] 7.3 Write telemetry tests
    - Unit test: mock Kuksa client returning known values → verify JSON
      output matches expected schema and values
    - Unit test: missing signals → verify null/default handling
    - Integration test (`#[ignore]`): start with real Kuksa and Mosquitto,
      verify telemetry messages arrive at expected interval
    - **Property 5: Telemetry Accuracy**
    - _Requirements: 03-REQ-4.1, 03-REQ-4.2, 03-REQ-4.E1_

  - [ ] 7.V Verify task group 7
    - [ ] `cargo test -p cloud-gateway-client` passes
    - [ ] Telemetry integration test passes with `make infra-up`
    - [ ] `cargo clippy -p cloud-gateway-client -- -D warnings` clean
    - [ ] Requirements 03-REQ-4.1–4.3 acceptance criteria met

- [ ] 8. Checkpoint — CLOUD_GATEWAY_CLIENT Complete
  - VIN, MQTT, command processing, result forwarding, telemetry all working
  - Commit and verify clean state

- [ ] 9. Mock COMPANION_APP CLI and Pairing
  - [ ] 9.1 Implement `pair` subcommand
    - Replace spec 01 skeleton `mock/companion-app-cli/main.go`
    - `pair --vin VIN --pin PIN --gateway-addr ADDR`
    - POST /api/v1/pair with JSON body, print returned token
    - _Requirements: 03-REQ-6.1, 03-REQ-6.5_

  - [ ] 9.2 Implement `lock`, `unlock`, `status` subcommands
    - `lock --vin VIN --token TOKEN --gateway-addr ADDR`
    - `unlock --vin VIN --token TOKEN --gateway-addr ADDR`
    - `status --vin VIN --token TOKEN --gateway-addr ADDR`
    - Each sends the appropriate REST request with bearer token, prints
      response
    - _Requirements: 03-REQ-6.2, 03-REQ-6.3, 03-REQ-6.4, 03-REQ-6.5_

  - [ ] 9.3 Add error handling
    - Gateway unreachable → error message + non-zero exit
    - 401/403/404 responses → meaningful error messages
    - _Requirements: 03-REQ-6.E1_

  - [ ] 9.4 Write CLI tests
    - Unit tests: argument parsing for each subcommand
    - Unit tests: HTTP request construction (method, path, headers, body)
      using `httptest.Server`
    - _Requirements: 03-REQ-6.1–6.5_

  - [ ] 9.V Verify task group 9
    - [ ] `cd mock/companion-app-cli && go test ./...` passes
    - [ ] `companion-app-cli --help` shows all subcommands
    - [ ] `go vet ./...` clean
    - [ ] Requirements 03-REQ-6.1–6.5 acceptance criteria met

- [ ] 10. Integration Tests
  - [ ] 10.1 Create integration test harness
    - Script or test file that starts all required services:
      `make infra-up` (Kuksa + Mosquitto), LOCKING_SERVICE,
      CLOUD_GATEWAY, CLOUD_GATEWAY_CLIENT
    - Wait for all services to be ready (health checks, log markers)
    - Clean up on test completion
    - _Requirements: 03-REQ-7.E1_

  - [ ] 10.2 Test pairing flow
    - Start CLOUD_GATEWAY_CLIENT → extract VIN and PIN from logs
    - Verify CLOUD_GATEWAY received registration
    - Call `POST /api/v1/pair` with correct VIN and PIN → verify token
    - Call with wrong PIN → verify 403
    - **Property 4: Pairing Authorization**
    - _Requirements: 03-REQ-7.1_

  - [ ] 10.3 Test lock command end-to-end
    - Set safe vehicle conditions via mock-sensors
    - Send lock via REST → verify 202 with command_id
    - Wait for command to propagate through MQTT → Kuksa → LOCKING_SERVICE
    - Read IsLocked from Kuksa → verify true
    - GET /status → verify is_locked = true, last_command.result = SUCCESS
    - **Property 1: Command Delivery**
    - **Property 2: Result Propagation**
    - _Requirements: 03-REQ-7.2, 03-REQ-7.3, 03-REQ-7.4_

  - [ ] 10.4 Test telemetry flow
    - Set known values via mock-sensors
    - Wait for telemetry interval
    - GET /status → verify values match what was set
    - **Property 5: Telemetry Accuracy**
    - _Requirements: 03-REQ-7.5_

  - [ ] 10.5 Test rejection propagation
    - Set unsafe speed via mock-sensors
    - Send lock via REST
    - Wait for rejection to propagate
    - GET /status → verify is_locked unchanged, last_command.result =
      REJECTED_SPEED
    - _Requirements: 03-REQ-7.2, 03-REQ-7.3_

  - [ ] 10.V Verify task group 10
    - [ ] All integration tests pass with infrastructure running
    - [ ] Tests skip cleanly when infrastructure is unavailable
    - [ ] Requirements 03-REQ-7.1–7.5 acceptance criteria met

- [ ] 11. Final Verification and Documentation
  - [ ] 11.1 Run full test suite
    - `make build && make test && make lint`
    - Verify no regressions in spec 01 and spec 02 tests
    - _Requirements: all_

  - [ ] 11.2 Run integration tests
    - Start all infrastructure and services
    - Run all integration tests
    - Verify all pass

  - [ ] 11.3 Update documentation
    - Document MQTT topic structure and message schemas in
      `docs/mqtt-protocol.md`
    - Document pairing flow in `docs/vehicle-pairing.md`
    - Update README if needed

  - [ ] 11.V Verify task group 11
    - [ ] `make build` succeeds
    - [ ] `make test` passes
    - [ ] `make lint` clean
    - [ ] Integration tests pass
    - [ ] No regressions from specs 01 and 02
    - [ ] All 03-REQ requirements verified

### Checkbox States

| Syntax   | Meaning                |
|----------|------------------------|
| `- [ ]`  | Not started (required) |
| `- [ ]*` | Not started (optional) |
| `- [x]`  | Completed              |
| `- [-]`  | In progress            |
| `- [~]`  | Queued                 |

## Traceability

| Requirement | Implemented By Task | Verified By Test |
|-------------|---------------------|------------------|
| 03-REQ-1.1 | 2.3 | `api/handlers_test.go` |
| 03-REQ-1.2 | 2.3 | `api/handlers_test.go` |
| 03-REQ-1.3 | 2.3 | `api/handlers_test.go` |
| 03-REQ-1.4 | 2.3 | `api/handlers_test.go` |
| 03-REQ-1.5 | 2.3 | `api/handlers_test.go` (verify 202, no MQTT wait) |
| 03-REQ-1.6 | 2.4 | Config parsing test |
| 03-REQ-1.E1 | 2.3 | `api/handlers_test.go` (unknown VIN) |
| 03-REQ-1.E2 | 2.2 | `api/middleware_test.go` |
| 03-REQ-1.E3 | 3.3 | Unit test (MQTT publish failure) |
| 03-REQ-2.1 | 3.1 | MQTT integration test (3.4) |
| 03-REQ-2.2 | 3.3 | MQTT integration test (3.4) |
| 03-REQ-2.3 | 3.2 | `mqtt/handlers_test.go` |
| 03-REQ-2.4 | 3.2 | `mqtt/handlers_test.go` |
| 03-REQ-2.5 | 3.1, 2.4 | Config parsing test |
| 03-REQ-2.E1 | 3.1 | MQTT reconnect test |
| 03-REQ-2.E2 | 3.2 | `mqtt/handlers_test.go` |
| 03-REQ-3.1 | 5.3 | MQTT integration test (5.4) |
| 03-REQ-3.2 | 6.1 | Command handler unit test (6.4) |
| 03-REQ-3.3 | 6.1 | Command handler unit test (6.4) |
| 03-REQ-3.4 | 6.2 | Result forwarder unit test (6.4) |
| 03-REQ-3.5 | 6.3 | Status handler unit test (6.4) |
| 03-REQ-3.6 | 5.2 | Config parsing test |
| 03-REQ-3.E1 | 7.2 | Reconnect test |
| 03-REQ-3.E2 | 7.2 | Reconnect test |
| 03-REQ-3.E3 | 6.1 | Unit test (invalid JSON) |
| 03-REQ-4.1 | 7.1 | Telemetry unit test (7.3) |
| 03-REQ-4.2 | 7.1 | Telemetry unit test (7.3) |
| 03-REQ-4.3 | 5.2, 7.1 | Config parsing test |
| 03-REQ-4.E1 | 7.1 | Unit test (missing signals) |
| 03-REQ-5.1 | 5.1 | VIN unit test (5.4) |
| 03-REQ-5.2 | 5.3 | MQTT integration test (5.4) |
| 03-REQ-5.3 | 3.2 | `mqtt/handlers_test.go` |
| 03-REQ-5.4 | 2.3 | `api/handlers_test.go` (pair endpoint) |
| 03-REQ-5.5 | 2.2 | `api/middleware_test.go` |
| 03-REQ-5.E1 | 2.3 | `api/handlers_test.go` (unknown VIN) |
| 03-REQ-5.E2 | 2.3 | `api/handlers_test.go` (wrong PIN) |
| 03-REQ-5.E3 | 5.1 | VIN persistence test (5.4) |
| 03-REQ-6.1 | 9.1 | CLI unit test (9.4) |
| 03-REQ-6.2 | 9.2 | CLI unit test (9.4) |
| 03-REQ-6.3 | 9.2 | CLI unit test (9.4) |
| 03-REQ-6.4 | 9.2 | CLI unit test (9.4) |
| 03-REQ-6.5 | 9.1, 9.2 | CLI unit test (9.4) |
| 03-REQ-6.E1 | 9.3 | CLI unit test (9.4) |
| 03-REQ-7.1 | 10.2 | Integration test `test_pairing_flow` |
| 03-REQ-7.2 | 10.3 | Integration test `test_lock_e2e` |
| 03-REQ-7.3 | 10.3 | Integration test `test_lock_e2e` |
| 03-REQ-7.4 | 10.3 | Integration test `test_lock_e2e` |
| 03-REQ-7.5 | 10.4 | Integration test `test_telemetry` |
| 03-REQ-7.E1 | 10.1 | Test skip behavior |

## Notes

- **MQTT client libraries:** Go uses `eclipse/paho.mqtt.golang` (well-maintained,
  supports QoS 2). Rust uses `rumqttc` (async, supports QoS 2).
- **Command-to-result correlation:** CLOUD_GATEWAY_CLIENT maintains the most
  recent `command_id` from an incoming command. When `LockResult` changes in
  DATA_BROKER, the response is published with that `command_id`. This is a
  simplification — in production, a queue of pending commands would handle
  concurrent commands. For the demo (single companion app, single vehicle),
  this is sufficient.
- **Token generation:** CLOUD_GATEWAY generates opaque bearer tokens using a
  secure random string (e.g., `crypto/rand` base64). Tokens do not expire in
  the demo. No JWT complexity needed.
- **In-memory state:** CLOUD_GATEWAY stores all state in memory. Restart
  clears pairings and vehicle entries. CLOUD_GATEWAY_CLIENT re-registers on
  startup, so vehicles reappear automatically. Pairings must be re-done after
  gateway restart.
- **Concurrent access:** CLOUD_GATEWAY's state store must be thread-safe
  (`sync.RWMutex` in Go). CLOUD_GATEWAY_CLIENT uses Tokio tasks with
  `Arc<Mutex>` or message passing for shared state.
- **Mosquitto QoS 2 support:** Eclipse Mosquitto fully supports QoS 2. No
  special configuration needed beyond the existing `infra/compose.yaml`.
- **No changes to LOCKING_SERVICE or mock-sensors:** These components from
  spec 02 are used as-is in integration tests.
