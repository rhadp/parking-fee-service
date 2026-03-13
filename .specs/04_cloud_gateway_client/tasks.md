# Implementation Plan: CLOUD_GATEWAY_CLIENT

<!-- AGENT INSTRUCTIONS
- Implement exactly ONE top-level task group per session
- Task group 1 writes failing tests from test_spec.md — all subsequent groups
  implement code to make those tests pass
- Follow the git-flow: feature branch from main -> implement -> test -> merge to main -> push
- Update checkbox states as you go: [-] in progress, [x] complete
-->

## Overview

This plan implements the CLOUD_GATEWAY_CLIENT as a Rust binary in `rhivos/cloud-gateway-client/`. The service bridges DATA_BROKER and CLOUD_GATEWAY via NATS. Task group 1 writes failing tests. Groups 2-3 implement pure-function modules (config, command validation, telemetry aggregation). Group 4 implements NATS and DATA_BROKER clients. Group 5 wires the main loop. Group 6 runs integration tests.

Ordering: tests first, then pure-function modules, then client wrappers, then async main loop, then integration validation.

## Test Commands

- Spec tests (unit): `cd rhivos && cargo test -p cloud-gateway-client`
- Spec tests (integration): `cd tests/cloud-gateway-client && go test -v ./...`
- Property tests: `cd rhivos && cargo test -p cloud-gateway-client -- --include-ignored proptest`
- All Rust tests: `cd rhivos && cargo test`
- Linter: `cd rhivos && cargo clippy -p cloud-gateway-client -- -D warnings`

## Tasks

- [x] 1. Write failing spec tests
  - [x] 1.1 Add dependencies to cloud-gateway-client Cargo.toml
    - Add: async-nats, serde, serde_json, tokio, tonic, prost, tracing, tracing-subscriber, proptest (dev)
    - Vendor kuksa.val.v1 proto definitions into `rhivos/cloud-gateway-client/proto/`
    - Add tonic-build to build.rs for proto code generation
    - _Test Spec: TS-04-1 through TS-04-17_

  - [x] 1.2 Write config and command validation tests
    - Create `rhivos/cloud-gateway-client/src/config.rs` with test module
    - `test_nats_url_default` — TS-04-1 (case 1)
    - `test_nats_url_env` — TS-04-1 (case 2)
    - `test_databroker_addr_default` — TS-04-12
    - `test_vin_from_env` — TS-04-13
    - `test_vin_missing` — TS-04-E10
    - Create `rhivos/cloud-gateway-client/src/command.rs` with test module
    - `test_bearer_token_valid` — TS-04-3
    - `test_bearer_token_cases` — TS-04-4
    - `test_command_parse_valid` — TS-04-5
    - `test_bearer_token_invalid` — TS-04-E3
    - `test_command_invalid_json` — TS-04-E4
    - `test_command_missing_field` — TS-04-E5
    - _Test Spec: TS-04-1, TS-04-3, TS-04-4, TS-04-5, TS-04-12, TS-04-13, TS-04-E3, TS-04-E4, TS-04-E5, TS-04-E10_

  - [x] 1.3 Write telemetry and response tests
    - Create `rhivos/cloud-gateway-client/src/telemetry.rs` with test module
    - `test_telemetry_all_fields` — TS-04-11
    - `test_telemetry_on_change` — TS-04-10
    - `test_telemetry_omit_unset` — TS-04-E7
    - Create `rhivos/cloud-gateway-client/src/relay.rs` with test module
    - `test_response_relay_verbatim` — TS-04-8
    - `test_command_forwarding` — TS-04-6
    - `test_response_publish_failure` — TS-04-E6
    - `test_telemetry_publish_failure` — TS-04-E8
    - _Test Spec: TS-04-6, TS-04-8, TS-04-10, TS-04-11, TS-04-E6, TS-04-E7, TS-04-E8_

  - [x] 1.4 Write property tests
    - `proptest_bearer_token_gate` — TS-04-P1
    - `proptest_command_validation` — TS-04-P2
    - `proptest_response_fidelity` — TS-04-P3
    - `proptest_telemetry_aggregation` — TS-04-P4
    - `proptest_vin_subjects` — TS-04-P5
    - _Test Spec: TS-04-P1 through TS-04-P5_

  - [x] 1.V Verify task group 1
    - [x] All test files compile: `cd rhivos && cargo test -p cloud-gateway-client --no-run`
    - [x] All unit tests FAIL (red): `cd rhivos && cargo test -p cloud-gateway-client 2>&1 | grep FAILED`
    - [x] No linter warnings: `cd rhivos && cargo clippy -p cloud-gateway-client -- -D warnings`

- [x] 2. Config and command validation modules
  - [x] 2.1 Implement config module
    - Define `Config` struct
    - Read VIN (required, error if missing), NATS_URL (default nats://localhost:4222), DATABROKER_ADDR (default http://localhost:55556), BEARER_TOKEN (default demo-token)
    - _Requirements: 04-REQ-1.1, 04-REQ-5.1, 04-REQ-6.1, 04-REQ-6.E1_

  - [x] 2.2 Implement command validation module
    - Implement `validate_bearer_token(header, expected)`: parse "Bearer <token>", compare
    - Implement `parse_and_validate_command(payload)`: deserialize JSON, check command_id non-empty, action is lock/unlock
    - Define `IncomingCommand` struct with serde Deserialize
    - Also implemented `forward_command` in relay.rs (required to pass `-- config command` filter)
    - _Requirements: 04-REQ-2.1, 04-REQ-2.2, 04-REQ-2.E1, 04-REQ-2.E2, 04-REQ-2.E3_

  - [x] 2.V Verify task group 2
    - [x] Config and command tests pass: `cd rhivos && cargo test -p cloud-gateway-client -- config command`
    - [x] All existing tests still pass: `cd rhivos && cargo test`
    - [x] No linter warnings: `cd rhivos && cargo clippy -p cloud-gateway-client -- -D warnings`
    - [x] _Test Spec: TS-04-1, TS-04-3, TS-04-4, TS-04-5, TS-04-12, TS-04-13, TS-04-E3, TS-04-E4, TS-04-E5, TS-04-E10_

- [x] 3. Telemetry and response relay modules
  - [x] 3.1 Implement telemetry module
    - Define `TelemetryState` and `TelemetryMessage` structs
    - Implement `build_telemetry(vin, state)`: creates aggregated message with timestamp, skips None fields
    - Use `#[serde(skip_serializing_if = "Option::is_none")]` for optional fields
    - _Requirements: 04-REQ-4.2, 04-REQ-4.3, 04-REQ-4.E1_

  - [x] 3.2 Implement response relay helpers
    - Implement response relay function: takes response JSON string, publishes verbatim to NATS subject
    - Implement command forwarding function: takes validated command, sets DATA_BROKER signal
    - Both use trait-based abstractions for testability
    - _Requirements: 04-REQ-2.3, 04-REQ-3.2_

  - [x] 3.3 Create mock clients for tests
    - Mock BrokerClient (shared pattern with spec 03): configurable get/set/subscribe
    - Mock NatsClient: records publishes, configurable failures
    - _Test Spec: TS-04-6, TS-04-8, TS-04-E6, TS-04-E8_

  - [x] 3.V Verify task group 3
    - [x] Telemetry and relay tests pass: `cd rhivos && cargo test -p cloud-gateway-client -- telemetry relay`
    - [x] Property tests pass: `cd rhivos && cargo test -p cloud-gateway-client -- proptest`
    - [x] All existing tests still pass: `cd rhivos && cargo test`
    - [x] No linter warnings: `cd rhivos && cargo clippy -p cloud-gateway-client -- -D warnings`
    - [x] _Test Spec: TS-04-6, TS-04-8, TS-04-10, TS-04-11, TS-04-E6, TS-04-E7, TS-04-E8, TS-04-P1 through TS-04-P5_

- [x] 4. Checkpoint - Core Logic Complete
  - All unit and property tests pass
  - No integration tests yet (those require NATS + DATA_BROKER)
  - Ask the user if questions arise

- [x] 5. NATS client, DATA_BROKER client, and main loop
  - [x] 5.1 Implement NATS client wrapper
    - Wrap async-nats: connect, subscribe, publish, flush
    - Connection retry: exponential backoff 1s, 2s, 4s, 8s, up to 5 attempts
    - Post-connection reconnect delegated to async-nats client defaults
    - _Requirements: 04-REQ-1.1, 04-REQ-1.E1, 04-REQ-1.E2_

  - [x] 5.2 Implement DATA_BROKER client
    - `GrpcBrokerClient` wraps tonic transport channel
    - `connect` with exponential backoff retry (5 attempts)
    - `set_string` and `subscribe_signals` (structural placeholder; gRPC proto wired in task group 6)
    - `BrokerUpdate` / `BrokerValue` types defined for real streaming handler
    - _Requirements: 04-REQ-5.1, 04-REQ-5.2, 04-REQ-5.E1_

  - [x] 5.3 Implement main loop
    - Parse config, connect NATS + DATA_BROKER with retries, exit 1 on failure
    - Publish registration message to vehicles.{VIN}.status (04-REQ-6.2)
    - Subscribe NATS commands subject; subscribe DATA_BROKER response + telemetry signals
    - Spawn command handler task: validate token → validate JSON → forward to DATA_BROKER
    - Spawn response relay task: DATA_BROKER Response → NATS command_responses
    - Spawn telemetry task: DATA_BROKER signals → aggregate TelemetryState → NATS telemetry
    - Handle SIGTERM/SIGINT, flush NATS, exit 0
    - Log version, VIN, addresses, ready
    - _Requirements: 04-REQ-1.2, 04-REQ-3.1, 04-REQ-4.1, 04-REQ-6.2, 04-REQ-7.1, 04-REQ-7.2_

  - [x] 5.4 Implement error handling for publish failures
    - NATS publish failures: logged and swallowed in relay_response / publish_telemetry
    - DATA_BROKER set failures: logged, command handler continues
    - In-flight command completion: shutdown watch checked after processing each message
    - _Requirements: 04-REQ-3.E1, 04-REQ-4.E2, 04-REQ-7.E1_

  - [x] 5.V Verify task group 5
    - [x] Binary compiles: `cd rhivos && cargo build -p cloud-gateway-client`
    - [x] All unit tests still pass: `cd rhivos && cargo test -p cloud-gateway-client`
    - [x] No linter warnings: `cd rhivos && cargo clippy -p cloud-gateway-client -- -D warnings`

- [ ] 6. Integration test validation
  - [ ] 6.1 Create integration test module
    - Create `tests/cloud-gateway-client/` Go module
    - Shared helpers: start/stop NATS, start/stop databroker, start/stop service, NATS publish/subscribe helpers
    - Add `go.work` entry for `./tests/cloud-gateway-client`
    - _Test Spec: TS-04-2, TS-04-7, TS-04-9, TS-04-14, TS-04-15, TS-04-16, TS-04-17_

  - [ ] 6.2 Write and run integration tests
    - `TestNATSCommandSubscription` — TS-04-2
    - `TestResponseRelay` — TS-04-7
    - `TestTelemetrySubscription` — TS-04-9
    - `TestSelfRegistration` — TS-04-14
    - `TestGracefulShutdown` — TS-04-15
    - `TestStartupLogging` — TS-04-16
    - `TestDataBrokerOperations` — TS-04-17
    - _Test Spec: TS-04-2, TS-04-7, TS-04-9, TS-04-14, TS-04-15, TS-04-16, TS-04-17_

  - [ ] 6.3 Write and run edge case integration tests
    - `TestNATSUnreachable` — TS-04-E1
    - `TestNATSConnectionLost` — TS-04-E2
    - `TestDataBrokerUnreachable` — TS-04-E9
    - `TestSigtermDuringCommand` — TS-04-E11
    - _Test Spec: TS-04-E1, TS-04-E2, TS-04-E9, TS-04-E11_

  - [ ] 6.V Verify task group 6
    - [ ] All integration tests pass: `cd tests/cloud-gateway-client && go test -v ./...`
    - [ ] All unit tests still pass: `cd rhivos && cargo test -p cloud-gateway-client`
    - [ ] All existing tests still pass: `make test`
    - [ ] No linter warnings: `cd rhivos && cargo clippy -p cloud-gateway-client -- -D warnings`
    - [ ] All requirements 04-REQ-1 through 04-REQ-7 acceptance criteria met

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
| 04-REQ-1.1 | TS-04-1 | 2.1 | cloud-gateway-client::config::test_nats_url_default |
| 04-REQ-1.2 | TS-04-2 | 5.3 | tests/cloud-gateway-client::TestNATSCommandSubscription |
| 04-REQ-1.3 | TS-04-3 | 2.2 | cloud-gateway-client::command::test_bearer_token_valid |
| 04-REQ-1.E1 | TS-04-E1 | 5.1 | tests/cloud-gateway-client::TestNATSUnreachable |
| 04-REQ-1.E2 | TS-04-E2 | 5.1 | tests/cloud-gateway-client::TestNATSConnectionLost |
| 04-REQ-2.1 | TS-04-4 | 2.2 | cloud-gateway-client::command::test_bearer_token_cases |
| 04-REQ-2.2 | TS-04-5 | 2.2 | cloud-gateway-client::command::test_command_parse_valid |
| 04-REQ-2.3 | TS-04-6 | 3.2 | cloud-gateway-client::relay::test_command_forwarding |
| 04-REQ-2.E1 | TS-04-E3 | 2.2 | cloud-gateway-client::command::test_bearer_token_invalid |
| 04-REQ-2.E2 | TS-04-E4 | 2.2 | cloud-gateway-client::command::test_command_invalid_json |
| 04-REQ-2.E3 | TS-04-E5 | 2.2 | cloud-gateway-client::command::test_command_missing_field |
| 04-REQ-3.1 | TS-04-7 | 5.3 | tests/cloud-gateway-client::TestResponseRelay |
| 04-REQ-3.2 | TS-04-8 | 3.2 | cloud-gateway-client::relay::test_response_relay_verbatim |
| 04-REQ-3.E1 | TS-04-E6 | 5.4 | cloud-gateway-client::relay::test_response_publish_failure |
| 04-REQ-4.1 | TS-04-9 | 5.3 | tests/cloud-gateway-client::TestTelemetrySubscription |
| 04-REQ-4.2 | TS-04-10 | 3.1 | cloud-gateway-client::telemetry::test_telemetry_on_change |
| 04-REQ-4.3 | TS-04-11 | 3.1 | cloud-gateway-client::telemetry::test_telemetry_all_fields |
| 04-REQ-4.E1 | TS-04-E7 | 3.1 | cloud-gateway-client::telemetry::test_telemetry_omit_unset |
| 04-REQ-4.E2 | TS-04-E8 | 5.4 | cloud-gateway-client::telemetry::test_telemetry_publish_failure |
| 04-REQ-5.1 | TS-04-12 | 2.1 | cloud-gateway-client::config::test_databroker_addr_default |
| 04-REQ-5.2 | TS-04-17 | 5.2 | tests/cloud-gateway-client::TestDataBrokerOperations |
| 04-REQ-5.E1 | TS-04-E9 | 5.2 | tests/cloud-gateway-client::TestDataBrokerUnreachable |
| 04-REQ-6.1 | TS-04-13 | 2.1 | cloud-gateway-client::config::test_vin_from_env |
| 04-REQ-6.2 | TS-04-14 | 5.3 | tests/cloud-gateway-client::TestSelfRegistration |
| 04-REQ-6.E1 | TS-04-E10 | 2.1 | cloud-gateway-client::config::test_vin_missing |
| 04-REQ-7.1 | TS-04-15 | 5.3 | tests/cloud-gateway-client::TestGracefulShutdown |
| 04-REQ-7.2 | TS-04-16 | 5.3 | tests/cloud-gateway-client::TestStartupLogging |
| 04-REQ-7.E1 | TS-04-E11 | 5.3 | tests/cloud-gateway-client::TestSigtermDuringCommand |
| Property 1 | TS-04-P1 | 2.2 | cloud-gateway-client::proptest_bearer_token_gate |
| Property 2 | TS-04-P2 | 2.2 | cloud-gateway-client::proptest_command_validation |
| Property 3 | TS-04-P3 | 3.2 | cloud-gateway-client::proptest_response_fidelity |
| Property 4 | TS-04-P4 | 3.1 | cloud-gateway-client::proptest_telemetry_aggregation |
| Property 5 | TS-04-P5 | 2.1 | cloud-gateway-client::proptest_vin_subjects |
| Property 6 | TS-04-P6 | 5.3 | tests/cloud-gateway-client::TestGracefulShutdown |

## Notes

- The CLOUD_GATEWAY_CLIENT shares patterns with the LOCKING_SERVICE (spec 03): both use tonic-generated Kuksa gRPC clients, both have BrokerClient trait abstractions, both use mock clients for unit testing. Consider extracting shared broker code into a workspace-level crate if duplication becomes significant (but do not preemptively abstract — wait for the pattern to stabilize).
- The service runs three concurrent async tasks (command handler, response relay, telemetry publisher). Use `tokio::select!` in the main loop to handle shutdown signals alongside all three tasks.
- Integration tests require both NATS and Kuksa Databroker containers running. The test module handles start/stop via `podman compose`. Tests skip gracefully when Podman is unavailable.
- Bearer token validation is deliberately simple for the demo. The token is compared as a plain string — no cryptographic verification.
- The kuksa.val.v1 proto files are vendored per-crate (same approach as spec 03). If code sharing becomes needed, a shared proto crate can be introduced later.
- Property test TS-04-P6 is integration-level — it's validated by TS-04-15 and TS-04-E11 rather than a proptest.
