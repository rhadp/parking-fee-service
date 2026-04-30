# Implementation Plan: CLOUD_GATEWAY_CLIENT

<!-- AGENT INSTRUCTIONS
- Implement exactly ONE top-level task group per session
- Task group 1 writes failing tests from test_spec.md — all subsequent groups
  implement code to make those tests pass
- Follow the git-flow: feature branch from develop -> implement -> test -> merge to develop
- Update checkbox states as you go: [-] in progress, [x] complete
-->

## Overview

This plan covers the implementation of the CLOUD_GATEWAY_CLIENT component, a
Rust service that bridges the vehicle's DATA_BROKER with the cloud-based
CLOUD_GATEWAY via NATS messaging. Implementation proceeds through 9 task groups:
writing failing tests first, then implementing modules incrementally (config,
validation, telemetry, NATS client, DATA_BROKER client, main wiring), followed
by integration tests and final wiring verification.

## Test Commands

- Spec tests: `cargo test -p cloud-gateway-client`
- Unit tests: `cargo test -p cloud-gateway-client`
- Property tests: `cargo test -p cloud-gateway-client -- property`
- Integration tests: `cd deployments && podman-compose up -d && cargo test -p cloud-gateway-client -- --ignored`
- All tests: `cargo test -p cloud-gateway-client -- --include-ignored`
- Single test: `cargo test -p cloud-gateway-client -- test_name`
- Linter: `cargo clippy -p cloud-gateway-client -- -D warnings`

## Tasks

- [x] 1. Write failing spec tests
  - [x] 1.1 Create unit tests for `config` module
    - Create `tests/test_config.rs` (or inline module) with tests for env var parsing and defaults
    - _Test Spec: TS-04-1, TS-04-2, TS-04-E1_
    - _Requirements: 04-REQ-1.1, 04-REQ-1.2, 04-REQ-1.3, 04-REQ-1.4, 04-REQ-1.E1_

  - [x] 1.2 Create unit tests for `command_validator` bearer token validation
    - Tests for valid token, missing header, wrong token, malformed header
    - _Test Spec: TS-04-3, TS-04-E2, TS-04-E3, TS-04-E4_
    - _Requirements: 04-REQ-5.1, 04-REQ-5.2, 04-REQ-5.E1, 04-REQ-5.E2_

  - [x] 1.3 Create unit tests for `command_validator` payload validation
    - Tests for valid lock/unlock, invalid JSON, missing fields, invalid action, door passthrough
    - _Test Spec: TS-04-4, TS-04-5, TS-04-6, TS-04-E5, TS-04-E6, TS-04-E7, TS-04-E8, TS-04-E9, TS-04-E10_
    - _Requirements: 04-REQ-6.1, 04-REQ-6.2, 04-REQ-6.4, 04-REQ-6.E1, 04-REQ-6.E2, 04-REQ-6.E3_

  - [x] 1.4 Create unit tests for `telemetry` state aggregation
    - Tests for first update, field omission, all-fields inclusion
    - _Test Spec: TS-04-7, TS-04-8, TS-04-9_
    - _Requirements: 04-REQ-8.1, 04-REQ-8.2, 04-REQ-8.3_

  - [x] 1.5 Create unit test for registration message format and edge cases
    - Test serialization format, NATS retry exhaustion, DATA_BROKER failure, invalid response JSON
    - _Test Spec: TS-04-10, TS-04-E11, TS-04-E12, TS-04-E13_
    - _Requirements: 04-REQ-4.1, 04-REQ-2.E1, 04-REQ-3.E1, 04-REQ-7.E1_

  - [x] 1.6 Create property tests
    - Property tests for authentication integrity, command validity, passthrough fidelity, response relay, telemetry completeness, startup determinism
    - _Test Spec: TS-04-P1, TS-04-P2, TS-04-P3, TS-04-P4, TS-04-P5, TS-04-P6_
    - _Requirements: 04-REQ-5.1, 04-REQ-6.1, 04-REQ-6.3, 04-REQ-7.1, 04-REQ-8.1, 04-REQ-9.1_

  - [x] 1.V Verify task group 1
    - [x] All spec tests exist and are syntactically valid
    - [x] All spec tests FAIL (red) -- no implementation yet
    - [x] No linter warnings introduced: `cargo clippy -p cloud-gateway-client -- -D warnings`

- [x] 2. Implement config and data models
  - [x] 2.1 Implement `Config` struct and `Config::from_env()` in `src/config.rs`
    - Read VIN (required), NATS_URL, DATABROKER_ADDR, BEARER_TOKEN from env
    - Apply defaults for optional vars
    - Exit with code 1 and descriptive error if VIN is missing
    - _Requirements: 04-REQ-1.1, 04-REQ-1.2, 04-REQ-1.3, 04-REQ-1.4, 04-REQ-1.E1_

  - [x] 2.2 Implement data model structs in `src/models.rs`
    - `CommandPayload`, `CommandResponse`, `TelemetryMessage`, `RegistrationMessage`, `SignalUpdate`
    - Derive appropriate serde traits, use `skip_serializing_if` for optional telemetry fields
    - _Requirements: 04-REQ-6.1, 04-REQ-8.2, 04-REQ-4.1_

  - [x] 2.3 Implement error types in `src/errors.rs`
    - `ConfigError`, `AuthError`, `ValidationError`, `NatsError`, `BrokerError`

  - [x] 2.V Verify task group 2
    - [x] Spec tests for this group pass: `cargo test -p cloud-gateway-client -- test_config`
    - [x] All existing tests still pass: `cargo test -p cloud-gateway-client`
    - [x] No linter warnings introduced: `cargo clippy -p cloud-gateway-client -- -D warnings`
    - [x] Requirements 04-REQ-1.1, 04-REQ-1.2, 04-REQ-1.3, 04-REQ-1.4, 04-REQ-1.E1 acceptance criteria met

- [x] 3. Implement command validation
  - [x] 3.1 Implement `validate_bearer_token()` in `src/command_validator.rs`
    - Extract Authorization header, verify `Bearer <token>` format, compare against configured token
    - Return `AuthError::MissingHeader` or `AuthError::InvalidToken` on failure
    - _Requirements: 04-REQ-5.1, 04-REQ-5.2, 04-REQ-5.E1, 04-REQ-5.E2_

  - [x] 3.2 Implement `validate_command_payload()` in `src/command_validator.rs`
    - Parse JSON, validate required fields (command_id, action, doors)
    - Reject empty command_id, invalid action values
    - Do NOT validate door values
    - _Requirements: 04-REQ-6.1, 04-REQ-6.2, 04-REQ-6.3, 04-REQ-6.4, 04-REQ-6.E1, 04-REQ-6.E2, 04-REQ-6.E3_

  - [x] 3.V Verify task group 3
    - [x] Spec tests for this group pass: `cargo test -p cloud-gateway-client -- test_command_validator`
    - [x] All existing tests still pass: `cargo test -p cloud-gateway-client`
    - [x] No linter warnings introduced: `cargo clippy -p cloud-gateway-client -- -D warnings`
    - [x] Requirements 04-REQ-5.1, 04-REQ-5.2, 04-REQ-5.E1, 04-REQ-5.E2, 04-REQ-6.1, 04-REQ-6.2, 04-REQ-6.E1, 04-REQ-6.E2, 04-REQ-6.E3 acceptance criteria met

- [x] 4. Implement telemetry state
  - [x] 4.1 Implement `TelemetryState::new()` and `TelemetryState::update()` in `src/telemetry.rs`
    - Maintain optional fields for each signal
    - On update: set the field value, serialize to JSON omitting unset fields, return `Some(json)` if state changed
    - _Requirements: 04-REQ-8.1, 04-REQ-8.2, 04-REQ-8.3_

  - [x] 4.2 Implement `RegistrationMessage` serialization
    - Verify JSON output format matches spec
    - _Requirements: 04-REQ-4.1_

  - [x] 4.V Verify task group 4
    - [x] Spec tests for this group pass: `cargo test -p cloud-gateway-client -- test_telemetry`
    - [x] Registration message test passes: `cargo test -p cloud-gateway-client -- test_registration`
    - [x] All existing tests still pass: `cargo test -p cloud-gateway-client`
    - [x] No linter warnings introduced: `cargo clippy -p cloud-gateway-client -- -D warnings`
    - [x] Requirements 04-REQ-8.1, 04-REQ-8.2, 04-REQ-8.3, 04-REQ-4.1 acceptance criteria met

- [x] 5. Implement NATS client
  - [x] 5.1 Implement `NatsClient::connect()` with exponential backoff retry
    - Retry delays: 1s, 2s, 4s, max 5 attempts
    - Return `NatsError::RetriesExhausted` on failure
    - _Requirements: 04-REQ-2.1, 04-REQ-2.2, 04-REQ-2.E1_

  - [x] 5.2 Implement `NatsClient::subscribe_commands()` for `vehicles.{VIN}.commands`
    - _Requirements: 04-REQ-2.3_

  - [x] 5.3 Implement `NatsClient::publish_registration()`, `publish_response()`, `publish_telemetry()`
    - Registration publishes to `vehicles.{VIN}.status`
    - Responses publish to `vehicles.{VIN}.command_responses`
    - Telemetry publishes to `vehicles.{VIN}.telemetry`
    - _Requirements: 04-REQ-4.1, 04-REQ-4.2, 04-REQ-7.1, 04-REQ-8.1_

  - [x] 5.4 Add `tracing` instrumentation to all NATS operations
    - INFO: connection, subscribe, publish success
    - WARN: authentication/validation failures
    - ERROR: connection failures, publish failures
    - _Requirements: 04-REQ-10.1, 04-REQ-10.2, 04-REQ-10.3, 04-REQ-10.4_

  - [x] 5.V Verify task group 5
    - [x] `cargo build -p cloud-gateway-client` compiles without errors
    - [x] All existing tests still pass: `cargo test -p cloud-gateway-client`
    - [x] No linter warnings introduced: `cargo clippy -p cloud-gateway-client -- -D warnings`

- [x] 6. Checkpoint - Pure Logic Complete
  - Ensure all unit tests and property tests pass.
  - Verify config, command validation, and telemetry modules are complete.
  - All pure-logic components (no I/O) are fully tested.

- [x] 7. Implement DATA_BROKER client and main wiring
  - [x] 7.1 Implement `BrokerClient::connect()` for gRPC connection to DATA_BROKER
    - Connect via tonic to the configured address
    - Return `BrokerError::ConnectionFailed` on failure
    - _Requirements: 04-REQ-3.1, 04-REQ-3.E1_

  - [x] 7.2 Implement `BrokerClient::write_command()` and signal subscriptions
    - `write_command()` writes to `Vehicle.Command.Door.Lock`
    - `subscribe_responses()` observes `Vehicle.Command.Door.Response`
    - `subscribe_telemetry()` observes IsLocked, Latitude, Longitude, SessionActive
    - _Requirements: 04-REQ-3.2, 04-REQ-3.3, 04-REQ-6.3_

  - [x] 7.3 Implement `main()` with startup sequencing
    - Sequence: config -> NATS connect -> DATA_BROKER connect -> registration -> spawn loops
    - Exit with code 1 on any startup failure
    - _Requirements: 04-REQ-9.1, 04-REQ-9.2_

  - [x] 7.4 Implement command processing, response relay, and telemetry loops
    - Command loop: receive from NATS, validate bearer token, validate payload, write to DATA_BROKER
    - Response relay: subscribe DATA_BROKER responses, validate JSON, publish to NATS
    - Telemetry loop: subscribe DATA_BROKER signals, update TelemetryState, publish to NATS
    - _Requirements: 04-REQ-5.2, 04-REQ-6.3, 04-REQ-7.1, 04-REQ-7.2, 04-REQ-7.E1, 04-REQ-8.1, 04-REQ-8.2_

  - [x] 7.5 Add `tracing-subscriber` initialization and structured logging
    - _Requirements: 04-REQ-10.1, 04-REQ-10.2, 04-REQ-10.3, 04-REQ-10.4_

  - [x] 7.V Verify task group 7
    - [x] `cargo build -p cloud-gateway-client` produces a working binary
    - [x] All existing tests still pass: `cargo test -p cloud-gateway-client`
    - [x] No linter warnings introduced: `cargo clippy -p cloud-gateway-client -- -D warnings`

- [ ] 8. Write and run integration tests
  - [ ] 8.1 Create integration test for end-to-end command flow
    - Publish authenticated command on NATS, verify it appears in DATA_BROKER
    - _Test Spec: TS-04-11_
    - _Requirements: 04-REQ-2.3, 04-REQ-5.2, 04-REQ-6.3_

  - [ ] 8.2 Create integration test for end-to-end response relay
    - Write response to DATA_BROKER, verify it appears on NATS
    - _Test Spec: TS-04-12_
    - _Requirements: 04-REQ-7.1, 04-REQ-7.2_

  - [ ] 8.3 Create integration test for end-to-end telemetry
    - Update signal in DATA_BROKER, verify telemetry JSON on NATS
    - _Test Spec: TS-04-13_
    - _Requirements: 04-REQ-8.1, 04-REQ-8.2_

  - [ ] 8.4 Create integration test for self-registration
    - Start service, verify registration message on NATS
    - _Test Spec: TS-04-14_
    - _Requirements: 04-REQ-4.1, 04-REQ-4.2_

  - [ ] 8.5 Create integration test for command rejection with invalid token
    - Publish command with wrong token, verify no DATA_BROKER write
    - _Test Spec: TS-04-15_
    - _Requirements: 04-REQ-5.E2_

  - [ ] 8.6 Create integration test for NATS reconnection
    - Start with NATS down, verify retry backoff and exit on exhaustion
    - _Test Spec: TS-04-16_
    - _Requirements: 04-REQ-2.2, 04-REQ-2.E1_

  - [ ] 8.V Verify task group 8
    - [ ] Spec tests for this group pass: `cargo test -p cloud-gateway-client -- --ignored`
    - [ ] All existing tests still pass: `cargo test -p cloud-gateway-client -- --include-ignored`
    - [ ] No linter warnings introduced: `cargo clippy -p cloud-gateway-client -- -D warnings`
    - [ ] Requirements 04-REQ-2.2, 04-REQ-2.3, 04-REQ-2.E1, 04-REQ-4.1, 04-REQ-5.2, 04-REQ-5.E2, 04-REQ-6.3, 04-REQ-7.1, 04-REQ-7.2, 04-REQ-8.1, 04-REQ-8.2 acceptance criteria met

- [ ] 9. Wiring verification

  - [ ] 9.1 Trace every execution path from design.md end-to-end
    - For each path, verify the entry point actually calls the next function
      in the chain (read the calling code, do not assume)
    - Confirm no function in the chain is a stub (`return []`, `return None`,
      `pass`, `raise NotImplementedError`) that was never replaced
    - Every path must be live in production code -- errata or deferrals do not
      satisfy this check
    - _Requirements: all_

  - [ ] 9.2 Verify return values propagate correctly
    - For every function in this spec that returns data consumed by a caller,
      confirm the caller receives and uses the return value
    - Grep for callers of each such function; confirm none discards the return
    - _Requirements: all_

  - [ ] 9.3 Run the integration smoke tests
    - All `TS-04-SMOKE-*` tests pass using real components (no stub bypass)
    - _Test Spec: TS-04-SMOKE-1, TS-04-SMOKE-2, TS-04-SMOKE-3, TS-04-SMOKE-4_

  - [ ] 9.4 Stub / dead-code audit
    - Search all files touched by this spec for: `return vec![]`, `return None`
      on non-Optional returns, empty method bodies, `// TODO`,
      `// stub`, `unimplemented!()`, `todo!()`
    - Each hit must be either: (a) justified with a comment explaining why it
      is intentional, or (b) replaced with a real implementation
    - Document any intentional stubs here with rationale

  - [ ] 9.5 Cross-spec entry point verification
    - For each execution path whose entry point is owned by another spec
      (e.g., CLOUD_GATEWAY calling into this service via NATS), grep the
      codebase to confirm the entry point is actually called from production
      code -- not just from tests
    - If the upstream caller does not exist, either implement it within this
      spec or file an issue and remove the path from design.md
    - _Requirements: all_

  - [ ] 9.V Verify wiring group
    - [ ] All smoke tests pass
    - [ ] No unjustified stubs remain in touched files
    - [ ] All execution paths from design.md are live (traceable in code)
    - [ ] All cross-spec entry points are called from production code
    - [ ] All existing tests still pass: `cargo test -p cloud-gateway-client -- --include-ignored`

## Traceability

| Requirement | Test Spec Entry | Implemented By Task | Verified By Test |
|-------------|-----------------|---------------------|------------------|
| 04-REQ-1.1 | TS-04-1 | 2.1 | test_config_reads_vin |
| 04-REQ-1.2 | TS-04-2 | 2.1 | test_config_custom_env |
| 04-REQ-1.3 | TS-04-2 | 2.1 | test_config_custom_env |
| 04-REQ-1.4 | TS-04-2 | 2.1 | test_config_custom_env |
| 04-REQ-1.E1 | TS-04-E1 | 2.1 | test_config_missing_vin |
| 04-REQ-2.1 | TS-04-11 | 5.1 | test_e2e_command_flow |
| 04-REQ-2.2 | TS-04-16 | 5.1 | test_nats_reconnection |
| 04-REQ-2.3 | TS-04-11 | 5.2 | test_e2e_command_flow |
| 04-REQ-2.E1 | TS-04-E11, TS-04-16 | 5.1 | test_nats_retries_exhausted |
| 04-REQ-3.1 | TS-04-11 | 7.1 | test_e2e_command_flow |
| 04-REQ-3.2 | TS-04-13 | 7.2 | test_e2e_telemetry |
| 04-REQ-3.3 | TS-04-12 | 7.2 | test_e2e_response_relay |
| 04-REQ-3.E1 | TS-04-E12 | 7.1 | test_broker_connection_failure |
| 04-REQ-4.1 | TS-04-10, TS-04-14 | 5.3 | test_registration_format, test_self_registration |
| 04-REQ-4.2 | TS-04-14 | 5.3 | test_self_registration |
| 04-REQ-5.1 | TS-04-3, TS-04-P1 | 3.1 | test_bearer_valid |
| 04-REQ-5.2 | TS-04-3, TS-04-P1, TS-04-11 | 3.1 | test_bearer_valid, test_e2e_command_flow |
| 04-REQ-5.E1 | TS-04-E2, TS-04-P1 | 3.1 | test_bearer_missing_header |
| 04-REQ-5.E2 | TS-04-E3, TS-04-E4, TS-04-P1, TS-04-15 | 3.1 | test_bearer_wrong_token, test_bearer_malformed |
| 04-REQ-6.1 | TS-04-4, TS-04-P2 | 3.2 | test_command_valid_lock |
| 04-REQ-6.2 | TS-04-4, TS-04-5, TS-04-P2 | 3.2 | test_command_valid_lock, test_command_valid_unlock |
| 04-REQ-6.3 | TS-04-P2, TS-04-P3, TS-04-11 | 3.2, 7.2 | test_e2e_command_flow |
| 04-REQ-6.4 | TS-04-6, TS-04-P3 | 3.2 | test_command_door_passthrough |
| 04-REQ-6.E1 | TS-04-E5, TS-04-P2 | 3.2 | test_command_invalid_json |
| 04-REQ-6.E2 | TS-04-E6, TS-04-E7, TS-04-E8, TS-04-E10, TS-04-P2 | 3.2 | test_command_missing_fields |
| 04-REQ-6.E3 | TS-04-E9, TS-04-P2 | 3.2 | test_command_invalid_action |
| 04-REQ-7.1 | TS-04-P4, TS-04-12 | 7.2, 7.4 | test_e2e_response_relay |
| 04-REQ-7.2 | TS-04-P4, TS-04-12 | 7.2, 7.4 | test_e2e_response_relay |
| 04-REQ-7.E1 | TS-04-E13 | 7.4 | test_response_invalid_json |
| 04-REQ-8.1 | TS-04-7, TS-04-P5, TS-04-13 | 4.1, 7.4 | test_telemetry_first_update, test_e2e_telemetry |
| 04-REQ-8.2 | TS-04-7, TS-04-9, TS-04-P5, TS-04-13 | 4.1, 7.4 | test_telemetry_all_fields, test_e2e_telemetry |
| 04-REQ-8.3 | TS-04-8, TS-04-P5 | 4.1 | test_telemetry_omits_unset |
| 04-REQ-9.1 | TS-04-P6, TS-04-14 | 7.3 | test_startup_sequence |
| 04-REQ-9.2 | TS-04-E1, TS-04-P6, TS-04-16 | 7.3 | test_startup_failure_exit |
| 04-REQ-10.1 | TS-04-SMOKE-1 | 5.4, 7.5 | smoke_startup |
| 04-REQ-10.2 | TS-04-SMOKE-1 | 5.4, 7.5 | smoke_startup |
| 04-REQ-10.3 | TS-04-E2, TS-04-E3 | 5.4, 7.5 | test_bearer_warnings |
| 04-REQ-10.4 | TS-04-SMOKE-1 | 5.4, 7.5 | smoke_startup |

## Notes

- Unit tests use mock implementations of NATS and DATA_BROKER clients; no infrastructure required.
- Integration tests (group 8) require running NATS and DATA_BROKER containers via `cd deployments && podman-compose up -d`.
- The `VIN` environment variable is mandatory; the service exits with code 1 if unset.
- Property tests for passthrough fidelity (TS-04-P3) and response relay (TS-04-P4) are integration-level and may need to be deferred to group 8 if they require real infrastructure.
