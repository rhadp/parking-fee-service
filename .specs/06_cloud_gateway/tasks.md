# Implementation Plan: CLOUD_GATEWAY

<!-- AGENT INSTRUCTIONS
- Implement exactly ONE top-level task group per session
- Task group 1 writes failing tests from test_spec.md — all subsequent groups
  implement code to make those tests pass
- Follow the git-flow: feature branch from main -> implement -> test -> merge to main -> push
- Update checkbox states as you go: [-] in progress, [x] complete
-->

## Overview

This plan implements the CLOUD_GATEWAY as a Go HTTP server in `backend/cloud-gateway/`. The service provides dual REST/NATS interfaces for routing lock/unlock commands between COMPANION_APPs and vehicles. Task group 1 writes failing tests. Groups 2-3 implement pure-function modules (model, config, auth, store). Group 4 implements HTTP handlers and NATS client. Group 5 wires up main. Group 6 runs integration tests with NATS.

Ordering: tests first, then data types, then pure-function modules (config, auth, store), then HTTP handlers, then NATS integration, then main loop.

## Test Commands

- Spec tests (unit): `cd backend && go test -v ./cloud-gateway/...`
- Spec tests (integration): `cd tests/cloud-gateway && go test -v ./...`
- Property tests: `cd backend && go test -v ./cloud-gateway/... -run Property`
- All tests: `cd backend && go test -v ./...`
- Linter: `cd backend && go vet ./cloud-gateway/...`

## Tasks

- [x] 1. Write failing spec tests
  - [x] 1.1 Set up Go module and test file structure
    - Ensure `backend/cloud-gateway/` has proper package structure
    - Create package directories: `model/`, `config/`, `auth/`, `store/`, `handler/`, `natsclient/`
    - Create test files: `config/config_test.go`, `auth/auth_test.go`, `store/store_test.go`, `handler/handler_test.go`
    - _Test Spec: TS-06-1 through TS-06-27_

  - [x] 1.2 Write config and auth unit tests
    - `TestLoadConfigFromFile` — TS-06-18
    - `TestConfigFields` — TS-06-19
    - `TestConfigDefaults` — TS-06-20
    - `TestConfigFileMissing` — TS-06-E10
    - `TestConfigInvalidJSON` — TS-06-E11
    - `TestTokenVINLoading` — TS-06-16
    - `TestTokenVINAuthorization` — TS-06-17
    - _Test Spec: TS-06-16, TS-06-17, TS-06-18, TS-06-19, TS-06-20, TS-06-E10, TS-06-E11_

  - [x] 1.3 Write store and model unit tests
    - `TestCommandStoredAsPending` — TS-06-4
    - `TestPendingStatusBeforeResponse` — TS-06-6
    - `TestSuccessAndFailedStatus` — TS-06-7
    - `TestResponsePayloadParsing` — TS-06-10
    - `TestCommandTimeout` — TS-06-11
    - `TestConfigurableTimeout` — TS-06-12
    - `TestCommandPayloadStructure` — TS-06-2
    - _Test Spec: TS-06-2, TS-06-4, TS-06-6, TS-06-7, TS-06-10, TS-06-11, TS-06-12_

  - [x] 1.4 Write handler integration tests (httptest)
    - `TestCommandSubmission` — TS-06-1
    - `TestCommandStatusQuery` — TS-06-5
    - `TestTokenValidationOnEndpoints` — TS-06-15
    - `TestHealthCheck` — TS-06-23
    - `TestContentTypeHeader` — TS-06-26
    - `TestErrorResponseFormat` — TS-06-27
    - `TestMissingAuthHeader` — TS-06-E1
    - `TestTokenNotAuthorizedForVIN` — TS-06-E2
    - `TestInvalidCommandPayload` — TS-06-E3
    - `TestInvalidCommandType` — TS-06-E4
    - `TestUnknownCommandID` — TS-06-E5
    - `TestAuthOnStatusQuery` — TS-06-E6
    - _Test Spec: TS-06-1, TS-06-5, TS-06-15, TS-06-23, TS-06-26, TS-06-27, TS-06-E1 through TS-06-E6_

  - [x] 1.5 Write property tests
    - `TestPropertyCommandRouting` — TS-06-P1
    - `TestPropertyAuthEnforcement` — TS-06-P2
    - `TestPropertyResponseStatusUpdate` — TS-06-P3
    - `TestPropertyCommandTimeout` — TS-06-P4
    - `TestPropertyPayloadValidation` — TS-06-P5
    - `TestPropertyConfigDefaults` — TS-06-P6
    - `TestPropertyNATSSubjects` — TS-06-P7
    - _Test Spec: TS-06-P1 through TS-06-P7_

  - [x] 1.V Verify task group 1
    - [x] All test files compile: `cd backend && go test -v ./cloud-gateway/... -run NONE`
    - [x] All spec tests FAIL (red): `cd backend && go test -v ./cloud-gateway/... 2>&1 | grep FAIL`
    - [x] No linter warnings: `cd backend && go vet ./cloud-gateway/...`

- [x] 2. Model, config, and auth modules
  - [x] 2.1 Implement model package
    - Define types: `Command`, `CommandStatus`, `CommandResponse`, `TokenMapping`, `Config`
    - Add JSON struct tags for all fields
    - Add `parseCommand([]byte) (*Command, error)` with validation
    - Add `parseResponse([]byte) (*CommandResponse, error)`
    - _Requirements: 06-REQ-1.2, 06-REQ-3.3_

  - [x] 2.2 Implement config package
    - `LoadConfig(path string) (*Config, error)`: read JSON file, unmarshal, apply defaults
    - `DefaultConfig() *Config`: port 8081, NATS nats://localhost:4222, timeout 30s, empty tokens
    - If file not found: return DefaultConfig, log warning
    - If invalid JSON: return error
    - _Requirements: 06-REQ-7.1, 06-REQ-7.2, 06-REQ-7.3, 06-REQ-7.E1, 06-REQ-7.E2_

  - [x] 2.3 Implement auth package
    - `ValidateToken(header string) (string, error)`: extract token from `Bearer <token>` format
    - `NewAuthenticator(tokens []TokenMapping) *Authenticator`
    - `(a *Authenticator) AuthorizeVIN(token, vin string) bool`: check token-VIN mapping
    - _Requirements: 06-REQ-6.1, 06-REQ-6.2, 06-REQ-6.3_

  - [x] 2.V Verify task group 2
    - [x] Config and auth tests pass: `cd backend && go test -v ./cloud-gateway/config/... ./cloud-gateway/auth/... ./cloud-gateway/model/...`
    - [x] All existing tests still pass: `cd backend && go test -v ./...`
    - [x] No linter warnings: `cd backend && go vet ./cloud-gateway/...`
    - [x] _Test Spec: TS-06-2, TS-06-10, TS-06-16, TS-06-17, TS-06-18, TS-06-19, TS-06-20, TS-06-E10, TS-06-E11, TS-06-P2, TS-06-P5, TS-06-P6_

- [x] 3. Store module
  - [x] 3.1 Implement store package
    - `NewStore() *Store`: create mutex-protected map
    - `Add(cmd CommandStatus)`: store command with pending status and creation timestamp
    - `Get(commandID string) (*CommandStatus, bool)`: retrieve by command ID
    - `UpdateFromResponse(resp CommandResponse)`: update status and reason for existing command
    - `ExpireTimedOut(timeout time.Duration)`: iterate map, set status to "timeout" for expired commands
    - Thread-safe via `sync.Mutex`
    - _Requirements: 06-REQ-1.4, 06-REQ-2.1, 06-REQ-2.2, 06-REQ-2.3, 06-REQ-3.2, 06-REQ-4.1_

  - [x] 3.V Verify task group 3
    - [x] Store tests pass: `cd backend && go test -v ./cloud-gateway/store/...`
    - [x] All existing tests still pass: `cd backend && go test -v ./...`
    - [x] No linter warnings: `cd backend && go vet ./cloud-gateway/...`
    - [x] _Test Spec: TS-06-4, TS-06-6, TS-06-7, TS-06-11, TS-06-12, TS-06-P3, TS-06-P4_

- [ ] 4. HTTP handlers and NATS client
  - [ ] 4.1 Implement handler package
    - `NewCommandHandler(store, natsPublisher, auth) http.HandlerFunc`:
      - Validate auth, parse command, store as pending, publish to NATS, return 202
      - Handle errors: 400, 401, 403
    - `NewStatusHandler(store, auth) http.HandlerFunc`:
      - Validate auth, lookup command, return status
      - Handle errors: 401, 403, 404
    - `HealthHandler() http.HandlerFunc`: return `{"status":"ok"}`
    - Set `Content-Type: application/json` on all responses
    - Use `{"error":"<message>"}` format for errors
    - Use interface for NATS publishing (testable without real NATS)
    - _Requirements: 06-REQ-1.1, 06-REQ-1.E1 through 06-REQ-1.E4, 06-REQ-2.1, 06-REQ-2.E1, 06-REQ-2.E2, 06-REQ-9.1, 06-REQ-10.1, 06-REQ-10.2_

  - [ ] 4.2 Implement natsclient package
    - `Connect(url string, maxRetries int) (*nats.Conn, error)`: exponential backoff retry
    - `PublishCommand(nc, vin, cmd, bearerToken) error`: publish to `vehicles.{vin}.commands` with Authorization header
    - `SubscribeResponses(nc, store) (*nats.Subscription, error)`: subscribe to `vehicles.*.command_responses`, update store
    - `SubscribeTelemetry(nc) (*nats.Subscription, error)`: subscribe to `vehicles.*.telemetry`, log
    - Handle invalid JSON in NATS messages: log and discard
    - Handle unknown command_ids: log warning and discard
    - _Requirements: 06-REQ-1.3, 06-REQ-3.1, 06-REQ-3.2, 06-REQ-3.E1, 06-REQ-3.E2, 06-REQ-5.1, 06-REQ-5.2, 06-REQ-5.E1, 06-REQ-8.1, 06-REQ-8.E1_

  - [ ] 4.V Verify task group 4
    - [ ] Handler tests pass: `cd backend && go test -v ./cloud-gateway/handler/...`
    - [ ] All existing tests still pass: `cd backend && go test -v ./...`
    - [ ] No linter warnings: `cd backend && go vet ./cloud-gateway/...`
    - [ ] _Test Spec: TS-06-1, TS-06-5, TS-06-15, TS-06-23, TS-06-26, TS-06-27, TS-06-E1 through TS-06-E6, TS-06-P1, TS-06-P7_

- [ ] 5. Main package and integration
  - [ ] 5.1 Implement main package
    - Read `CONFIG_PATH` env var (default "config.json")
    - Call LoadConfig, create Authenticator, create Store
    - Connect to NATS with retries
    - Subscribe to command responses and telemetry
    - Start timeout expiry goroutine (periodic, e.g., every 5 seconds)
    - Register routes using Go 1.22 ServeMux patterns:
      - `POST /vehicles/{vin}/commands` → CommandHandler
      - `GET /vehicles/{vin}/commands/{command_id}` → StatusHandler
      - `GET /health` → HealthHandler
    - Start HTTP server on configured port
    - Log version, port, NATS URL, token count at startup
    - Handle SIGTERM/SIGINT: drain NATS, `http.Server.Shutdown()`
    - Use `log/slog` for structured logging
    - _Requirements: 06-REQ-7.1, 06-REQ-8.1, 06-REQ-8.2, 06-REQ-9.1, 06-REQ-9.2, 06-REQ-9.3_

  - [ ] 5.2 Add nats.go dependency
    - Run `go get github.com/nats-io/nats.go`
    - Update go.mod and go.sum
    - _Requirements: 06-REQ-8.1_

  - [ ] 5.V Verify task group 5
    - [ ] Binary builds: `cd backend && go build ./cloud-gateway/...`
    - [ ] All unit tests pass: `cd backend && go test -v ./cloud-gateway/...`
    - [ ] All existing tests still pass: `cd backend && go test -v ./...`
    - [ ] No linter warnings: `cd backend && go vet ./cloud-gateway/...`

- [ ] 6. Integration test validation
  - [ ] 6.1 Create integration test module
    - Create `tests/cloud-gateway/` Go module
    - Shared helpers: start/stop NATS, start/stop service, NATS publish/subscribe helpers
    - Add `go.work` entry for `./tests/cloud-gateway`
    - _Test Spec: TS-06-3, TS-06-8, TS-06-9, TS-06-13, TS-06-14, TS-06-21, TS-06-22, TS-06-24, TS-06-25_

  - [ ] 6.2 Write and run integration tests
    - `TestBearerTokenInNATSHeader` — TS-06-3
    - `TestNATSResponseSubscription` — TS-06-8
    - `TestResponseUpdatesStore` — TS-06-9
    - `TestTelemetrySubscription` — TS-06-13
    - `TestTelemetryLogging` — TS-06-14
    - `TestNATSConnection` — TS-06-21
    - `TestNATSSubscriptionsActive` — TS-06-22
    - _Test Spec: TS-06-3, TS-06-8, TS-06-9, TS-06-13, TS-06-14, TS-06-21, TS-06-22_

  - [ ] 6.3 Write and run lifecycle and edge case integration tests
    - `TestStartupLogging` — TS-06-24
    - `TestGracefulShutdown` — TS-06-25
    - `TestInvalidNATSResponseJSON` — TS-06-E7
    - `TestUnknownCommandIDInNATS` — TS-06-E8
    - `TestInvalidTelemetryJSON` — TS-06-E9
    - `TestNATSUnreachable` — TS-06-E12
    - _Test Spec: TS-06-24, TS-06-25, TS-06-E7, TS-06-E8, TS-06-E9, TS-06-E12_

  - [ ] 6.V Verify task group 6
    - [ ] All integration tests pass: `cd tests/cloud-gateway && go test -v ./...`
    - [ ] All unit tests still pass: `cd backend && go test -v ./cloud-gateway/...`
    - [ ] All existing tests still pass: `make test`
    - [ ] No linter warnings: `cd backend && go vet ./cloud-gateway/...`
    - [ ] All requirements 06-REQ-1 through 06-REQ-10 acceptance criteria met

- [ ] 7. Checkpoint - All Tests Green
  - All unit, integration, and property tests pass
  - Binary starts, serves REST requests, routes to NATS, shuts down cleanly
  - Ask the user if questions arise

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
| 06-REQ-1.1 | TS-06-1 | 4.1 | handler::TestCommandSubmission |
| 06-REQ-1.2 | TS-06-2 | 2.1 | model::TestCommandPayloadStructure |
| 06-REQ-1.3 | TS-06-3 | 4.2 | tests/cloud-gateway::TestBearerTokenInNATSHeader |
| 06-REQ-1.4 | TS-06-4 | 3.1 | store::TestCommandStoredAsPending |
| 06-REQ-1.E1 | TS-06-E1 | 4.1 | handler::TestMissingAuthHeader |
| 06-REQ-1.E2 | TS-06-E2 | 4.1 | handler::TestTokenNotAuthorizedForVIN |
| 06-REQ-1.E3 | TS-06-E3 | 4.1 | handler::TestInvalidCommandPayload |
| 06-REQ-1.E4 | TS-06-E4 | 4.1 | handler::TestInvalidCommandType |
| 06-REQ-2.1 | TS-06-5 | 4.1 | handler::TestCommandStatusQuery |
| 06-REQ-2.2 | TS-06-6 | 3.1 | store::TestPendingStatusBeforeResponse |
| 06-REQ-2.3 | TS-06-7 | 3.1 | store::TestSuccessAndFailedStatus |
| 06-REQ-2.E1 | TS-06-E5 | 4.1 | handler::TestUnknownCommandID |
| 06-REQ-2.E2 | TS-06-E6 | 4.1 | handler::TestAuthOnStatusQuery |
| 06-REQ-3.1 | TS-06-8 | 4.2 | tests/cloud-gateway::TestNATSResponseSubscription |
| 06-REQ-3.2 | TS-06-9 | 4.2 | tests/cloud-gateway::TestResponseUpdatesStore |
| 06-REQ-3.3 | TS-06-10 | 2.1 | model::TestResponsePayloadParsing |
| 06-REQ-3.E1 | TS-06-E7 | 4.2 | tests/cloud-gateway::TestInvalidNATSResponseJSON |
| 06-REQ-3.E2 | TS-06-E8 | 4.2 | tests/cloud-gateway::TestUnknownCommandIDInNATS |
| 06-REQ-4.1 | TS-06-11 | 3.1 | store::TestCommandTimeout |
| 06-REQ-4.2 | TS-06-12 | 2.2 | config::TestConfigurableTimeout |
| 06-REQ-5.1 | TS-06-13 | 4.2 | tests/cloud-gateway::TestTelemetrySubscription |
| 06-REQ-5.2 | TS-06-14 | 4.2 | tests/cloud-gateway::TestTelemetryLogging |
| 06-REQ-5.E1 | TS-06-E9 | 4.2 | tests/cloud-gateway::TestInvalidTelemetryJSON |
| 06-REQ-6.1 | TS-06-15 | 4.1 | handler::TestTokenValidationOnEndpoints |
| 06-REQ-6.2 | TS-06-16 | 2.2 | config::TestTokenVINLoading |
| 06-REQ-6.3 | TS-06-17 | 2.3 | auth::TestTokenVINAuthorization |
| 06-REQ-7.1 | TS-06-18 | 2.2 | config::TestLoadConfigFromFile |
| 06-REQ-7.2 | TS-06-19 | 2.2 | config::TestConfigFields |
| 06-REQ-7.3 | TS-06-20 | 2.2 | config::TestConfigDefaults |
| 06-REQ-7.E1 | TS-06-E10 | 2.2 | config::TestConfigFileMissing |
| 06-REQ-7.E2 | TS-06-E11 | 2.2 | config::TestConfigInvalidJSON |
| 06-REQ-8.1 | TS-06-21 | 4.2 | tests/cloud-gateway::TestNATSConnection |
| 06-REQ-8.2 | TS-06-22 | 4.2 | tests/cloud-gateway::TestNATSSubscriptionsActive |
| 06-REQ-8.E1 | TS-06-E12 | 4.2 | tests/cloud-gateway::TestNATSUnreachable |
| 06-REQ-9.1 | TS-06-23 | 4.1 | handler::TestHealthCheck |
| 06-REQ-9.2 | TS-06-24 | 5.1 | tests/cloud-gateway::TestStartupLogging |
| 06-REQ-9.3 | TS-06-25 | 5.1 | tests/cloud-gateway::TestGracefulShutdown |
| 06-REQ-10.1 | TS-06-26 | 4.1 | handler::TestContentTypeHeader |
| 06-REQ-10.2 | TS-06-27 | 4.1 | handler::TestErrorResponseFormat |
| Property 1 | TS-06-P1 | 4.1, 4.2 | handler::TestPropertyCommandRouting |
| Property 2 | TS-06-P2 | 2.3 | auth::TestPropertyAuthEnforcement |
| Property 3 | TS-06-P3 | 3.1 | store::TestPropertyResponseStatusUpdate |
| Property 4 | TS-06-P4 | 3.1 | store::TestPropertyCommandTimeout |
| Property 5 | TS-06-P5 | 2.1 | model::TestPropertyPayloadValidation |
| Property 6 | TS-06-P6 | 2.2 | config::TestPropertyConfigDefaults |
| Property 7 | TS-06-P7 | 4.2 | natsclient::TestPropertyNATSSubjects |

## Notes

- The CLOUD_GATEWAY uses `github.com/nats-io/nats.go` as its only external dependency. All HTTP handling uses Go standard library.
- Property tests in Go use table-driven tests with randomized inputs via `math/rand`. Go does not have a direct equivalent to Rust's `proptest`.
- HTTP handler tests use `net/http/httptest` for in-process testing. The NATS publisher is abstracted behind an interface so handlers can be tested without a real NATS connection.
- Integration tests requiring NATS live in `tests/cloud-gateway/` and use the containerized nats-server from `deployments/compose.yml`. Tests skip when NATS is unavailable.
- The command store is a simple `sync.Mutex`-protected `map[string]*CommandStatus`. This is adequate for the demo — no persistence or expiry beyond timeout is needed.
- The timeout expiry goroutine runs periodically (e.g., every 5 seconds) and calls `store.ExpireTimedOut()`. This is simpler than per-command timers and sufficient for demo accuracy.
- Go 1.22 `ServeMux` supports `POST /path/{param}` and `GET /path/{param}` pattern matching natively.
