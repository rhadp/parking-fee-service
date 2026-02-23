# Implementation Plan: RHIVOS Safety Partition (Phase 2.1)

<!-- AGENT INSTRUCTIONS
- Implement exactly ONE top-level task group per session
- Task group 1 writes failing tests from test_spec.md — all subsequent groups
  implement code to make those tests pass
- Follow the git-flow: feature branch from develop -> implement -> test -> merge to develop -> push
- Update checkbox states as you go: [-] in progress, [x] complete
-->

## Overview

This plan implements the RHIVOS safety-partition services for the SDV Parking
Demo System. The approach is test-first: task group 1 creates failing tests
that encode all test contracts from `test_spec.md`. Subsequent groups build
the services (DATA_BROKER configuration, LOCKING_SERVICE, CLOUD_GATEWAY_CLIENT,
mock sensors) to make those tests pass incrementally.

Ordering rationale:
1. Tests first (red) — establishes the verification baseline
2. DATA_BROKER configuration — foundational; all services depend on it
3. Shared databroker-client crate — reusable gRPC client for Kuksa
4. Mock sensor tools — needed to set up test scenarios for other services
5. LOCKING_SERVICE — core safety logic, depends on DATA_BROKER and sensors
6. CLOUD_GATEWAY_CLIENT — depends on DATA_BROKER and MQTT
7. Integration testing — end-to-end verification of all services together

## Test Commands

- Unit tests (all Rust): `cd rhivos && cargo test`
- Unit tests (specific crate): `cd rhivos && cargo test -p locking-service`
- Unit tests (specific crate): `cd rhivos && cargo test -p cloud-gateway-client`
- Unit tests (specific crate): `cd rhivos && cargo test -p mock-sensors`
- Unit tests (specific crate): `cd rhivos && cargo test -p databroker-client`
- Integration tests: `cd rhivos && cargo test --test integration`
- Linter: `cd rhivos && cargo clippy -- -D warnings`
- Spec tests (structural): `cd tests/safety && go test -v -count=1 -run TestStructure ./...`
- Spec tests (all): `cd tests/safety && go test -v -count=1 ./...`
- All tests: `make test`

## Tasks

- [x] 1. Write failing spec tests
  - [x] 1.1 Set up test module structure
    - Create `tests/safety/go.mod` as a standalone Go module
      (module path: `github.com/rhadp/parking-fee-service/tests/safety`)
    - Create `helpers_test.go` with shared test helpers:
      `repoRoot(t)`, `assertFileExists`, `assertFileContains`,
      `assertDirExists`, `execCommand`, `waitForPort`,
      `databrokerGet`, `databrokerSet`, `mqttPublish`, `mqttSubscribe`
    - _Test Spec: all (shared infrastructure)_

  - [x] 1.2 Write structural and configuration tests
    - Translate TS-02-1 (VSS overlay file) into Go test
    - Translate TS-02-26, TS-02-27 (UDS usage in source) into Go tests
    - Translate TS-02-28 (configurable endpoint) into Go test
    - Translate TS-02-29 (UDS socket path env var) into Go test
    - Translate TS-02-31 (integration test file exists) into Go test
    - Group under `TestStructure_*` and `TestConfig_*` naming conventions
    - _Test Spec: TS-02-1, TS-02-26, TS-02-27, TS-02-28, TS-02-29, TS-02-31_

  - [x] 1.3 Write DATA_BROKER integration tests
    - Translate TS-02-2 (standard signals) into Rust integration test
    - Translate TS-02-3 (UDS endpoint) into Rust integration test
    - Translate TS-02-4 (TCP endpoint) into Rust integration test
    - Translate TS-02-5 (bearer token) into Rust integration test
    - Group in `rhivos/safety-tests/tests/databroker_tests.rs`
    - _Test Spec: TS-02-2, TS-02-3, TS-02-4, TS-02-5_

  - [x] 1.4 Write LOCKING_SERVICE unit and integration tests
    - Translate TS-02-7 (JSON parsing) into Rust unit test in locking-service
    - Translate TS-02-6, TS-02-8, TS-02-9 (subscribe, lock, unlock) into
      Rust integration tests
    - Translate TS-02-10, TS-02-11, TS-02-12 (safety rejections) into Rust
      integration tests
    - Translate TS-02-13, TS-02-14 (response writing) into Rust integration tests
    - _Test Spec: TS-02-6 through TS-02-14_

  - [x] 1.5 Write CLOUD_GATEWAY_CLIENT integration tests
    - Translate TS-02-15 (MQTT connect) into Rust integration test
    - Translate TS-02-16, TS-02-17 (command subscription, relay) into Rust
      integration tests
    - Translate TS-02-18 (response relay to MQTT) into Rust integration test
    - Translate TS-02-19, TS-02-20 (telemetry subscription, publishing) into
      Rust integration tests
    - _Test Spec: TS-02-15 through TS-02-20_

  - [x] 1.6 Write mock sensor tests
    - Translate TS-02-21, TS-02-22, TS-02-23 (sensor write) into Rust
      integration tests
    - Translate TS-02-24 (exit code) into Rust integration test
    - Translate TS-02-25 (usage message) into Rust unit test
    - _Test Spec: TS-02-21 through TS-02-25_

  - [x] 1.7 Write edge case and property tests
    - Translate TS-02-E1 through TS-02-E15 into Rust integration tests
    - Translate TS-02-P1 through TS-02-P8 into Rust integration and unit tests
    - Group edge cases under `test_edge_*` naming
    - Group property tests under `test_property_*` naming
    - _Test Spec: TS-02-E1 through TS-02-E15, TS-02-P1 through TS-02-P8_

  - [x] 1.V Verify task group 1
    - [x] All spec tests exist and are syntactically valid:
      `cd tests/safety && go vet ./...`
    - [x] Rust tests compile:
      `cd rhivos && cargo test --no-run`
    - [x] No linter warnings introduced:
      `cd rhivos && cargo clippy -- -D warnings`

- [x] 2. DATA_BROKER configuration
  - [x] 2.1 Create VSS overlay file
    - Create `infra/kuksa/vss-overlay.json` with custom signal definitions:
      Vehicle.Command.Door.Lock (string) and
      Vehicle.Command.Door.Response (string)
    - _Requirements: 02-REQ-1.1_

  - [x] 2.2 Update docker-compose.yml for DATA_BROKER
    - Add VSS overlay volume mount to kuksa-databroker service
    - Add UDS socket bind-mount (`/tmp/kuksa-databroker.sock`)
    - Add `--vss` flag pointing to overlay file
    - Ensure both TCP (:55556) and UDS endpoints are available
    - _Requirements: 02-REQ-1.2, 02-REQ-1.3, 02-REQ-1.4_

  - [x] 2.3 Configure access control tokens
    - Create token configuration file at `infra/kuksa/tokens.json`
      (or use Kuksa's native token format)
    - Define tokens for each service with appropriate write permissions
    - Document token usage in service configuration
    - If Kuksa's token system does not support per-signal write control,
      document the limitation and use a simplified approach
    - _Requirements: 02-REQ-1.5_

  - [x] 2.V Verify task group 2
    - [x] DATA_BROKER starts with `make infra-up`
    - [x] Custom signals accessible:
      `grpcurl -plaintext localhost:55556 kuksa.val.v1.VAL/Get` (or equivalent)
    - [x] UDS endpoint reachable
    - [x] Spec tests TS-02-1 through TS-02-5 pass
    - [x] No regressions: `make test`

- [x] 3. Shared databroker-client crate
  - [x] 3.1 Create databroker-client crate
    - Create `rhivos/databroker-client/Cargo.toml` with dependencies:
      tonic, prost, tokio, tower (for UDS support)
    - Add crate to workspace members in `rhivos/Cargo.toml`
    - _Requirements: 02-REQ-7.1, 02-REQ-7.2, 02-REQ-7.3_

  - [x] 3.2 Implement Kuksa gRPC client wrapper
    - Implement `DatabrokerClient::connect()` supporting both UDS and TCP
    - Implement `get_value()`, `set_value()`, `subscribe()`
    - Implement `DataValue` enum (Bool, Float, Double, String)
    - Use Kuksa Databroker's proto definitions (from the Kuksa project)
    - _Requirements: 02-REQ-7.1, 02-REQ-7.2, 02-REQ-7.3, 02-REQ-7.4_

  - [x] 3.3 Add unit tests for databroker-client
    - Test value type conversions
    - Test endpoint parsing (UDS vs TCP)
    - Test error type mapping
    - _Requirements: 02-REQ-7.4_

  - [x] 3.V Verify task group 3
    - [x] Crate builds: `cd rhivos && cargo build -p databroker-client`
    - [x] Unit tests pass: `cd rhivos && cargo test -p databroker-client`
    - [x] No linter warnings: `cd rhivos && cargo clippy -p databroker-client -- -D warnings`
    - [x] No regressions: `cd rhivos && cargo test`

- [x] 4. Mock sensor CLI tools
  - [x] 4.1 Create mock-sensors crate
    - Create `rhivos/mock-sensors/Cargo.toml` with dependencies:
      clap, databroker-client, tokio, serde_json
    - Add crate to workspace members in `rhivos/Cargo.toml`
    - Define three binary targets: location-sensor, speed-sensor, door-sensor
    - _Requirements: 02-REQ-6.1, 02-REQ-6.2, 02-REQ-6.3_

  - [x] 4.2 Implement location-sensor CLI
    - CLI args: `--lat <f64>`, `--lon <f64>`, optional `--endpoint <string>`
    - Connect to DATA_BROKER, write Latitude and Longitude, exit
    - Display usage on missing args, error on invalid values
    - _Requirements: 02-REQ-6.1, 02-REQ-6.4, 02-REQ-6.5_

  - [x] 4.3 Implement speed-sensor CLI
    - CLI args: `--speed <f32>`, optional `--endpoint <string>`
    - Connect to DATA_BROKER, write Vehicle.Speed, exit
    - Display usage on missing args, error on invalid values
    - _Requirements: 02-REQ-6.2, 02-REQ-6.4, 02-REQ-6.5_

  - [x] 4.4 Implement door-sensor CLI
    - CLI args: `--open <bool>`, optional `--endpoint <string>`
    - Connect to DATA_BROKER, write IsOpen, exit
    - Display usage on missing args, error on invalid values
    - _Requirements: 02-REQ-6.3, 02-REQ-6.4, 02-REQ-6.5_

  - [x] 4.5 Add unit and integration tests
    - Unit tests: CLI argument parsing, value conversion, usage output
    - Integration tests: verify values written to DATA_BROKER (require infra)
    - _Test Spec: TS-02-21 through TS-02-25_

  - [x] 4.V Verify task group 4
    - [x] All three sensor binaries build:
      `cd rhivos && cargo build --bin location-sensor --bin speed-sensor --bin door-sensor`
    - [x] Unit tests pass: `cd rhivos && cargo test -p mock-sensors`
    - [ ] Integration tests pass (with infra):
      `cd rhivos && cargo test -p mock-sensors --test integration`
    - [x] Spec tests TS-02-21 through TS-02-25 pass
    - [x] No linter warnings: `cd rhivos && cargo clippy -p mock-sensors -- -D warnings`
    - [x] No regressions: `cd rhivos && cargo test`

- [x] 5. LOCKING_SERVICE implementation
  - [x] 5.1 Create locking-service module structure
    - Create `rhivos/locking-service/src/command.rs` with LockCommand,
      LockAction, CommandResponse, CommandStatus types
    - Create `rhivos/locking-service/src/safety.rs` with SafetyChecker
    - Create `rhivos/locking-service/src/databroker.rs` with DATA_BROKER
      client integration (using databroker-client crate)
    - Add databroker-client, serde, serde_json, tracing to Cargo.toml
    - _Requirements: 02-REQ-2.1, 02-REQ-2.2_

  - [x] 5.2 Implement command parsing and validation
    - Implement JSON deserialization for LockCommand
    - Validate required fields (command_id, action)
    - Validate action values ("lock" / "unlock")
    - Return appropriate error reasons for invalid payloads
    - _Requirements: 02-REQ-2.2, 02-REQ-2.E1, 02-REQ-2.E2, 02-REQ-2.E3_

  - [x] 5.3 Implement safety constraint checking
    - Implement speed check: read Vehicle.Speed, reject if > 0
    - Implement door ajar check: read IsOpen, reject if true
    - Handle missing signal values (treat as safe per 02-REQ-3.E1, 02-REQ-3.E2)
    - Return specific reason strings ("vehicle_moving", "door_open")
    - _Requirements: 02-REQ-3.1, 02-REQ-3.2, 02-REQ-3.3, 02-REQ-3.E1, 02-REQ-3.E2_

  - [x] 5.4 Implement main service loop
    - Connect to DATA_BROKER via UDS
    - Subscribe to Vehicle.Command.Door.Lock
    - For each command: parse, check safety, execute (write IsLocked), respond
    - Write Vehicle.Command.Door.Response with command_id, status, reason
    - _Requirements: 02-REQ-2.1, 02-REQ-2.3, 02-REQ-2.4, 02-REQ-3.4, 02-REQ-3.5_

  - [x] 5.5 Add unit and integration tests
    - Unit tests: command parsing (valid, invalid, missing fields), safety
      constraint logic (mocked DATA_BROKER values), response serialization
    - Integration tests: full command flow with running DATA_BROKER
    - _Test Spec: TS-02-6 through TS-02-14_

  - [x] 5.V Verify task group 5
    - [x] Service builds: `cd rhivos && cargo build -p locking-service`
    - [x] Unit tests pass: `cd rhivos && cargo test -p locking-service`
    - [x] Integration tests pass (with infra):
      `cd rhivos && cargo test -p locking-service --test integration`
    - [x] Spec tests TS-02-6 through TS-02-14 pass
    - [x] Property tests TS-02-P1 through TS-02-P4 pass
    - [x] Edge case tests TS-02-E3 through TS-02-E7 pass
    - [x] No linter warnings: `cd rhivos && cargo clippy -p locking-service -- -D warnings`
    - [x] No regressions: `cd rhivos && cargo test`

- [x] 6. CLOUD_GATEWAY_CLIENT implementation
  - [x] 6.1 Create cloud-gateway-client module structure
    - Create `rhivos/cloud-gateway-client/src/mqtt.rs` with MQTT client wrapper
    - Create `rhivos/cloud-gateway-client/src/commands.rs` with command
      validation and DATA_BROKER writing
    - Create `rhivos/cloud-gateway-client/src/telemetry.rs` with telemetry
      subscription and MQTT publishing
    - Create `rhivos/cloud-gateway-client/src/service.rs` with service
      orchestration (uses databroker-client crate directly)
    - Add rumqttc, databroker-client, serde, serde_json, tracing to Cargo.toml
    - _Requirements: 02-REQ-4.1, 02-REQ-4.2_

  - [x] 6.2 Implement MQTT client with reconnection
    - Connect to MQTT broker at configurable address
    - Subscribe to `vehicles/{vin}/commands`
    - Implement exponential backoff retry for connection and reconnection
    - Log retry attempts
    - _Requirements: 02-REQ-4.1, 02-REQ-4.2, 02-REQ-4.E1, 02-REQ-4.E2_

  - [x] 6.3 Implement command relay (MQTT -> DATA_BROKER)
    - Receive MQTT command messages
    - Validate JSON structure
    - Write validated command to Vehicle.Command.Door.Lock via DATA_BROKER UDS
    - Discard invalid messages with logging
    - _Requirements: 02-REQ-4.3, 02-REQ-4.E3_

  - [x] 6.4 Implement response relay (DATA_BROKER -> MQTT)
    - Subscribe to Vehicle.Command.Door.Response in DATA_BROKER
    - Publish response to `vehicles/{vin}/command_responses` MQTT topic
    - _Requirements: 02-REQ-4.4_

  - [x] 6.5 Implement telemetry publishing
    - Subscribe to vehicle state signals in DATA_BROKER: IsLocked, IsOpen,
      Latitude, Longitude, Speed
    - On signal change, publish telemetry JSON to
      `vehicles/{vin}/telemetry` MQTT topic
    - _Requirements: 02-REQ-5.1, 02-REQ-5.2_

  - [x] 6.6 Add unit and integration tests
    - Unit tests: command validation, telemetry message formatting,
      MQTT message construction
    - Integration tests: MQTT connectivity, command relay, response relay,
      telemetry publishing (require infra)
    - _Test Spec: TS-02-15 through TS-02-20_

  - [x] 6.V Verify task group 6
    - [x] Service builds: `cd rhivos && cargo build -p cloud-gateway-client`
    - [x] Unit tests pass: `cd rhivos && cargo test -p cloud-gateway-client`
    - [x] Integration tests pass (with infra):
      `cd rhivos && cargo test -p cloud-gateway-client --test integration`
    - [x] Spec tests TS-02-15 through TS-02-20 pass
    - [x] Property tests TS-02-P5, TS-02-P6 pass
    - [x] Edge case tests TS-02-E8 through TS-02-E11 pass
    - [x] No linter warnings: `cd rhivos && cargo clippy -p cloud-gateway-client -- -D warnings`
    - [x] No regressions: `cd rhivos && cargo test`

- [ ] 7. End-to-end integration testing
  - [ ] 7.1 Create end-to-end integration test suite
    - Create `rhivos/tests/integration.rs` (or `tests/e2e/`) with full
      end-to-end tests
    - Test: MQTT command -> CGC -> DATA_BROKER -> LS -> DATA_BROKER -> CGC -> MQTT response
    - Test: lock command succeeds when safe
    - Test: lock command rejected when vehicle moving
    - Test: lock command rejected when door open
    - Test: unlock command succeeds when safe
    - Test: telemetry published on signal changes
    - _Test Spec: TS-02-30, TS-02-32_

  - [ ] 7.2 Run all property tests
    - Verify TS-02-P1 through TS-02-P8 all pass
    - Fix any failures discovered in end-to-end context
    - _Test Spec: TS-02-P1 through TS-02-P8_

  - [ ] 7.3 Run all edge case tests
    - Verify TS-02-E1 through TS-02-E15 all pass
    - Fix any failures discovered in end-to-end context
    - _Test Spec: TS-02-E1 through TS-02-E15_

  - [ ] 7.4 Run full quality gate
    - `cd rhivos && cargo build`
    - `cd rhivos && cargo test`
    - `cd rhivos && cargo clippy -- -D warnings`
    - `cd rhivos && cargo test --test integration` (with infra)
    - `cd tests/safety && go test -v -count=1 ./...`
    - `make test`
    - _All requirements_

  - [ ] 7.V Verify task group 7
    - [ ] All 55 spec tests pass (32 acceptance + 15 edge + 8 property)
    - [ ] `cargo clippy -- -D warnings` exits 0
    - [ ] All integration tests pass with infra running
    - [ ] No regressions in Phase 1 tests
    - [ ] All changes committed and pushed
    - [ ] `git status` shows clean working tree

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
| 02-REQ-1.1 | TS-02-1 | 2.1 | `TestStructure_VssOverlay` |
| 02-REQ-1.2 | TS-02-2 | 2.2 | `test_standard_vss_signals` |
| 02-REQ-1.3 | TS-02-3 | 2.2 | `test_databroker_uds_endpoint` |
| 02-REQ-1.4 | TS-02-4 | 2.2 | `test_databroker_tcp_endpoint` |
| 02-REQ-1.5 | TS-02-5 | 2.3 | `test_databroker_bearer_token` |
| 02-REQ-1.E1 | TS-02-E1 | 2.2 | `test_edge_unknown_signal_write` |
| 02-REQ-1.E2 | TS-02-E2 | 2.3 | `test_edge_missing_bearer_token` |
| 02-REQ-2.1 | TS-02-6 | 5.4 | `test_locking_subscribes_to_commands` |
| 02-REQ-2.2 | TS-02-7 | 5.2 | `test_locking_parses_command_json` |
| 02-REQ-2.3 | TS-02-8 | 5.4 | `test_locking_executes_lock` |
| 02-REQ-2.4 | TS-02-9 | 5.4 | `test_locking_executes_unlock` |
| 02-REQ-2.E1 | TS-02-E3 | 5.2 | `test_edge_invalid_json_command` |
| 02-REQ-2.E2 | TS-02-E4 | 5.2 | `test_edge_unknown_action` |
| 02-REQ-2.E3 | TS-02-E5 | 5.2 | `test_edge_missing_fields` |
| 02-REQ-3.1 | TS-02-10 | 5.3 | `test_locking_rejects_lock_vehicle_moving` |
| 02-REQ-3.2 | TS-02-11 | 5.3 | `test_locking_rejects_lock_door_open` |
| 02-REQ-3.3 | TS-02-12 | 5.3 | `test_locking_rejects_unlock_vehicle_moving` |
| 02-REQ-3.4 | TS-02-13 | 5.3, 5.4 | `test_locking_failure_response_has_reason` |
| 02-REQ-3.5 | TS-02-14 | 5.4 | `test_locking_success_response_and_state` |
| 02-REQ-3.E1 | TS-02-E6 | 5.3 | `test_edge_speed_not_set_defaults_safe` |
| 02-REQ-3.E2 | TS-02-E7 | 5.3 | `test_edge_door_not_set_defaults_safe` |
| 02-REQ-4.1 | TS-02-15 | 6.2 | `test_cgc_connects_to_mqtt` |
| 02-REQ-4.2 | TS-02-16 | 6.2 | `test_cgc_subscribes_to_command_topic` |
| 02-REQ-4.3 | TS-02-17 | 6.3 | `test_cgc_writes_command_to_databroker` |
| 02-REQ-4.4 | TS-02-18 | 6.4 | `test_cgc_relays_response_to_mqtt` |
| 02-REQ-4.E1 | TS-02-E8 | 6.2 | `test_edge_mqtt_unreachable_startup` |
| 02-REQ-4.E2 | TS-02-E9 | 6.2 | `test_edge_mqtt_connection_lost` |
| 02-REQ-4.E3 | TS-02-E10 | 6.3 | `test_edge_invalid_mqtt_json` |
| 02-REQ-5.1 | TS-02-19 | 6.5 | `test_cgc_subscribes_to_state_signals` |
| 02-REQ-5.2 | TS-02-20 | 6.5 | `test_cgc_publishes_telemetry` |
| 02-REQ-5.E1 | TS-02-E11 | 6.5 | `test_edge_databroker_unreachable_telemetry` |
| 02-REQ-6.1 | TS-02-21 | 4.2 | `test_location_sensor_writes` |
| 02-REQ-6.2 | TS-02-22 | 4.3 | `test_speed_sensor_writes` |
| 02-REQ-6.3 | TS-02-23 | 4.4 | `test_door_sensor_writes` |
| 02-REQ-6.4 | TS-02-24 | 4.2, 4.3, 4.4 | `test_sensor_exit_code_success` |
| 02-REQ-6.5 | TS-02-25 | 4.2, 4.3, 4.4 | `test_sensor_usage_without_args` |
| 02-REQ-6.E1 | TS-02-E12 | 4.2, 4.3, 4.4 | `test_edge_sensor_databroker_unreachable` |
| 02-REQ-6.E2 | TS-02-E13 | 4.2, 4.3, 4.4 | `test_edge_sensor_invalid_value` |
| 02-REQ-7.1 | TS-02-26 | 5.4 | `TestConfig_LockingServiceUDS` |
| 02-REQ-7.2 | TS-02-27 | 6.1 | `TestConfig_CloudGatewayClientUDS` |
| 02-REQ-7.3 | TS-02-28 | 4.1 | `TestConfig_SensorConfigurableEndpoint` |
| 02-REQ-7.4 | TS-02-29 | 3.2, 5.4, 6.1 | `TestConfig_UdsSocketPathEnv` |
| 02-REQ-7.E1 | TS-02-E14 | 5.4, 6.1 | `test_edge_uds_socket_missing` |
| 02-REQ-8.1 | TS-02-30 | 7.1 | `test_e2e_lock_command_flow` |
| 02-REQ-8.2 | TS-02-31 | 7.1 | `TestStructure_IntegrationTestExists` |
| 02-REQ-8.3 | TS-02-32 | 7.1 | `test_integration_requires_infra` |
| 02-REQ-8.E1 | TS-02-E15 | 7.1 | `test_edge_integration_no_infra` |
| Property 1 | TS-02-P1 | 5.4 | `test_property_command_response_pairing` |
| Property 2 | TS-02-P2 | 5.3 | `test_property_safety_constraint_speed` |
| Property 3 | TS-02-P3 | 5.3 | `test_property_door_ajar_protection` |
| Property 4 | TS-02-P4 | 5.4 | `test_property_lock_state_consistency` |
| Property 5 | TS-02-P5 | 6.3 | `test_property_mqtt_relay_integrity` |
| Property 6 | TS-02-P6 | 6.5 | `test_property_telemetry_coverage` |
| Property 7 | TS-02-P7 | 5.4, 6.1 | `TestConfig_UdsExclusivity` |
| Property 8 | TS-02-P8 | 4.5 | `test_property_sensor_idempotency` |

## Notes

- **Test implementation:** Structural/configuration tests are Go tests in
  `tests/safety/`. All functional tests (unit, integration, property, edge
  case) are Rust tests within the `rhivos/` workspace, using `cargo test`.
- **Infrastructure dependency:** Integration tests, property tests, and most
  edge case tests require running infrastructure (`make infra-up`). Unit tests
  and structural tests do not.
- **DATA_BROKER is Kuksa Databroker:** No custom code wraps DATA_BROKER. The
  implementation work is configuration (VSS overlay, tokens, docker-compose).
  The shared `databroker-client` crate wraps Kuksa's gRPC API for use by
  the Rust services.
- **Session sizing:** Task groups 2 and 3 are small (configuration and shared
  library). Task groups 5 and 6 are larger (full service implementations).
  Task group 7 is validation-focused. Each group is scoped for one coding
  session.
- **Kuksa proto definitions:** The `databroker-client` crate needs Kuksa's
  proto files. These can be vendored into the repo or fetched at build time.
  The approach should be consistent with the Phase 1 proto handling pattern.
