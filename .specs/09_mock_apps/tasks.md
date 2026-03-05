# Implementation Tasks: Mock Apps (Spec 09)

> Task breakdown for implementing all mock/demo tools.
> Implements design from `.specs/09_mock_apps/design.md`.
> Validates against `.specs/09_mock_apps/test_spec.md`.

## Dependencies

| Spec | Relationship |
|------|-------------|
| 01_project_setup | Requires Go module structure, proto definitions, generated gRPC code, Go workspace (`go.work`), Rust workspace (`rhivos/Cargo.toml`) |
| 02_data_broker | Mock sensors write to DATA_BROKER via kuksa.val.v1 gRPC API |
| 05_parking_fee_service | Mock PARKING_APP CLI calls PARKING_FEE_SERVICE REST API |
| 06_cloud_gateway | Mock COMPANION_APP CLI calls CLOUD_GATEWAY REST API |
| 07_update_service | Mock PARKING_APP CLI calls UPDATE_SERVICE gRPC API |
| 08_parking_operator_adaptor | Mock PARKING_APP CLI calls PARKING_OPERATOR_ADAPTOR gRPC API; mock PARKING_OPERATOR receives calls from PARKING_OPERATOR_ADAPTOR |

## Test Commands

| Action | Command |
|--------|---------|
| Sensor tests | `cd rhivos && cargo test -p location-sensor -p speed-sensor -p door-sensor` |
| Sensor lint | `cd rhivos && cargo clippy -p location-sensor -p speed-sensor -p door-sensor` |
| Sensor build | `cd rhivos && cargo build -p location-sensor -p speed-sensor -p door-sensor` |
| parking-operator tests | `cd mock/parking-operator && go test ./... -v` |
| parking-operator lint | `cd mock/parking-operator && go vet ./...` |
| parking-app-cli tests | `cd mock/parking-app-cli && go test ./... -v` |
| parking-app-cli lint | `cd mock/parking-app-cli && go vet ./...` |
| companion-app-cli tests | `cd mock/companion-app-cli && go test ./... -v` |
| companion-app-cli lint | `cd mock/companion-app-cli && go vet ./...` |

---

## Group 1: Write Failing Spec Tests

**Goal:** Create test files that encode all test specifications. Tests must compile but fail (red phase of red-green-refactor).

### Task 1.1: Create Rust sensor test scaffolding

Add the three sensor crates to the Rust workspace (`rhivos/Cargo.toml`). Create minimal `Cargo.toml` and `src/main.rs` for each crate with stub `main()` functions. Add inline test modules with the following tests:

- **location-sensor:**
  - `test_missing_lat_lon_exits_with_error` (TS-09-E1)
  - `test_missing_lon_exits_with_error` (TS-09-E1)
  - `test_writes_correct_latitude_and_longitude` (TS-09-1) -- may use a mock gRPC server or mark as integration test

- **speed-sensor:**
  - `test_missing_speed_exits_with_error` (TS-09-E1)
  - `test_writes_correct_speed` (TS-09-2)

- **door-sensor:**
  - `test_missing_open_or_closed_exits_with_error` (TS-09-E1)
  - `test_writes_open_true` (TS-09-3)
  - `test_writes_closed_false` (TS-09-4)

**Files:**
- `rhivos/Cargo.toml` (updated workspace members)
- `rhivos/location-sensor/Cargo.toml`
- `rhivos/location-sensor/src/main.rs`
- `rhivos/speed-sensor/Cargo.toml`
- `rhivos/speed-sensor/src/main.rs`
- `rhivos/door-sensor/Cargo.toml`
- `rhivos/door-sensor/src/main.rs`

**Verify:** `cd rhivos && cargo test -p location-sensor -p speed-sensor -p door-sensor` -- tests compile but fail.

### Task 1.2: Create parking-operator test scaffolding

Create `mock/parking-operator/go.mod` and minimal stub files (`main.go`, `handler.go`, `session.go`, `models.go`). Create `mock/parking-operator/handler_test.go` with:

- `TestStartSession_Valid` (TS-09-7)
- `TestStopSession_Valid` (TS-09-8)
- `TestGetStatus_ReturnsAllSessions` (TS-09-9)
- `TestGetStatus_EmptyWhenNoSessions` (TS-09-E9)
- `TestStartSession_MalformedBody` (TS-09-E7)
- `TestStopSession_UnknownSession` (TS-09-E8)
- `TestSessionStoreConsistency` (TS-09-P8)

Add `mock/parking-operator` to the root `go.work` file.

**Files:** `mock/parking-operator/go.mod`, `mock/parking-operator/main.go`, `mock/parking-operator/handler.go`, `mock/parking-operator/session.go`, `mock/parking-operator/models.go`, `mock/parking-operator/handler_test.go`

**Verify:** `cd mock/parking-operator && go test ./... -v` -- tests compile but fail.

### Task 1.3: Create parking-app-cli test scaffolding

Create `mock/parking-app-cli/go.mod` and minimal stub files. Create test files with:

- `TestSubcommandDispatch_UnknownCommand` (TS-09-E4)
- `TestSubcommandDispatch_NoArguments` (TS-09-E4)
- `TestLookup_MissingFlags` (TS-09-E3)
- `TestAdapterInfo_MissingFlags` (TS-09-E3)
- `TestInstall_MissingFlags` (TS-09-E3)
- `TestRemove_MissingFlags` (TS-09-E3)
- `TestStatus_MissingFlags` (TS-09-E3)
- `TestStartSession_MissingFlags` (TS-09-E3)
- `TestStopSession_MissingFlags` (TS-09-E3)
- `TestLookup_CorrectRESTEndpoint` (TS-09-P1)
- `TestAdapterInfo_CorrectRESTEndpoint` (TS-09-P2)
- `TestInstall_CorrectGRPCMethod` (TS-09-P3)
- `TestServiceUnreachable_REST` (TS-09-E5)
- `TestServiceUnreachable_GRPC` (TS-09-E6)

Add `mock/parking-app-cli` to the root `go.work` file.

**Files:** `mock/parking-app-cli/go.mod`, `mock/parking-app-cli/main.go`, `mock/parking-app-cli/cmd/*.go` (stubs)

**Verify:** `cd mock/parking-app-cli && go test ./... -v` -- tests compile but fail.

### Task 1.4: Create companion-app-cli test scaffolding

Create `mock/companion-app-cli/go.mod` and minimal stub files. Create test files with:

- `TestSubcommandDispatch_UnknownCommand` (TS-09-E4)
- `TestSubcommandDispatch_NoArguments` (TS-09-E4)
- `TestLock_MissingFlags` (TS-09-E3)
- `TestUnlock_MissingFlags` (TS-09-E3)
- `TestStatus_MissingFlags` (TS-09-E3)
- `TestLock_CorrectPayload` (TS-09-P4)
- `TestUnlock_CorrectPayload` (TS-09-P5)
- `TestStatus_CorrectEndpoint` (TS-09-P6)
- `TestBearerToken_IncludedInRequests` (TS-09-P4)
- `TestServiceUnreachable_REST` (TS-09-E5)

Add `mock/companion-app-cli` to the root `go.work` file.

**Files:** `mock/companion-app-cli/go.mod`, `mock/companion-app-cli/main.go`, `mock/companion-app-cli/cmd/*.go` (stubs)

**Verify:** `cd mock/companion-app-cli && go test ./... -v` -- tests compile but fail.

---

## Group 2: Implement Mock Sensors

**Goal:** Implement the three Rust CLI sensor tools that write VSS signals to DATA_BROKER.

### Task 2.1: Implement location-sensor

Implement `rhivos/location-sensor/src/main.rs`:

1. Parse CLI arguments using `clap`: `--lat` (f64, required), `--lon` (f64, required), `--broker-addr` (string, default `http://localhost:55556`).
2. Connect to DATA_BROKER via gRPC (tonic client for kuksa.val.v1).
3. Send `SetRequest` for `Vehicle.CurrentLocation.Latitude` with the specified latitude value.
4. Send `SetRequest` for `Vehicle.CurrentLocation.Longitude` with the specified longitude value.
5. Print confirmation message to stdout (e.g., `Set Vehicle.CurrentLocation.Latitude = 48.1351`).
6. Exit with code 0 on success; exit with code 1 and error message on failure.

**Files:** `rhivos/location-sensor/Cargo.toml`, `rhivos/location-sensor/src/main.rs`

**Verify:** Location sensor tests from Task 1.1 pass.

### Task 2.2: Implement speed-sensor

Implement `rhivos/speed-sensor/src/main.rs`:

1. Parse CLI arguments: `--speed` (f32, required), `--broker-addr` (string, default `http://localhost:55556`).
2. Connect to DATA_BROKER via gRPC.
3. Send `SetRequest` for `Vehicle.Speed` with the specified speed value.
4. Print confirmation and exit.

**Files:** `rhivos/speed-sensor/Cargo.toml`, `rhivos/speed-sensor/src/main.rs`

**Verify:** Speed sensor tests from Task 1.1 pass.

### Task 2.3: Implement door-sensor

Implement `rhivos/door-sensor/src/main.rs`:

1. Parse CLI arguments: `--open` (bool flag) and `--closed` (bool flag), mutually exclusive, one required. `--broker-addr` (string, default `http://localhost:55556`).
2. Connect to DATA_BROKER via gRPC.
3. Send `SetRequest` for `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` with `true` (--open) or `false` (--closed).
4. Print confirmation and exit.

**Files:** `rhivos/door-sensor/Cargo.toml`, `rhivos/door-sensor/src/main.rs`

**Verify:** Door sensor tests from Task 1.1 pass. `cd rhivos && cargo clippy -p location-sensor -p speed-sensor -p door-sensor` reports no warnings.

---

## Group 3: Implement Mock PARKING_OPERATOR

**Goal:** Implement the mock PARKING_OPERATOR Go REST server.

### Task 3.1: Implement data model and session store

Create `mock/parking-operator/models.go` with request/response types:

- `StartRequest` (vehicle_id, zone_id, timestamp)
- `StartResponse` (session_id, status)
- `StopRequest` (session_id)
- `StopResponse` (session_id, duration_seconds, fee, status)
- `Session` (session_id, vehicle_id, zone_id, start_time, status)
- `ErrorResponse` (error)

Create `mock/parking-operator/session.go` with in-memory `SessionStore`:

- `NewSessionStore() *SessionStore`
- `Create(vehicleID, zoneID string) *Session` -- generates UUID, stores with status "active"
- `Stop(sessionID string) (*StopResponse, error)` -- calculates duration and fee (rate: 2.50 EUR/hr), marks "completed"
- `List() []Session` -- returns all sessions

**Files:** `mock/parking-operator/models.go`, `mock/parking-operator/session.go`

### Task 3.2: Implement HTTP handlers

Create `mock/parking-operator/handler.go`:

- `HandleStartParking(store *SessionStore) http.HandlerFunc` -- parse JSON body, validate, create session, return 200 with session data or 400 on error.
- `HandleStopParking(store *SessionStore) http.HandlerFunc` -- parse JSON body, validate, stop session, return 200/404/400.
- `HandleParkingStatus(store *SessionStore) http.HandlerFunc` -- return all sessions as JSON array.
- `writeJSON` and `writeError` helpers.

**Files:** `mock/parking-operator/handler.go`

### Task 3.3: Implement server entry point

Create `mock/parking-operator/main.go`:

- Read port from `PORT` env var or `-port` flag (default: 9090).
- Register routes: `POST /parking/start`, `POST /parking/stop`, `GET /parking/status`.
- Start HTTP server and log startup message.

**Files:** `mock/parking-operator/main.go`

**Verify:** `cd mock/parking-operator && go test ./... -v` -- all tests pass. `cd mock/parking-operator && go vet ./...` -- no issues.

---

## Group 4: Implement parking-app-cli

**Goal:** Implement the mock PARKING_APP CLI with all 9 subcommands.

### Task 4.1: Create shared internal packages

Create the internal packages for parking-app-cli:

- `internal/config/config.go` -- Read environment variables (`PARKING_FEE_SERVICE_URL`, `UPDATE_SERVICE_ADDR`, `PARKING_ADAPTOR_ADDR`, `DATA_BROKER_ADDR`) with defaults. Flag-overrides-env precedence.
- `internal/output/output.go` -- `PrintJSON(v interface{})` for indented JSON to stdout. `PrintError(format, args)` for errors to stderr.
- `internal/restclient/client.go` -- HTTP client wrapper with 10-second timeout.
- `internal/grpcclient/client.go` -- gRPC `Dial(addr string)` helper with insecure credentials and 10-second timeout.

**Files:** `mock/parking-app-cli/internal/config/config.go`, `mock/parking-app-cli/internal/output/output.go`, `mock/parking-app-cli/internal/restclient/client.go`, `mock/parking-app-cli/internal/grpcclient/client.go`

### Task 4.2: Implement subcommand dispatch

Implement `mock/parking-app-cli/main.go` with subcommand dispatch:

- Parse `os.Args[1]` and route to the appropriate handler.
- Print usage and exit code 1 for unknown subcommands or no arguments.
- List all 9 available subcommands in usage message.

**Files:** `mock/parking-app-cli/main.go`

### Task 4.3: Implement REST subcommands (lookup, adapter-info)

Create `mock/parking-app-cli/cmd/lookup.go`:

- Parse `--lat` and `--lon` flags.
- Send GET to `{PARKING_FEE_SERVICE_URL}/operators?lat={lat}&lon={lon}`.
- Print response JSON or error.

Create `mock/parking-app-cli/cmd/adapter_info.go`:

- Parse `--operator-id` flag.
- Send GET to `{PARKING_FEE_SERVICE_URL}/operators/{id}/adapter`.
- Print response JSON or error.

**Files:** `mock/parking-app-cli/cmd/lookup.go`, `mock/parking-app-cli/cmd/adapter_info.go`

### Task 4.4: Implement gRPC subcommands (install, watch, list, remove, status)

Create subcommand files under `mock/parking-app-cli/cmd/`:

- `install.go` -- Parse `--image-ref`, `--checksum`. Call `InstallAdapter`. Print response.
- `watch.go` -- Call `WatchAdapterStates` (streaming). Print events. Handle Ctrl+C.
- `list.go` -- Call `ListAdapters`. Print response.
- `remove.go` -- Parse `--adapter-id`. Call `RemoveAdapter`. Print response.
- `status.go` -- Parse `--adapter-id`. Call `GetAdapterStatus`. Print response.

**Files:** `mock/parking-app-cli/cmd/install.go`, `mock/parking-app-cli/cmd/watch.go`, `mock/parking-app-cli/cmd/list.go`, `mock/parking-app-cli/cmd/remove.go`, `mock/parking-app-cli/cmd/status.go`

### Task 4.5: Implement session management subcommands (start-session, stop-session)

Create subcommand files:

- `start_session.go` -- Parse `--zone-id`. Dial PARKING_OPERATOR_ADAPTOR. Call `StartSession`. Print response.
- `stop_session.go` -- Parse `--session-id`. Dial PARKING_OPERATOR_ADAPTOR. Call `StopSession`. Print response.

**Files:** `mock/parking-app-cli/cmd/start_session.go`, `mock/parking-app-cli/cmd/stop_session.go`

**Verify:** `cd mock/parking-app-cli && go test ./... -v` -- all tests pass. `cd mock/parking-app-cli && go vet ./...` -- no issues. `go build ./mock/parking-app-cli/...` -- builds successfully.

---

## Group 5: Implement companion-app-cli

**Goal:** Implement the mock COMPANION_APP CLI with all 3 subcommands.

### Task 5.1: Create shared internal packages

Create the internal packages for companion-app-cli:

- `internal/config/config.go` -- Read `CLOUD_GATEWAY_URL`, `BEARER_TOKEN` with defaults.
- `internal/output/output.go` -- Same pattern as parking-app-cli.
- `internal/restclient/client.go` -- HTTP client wrapper with bearer token support.

**Files:** `mock/companion-app-cli/internal/config/config.go`, `mock/companion-app-cli/internal/output/output.go`, `mock/companion-app-cli/internal/restclient/client.go`

### Task 5.2: Implement subcommand dispatch and commands

Implement `mock/companion-app-cli/main.go` with dispatch for `lock`, `unlock`, `status`.

Create subcommand files:

- `cmd/lock.go` -- Parse `--vin`. Generate UUID. POST to `/vehicles/{vin}/commands` with `{"command_id": "<uuid>", "type": "lock", "doors": ["driver"]}`. Include bearer token header. Warn if token missing.
- `cmd/unlock.go` -- Same as lock but `"type": "unlock"`.
- `cmd/status.go` -- Parse `--vin`. GET `/vehicles/{vin}/status`. Include bearer token header.

**Files:** `mock/companion-app-cli/main.go`, `mock/companion-app-cli/cmd/lock.go`, `mock/companion-app-cli/cmd/unlock.go`, `mock/companion-app-cli/cmd/status.go`

**Verify:** `cd mock/companion-app-cli && go test ./... -v` -- all tests pass. `cd mock/companion-app-cli && go vet ./...` -- no issues. `go build ./mock/companion-app-cli/...` -- builds successfully.

---

## Group 6: Checkpoint

**Goal:** Final validation that all requirements are met and all tests pass.

### Task 6.1: Run full test suite

Run the complete test suite for all components:

- `cd rhivos && cargo test -p location-sensor -p speed-sensor -p door-sensor` -- all tests pass.
- `cd mock/parking-operator && go test ./... -v` -- all tests pass.
- `cd mock/parking-app-cli && go test ./... -v` -- all tests pass.
- `cd mock/companion-app-cli && go test ./... -v` -- all tests pass.

### Task 6.2: Run linters

- `cd rhivos && cargo clippy -p location-sensor -p speed-sensor -p door-sensor` -- no warnings.
- `cd mock/parking-operator && go vet ./...` -- no issues.
- `cd mock/parking-app-cli && go vet ./...` -- no issues.
- `cd mock/companion-app-cli && go vet ./...` -- no issues.

### Task 6.3: Build verification

- `cd rhivos && cargo build -p location-sensor -p speed-sensor -p door-sensor` -- all binaries build.
- `go build ./mock/parking-operator/...` -- builds successfully.
- `go build ./mock/parking-app-cli/...` -- builds successfully.
- `go build ./mock/companion-app-cli/...` -- builds successfully.

### Task 6.4: Review Definition of Done

Confirm all items in the design.md Definition of Done are satisfied.

---

## Traceability

| Task | Requirement(s) | Test(s) |
|------|----------------|---------|
| 1.1 | 09-REQ-1, 09-REQ-2, 09-REQ-3, 09-REQ-7.1 | TS-09-1, TS-09-2, TS-09-3, TS-09-4, TS-09-E1, TS-09-E2, TS-09-P7 |
| 1.2 | 09-REQ-6 | TS-09-7, TS-09-8, TS-09-9, TS-09-E7, TS-09-E8, TS-09-E9, TS-09-P8 |
| 1.3 | 09-REQ-4, 09-REQ-8.1 | TS-09-5, TS-09-P1, TS-09-P2, TS-09-P3, TS-09-E3, TS-09-E4, TS-09-E5, TS-09-E6 |
| 1.4 | 09-REQ-5 | TS-09-6, TS-09-P4, TS-09-P5, TS-09-P6, TS-09-E3, TS-09-E4, TS-09-E5 |
| 2.1 | 09-REQ-1.1, 09-REQ-7.1, 09-REQ-8.2 | TS-09-1, TS-09-E1, TS-09-E2 |
| 2.2 | 09-REQ-2.1, 09-REQ-7.1, 09-REQ-8.2 | TS-09-2, TS-09-E1, TS-09-E2 |
| 2.3 | 09-REQ-3.1, 09-REQ-7.1, 09-REQ-8.2 | TS-09-3, TS-09-4, TS-09-E1, TS-09-E2 |
| 3.1, 3.2, 3.3 | 09-REQ-6.1, 09-REQ-6.2, 09-REQ-6.3 | TS-09-7, TS-09-8, TS-09-9, TS-09-E7, TS-09-E8, TS-09-E9, TS-09-P8 |
| 4.1, 4.2, 4.3 | 09-REQ-4.1, 09-REQ-4.2, 09-REQ-8.1 | TS-09-5, TS-09-P1, TS-09-P2, TS-09-E3, TS-09-E4, TS-09-E5 |
| 4.4, 4.5 | 09-REQ-4.3, 09-REQ-8.1 | TS-09-P3, TS-09-E6 |
| 5.1, 5.2 | 09-REQ-5.1 | TS-09-6, TS-09-P4, TS-09-P5, TS-09-P6, TS-09-E3, TS-09-E5 |
| 6.1, 6.2, 6.3, 6.4 | All | All |
