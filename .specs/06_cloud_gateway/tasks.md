# Implementation Tasks: CLOUD_GATEWAY (Spec 06)

> Task breakdown for implementing the CLOUD_GATEWAY cloud service.
> Implements design from `.specs/06_cloud_gateway/design.md`.
> Validates against `.specs/06_cloud_gateway/test_spec.md`.

<!-- AGENT INSTRUCTIONS
- Implement exactly ONE top-level task group per session
- Task group 1 writes failing tests from test_spec.md — all subsequent groups
  implement code to make those tests pass
- Follow the git-flow: feature branch from develop -> implement -> test -> merge to develop -> push
- Update checkbox states as you go: [-] in progress, [x] complete
-->

## Dependencies

| Spec | From Group | To Group | Relationship |
|------|-----------|----------|--------------|
| 01_project_setup | 2 | 1 | Uses repo structure and Go project skeleton from group 2 |

## Test Commands

| Action | Command |
|--------|---------|
| Unit tests | `cd backend/cloud-gateway && go test ./... -v` |
| Lint | `cd backend/cloud-gateway && go vet ./...` |
| Run specific test | `cd backend/cloud-gateway && go test -v -run TestName` |

## Tasks

- [x] 1. Write failing spec tests
  - [x] 1.1 Initialize Go module
    - Create `backend/cloud-gateway/go.mod` with module path `github.com/rhadp/parking-fee-service/backend/cloud-gateway` and Go 1.22+ directive
    - Add dependencies: `github.com/nats-io/nats.go`, `github.com/nats-io/nats-server/v2` (for embedded test server)
    - Add module to root `go.work` file if it exists
    - **Files:** `backend/cloud-gateway/go.mod`

  - [x] 1.2 Create model and stub files
    - Create `backend/cloud-gateway/model.go` with minimal struct definitions for `CommandRequest`, `CommandStatus`, `NATSCommand`, `NATSCommandResponse`, `TelemetryData`, `ErrorResponse` so test files compile
    - Create `backend/cloud-gateway/auth.go` with stub `TokenStore` type and `NewTokenStore()` constructor
    - Create `backend/cloud-gateway/store.go` with stub `CommandStore` and `TelemetryStore` types
    - Create `backend/cloud-gateway/nats_client.go` with stub `NATSClient` type
    - Create `backend/cloud-gateway/handler.go` with stub handler function signatures
    - **Files:** `backend/cloud-gateway/model.go`, `backend/cloud-gateway/auth.go`, `backend/cloud-gateway/store.go`, `backend/cloud-gateway/nats_client.go`, `backend/cloud-gateway/handler.go`

  - [x] 1.3 Write handler tests
    - Create `backend/cloud-gateway/handler_test.go` with test functions covering:
      - `TestCommandSubmission` (TS-06-1)
      - `TestBearerTokenValid` (TS-06-2)
      - `TestCommandResponseForwarding` (TS-06-4)
      - `TestHealthCheck` (TS-06-5)
      - `TestMissingAuthHeader` (TS-06-E1)
      - `TestInvalidBearerToken` (TS-06-E2)
      - `TestTokenWrongVIN` (TS-06-E3)
      - `TestMissingRequiredFields` (TS-06-E4) -- table-driven sub-tests
      - `TestInvalidCommandType` (TS-06-E5)
      - `TestUnknownVIN` (TS-06-E6)
      - `TestUnknownCommandID` (TS-06-E7)
      - `TestUndefinedRoute` (TS-06-E9)
    - Tests use `httptest.NewRecorder` and should compile but fail
    - _Test Spec: TS-06-1, TS-06-2, TS-06-4, TS-06-5, TS-06-E1 through TS-06-E7, TS-06-E9_

  - [x] 1.4 Write NATS integration tests
    - Create `backend/cloud-gateway/nats_client_test.go` with test functions covering:
      - `TestNATSCommandRelay` (TS-06-3)
      - `TestNATSResponseSubscription` (TS-06-4)
      - `TestTelemetryReception` (TS-06-6)
      - `TestNATSUnavailable` (TS-06-E8)
      - `TestInvalidTelemetryJSON` (TS-06-E10)
    - Tests use embedded NATS server; should compile but fail
    - _Test Spec: TS-06-3, TS-06-6, TS-06-E8, TS-06-E10_

  - [x] 1.5 Write property tests
    - Create `backend/cloud-gateway/property_test.go` with test functions covering:
      - `TestPropertyTokenVINBinding` (TS-06-P1)
      - `TestPropertyCommandToNATSSubject` (TS-06-P2)
      - `TestPropertyResponseCorrelation` (TS-06-P3)
      - `TestPropertyStatusLifecycle` (TS-06-P4)
      - `TestPropertyRESTToNATSFieldMapping` (TS-06-P5)
      - `TestPropertyResponseFormatConsistency` (TS-06-P6)
      - `TestPropertyHealthEndpointIndependence` (TS-06-P7)
    - _Test Spec: TS-06-P1 through TS-06-P7_

  - [x] 1.V Verify task group 1
    - [x] All spec tests exist and are syntactically valid
    - [x] All spec tests FAIL (red) -- no implementation yet
    - [x] No linter warnings introduced: `cd backend/cloud-gateway && go vet ./...`

- [x] 2. Implement REST API and token validation
  - [x] 2.1 Implement data model types
    - Complete `backend/cloud-gateway/model.go` with full struct definitions including JSON tags
    - Include `CommandRequest`, `CommandStatus`, `NATSCommand`, `NATSCommandResponse`, `TelemetryData`, `ErrorResponse`
    - _Requirements: 06-REQ-1, 06-REQ-4, 06-REQ-8_

  - [x] 2.2 Implement token store and auth middleware
    - Complete `backend/cloud-gateway/auth.go` with:
      - `TokenStore` with `tokens map[string]string` (token -> VIN)
      - `NewTokenStore(tokens map[string]string)` constructor
      - `ValidateToken(token, vin string) (bool, error)` method returning validation result
      - `AuthMiddleware(tokenStore *TokenStore) func(http.Handler) http.Handler` middleware function
      - Demo tokens: `"companion-token-vehicle-1" -> "VIN12345"`, `"companion-token-vehicle-2" -> "VIN67890"`
    - _Requirements: 06-REQ-2.1, 06-REQ-2.2_

  - [x] 2.3 Implement command store
    - Complete `backend/cloud-gateway/store.go` with:
      - `CommandStore` with thread-safe in-memory map for command statuses
      - `StoreCommand(cmdID, status string)` method
      - `UpdateCommandStatus(cmdID, status, reason string)` method (respects terminal state -- no update if already success/failed)
      - `GetCommandStatus(cmdID string) (*CommandStatus, bool)` method
      - `TelemetryStore` with thread-safe in-memory map for latest telemetry per VIN
      - `StoreTelemetry(vin string, data TelemetryData)` and `GetTelemetry(vin string) (*TelemetryData, bool)` methods
    - _Requirements: 06-REQ-4.1, 06-REQ-5.1_

  - [x] 2.4 Implement REST handlers
    - Complete `backend/cloud-gateway/handler.go` with:
      - `HandleHealth(w, r)` -- returns `{"status":"ok"}`
      - `HandleCommandSubmit(commandStore, natsClient) http.HandlerFunc` -- validates body, stores command as pending, publishes to NATS, returns 202
      - `HandleCommandStatus(commandStore) http.HandlerFunc` -- returns command status by ID
      - `writeJSON(w, status, data)` and `writeError(w, status, message)` helper functions
      - Default 404 handler for undefined routes returning JSON error
    - _Requirements: 06-REQ-1, 06-REQ-4.2, 06-REQ-6, 06-REQ-8_

  - [x] 2.V Verify task group 2
    - [x] Auth tests pass: `cd backend/cloud-gateway && go test -v -run TestBearer`
    - [x] Handler tests pass (except NATS-dependent ones): `cd backend/cloud-gateway && go test -v -run "TestHealthCheck|TestMissing|TestInvalid|TestUnknown|TestUndefined"`
    - [x] No linter warnings: `cd backend/cloud-gateway && go vet ./...`
    - [x] Requirements 06-REQ-2, 06-REQ-6, 06-REQ-8 acceptance criteria met

- [x] 3. Implement NATS client and command relay
  - [x] 3.1 Implement NATS client connection
    - Complete `backend/cloud-gateway/nats_client.go` with:
      - `NATSClient` struct wrapping `*nats.Conn`
      - `NewNATSClient(url string) (*NATSClient, error)` constructor
      - `Close()` method
      - `IsConnected() bool` method
    - _Requirements: 06-REQ-3.1_

  - [x] 3.2 Implement command publishing
    - Add to `backend/cloud-gateway/nats_client.go`:
      - `PublishCommand(vin string, cmd NATSCommand) error` -- publishes JSON to `vehicles.{vin}.commands`
      - Returns error if NATS connection is down
    - _Requirements: 06-REQ-1.1, 06-REQ-3.1, 06-REQ-7.1_

  - [x] 3.3 Implement response subscription
    - Add to `backend/cloud-gateway/nats_client.go`:
      - `SubscribeCommandResponses(vin string, handler func(NATSCommandResponse))` -- subscribes to `vehicles.{vin}.command_responses`
      - Parses JSON response and invokes handler callback
    - _Requirements: 06-REQ-3.2, 06-REQ-4.1_

  - [x] 3.4 Wire NATS into REST handlers
    - Update `HandleCommandSubmit` to use `NATSClient.PublishCommand`
    - Return 503 when NATS publish fails due to connection issue
    - Wire response subscription to update `CommandStore` on received responses
    - _Requirements: 06-REQ-1.1, 06-REQ-3.E1_

  - [x] 3.V Verify task group 3
    - [x] NATS tests pass: `cd backend/cloud-gateway && go test -v -run "TestNATS|TestCommandSubmission|TestCommandResponse"`
    - [x] All existing tests still pass: `cd backend/cloud-gateway && go test ./... -v`
    - [x] No linter warnings: `cd backend/cloud-gateway && go vet ./...`
    - [x] Requirements 06-REQ-1, 06-REQ-3, 06-REQ-7 acceptance criteria met

- [x] 4. Implement response forwarding and telemetry
  - [x] 4.1 Implement telemetry subscription
    - Add to `backend/cloud-gateway/nats_client.go`:
      - `SubscribeTelemetry(vin string, handler func(TelemetryData))` -- subscribes to `vehicles.{vin}.telemetry`
      - Parses JSON; logs and discards invalid JSON messages
    - _Requirements: 06-REQ-5.1, 06-REQ-5.E1_

  - [x] 4.2 Implement configuration
    - Create `backend/cloud-gateway/config.go` with:
      - `Config` struct: `HTTPPort`, `NATSURL`, `KnownVINs []string`
      - `LoadConfig()` -- reads from environment variables with sensible defaults
      - Default NATS URL: `nats://localhost:4222`
      - Default HTTP port: `8081`
      - Default known VINs: `["VIN12345", "VIN67890"]`
    - _Requirements: 06-REQ-7.2_

  - [x] 4.3 Implement main.go server wiring
    - Create `backend/cloud-gateway/main.go` with:
      - Load configuration
      - Create token store with demo tokens
      - Create command store and telemetry store
      - Connect NATS client
      - Subscribe to command responses and telemetry for all known VINs
      - Set up HTTP routes with auth middleware:
        - `GET /health` -> `HandleHealth` (no auth)
        - `POST /vehicles/{vin}/commands` -> `HandleCommandSubmit` (auth required)
        - `GET /vehicles/{vin}/commands/{command_id}` -> `HandleCommandStatus` (auth required)
      - Register default 404 handler
      - Start HTTP server
    - _Requirements: all_

  - [x] 4.4 Wire property tests
    - Ensure all property tests (TS-06-P1 through TS-06-P7) pass with the full implementation
    - _Test Spec: TS-06-P1 through TS-06-P7_

  - [x] 4.V Verify task group 4
    - [x] Telemetry tests pass: `cd backend/cloud-gateway && go test -v -run "TestTelemetry"`
    - [x] Property tests pass: `cd backend/cloud-gateway && go test -v -run "TestProperty"`
    - [x] All tests pass: `cd backend/cloud-gateway && go test ./... -v`
    - [x] No linter warnings: `cd backend/cloud-gateway && go vet ./...`
    - [x] Build succeeds: `cd backend/cloud-gateway && go build .`
    - [x] Requirements 06-REQ-5 acceptance criteria met

- [ ] 5. Integration tests
  - [ ] 5.1 End-to-end command flow test
    - Create `backend/cloud-gateway/integration_test.go` with:
      - `TestEndToEndCommandFlow` -- full cycle: submit command via REST, verify NATS publish, simulate NATS response, query status via REST
      - Uses embedded NATS server and httptest server
    - _Test Spec: TS-06-1, TS-06-4_

  - [ ] 5.2 Multi-vehicle routing test
    - Add to integration test file:
      - `TestMultiVehicleRouting` -- submit commands for two different VINs, verify each reaches the correct NATS subject and responses route back correctly
    - _Test Spec: TS-06-3_
    - _Requirements: 06-REQ-7.1, 06-REQ-7.2_

  - [ ] 5.3 Error scenario integration tests
    - Add to integration test file:
      - `TestNATSDisconnectRecovery` -- verify 503 on NATS failure, recovery on reconnect
      - `TestConcurrentCommandSubmission` -- submit multiple commands concurrently, verify no race conditions
    - _Test Spec: TS-06-E8_

  - [ ] 5.V Verify task group 5
    - [ ] Integration tests pass: `cd backend/cloud-gateway && go test -v -run "TestEndToEnd|TestMultiVehicle|TestNATSDisconnect|TestConcurrent"`
    - [ ] All tests pass: `cd backend/cloud-gateway && go test ./... -v`
    - [ ] All tests pass with race detector: `cd backend/cloud-gateway && go test -race ./... -v`
    - [ ] No linter warnings: `cd backend/cloud-gateway && go vet ./...`

- [ ] 6. Checkpoint -- CLOUD_GATEWAY Complete
  - [ ] 6.1 Run full test suite
    - `cd backend/cloud-gateway && go test ./... -v`
    - `cd backend/cloud-gateway && go test -race ./... -v`
    - Confirm all tests pass

  - [ ] 6.2 Run linter
    - `cd backend/cloud-gateway && go vet ./...`
    - Confirm no issues

  - [ ] 6.3 Verify build
    - `cd backend/cloud-gateway && go build .`
    - Confirm binary builds successfully

  - [ ] 6.4 Smoke test
    - Start NATS server (containerized): `podman run -d --name nats-test -p 4222:4222 nats:latest`
    - Start CLOUD_GATEWAY: `cd backend/cloud-gateway && go run .`
    - Verify: `curl http://localhost:8081/health` returns `{"status":"ok"}`
    - Verify: `curl -X POST http://localhost:8081/vehicles/VIN12345/commands -H "Authorization: Bearer companion-token-vehicle-1" -H "Content-Type: application/json" -d '{"command_id":"test-1","type":"lock","doors":["driver"]}'` returns 202
    - Stop services

  - [ ] 6.5 Review Definition of Done
    - Confirm all items in design.md Definition of Done are satisfied
    - Ensure all requirements are covered by passing tests

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
| 06-REQ-1.1 | TS-06-1 | 2.4, 3.2, 3.4 | `TestCommandSubmission` |
| 06-REQ-1.2 | TS-06-1, TS-06-P5 | 2.4, 3.2 | `TestCommandSubmission`, `TestPropertyRESTToNATSFieldMapping` |
| 06-REQ-1.E1 | TS-06-E4 | 2.4 | `TestMissingRequiredFields` |
| 06-REQ-1.E2 | TS-06-E5 | 2.4 | `TestInvalidCommandType` |
| 06-REQ-2.1 | TS-06-2, TS-06-P1 | 2.2 | `TestBearerTokenValid`, `TestPropertyTokenVINBinding` |
| 06-REQ-2.2 | TS-06-2, TS-06-P1 | 2.2 | `TestBearerTokenValid`, `TestPropertyTokenVINBinding` |
| 06-REQ-2.E1 | TS-06-E1 | 2.2 | `TestMissingAuthHeader` |
| 06-REQ-2.E2 | TS-06-E2 | 2.2 | `TestInvalidBearerToken` |
| 06-REQ-2.E3 | TS-06-E3 | 2.2 | `TestTokenWrongVIN` |
| 06-REQ-3.1 | TS-06-3, TS-06-P2 | 3.1, 3.2 | `TestNATSCommandRelay`, `TestPropertyCommandToNATSSubject` |
| 06-REQ-3.2 | TS-06-4 | 3.3 | `TestNATSResponseSubscription` |
| 06-REQ-3.E1 | TS-06-E8 | 3.4 | `TestNATSUnavailable` |
| 06-REQ-4.1 | TS-06-4, TS-06-P3 | 2.3, 3.3 | `TestCommandResponseForwarding`, `TestPropertyResponseCorrelation` |
| 06-REQ-4.2 | TS-06-4, TS-06-P4 | 2.3, 2.4 | `TestCommandResponseForwarding`, `TestPropertyStatusLifecycle` |
| 06-REQ-4.E1 | TS-06-E7 | 2.4 | `TestUnknownCommandID` |
| 06-REQ-5.1 | TS-06-6 | 4.1 | `TestTelemetryReception` |
| 06-REQ-5.E1 | TS-06-E10 | 4.1 | `TestInvalidTelemetryJSON` |
| 06-REQ-6.1 | TS-06-5, TS-06-P7 | 2.4 | `TestHealthCheck`, `TestPropertyHealthEndpointIndependence` |
| 06-REQ-6.E1 | TS-06-5, TS-06-P7 | 2.4 | `TestHealthCheck`, `TestPropertyHealthEndpointIndependence` |
| 06-REQ-7.1 | TS-06-3, TS-06-P2 | 3.2 | `TestNATSCommandRelay`, `TestPropertyCommandToNATSSubject` |
| 06-REQ-7.2 | TS-06-3 | 4.2, 4.3 | `TestMultiVehicleRouting` |
| 06-REQ-7.E1 | TS-06-E6 | 2.4 | `TestUnknownVIN` |
| 06-REQ-8.1 | TS-06-P6 | 2.4 | `TestPropertyResponseFormatConsistency` |
| 06-REQ-8.2 | TS-06-P6 | 2.4 | `TestPropertyResponseFormatConsistency` |
| 06-REQ-8.E1 | TS-06-E9 | 2.4 | `TestUndefinedRoute` |
| 06-REQ-8.E2 | (covered by recovery middleware) | 2.4 | `TestEndToEndCommandFlow` |

## Notes

- All NATS integration tests use an embedded NATS server (`github.com/nats-io/nats-server/v2/server`) to avoid external infrastructure dependencies.
- The `go test -race` flag should be used to detect race conditions in the concurrent command/telemetry stores.
- The command store uses `sync.RWMutex` for thread-safe access.
- Demo tokens are hardcoded for simplicity; in production, tokens would come from an external auth provider.
