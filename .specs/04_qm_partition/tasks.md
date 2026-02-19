# Implementation Plan: PARKING_OPERATOR_ADAPTOR + UPDATE_SERVICE

<!-- AGENT INSTRUCTIONS
- Implement exactly ONE top-level task group per session
- Follow the git-flow: feature branch from develop -> implement -> test -> merge to develop -> push
- Update checkbox states as you go: [-] in progress, [x] complete
- The Mock PARKING_OPERATOR (Go) should be built first — the adaptor's REST
  client and integration tests depend on it
- PARKING_OPERATOR_ADAPTOR and UPDATE_SERVICE are independent Rust crates;
  after the mock operator is ready they can be developed in either order
- Integration tests require: Kuksa, LOCKING_SERVICE, Mock PARKING_OPERATOR,
  PARKING_OPERATOR_ADAPTOR, and (for lifecycle tests) UPDATE_SERVICE + podman
-->

## Overview

This plan implements the QM partition services in dependency order:

1. Mock PARKING_OPERATOR (Go REST) — needed by the adaptor's REST client and
   integration tests.
2. PARKING_OPERATOR_ADAPTOR config, session state, and REST client (Rust).
3. PARKING_OPERATOR_ADAPTOR lock watcher and gRPC server (Rust).
4. UPDATE_SERVICE state machine and persistence (Rust).
5. UPDATE_SERVICE podman wrapper, offloading, and gRPC server (Rust).
6. Mock PARKING_APP CLI — update spec 01 skeleton with real subcommands (Go).
7. End-to-end integration tests.

## Test Commands

- Go unit tests (parking-operator): `cd mock/parking-operator && go test ./...`
- Go unit tests (parking-app-cli): `cd mock/parking-app-cli && go test ./...`
- Rust unit tests: `cd rhivos && cargo test --workspace`
- Rust unit tests (adaptor): `cd rhivos && cargo test -p parking-operator-adaptor`
- Rust unit tests (update-service): `cd rhivos && cargo test -p update-service`
- All tests: `make test`
- Go linter: `cd mock/parking-operator && go vet ./...`
- Rust linter: `cd rhivos && cargo clippy --workspace -- -D warnings`
- All linters: `make lint`
- Build all: `make build`
- Integration tests: requires `make infra-up` + running services (see group 9)

## Tasks

- [x] 1. Mock PARKING_OPERATOR (Go REST Service)
  - [x] 1.1 Create project structure and data models
    - Create `mock/parking-operator/go.mod` (if not already scaffolded)
    - Create `mock/parking-operator/main.go` with CLI flag parsing
      (`--listen-addr`, `--rate-type`, `--rate-amount`, `--currency`)
    - Define session data model, rate config, and in-memory session store
    - _Requirements: 04-REQ-6.6_

  - [x] 1.2 Implement session endpoints
    - `POST /parking/start`: accept `{vehicle_id, zone_id, timestamp}`,
      generate `session_id`, store session, return `{session_id, status, rate}`
    - `POST /parking/stop`: accept `{session_id, timestamp}`, look up session,
      calculate fee, mark completed, return `{session_id, status, total_fee,
      duration_seconds, currency}`
    - Handle edge cases: unknown session_id → 404, duplicate start for same
      vehicle → return existing session
    - _Requirements: 04-REQ-6.1, 04-REQ-6.2, 04-REQ-6.E1, 04-REQ-6.E2_

  - [x] 1.3 Implement query endpoints
    - `GET /parking/sessions/{id}`: return session details including current
      fee for active sessions
    - `GET /parking/rate`: return configured `{zone_id, rate_type, rate_amount,
      currency}`
    - _Requirements: 04-REQ-6.3, 04-REQ-6.4_

  - [x] 1.4 Implement fee calculation
    - `per_minute`: `rate_amount × ceil(duration_minutes)`
    - `flat`: fixed `rate_amount` regardless of duration
    - Current fee for active sessions: calculate based on elapsed time
    - _Requirements: 04-REQ-6.5_

  - [x] 1.5 Write unit tests
    - Test each endpoint with `httptest.Server`
    - Test fee calculation with known durations and rates (per_minute and flat)
    - Test edge cases: unknown session 404, duplicate start returns existing
    - **Property 7: Fee Calculation Accuracy**
    - **Validates: 04-REQ-6.1, 04-REQ-6.2, 04-REQ-6.3, 04-REQ-6.4, 04-REQ-6.5**

  - [x] 1.V Verify task group 1
    - [x] `cd mock/parking-operator && go test ./...` passes
    - [x] `cd mock/parking-operator && go vet ./...` clean
    - [x] `go build` produces binary
    - [x] Fee calculation correct for per_minute and flat rates
    - [x] Requirements 04-REQ-6.1–6.6, 04-REQ-6.E1, 04-REQ-6.E2 met

- [x] 2. PARKING_OPERATOR_ADAPTOR: Config, Session, and REST Client
  - [x] 2.1 Create config module
    - Create `rhivos/parking-operator-adaptor/src/config.rs`
    - Parse env vars: `LISTEN_ADDR`, `DATABROKER_ADDR`, `PARKING_OPERATOR_URL`,
      `ZONE_ID`, `VEHICLE_VIN`
    - Defaults: `LISTEN_ADDR=0.0.0.0:50054`, `DATABROKER_ADDR=localhost:55555`
    - _Requirements: 04-REQ-2.6_

  - [x] 2.2 Create session state module
    - Create `rhivos/parking-operator-adaptor/src/session.rs`
    - Define `ParkingSession`, `RateType`, `SessionStatus` structs
    - Session state: `Arc<Mutex<Option<ParkingSession>>>`
    - Methods: `is_active()`, `complete(stop_response)`, `current_fee()`
    - _Requirements: 04-REQ-1.2, 04-REQ-1.4 (prerequisite)_

  - [x] 2.3 Create PARKING_OPERATOR REST client
    - Create `rhivos/parking-operator-adaptor/src/operator_client.rs`
    - Use `reqwest` for HTTP calls
    - `start_session(vehicle_id, zone_id, timestamp)` → POST /parking/start
    - `stop_session(session_id, timestamp)` → POST /parking/stop
    - `get_rate(zone_id)` → GET /parking/rate
    - `get_session(session_id)` → GET /parking/sessions/{id}
    - Define request/response JSON structs with serde
    - _Requirements: 04-REQ-1.2, 04-REQ-1.4, 04-REQ-2.5_

  - [x] 2.4 Write unit tests
    - Config: test defaults, test env var overrides
    - Session: test state transitions, current_fee calculation
    - Operator client: use `mockito` or `wiremock` to mock HTTP responses,
      verify request payloads and response parsing
    - Test operator unreachable → error propagation
    - _Requirements: 04-REQ-2.6, 04-REQ-1.E1_

  - [x] 2.V Verify task group 2
    - [x] `cargo test -p parking-operator-adaptor` passes
    - [x] `cargo clippy -p parking-operator-adaptor -- -D warnings` clean
    - [x] Requirements 04-REQ-2.6, 04-REQ-1.E1 acceptance criteria met

- [x] 3. PARKING_OPERATOR_ADAPTOR: Lock Watcher and gRPC Server
  - [x] 3.1 Create lock watcher
    - Create `rhivos/parking-operator-adaptor/src/lock_watcher.rs`
    - Subscribe to `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` on DATA_BROKER
    - On `IsLocked = true` AND no active session → call operator `start_session`,
      update session state, write `SessionActive = true` to DATA_BROKER
    - On `IsLocked = false` AND active session → call operator `stop_session`,
      complete session state, write `SessionActive = false` to DATA_BROKER
    - On operator error (start): log error, do not set SessionActive
    - Ignore duplicate events (lock while locked, unlock while unlocked)
    - _Requirements: 04-REQ-1.1, 04-REQ-1.2, 04-REQ-1.3, 04-REQ-1.4,
      04-REQ-1.5, 04-REQ-1.E1, 04-REQ-1.E2, 04-REQ-1.E3_

  - [x] 3.2 Create gRPC server
    - Create `rhivos/parking-operator-adaptor/src/grpc_server.rs`
    - Implement `ParkingAdapter` service from `parking_adapter.proto`
    - `StartSession`: start session with operator, write SessionActive, return
      session info; if already active, return existing session
    - `StopSession`: stop active session, write SessionActive=false, return fee
      summary; unknown session_id → NOT_FOUND
    - `GetStatus`: return current session state
    - `GetRate`: query operator's GET /parking/rate, return rate info
    - _Requirements: 04-REQ-2.1, 04-REQ-2.2, 04-REQ-2.3, 04-REQ-2.4,
      04-REQ-2.5, 04-REQ-2.E1, 04-REQ-2.E2_

  - [x] 3.3 Wire up main.rs
    - Replace spec 01 skeleton with real implementation
    - Initialize: config → Kuksa client → operator client → session state →
      spawn lock watcher task → start gRPC server
    - Graceful shutdown on SIGINT/SIGTERM
    - _Requirements: 04-REQ-1.1, 04-REQ-2.1_

  - [x] 3.4 Write unit tests
    - Lock watcher: mock Kuksa (trait-based) emitting IsLocked events + mock
      operator client; verify start/stop calls, SessionActive writes,
      duplicate event handling, operator error handling
    - gRPC server: start on random port, call each RPC, verify responses
    - Test StartSession while active → returns existing
    - Test StopSession with unknown ID → NOT_FOUND
    - **Property 1: Event-Session Invariant**
    - **Property 2: Session Idempotency**
    - **Property 8: SessionActive Signal Accuracy**
    - **Validates: 04-REQ-1.1–1.5, 04-REQ-1.E1–1.E3, 04-REQ-2.1–2.5,
      04-REQ-2.E1–2.E2**

  - [x] 3.V Verify task group 3
    - [x] `cargo test -p parking-operator-adaptor` passes all tests
    - [x] `cargo clippy -p parking-operator-adaptor -- -D warnings` clean
    - [x] Requirements 04-REQ-1.1–1.5, 04-REQ-2.1–2.5, edge cases met

- [x] 4. Checkpoint — PARKING_OPERATOR_ADAPTOR Complete
  - Mock PARKING_OPERATOR and PARKING_OPERATOR_ADAPTOR both working
  - Commit and verify clean state

- [x] 5. UPDATE_SERVICE: State Machine and Persistence
  - [x] 5.1 Create config module
    - Create `rhivos/update-service/src/config.rs`
    - Parse env vars / CLI flags: `LISTEN_ADDR` (default: `0.0.0.0:50053`),
      `DATA_DIR` (default: `./data`), `OFFLOAD_TIMEOUT` (default: `5m`)
    - _Requirements: 04-REQ-4.6_

  - [x] 5.2 Create adapter state machine
    - Create `rhivos/update-service/src/state.rs`
    - Define `AdapterState` enum: Unknown, Installing, Running, Stopped,
      Error(String), Offloading
    - Implement `can_transition_to()` enforcing valid transitions
    - Define `AdapterEntry` struct with all fields from design doc
    - Implement `transition()` method that validates and applies transitions
    - _Requirements: 04-REQ-3.4_

  - [x] 5.3 Create persistence module
    - Persistence in `state.rs` or separate helper
    - Save adapter entries to `{data_dir}/adapters.json`
    - Load on startup, handle missing file gracefully (empty list)
    - _Requirements: 04-REQ-3.5_

  - [x] 5.4 Write property tests for state machine
    - Valid transitions: verify all edges in the state diagram are accepted
    - Invalid transitions: verify all non-edges are rejected
    - Property-based test: random sequences of transitions, verify only
      valid transitions succeed
    - Persistence round-trip: write state, read back, verify equality
    - **Property 3: State Machine Integrity**
    - **Property 6: Persistence Round-Trip**
    - **Validates: 04-REQ-3.4, 04-REQ-3.5**

  - [x] 5.V Verify task group 5
    - [x] `cargo test -p update-service` passes
    - [x] `cargo clippy -p update-service -- -D warnings` clean
    - [x] All valid state transitions accepted, invalid ones rejected
    - [x] Persistence round-trip verified
    - [x] Requirements 04-REQ-3.4, 04-REQ-3.5, 04-REQ-4.6 met

- [ ] 6. UPDATE_SERVICE: Podman, Offloading, and gRPC Server
  - [ ] 6.1 Create podman CLI wrapper
    - Create `rhivos/update-service/src/podman.rs`
    - Trait-based design for testability (mock command executor)
    - `create_and_start(name, image, env_vars, network)` → podman create + start
    - `stop_and_remove(name)` → podman stop + rm
    - `is_running(name)` → podman inspect
    - `list(prefix)` → podman ps with filter
    - _Requirements: 04-REQ-3.1, 04-REQ-3.3_

  - [ ] 6.2 Create offload timer manager
    - Create `rhivos/update-service/src/offload.rs`
    - `OffloadManager` with configurable timeout (default: 5 minutes)
    - `start_timer(adapter_id, callback)` → start tokio timer
    - `cancel_timer(adapter_id)` → cancel running timer
    - Timer expiry → invoke callback to offload the adapter
    - _Requirements: 04-REQ-5.1, 04-REQ-5.2, 04-REQ-5.3, 04-REQ-5.4,
      04-REQ-5.E1_

  - [ ] 6.3 Create gRPC server
    - Create `rhivos/update-service/src/grpc_server.rs`
    - Implement `UpdateService` from `update_service.proto`
    - `InstallAdapter`: create container via podman, transition UNKNOWN →
      INSTALLING → RUNNING (or ERROR), persist state, return response
    - `RemoveAdapter`: stop+remove container, transition to STOPPED, persist,
      cancel offload timer if running
    - `ListAdapters`: return all adapters with current state
    - `GetAdapterStatus`: return single adapter info or NOT_FOUND
    - `WatchAdapterStates`: use broadcast channel to stream state events
    - Pass env vars to container: DATABROKER_ADDR, PARKING_OPERATOR_URL,
      ZONE_ID, VEHICLE_VIN, LISTEN_ADDR
    - Handle edge cases: already RUNNING → return existing, unknown
      adapter_id → NOT_FOUND, podman failure → ERROR state
    - _Requirements: 04-REQ-4.1, 04-REQ-4.2, 04-REQ-4.3, 04-REQ-4.4,
      04-REQ-4.5, 04-REQ-3.1, 04-REQ-3.2, 04-REQ-3.3, 04-REQ-3.6,
      04-REQ-3.E1, 04-REQ-3.E2, 04-REQ-3.E3, 04-REQ-4.E1, 04-REQ-4.E2_

  - [ ] 6.4 Wire up main.rs
    - Replace spec 01 skeleton with real implementation
    - Initialize: config → load persisted state → reconcile with podman ps →
      start gRPC server → run offload timers for adapters with ended sessions
    - Graceful shutdown: stop offload timers, persist final state
    - _Requirements: 04-REQ-3.6, 04-REQ-4.1_

  - [ ] 6.5 Write unit tests
    - Podman wrapper: mock command executor, verify correct CLI invocations
    - Offload timer: use `tokio::time::pause()` to control time, verify timer
      fires after timeout, verify cancellation prevents firing
    - gRPC server: start on random port, test InstallAdapter, RemoveAdapter,
      ListAdapters, GetAdapterStatus, WatchAdapterStates
    - Test edge cases: already running, unknown adapter, podman failure
    - State reconciliation: persisted state with mock podman, verify
      reconciliation logic
    - **Property 4: Podman-State Consistency**
    - **Property 5: Offloading Timer Correctness**
    - **Validates: 04-REQ-3.1–3.6, 04-REQ-3.E1–3.E3, 04-REQ-4.1–4.5,
      04-REQ-4.E1–4.E2, 04-REQ-5.1–5.4, 04-REQ-5.E1**

  - [ ] 6.V Verify task group 6
    - [ ] `cargo test -p update-service` passes all tests
    - [ ] `cargo clippy -p update-service -- -D warnings` clean
    - [ ] Requirements 04-REQ-3.1–3.6, 04-REQ-4.1–4.6, 04-REQ-5.1–5.4 met

- [ ] 7. Checkpoint — UPDATE_SERVICE Complete
  - UPDATE_SERVICE state machine, podman, offloading, gRPC all working
  - Commit and verify clean state

- [ ] 8. Mock PARKING_APP CLI
  - [ ] 8.1 Implement UPDATE_SERVICE subcommands
    - Replace spec 01 skeleton `mock/parking-app-cli/main.go` with real
      implementation
    - `install-adapter --image-ref <ref> [--checksum <sha>]` → gRPC
      InstallAdapter, print response
    - `list-adapters` → gRPC ListAdapters, print table
    - `remove-adapter --adapter-id <id>` → gRPC RemoveAdapter, print response
    - `adapter-status --adapter-id <id>` → gRPC GetAdapterStatus, print info
    - _Requirements: 04-REQ-7.1_

  - [ ] 8.2 Implement PARKING_OPERATOR_ADAPTOR subcommands
    - `start-session --zone-id <zone> --vehicle-vin <vin>` → gRPC
      StartSession, print session info
    - `stop-session --session-id <id>` → gRPC StopSession, print fee summary
    - `get-status [--session-id <id>]` → gRPC GetStatus, print state
    - `get-rate --zone-id <zone>` → gRPC GetRate, print rate info
    - _Requirements: 04-REQ-7.2_

  - [ ] 8.3 Implement watch-adapters streaming
    - `watch-adapters` → gRPC WatchAdapterStates server stream
    - Print each `AdapterStateEvent` as it arrives, until SIGINT
    - _Requirements: 04-REQ-7.3_

  - [ ] 8.4 Add flags and error handling
    - Global flags: `--update-service-addr` (default: `localhost:50053`),
      `--adapter-addr` (default: `localhost:50054`)
    - Service unreachable → print error, exit non-zero
    - _Requirements: 04-REQ-7.4, 04-REQ-7.E1_

  - [ ] 8.5 Write tests
    - Argument parsing for each subcommand
    - gRPC request construction (use a mock gRPC server or verify call params)
    - Error handling: unreachable service → non-zero exit
    - **Validates: 04-REQ-7.1, 04-REQ-7.2, 04-REQ-7.3, 04-REQ-7.4,
      04-REQ-7.E1**

  - [ ] 8.V Verify task group 8
    - [ ] `cd mock/parking-app-cli && go test ./...` passes
    - [ ] `cd mock/parking-app-cli && go vet ./...` clean
    - [ ] `parking-app-cli --help` shows all subcommands
    - [ ] Requirements 04-REQ-7.1–7.4, 04-REQ-7.E1 met

- [ ] 9. Integration Tests
  - [ ] 9.1 Create integration test harness
    - Test infrastructure: `make infra-up` (Kuksa + Mosquitto),
      LOCKING_SERVICE, mock PARKING_OPERATOR, PARKING_OPERATOR_ADAPTOR
      (standalone, not in container for session flow tests)
    - Wait for services to be ready (gRPC health checks, HTTP health checks)
    - Tests skip cleanly if infrastructure is unavailable
    - _Requirements: 04-REQ-8.E1_

  - [ ] 9.2 Test session flow end-to-end
    - Set safe conditions via mock-sensors
    - Lock via mock-sensors → verify PARKING_OPERATOR_ADAPTOR starts session →
      verify `SessionActive = true` in Kuksa → verify mock PARKING_OPERATOR
      has active session
    - Unlock via mock-sensors → verify session stopped → verify
      `SessionActive = false` → verify mock PARKING_OPERATOR has completed
      session with calculated fee
    - **Property 1: Event-Session Invariant**
    - **Property 8: SessionActive Signal Accuracy**
    - _Requirements: 04-REQ-8.1, 04-REQ-8.2_

  - [ ] 9.3 Test adapter lifecycle via CLI
    - Use parking-app-cli to `install-adapter` → verify adapter state RUNNING
      via `list-adapters`
    - Use `remove-adapter` → verify state STOPPED
    - Requires podman available; skip if not
    - _Requirements: 04-REQ-8.3_

  - [ ] 9.4 Test offloading
    - Install adapter → start session → stop session → wait for offload
      timeout (use short timeout, e.g., 5s for test) → verify adapter state
      becomes UNKNOWN
    - Requires podman; skip if not available
    - **Property 5: Offloading Timer Correctness**
    - _Requirements: 04-REQ-8.4_

  - [ ] 9.V Verify task group 9
    - [ ] All integration tests pass with infrastructure running
    - [ ] Tests skip cleanly when infrastructure is unavailable
    - [ ] Requirements 04-REQ-8.1–8.4 acceptance criteria met

- [ ] 10. Final Verification and Documentation
  - [ ] 10.1 Run full test suite
    - `make build && make test && make lint`
    - Verify no regressions in specs 01, 02, and 03 tests
    - _Requirements: all_

  - [ ] 10.2 Run integration tests
    - Start all infrastructure and services
    - Run all integration tests (session flow + lifecycle + offloading)
    - Verify all pass

  - [ ] 10.3 Update documentation
    - Document mock PARKING_OPERATOR REST API in `docs/parking-operator-api.md`
    - Document adapter lifecycle and offloading in `docs/adapter-lifecycle.md`
    - Update README if needed

  - [ ] 10.V Verify task group 10
    - [ ] `make build` succeeds
    - [ ] `make test` passes
    - [ ] `make lint` clean
    - [ ] Integration tests pass
    - [ ] No regressions from specs 01, 02, and 03
    - [ ] All 04-REQ requirements verified

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
| 04-REQ-1.1 | 3.1, 3.3 | Lock watcher unit test (3.4) |
| 04-REQ-1.2 | 3.1 | Lock watcher unit test (3.4) |
| 04-REQ-1.3 | 3.1 | Lock watcher unit test (3.4), integration test (9.2) |
| 04-REQ-1.4 | 3.1 | Lock watcher unit test (3.4) |
| 04-REQ-1.5 | 3.1 | Lock watcher unit test (3.4), integration test (9.2) |
| 04-REQ-1.E1 | 3.1 | Lock watcher unit test (3.4) — operator error |
| 04-REQ-1.E2 | 3.1 | Lock watcher unit test (3.4) — duplicate lock |
| 04-REQ-1.E3 | 3.1 | Lock watcher unit test (3.4) — duplicate unlock |
| 04-REQ-2.1 | 3.2 | gRPC server unit test (3.4) |
| 04-REQ-2.2 | 3.2 | gRPC server unit test (3.4) |
| 04-REQ-2.3 | 3.2 | gRPC server unit test (3.4) |
| 04-REQ-2.4 | 3.2 | gRPC server unit test (3.4) |
| 04-REQ-2.5 | 3.2 | gRPC server unit test (3.4) |
| 04-REQ-2.6 | 2.1 | Config unit test (2.4) |
| 04-REQ-2.E1 | 3.2 | gRPC server unit test (3.4) — StartSession while active |
| 04-REQ-2.E2 | 3.2 | gRPC server unit test (3.4) — unknown session_id |
| 04-REQ-3.1 | 6.1, 6.3 | Podman wrapper test (6.5), gRPC test (6.5) |
| 04-REQ-3.2 | 6.3 | gRPC server test (6.5) — env vars |
| 04-REQ-3.3 | 6.1, 6.3 | Podman wrapper test (6.5), gRPC test (6.5) |
| 04-REQ-3.4 | 5.2 | State machine property test (5.4) |
| 04-REQ-3.5 | 5.3 | Persistence round-trip test (5.4) |
| 04-REQ-3.6 | 6.4 | State reconciliation test (6.5) |
| 04-REQ-3.E1 | 6.3 | gRPC test (6.5) — podman failure |
| 04-REQ-3.E2 | 6.3 | gRPC test (6.5) — already running |
| 04-REQ-3.E3 | 6.3 | gRPC test (6.5) — image not found |
| 04-REQ-4.1 | 6.3, 6.4 | gRPC server test (6.5) |
| 04-REQ-4.2 | 6.3 | gRPC server test (6.5) — InstallAdapter response |
| 04-REQ-4.3 | 6.3 | gRPC server test (6.5) — WatchAdapterStates |
| 04-REQ-4.4 | 6.3 | gRPC server test (6.5) — ListAdapters |
| 04-REQ-4.5 | 6.3 | gRPC server test (6.5) — GetAdapterStatus |
| 04-REQ-4.6 | 5.1 | Config unit test (5.4) |
| 04-REQ-4.E1 | 6.3 | gRPC server test (6.5) — unknown adapter |
| 04-REQ-4.E2 | 6.3 | gRPC server test (6.5) — unknown adapter |
| 04-REQ-5.1 | 6.2 | Offload timer test (6.5) |
| 04-REQ-5.2 | 6.2 | Offload timer test (6.5) |
| 04-REQ-5.3 | 6.2 | Offload timer test (6.5) — cancel on new session |
| 04-REQ-5.4 | 5.1 | Config test (5.4) |
| 04-REQ-5.E1 | 6.2, 6.3 | Offload timer test (6.5) — manual remove cancels |
| 04-REQ-6.1 | 1.2 | Handler test (1.5) |
| 04-REQ-6.2 | 1.2 | Handler test (1.5) |
| 04-REQ-6.3 | 1.3 | Handler test (1.5) |
| 04-REQ-6.4 | 1.3 | Handler test (1.5) |
| 04-REQ-6.5 | 1.4 | Fee calculation test (1.5) |
| 04-REQ-6.6 | 1.1 | Config/flag parsing test (1.5) |
| 04-REQ-6.E1 | 1.2 | Handler test (1.5) — unknown session 404 |
| 04-REQ-6.E2 | 1.2 | Handler test (1.5) — duplicate start |
| 04-REQ-7.1 | 8.1 | CLI test (8.5) |
| 04-REQ-7.2 | 8.2 | CLI test (8.5) |
| 04-REQ-7.3 | 8.3 | CLI test (8.5) |
| 04-REQ-7.4 | 8.4 | CLI test (8.5) |
| 04-REQ-7.E1 | 8.4 | CLI test (8.5) — unreachable service |
| 04-REQ-8.1 | 9.2 | Integration test `test_session_start` |
| 04-REQ-8.2 | 9.2 | Integration test `test_session_stop` |
| 04-REQ-8.3 | 9.3 | Integration test `test_adapter_lifecycle` |
| 04-REQ-8.4 | 9.4 | Integration test `test_offloading` |
| 04-REQ-8.E1 | 9.1 | Test skip behavior |

## Notes

- **Mock PARKING_OPERATOR goes first:** The adaptor's REST client depends on
  the mock operator's API. Building and testing the mock operator first
  provides a working target for adaptor development and integration tests.
- **Standalone adaptor for session tests:** Integration tests for the session
  flow (lock → session start) run the PARKING_OPERATOR_ADAPTOR as a standalone
  process, not inside a podman container. This avoids podman dependency for
  the most critical test path.
- **Podman-dependent tests:** Adapter lifecycle (install/remove) and offloading
  integration tests require podman. These tests are skipped if podman is not
  available, similar to how spec 02 integration tests skip without Kuksa.
- **Offload timer in tests:** Integration tests for offloading use a short
  timeout (e.g., 5 seconds via `OFFLOAD_TIMEOUT=5s`) to avoid long waits.
- **Trait-based mocking:** The podman wrapper uses a trait so unit tests can
  inject a mock command executor. The Kuksa client and operator client are
  also trait-based for the same reason.
- **No changes to LOCKING_SERVICE, mock-sensors, or DATA_BROKER:** These
  components from specs 01–02 are used as-is.
