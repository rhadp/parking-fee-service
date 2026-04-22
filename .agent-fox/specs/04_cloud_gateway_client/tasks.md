# Implementation Plan: CLOUD_GATEWAY_CLIENT

## Overview

This plan covers the implementation of the CLOUD_GATEWAY_CLIENT component, a Rust service that bridges the vehicle's DATA_BROKER with the cloud-based CLOUD_GATEWAY via NATS messaging. Implementation proceeds through 9 task groups: writing failing tests first, then implementing modules incrementally (config, validation, telemetry, NATS client, DATA_BROKER client, main wiring), followed by integration tests and final verification.

## Tasks

- [x] 1. Write failing spec tests
  - [x] 1.1 Create unit tests for `config` module _Test Spec:_ TS-04-1, TS-04-2, TS-04-E1 _Requirements:_ [04-REQ-1.1], [04-REQ-1.2], [04-REQ-1.3], [04-REQ-1.4], [04-REQ-1.E1]
  - [x] 1.2 Create unit tests for `command_validator` bearer token validation _Test Spec:_ TS-04-3, TS-04-E2, TS-04-E3, TS-04-E4 _Requirements:_ [04-REQ-5.1], [04-REQ-5.2], [04-REQ-5.E1], [04-REQ-5.E2]
  - [x] 1.3 Create unit tests for `command_validator` payload validation _Test Spec:_ TS-04-4, TS-04-5, TS-04-E5, TS-04-E6, TS-04-E7, TS-04-E8, TS-04-E9, TS-04-E10, TS-04-6 _Requirements:_ [04-REQ-6.1], [04-REQ-6.2], [04-REQ-6.4], [04-REQ-6.E1], [04-REQ-6.E2], [04-REQ-6.E3]
  - [x] 1.4 Create unit tests for `telemetry` state aggregation _Test Spec:_ TS-04-7, TS-04-8, TS-04-9 _Requirements:_ [04-REQ-8.1], [04-REQ-8.2], [04-REQ-8.3]
  - [x] 1.5 Create unit test for registration message format _Test Spec:_ TS-04-P1 _Requirements:_ [04-REQ-4.1]
  - [x] 1.6 Create property tests for command validation, passthrough, response relay, telemetry, and startup _Test Spec:_ TS-04-P2, TS-04-P3, TS-04-P4, TS-04-P5, TS-04-P6 _Requirements:_ [04-REQ-6.1], [04-REQ-6.3], [04-REQ-7.1], [04-REQ-8.1], [04-REQ-9.1]
  - [x] 1.V Verify task group 1: `cargo test -p cloud-gateway-client` compiles and all tests fail (not compile errors, but assertion failures)
- [x] 2. Implement config and data models
  - [x] 2.1 Implement `Config` struct and `Config::from_env()` in `src/config.rs` _Requirements:_ [04-REQ-1.1], [04-REQ-1.2], [04-REQ-1.3], [04-REQ-1.4], [04-REQ-1.E1]
  - [x] 2.2 Implement data model structs (`CommandPayload`, `CommandResponse`, `TelemetryMessage`, `RegistrationMessage`, `SignalUpdate`) in `src/models.rs` _Requirements:_ [04-REQ-6.1], [04-REQ-8.2], [04-REQ-4.1]
  - [x] 2.3 Implement error types (`ConfigError`, `AuthError`, `ValidationError`, `NatsError`, `BrokerError`) in `src/errors.rs`
  - [x] 2.V Verify task group 2: unit tests for config (TS-04-1, TS-04-2, TS-04-E1) pass
- [x] 3. Implement command validation
  - [x] 3.1 Implement `validate_bearer_token()` in `src/command_validator.rs` _Requirements:_ [04-REQ-5.1], [04-REQ-5.2], [04-REQ-5.E1], [04-REQ-5.E2]
  - [x] 3.2 Implement `validate_command_payload()` in `src/command_validator.rs` _Requirements:_ [04-REQ-6.1], [04-REQ-6.2], [04-REQ-6.3], [04-REQ-6.4], [04-REQ-6.E1], [04-REQ-6.E2], [04-REQ-6.E3]
  - [x] 3.V Verify task group 3: unit tests for command validation (TS-04-3, TS-04-E2, TS-04-E3, TS-04-E4, TS-04-4, TS-04-5, TS-04-E5 through TS-04-E10, TS-04-6) pass
- [x] 4. Implement telemetry state
  - [x] 4.1 Implement `TelemetryState::new()` and `TelemetryState::update()` in `src/telemetry.rs` _Requirements:_ [04-REQ-8.1], [04-REQ-8.2], [04-REQ-8.3]
  - [x] 4.2 Verify unit tests for telemetry (TS-04-7, TS-04-8, TS-04-9) pass
  - [x] 4.3 Verify unit test for registration message (TS-04-P1) passes
  - [x] 4.V Verify task group 4
- [ ] 5. Implement NATS client
  - [ ] 5.1 Implement `NatsClient::connect()` with exponential backoff retry (1s, 2s, 4s, max 5 attempts) _Requirements:_ [04-REQ-2.1], [04-REQ-2.2], [04-REQ-2.E1]
  - [ ] 5.2 Implement `NatsClient::subscribe_commands()` for `vehicles.{VIN}.commands` _Requirements:_ [04-REQ-2.3]
  - [ ] 5.3 Implement `NatsClient::publish_registration()`, `publish_response()`, `publish_telemetry()` for outbound NATS messages _Requirements:_ [04-REQ-4.1], [04-REQ-4.2], [04-REQ-7.1], [04-REQ-8.1]
  - [ ] 5.4 Add `tracing` instrumentation to all NATS operations _Requirements:_ [04-REQ-10.1], [04-REQ-10.2], [04-REQ-10.3], [04-REQ-10.4]
  - [ ] 5.V Verify task group 5: `cargo build -p cloud-gateway-client` compiles without errors
- [ ] 6. Implement DATA_BROKER client
  - [ ] 6.1 Implement `BrokerClient::connect()` for gRPC connection to DATA_BROKER _Requirements:_ [04-REQ-3.1], [04-REQ-3.E1]
  - [ ] 6.2 Implement `BrokerClient::write_command()` to write to `Vehicle.Command.Door.Lock` _Requirements:_ [04-REQ-6.3]
  - [ ] 6.3 Implement `BrokerClient::subscribe_responses()` to observe `Vehicle.Command.Door.Response` _Requirements:_ [04-REQ-3.3], [04-REQ-7.1], [04-REQ-7.2], [04-REQ-7.E1]
  - [ ] 6.4 Implement `BrokerClient::subscribe_telemetry()` to observe IsLocked, Latitude, Longitude, SessionActive signals _Requirements:_ [04-REQ-3.2]
  - [ ] 6.5 Add `tracing` instrumentation to all DATA_BROKER operations _Requirements:_ [04-REQ-10.1], [04-REQ-10.2], [04-REQ-10.4]
  - [ ] 6.V Verify task group 6: `cargo build -p cloud-gateway-client` compiles without errors
- [ ] 7. Implement main and wiring
  - [ ] 7.1 Implement `main()` with startup sequencing: config -> NATS -> DATA_BROKER -> registration -> spawn tasks _Requirements:_ [04-REQ-9.1], [04-REQ-9.2]
  - [ ] 7.2 Implement command processing loop: receive from NATS, validate, write to DATA_BROKER _Requirements:_ [04-REQ-5.2], [04-REQ-6.3]
  - [ ] 7.3 Implement response relay loop: subscribe DATA_BROKER responses, publish to NATS _Requirements:_ [04-REQ-7.1], [04-REQ-7.2]
  - [ ] 7.4 Implement telemetry loop: subscribe DATA_BROKER signals, aggregate, publish to NATS _Requirements:_ [04-REQ-8.1], [04-REQ-8.2]
  - [ ] 7.5 Add `tracing-subscriber` initialization and structured logging _Requirements:_ [04-REQ-10.1], [04-REQ-10.2], [04-REQ-10.3], [04-REQ-10.4]
  - [ ] 7.V Verify task group 7: `cargo build -p cloud-gateway-client` produces a working binary
- [ ] 8. Write and run integration tests
  - [ ] 8.1 Create integration test for end-to-end command flow _Test Spec:_ TS-04-10 _Requirements:_ [04-REQ-2.3], [04-REQ-5.2], [04-REQ-6.3]
  - [ ] 8.2 Create integration test for end-to-end response relay _Test Spec:_ TS-04-11 _Requirements:_ [04-REQ-7.1], [04-REQ-7.2]
  - [ ] 8.3 Create integration test for end-to-end telemetry _Test Spec:_ TS-04-12 _Requirements:_ [04-REQ-8.1], [04-REQ-8.2]
  - [ ] 8.4 Create integration test for self-registration _Test Spec:_ TS-04-13 _Requirements:_ [04-REQ-4.1], [04-REQ-4.2]
  - [ ] 8.5 Create integration test for command rejection with invalid token _Test Spec:_ TS-04-14 _Requirements:_ [04-REQ-5.E2]
  - [ ] 8.V Verify task group 8: all integration tests pass with `cargo test -p cloud-gateway-client -- --ignored`
- [ ] 9. Wiring verification
  - [ ] 9.1 Run smoke test: service starts with valid config _Test Spec:_ TS-04-SMOKE-1 _Requirements:_ [04-REQ-2.1], [04-REQ-3.1]
  - [ ] 9.2 Run smoke test: service exits on missing VIN _Test Spec:_ TS-04-SMOKE-2 _Requirements:_ [04-REQ-1.E1]
  - [ ] 9.3 Run smoke test: registration message published on startup _Test Spec:_ TS-04-SMOKE-3 _Requirements:_ [04-REQ-4.1]
  - [ ] 9.4 Run NATS reconnection test _Test Spec:_ TS-04-15 _Requirements:_ [04-REQ-2.2], [04-REQ-2.E1]
  - [ ] 9.5 Verify `cargo test -p cloud-gateway-client` passes all unit tests
  - [ ] 9.V Verify task group 9: all integration and smoke tests pass with containers running

## Test Commands

- Unit tests: `cargo test -p cloud-gateway-client`
- Integration tests (requires NATS + DATA_BROKER containers): `cd deployments && podman-compose up -d` then `cargo test -p cloud-gateway-client -- --ignored`
- All tests: `cargo test -p cloud-gateway-client -- --include-ignored`
- Single test: `cargo test -p cloud-gateway-client -- test_name`
- With logging: `RUST_LOG=debug cargo test -p cloud-gateway-client -- --nocapture`

## Traceability

| Requirement | Test Spec Entry | Implemented By Task | Verified By Test |
|-------------|----------------|---------------------|------------------|
| [04-REQ-1.1] | TS-04-1 | 2.1 | TS-04-1 |
| [04-REQ-1.2] | TS-04-2 | 2.1 | TS-04-2 |
| [04-REQ-1.3] | TS-04-2 | 2.1 | TS-04-2 |
| [04-REQ-1.4] | TS-04-2 | 2.1 | TS-04-2 |
| [04-REQ-1.E1] | TS-04-E1 | 2.1 | TS-04-E1 |
| [04-REQ-2.1] | TS-04-10 | 5.1 | TS-04-10 |
| [04-REQ-2.2] | TS-04-15 | 5.1 | TS-04-15 |
| [04-REQ-2.3] | TS-04-10 | 5.2 | TS-04-10 |
| [04-REQ-2.E1] | TS-04-15 | 5.1 | TS-04-15 |
| [04-REQ-3.1] | TS-04-10 | 6.1 | TS-04-10 |
| [04-REQ-3.2] | TS-04-12 | 6.4 | TS-04-12 |
| [04-REQ-3.3] | TS-04-11 | 6.3 | TS-04-11 |
| [04-REQ-3.E1] | - | 6.1 | - |
| [04-REQ-4.1] | TS-04-P1, TS-04-13 | 5.3 | TS-04-P1, TS-04-13 |
| [04-REQ-4.2] | TS-04-13 | 5.3 | TS-04-13 |
| [04-REQ-5.1] | TS-04-3 | 3.1 | TS-04-3 |
| [04-REQ-5.2] | TS-04-3, TS-04-10 | 3.1 | TS-04-3, TS-04-10 |
| [04-REQ-5.E1] | TS-04-E2 | 3.1 | TS-04-E2 |
| [04-REQ-5.E2] | TS-04-E3, TS-04-E4, TS-04-14 | 3.1 | TS-04-E3, TS-04-E4, TS-04-14 |
| [04-REQ-6.1] | TS-04-4 | 3.2 | TS-04-4 |
| [04-REQ-6.2] | TS-04-4, TS-04-5 | 3.2 | TS-04-4, TS-04-5 |
| [04-REQ-6.3] | TS-04-10 | 3.2, 6.2 | TS-04-10 |
| [04-REQ-6.4] | TS-04-6 | 3.2 | TS-04-6 |
| [04-REQ-6.E1] | TS-04-E5 | 3.2 | TS-04-E5 |
| [04-REQ-6.E2] | TS-04-E6, TS-04-E7, TS-04-E8, TS-04-E10 | 3.2 | TS-04-E6, TS-04-E7, TS-04-E8, TS-04-E10 |
| [04-REQ-6.E3] | TS-04-E9 | 3.2 | TS-04-E9 |
| [04-REQ-7.1] | TS-04-11 | 6.3, 7.3 | TS-04-11 |
| [04-REQ-7.2] | TS-04-11 | 6.3, 7.3 | TS-04-11 |
| [04-REQ-7.E1] | - | 6.3 | - |
| [04-REQ-8.1] | TS-04-7, TS-04-12 | 4.1, 7.4 | TS-04-7, TS-04-12 |
| [04-REQ-8.2] | TS-04-7, TS-04-9, TS-04-12 | 4.1, 7.4 | TS-04-7, TS-04-9, TS-04-12 |
| [04-REQ-8.3] | TS-04-8 | 4.1 | TS-04-8 |
| [04-REQ-9.1] | TS-04-13 | 7.1 | TS-04-13 |
| [04-REQ-9.2] | TS-04-E1, TS-04-15 | 7.1 | TS-04-E1, TS-04-15 |
| [04-REQ-10.1] | - | 5.4, 6.5, 7.5 | - |
| [04-REQ-10.2] | - | 5.4, 6.5, 7.5 | - |
| [04-REQ-10.3] | - | 5.4, 7.5 | - |
| [04-REQ-10.4] | - | 5.4, 6.5, 7.5 | - |

## Notes

- Unit tests use mock implementations of NATS and DATA_BROKER clients; no infrastructure required.
- Integration tests (group 8) require running NATS and DATA_BROKER containers via `podman compose up`.
- The `VIN` environment variable is mandatory; the service exits with code 1 if unset.
