# Tasks: CLOUD_GATEWAY (Spec 06)

> Implementation tasks for the CLOUD_GATEWAY service.
> Implements requirements from `.specs/06_cloud_gateway/requirements.md`.
> Design details in `.specs/06_cloud_gateway/design.md`.
> Test cases in `.specs/06_cloud_gateway/test_spec.md`.

## Dependencies

| Spec | From Group | To Group | Relationship |
|------|-----------|----------|--------------|
| 01_project_setup | 2 | 1 | Requires Go module structure (`go.work`, `backend/cloud-gateway/go.mod`), build system (`Makefile`), and local NATS infrastructure (`infra/docker-compose.yml`) |

## Test Commands

```bash
# Unit tests
cd backend/cloud-gateway && go test ./... -v

# Lint
cd backend/cloud-gateway && go vet ./...

# Integration tests (requires running NATS server)
make infra-up
cd backend/cloud-gateway && go test ./... -v -tags=integration
make infra-down
```

## Task Groups

### Group 1: Write Failing Spec Tests

**Goal:** Create Go test files that encode the test specifications as failing tests. These tests define the expected behavior before any implementation exists.

#### Task 1.1: Initialize Go Module

Create `backend/cloud-gateway/go.mod` with module path `github.com/rhadp/parking-fee-service/backend/cloud-gateway` and Go 1.22+ version. Add `github.com/nats-io/nats.go` as a dependency. Ensure the module is listed in the root `go.work` file.

**Acceptance criteria:**
- `go.mod` exists with correct module path and Go version.
- `go mod tidy` completes without errors.
- The module is referenced in the root `go.work`.

#### Task 1.2: Write Auth Middleware Tests

Create `backend/cloud-gateway/auth_test.go` with tests for:
- Valid bearer token passes through (TS-06-1 auth aspect).
- Missing Authorization header returns 401 (TS-06-E1).
- Invalid token returns 401 (TS-06-2).
- Non-Bearer scheme returns 401.

**Acceptance criteria:**
- Tests compile but fail (no implementation yet).
- Tests cover all three 401 error codes: `missing_authorization`, `invalid_token`, `invalid_auth_scheme`.

#### Task 1.3: Write Handler Tests

Create `backend/cloud-gateway/handlers_test.go` with tests for:
- Valid POST command returns 200 with correct response (TS-06-1).
- Invalid VIN format returns 400 (TS-06-E2).
- Malformed JSON body returns 400 (TS-06-E4).
- Missing required fields returns 400 (TS-06-E5).
- Invalid command type returns 400 (TS-06-E7).
- GET status returns vehicle state (TS-06-3).
- GET status for unknown VIN returns defaults (TS-06-P1).

**Acceptance criteria:**
- Tests compile but fail (no implementation yet).
- Tests use `httptest.NewRecorder` for HTTP testing.

#### Task 1.4: Write Protocol Translation Tests

Create `backend/cloud-gateway/protocol_test.go` with tests for:
- REST-to-NATS field mapping: `type` -> `action`, added fields (`source`, `vin`, `timestamp`) (TS-06-5).
- NATS-to-REST response mapping: `command_id`, `status`, optional `reason`.

**Acceptance criteria:**
- Tests compile but fail.
- Tests verify exact field presence and absence of unexpected fields.

#### Task 1.5: Write Correlation Tests

Create `backend/cloud-gateway/correlation_test.go` with tests for:
- Response with matching `command_id` is delivered (TS-06-4).
- Timeout when no response arrives (TS-06-E3).
- Concurrent pending requests are isolated.
- Cleanup occurs after response or timeout.

**Acceptance criteria:**
- Tests compile but fail.
- Timeout test uses a short timeout (e.g., 1 second) for test speed.

**Verify:** `cd backend/cloud-gateway && go test ./... -v` -- all tests compile but fail.

---

### Group 2: NATS Client Setup and Connection Management

**Goal:** Implement the NATS client with connection management, publishing, subscribing, and reconnection logic.

#### Task 2.1: Define Shared Types

Create `backend/cloud-gateway/types.go` with:
- `CommandRequest` struct (REST input): `CommandID`, `Type`, `Doors`.
- `NATSCommand` struct (NATS output): `CommandID`, `Action`, `Doors`, `Source`, `VIN`, `Timestamp`.
- `CommandResponse` struct: `CommandID`, `Status`, `Reason`, `Timestamp`.
- `VehicleState` struct: `VIN`, `Locked`, `ParkingActive`, `Latitude`, `Longitude`, `LastUpdated`.
- `ErrorResponse` struct: `Error`, `Message`.

**Acceptance criteria:**
- All types have appropriate JSON struct tags.
- Types are exported for use across files.

#### Task 2.2: Implement NATS Client

Create `backend/cloud-gateway/nats_client.go` with:
- `NATSClient` struct holding a `*nats.Conn` and connection state.
- `NewNATSClient(url string)` constructor that connects with reconnection options (max reconnects, reconnect wait, exponential backoff).
- `Publish(subject string, data []byte) error` method.
- `Subscribe(subject string, handler nats.MsgHandler) (*nats.Subscription, error)` method.
- `IsConnected() bool` method to check connection status.
- `Close()` method for graceful shutdown.
- Disconnection and reconnection handlers that log events and update connection state.

**Acceptance criteria:**
- NATS client connects to the server specified by `NATS_URL`.
- Reconnection is configured with exponential backoff.
- `IsConnected()` returns accurate connection state.

#### Task 2.3: Write NATS Client Tests

Create `backend/cloud-gateway/nats_client_test.go` with unit tests for:
- Connection to a test NATS server.
- Publish and subscribe round-trip.
- `IsConnected()` returns false when disconnected.

**Acceptance criteria:**
- Tests pass using an embedded or test NATS server (or are skipped if NATS is unavailable).

**Verify:** `cd backend/cloud-gateway && go test ./... -v -run TestNATS` -- NATS tests pass.

---

### Group 3: REST API Handlers

**Goal:** Implement the HTTP server, auth middleware, and REST endpoint handlers.

#### Task 3.1: Implement Bearer Token Auth Middleware

Create `backend/cloud-gateway/auth.go` with:
- `AuthMiddleware(validTokens []string)` function returning an `http.Handler` middleware.
- Extracts `Authorization` header, validates `Bearer` scheme, checks token against valid set.
- Returns appropriate 401 JSON error responses.

**Acceptance criteria:**
- Auth middleware tests from Task 1.2 pass.
- Three distinct error codes: `missing_authorization`, `invalid_token`, `invalid_auth_scheme`.

#### Task 3.2: Implement Correlation Manager

Create `backend/cloud-gateway/correlation.go` with:
- `CorrelationManager` struct with a mutex-protected map of `command_id` to `chan CommandResponse`.
- `Register(commandID string) chan CommandResponse` -- creates and stores a channel.
- `Deliver(commandID string, response CommandResponse) bool` -- delivers response if pending.
- `Remove(commandID string)` -- cleans up a pending entry.

**Acceptance criteria:**
- Correlation tests from Task 1.5 pass.
- Concurrent access is safe (mutex-protected).

#### Task 3.3: Implement Command Handler

Create `backend/cloud-gateway/handlers.go` with:
- `HandleCommand(natsClient, correlationMgr)` handler for `POST /vehicles/{vin}/commands`.
- Validates VIN format (alphanumeric, 5-20 characters).
- Parses and validates request body (required fields, valid command type).
- Translates REST command to NATS message using protocol translator.
- Publishes to `vehicles.{VIN}.commands` via NATS client.
- Waits for correlated response with timeout.
- Returns appropriate HTTP response.

**Acceptance criteria:**
- Handler tests from Task 1.3 (command-related) pass.
- All 400-level validation errors return correct error codes.

#### Task 3.4: Implement Status Handler

Add to `backend/cloud-gateway/handlers.go`:
- `HandleStatus(vehicleStates)` handler for `GET /vehicles/{vin}/status`.
- Validates VIN format.
- Reads from the in-memory vehicle state map.
- Returns current state or default values if no telemetry received.

**Acceptance criteria:**
- Handler tests from Task 1.3 (status-related) pass.
- Unknown VINs return default state with `last_updated: null`.

#### Task 3.5: Implement Protocol Translator

Create `backend/cloud-gateway/protocol.go` with:
- `RESTToNATS(req CommandRequest, vin string) NATSCommand` -- maps `Type` to `Action`, adds `Source`, `VIN`, `Timestamp`.
- `NATSToREST(resp CommandResponse) CommandResponse` -- passes through fields.

**Acceptance criteria:**
- Protocol translation tests from Task 1.4 pass.
- Field mapping is exact: no extra fields, no missing fields.

#### Task 3.6: Implement HTTP Server and Routing

Create `backend/cloud-gateway/server.go` with:
- `NewServer(natsClient, correlationMgr, vehicleStates, validTokens)` constructor.
- Route registration: `POST /vehicles/{vin}/commands`, `GET /vehicles/{vin}/status`.
- Auth middleware applied to all routes.
- Method checking (405 for unsupported methods).

Create `backend/cloud-gateway/main.go` with:
- Entry point that reads configuration from environment variables.
- Initializes NATS client, correlation manager, vehicle state map.
- Starts NATS subscriptions (command_responses, telemetry).
- Starts HTTP server on configured port.
- Graceful shutdown on SIGTERM/SIGINT.

**Acceptance criteria:**
- Server starts and listens on port 8081.
- All routes are registered and protected by auth middleware.
- Graceful shutdown works.

**Verify:** `cd backend/cloud-gateway && go test ./... -v` -- all unit tests pass.
**Verify:** `cd backend/cloud-gateway && go vet ./...` -- no lint issues.

---

### Group 4: Protocol Translation and Response Correlation

**Goal:** Implement the NATS subscription handlers that process command responses and telemetry, completing the end-to-end flow.

#### Task 4.1: Implement Command Response Subscription

Add NATS subscription handler for `vehicles.*.command_responses`:
- Parse incoming NATS message as JSON.
- Extract `command_id` from the message.
- Deliver to the correlation manager.
- Log warnings for messages with missing `command_id` or invalid JSON.

**Acceptance criteria:**
- Responses are delivered to the correct pending REST request.
- Invalid JSON responses are logged and discarded.
- Responses with missing `command_id` are logged and discarded.

#### Task 4.2: Implement Telemetry Subscription

Add NATS subscription handler for `vehicles.*.telemetry`:
- Parse incoming NATS message as JSON.
- Extract VIN from the message or the NATS subject.
- Update the vehicle state map with the latest telemetry.
- Only update if the new timestamp is more recent than the stored one.

**Acceptance criteria:**
- Telemetry updates the vehicle state map.
- Older telemetry does not overwrite newer state.
- GET status reflects the latest telemetry.

#### Task 4.3: Implement NATS Unavailable Handling

Add NATS connection state checking to the command handler:
- Before publishing, check `natsClient.IsConnected()`.
- If disconnected, return `503 Service Unavailable` immediately.

**Acceptance criteria:**
- TS-06-E6 passes: commands return 503 when NATS is down.

**Verify:** `cd backend/cloud-gateway && go test ./... -v` -- all tests pass.

---

### Group 5: Integration Testing with NATS Server

**Goal:** Write and run integration tests that exercise the full CLOUD_GATEWAY with a real NATS server.

#### Task 5.1: Write Integration Tests

Create integration tests (build-tagged with `//go:build integration`) in `backend/cloud-gateway/integration_test.go`:
- Test full round-trip: REST POST -> NATS publish -> simulated NATS response -> REST response (TS-06-1, TS-06-4).
- Test timeout: REST POST with no NATS response -> 504 (TS-06-E3).
- Test telemetry: publish telemetry on NATS -> GET status returns updated state (TS-06-3).
- Test auth rejection in full stack (TS-06-2, TS-06-E1).
- Test protocol translation end-to-end (TS-06-5).

**Acceptance criteria:**
- All integration tests pass with `make infra-up` (NATS running).
- Tests clean up NATS subscriptions after completion.

#### Task 5.2: Verify Build and Lint

Run full verification:
- `cd backend/cloud-gateway && go build ./...` -- compiles without errors.
- `cd backend/cloud-gateway && go test ./... -v` -- all unit tests pass.
- `cd backend/cloud-gateway && go vet ./...` -- no issues.
- `make infra-up && cd backend/cloud-gateway && go test ./... -v -tags=integration && make infra-down` -- integration tests pass.

**Acceptance criteria:**
- All commands exit with code 0.
- No test failures or lint warnings.

**Verify:** All test and lint commands pass cleanly.

---

### Group 6: Checkpoint

**Goal:** Final verification that all requirements are met and all tests pass.

#### Task 6.1: Requirements Verification

Verify each requirement against passing tests:
- 06-REQ-1.1: TS-06-1, TS-06-E2, TS-06-E4, TS-06-E5, TS-06-E7 pass.
- 06-REQ-2.1: TS-06-3, TS-06-P1 pass.
- 06-REQ-3.1: TS-06-1, TS-06-E6 pass.
- 06-REQ-4.1: TS-06-4, TS-06-E3 pass.
- 06-REQ-5.1: TS-06-1, TS-06-2, TS-06-E1 pass.
- 06-REQ-6.1: TS-06-4, TS-06-5 pass.
- 06-REQ-7.1: TS-06-E6 pass, reconnection verified.

#### Task 6.2: Code Quality Check

- `go vet ./...` reports no issues.
- No TODO or FIXME comments remain in production code.
- All exported types and functions have doc comments.
- `go build ./...` succeeds.

**Acceptance criteria:**
- All requirements are covered by at least one passing test.
- Code is clean, linted, and well-documented.

---

## Traceability Matrix

| Requirement | Test Cases | Task Groups |
|-------------|-----------|-------------|
| 06-REQ-1.1 | TS-06-1, TS-06-E2, TS-06-E4, TS-06-E5, TS-06-E7 | G1 (1.3), G3 (3.3) |
| 06-REQ-2.1 | TS-06-3, TS-06-P1 | G1 (1.3), G3 (3.4), G4 (4.2) |
| 06-REQ-3.1 | TS-06-1, TS-06-E6 | G1 (1.3), G2 (2.2), G3 (3.3), G4 (4.3) |
| 06-REQ-4.1 | TS-06-4, TS-06-E3 | G1 (1.5), G3 (3.2, 3.3), G4 (4.1) |
| 06-REQ-5.1 | TS-06-1, TS-06-2, TS-06-E1 | G1 (1.2), G3 (3.1) |
| 06-REQ-6.1 | TS-06-4, TS-06-5 | G1 (1.4), G3 (3.5) |
| 06-REQ-7.1 | TS-06-E6 | G2 (2.2, 2.3), G4 (4.3) |
