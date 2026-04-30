# Implementation Plan: CLOUD_GATEWAY

<!-- AGENT INSTRUCTIONS
- Implement exactly ONE top-level task group per session
- Task group 1 writes failing tests from test_spec.md — all subsequent groups
  implement code to make those tests pass
- Follow the git-flow: feature branch from main -> implement -> test -> merge to main -> push
- Update checkbox states as you go: [-] in progress, [x] complete
-->

## Overview

This plan implements the CLOUD_GATEWAY as a Go service in `backend/cloud-gateway/`. The service provides a REST API for COMPANION_APPs (command submission, status query, health) and a NATS interface for vehicle communication (command relay, response collection, telemetry logging). Task group 1 writes failing tests. Groups 2-3 implement pure-function modules (model, config, auth, store). Group 4 implements NATS client and HTTP handlers. Group 5 wires everything in main. Group 6 runs integration smoke tests.

Ordering: tests first, then data types, then pure-function modules (config, auth, store), then NATS client + HTTP handlers, then main wiring, then integration verification.

## Test Commands

- Spec tests: `cd backend && go test -v ./cloud-gateway/...`
- Property tests: `cd backend && go test -v ./cloud-gateway/... -run Property`
- Integration tests (requires NATS): `cd backend && go test -v ./cloud-gateway/... -tags=integration`
- All tests: `cd backend && go test -v ./...`
- Race detector: `cd backend && go test -race -v ./cloud-gateway/...`
- Linter: `cd backend && go vet ./cloud-gateway/...`

## Tasks

- [x] 1. Write failing spec tests
  - [x] 1.1 Set up Go module and test file structure
    - Ensure `backend/cloud-gateway/` has `go.mod` (or is part of Go workspace)
    - Create package directories: `model/`, `config/`, `auth/`, `store/`, `handler/`, `natsclient/`
    - Create test files: `config/config_test.go`, `auth/auth_test.go`, `store/store_test.go`, `handler/handler_test.go`, `natsclient/natsclient_test.go`
    - Add `nats.go` dependency: `go get github.com/nats-io/nats.go`

  - [x] 1.2 Write config and auth package tests
    - `TestLoadConfigFromFile` -- TS-06-11
    - `TestConfigTokenVINLookup` -- TS-06-12
    - `TestBearerTokenValidation` -- TS-06-8
    - `TestVINAuthorizationCheck` -- TS-06-9
    - `TestMissingAuthorizationHeader` -- TS-06-E4
    - `TestInvalidToken` -- TS-06-E5
    - `TestConfigFileMissing` -- TS-06-E7
    - `TestConfigFileInvalidJSON` -- TS-06-E8
    - _Test Spec: TS-06-8, TS-06-9, TS-06-11, TS-06-12, TS-06-E4, TS-06-E5, TS-06-E7, TS-06-E8_

  - [x] 1.3 Write store package tests
    - `TestCommandTimeout` -- TS-06-3
    - `TestResponseStoreThreadSafety` -- TS-06-5
    - _Test Spec: TS-06-3, TS-06-5_

  - [x] 1.4 Write handler integration tests (httptest)
    - `TestCommandSubmissionSuccess` -- TS-06-1 (handler-level, mock NATS)
    - `TestCommandStatusQuerySuccess` -- TS-06-4
    - `TestHealthCheck` -- TS-06-10
    - `TestContentTypeHeader` -- TS-06-13
    - `TestInvalidCommandPayload` -- TS-06-E1
    - `TestInvalidCommandType` -- TS-06-E2
    - `TestCommandNotFound` -- TS-06-E3
    - `TestErrorResponseFormat` -- TS-06-E9
    - _Test Spec: TS-06-1, TS-06-4, TS-06-10, TS-06-13, TS-06-E1, TS-06-E2, TS-06-E3, TS-06-E9_

  - [x] 1.5 Write property tests
    - `TestPropertyTokenVINIsolation` -- TS-06-P1
    - `TestPropertyResponseStoreConsistency` -- TS-06-P2
    - `TestPropertyTimeoutCompleteness` -- TS-06-P3
    - `TestPropertyAuthenticationGate` -- TS-06-P4
    - `TestPropertyTimeoutCancellation` -- TS-06-P5
    - `TestPropertyNATSHeaderPropagation` -- TS-06-P6 (integration tag, requires NATS)
    - _Test Spec: TS-06-P1 through TS-06-P6_

  - [x] 1.V Verify task group 1
    - [x] All test files compile: `cd backend && go test -v ./cloud-gateway/... -run NONE`
    - [ ] All spec tests pass (implementation was pre-existing from groups 2-6): `cd backend && go test -v ./cloud-gateway/...`
    - [x] No linter warnings: `cd backend && go vet ./cloud-gateway/...`

- [x] 2. Model and config modules
  - [x] 2.1 Implement model package
    - Define types: `Config`, `TokenMapping`, `Command`, `CommandResponse`
    - Add JSON struct tags for all fields
    - `CommandResponse.Reason` uses `omitempty` tag
    - _Requirements: 06-REQ-6.2, 06-REQ-7.2_

  - [x] 2.2 Implement config package
    - `LoadConfig(path string) (*Config, error)`: read JSON file, unmarshal into Config
    - `(c *Config) GetVINForToken(token string) (string, bool)`: lookup VIN by token
    - If file not found or invalid JSON: return error
    - _Requirements: 06-REQ-6.1, 06-REQ-6.2, 06-REQ-6.E1_

  - [x] 2.3 Implement auth middleware
    - `Middleware(cfg *Config) func(http.Handler) http.Handler`
    - Extract `Authorization: Bearer <token>` from header
    - Validate token via `cfg.GetVINForToken(token)` -- return 401 if not found
    - Extract VIN from URL path -- return 403 if VIN mismatch
    - Skip auth for `/health` endpoint
    - Set `Content-Type: application/json` on error responses
    - _Requirements: 06-REQ-3.1, 06-REQ-3.2, 06-REQ-3.E1_

  - [x] 2.V Verify task group 2
    - [x] Config and auth tests pass: `cd backend && go test -v ./cloud-gateway/config/... ./cloud-gateway/auth/...`
    - [x] All existing tests still pass: `cd backend && go test -v ./...`
    - [x] No linter warnings: `cd backend && go vet ./cloud-gateway/...`
    - [x] _Test Spec: TS-06-8, TS-06-9, TS-06-11, TS-06-12, TS-06-E4, TS-06-E5, TS-06-E7, TS-06-E8, TS-06-P1, TS-06-P4_

- [x] 3. Store module
  - [x] 3.1 Implement store package
    - `NewStore() *Store`: initialize empty response map and timer map with mutex
    - `StoreResponse(resp CommandResponse)`: store response, cancel timer if exists
    - `GetResponse(commandID string) (*CommandResponse, bool)`: lookup by command ID
    - `StartTimeout(commandID string, duration time.Duration)`: start goroutine that stores `{status:"timeout"}` after duration
    - Timeout timer is cancelled when a real response arrives via `StoreResponse`
    - _Requirements: 06-REQ-1.3, 06-REQ-2.2_

  - [x] 3.V Verify task group 3
    - [x] Store tests pass: `cd backend && go test -v ./cloud-gateway/store/...`
    - [x] Race detection clean: `cd backend && go test -race -v ./cloud-gateway/store/...`
    - [x] All existing tests still pass: `cd backend && go test -v ./...`
    - [x] No linter warnings: `cd backend && go vet ./cloud-gateway/...`
    - [x] _Test Spec: TS-06-3, TS-06-5, TS-06-P2, TS-06-P3, TS-06-P5_

- [x] 4. NATS client and HTTP handlers
  - [x] 4.1 Implement natsclient package
    - `Connect(url string, maxRetries int) (*NATSClient, error)`: connect with exponential backoff (1s, 2s, 4s), up to maxRetries attempts
    - `PublishCommand(vin string, cmd Command, token string) error`: publish to `vehicles.{vin}.commands` with `Authorization` header
    - `SubscribeResponses(store *Store) error`: subscribe to `vehicles.*.command_responses`, parse JSON, store via `store.StoreResponse`
    - `SubscribeTelemetry() error`: subscribe to `vehicles.*.telemetry`, log payload
    - `Drain() error`: drain NATS connection for graceful shutdown
    - _Requirements: 06-REQ-1.2, 06-REQ-5.1, 06-REQ-5.2, 06-REQ-5.3, 06-REQ-5.E1, 06-REQ-5.E2_

  - [x] 4.2 Implement handler package
    - `NewSubmitCommandHandler(nc *NATSClient, store *Store, timeout time.Duration) http.HandlerFunc`:
      - Parse JSON body into Command
      - Validate required fields (command_id, type, doors) -- 400 if missing
      - Validate type is "lock" or "unlock" -- 400 if invalid
      - Extract token from request context (set by auth middleware)
      - Extract VIN from URL path
      - Call `nc.PublishCommand(vin, cmd, token)`
      - Call `store.StartTimeout(cmd.CommandID, timeout)`
      - Return HTTP 202 with command JSON
    - `NewGetCommandStatusHandler(store *Store) http.HandlerFunc`:
      - Extract command_id from URL path
      - Call `store.GetResponse(commandID)` -- 404 if not found
      - Return HTTP 200 with response JSON
    - `HealthHandler() http.HandlerFunc`:
      - Return `{"status":"ok"}` with 200
    - Set `Content-Type: application/json` on all responses
    - _Requirements: 06-REQ-1.1, 06-REQ-1.E1, 06-REQ-1.E2, 06-REQ-2.1, 06-REQ-2.E1, 06-REQ-4.1, 06-REQ-7.1, 06-REQ-7.2_

  - [x] 4.V Verify task group 4
    - [x] Handler tests pass: `cd backend && go test -v ./cloud-gateway/handler/...`
    - [x] All spec tests pass: `cd backend && go test -v ./cloud-gateway/...`
    - [x] All existing tests still pass: `cd backend && go test -v ./...`
    - [x] No linter warnings: `cd backend && go vet ./cloud-gateway/...`
    - [x] _Test Spec: TS-06-1, TS-06-4, TS-06-10, TS-06-13, TS-06-E1, TS-06-E2, TS-06-E3, TS-06-E9_

- [x] 5. Main wiring and lifecycle
  - [x] 5.1 Implement main package
    - Read `CONFIG_PATH` env var (default "config.json")
    - Call `config.LoadConfig` -- exit non-zero on error
    - Call `natsclient.Connect` with configured NATS URL and 5 retries -- exit non-zero on failure
    - Call `nc.SubscribeResponses(store)` and `nc.SubscribeTelemetry()`
    - Create store via `store.NewStore()`
    - Register routes using Go 1.22 ServeMux patterns:
      - `POST /vehicles/{vin}/commands` -> auth middleware + SubmitCommandHandler
      - `GET /vehicles/{vin}/commands/{command_id}` -> auth middleware + GetCommandStatusHandler
      - `GET /health` -> HealthHandler
    - Start HTTP server on configured port
    - Log version, port, NATS URL, token count at startup
    - Handle SIGTERM/SIGINT: drain NATS, `http.Server.Shutdown()`, exit 0
    - Use `log/slog` for structured logging
    - _Requirements: 06-REQ-6.1, 06-REQ-6.3, 06-REQ-8.1, 06-REQ-8.2_

  - [x] 5.2 Create default config.json
    - Port 8081, NATS URL `nats://localhost:4222`, timeout 30s
    - At least one demo token-VIN pair
    - Place in `backend/cloud-gateway/config.json`

  - [x] 5.V Verify task group 5
    - [x] Binary builds: `cd backend && go build ./cloud-gateway/...`
    - [x] Startup logging test passes: TS-06-15
    - [x] Graceful shutdown test passes: TS-06-14 (skips when NATS unavailable)
    - [x] All spec tests pass: `cd backend && go test -v ./cloud-gateway/...`
    - [x] All existing tests still pass: `cd backend && go test -v ./...`
    - [x] No linter warnings: `cd backend && go vet ./cloud-gateway/...`

- [ ] 6. Wiring verification

  - [ ] 6.1 Trace every execution path from design.md end-to-end
    - For each path, verify the entry point actually calls the next function
      in the chain (read the calling code, do not assume)
    - Confirm no function in the chain is a stub (`return nil`, `// TODO`,
      `// stub`, `panic("not implemented")`) that was never replaced
    - Every path must be live in production code -- errata or deferrals do not
      satisfy this check
    - _Requirements: all_

  - [ ] 6.2 Verify return values propagate correctly
    - For every function in this spec that returns data consumed by a caller,
      confirm the caller receives and uses the return value
    - Grep for callers of each such function; confirm none discards the return
    - _Requirements: all_

  - [ ] 6.3 Run the integration smoke tests
    - All `TS-06-SMOKE-*` tests pass using real components (no stub bypass)
    - _Test Spec: TS-06-SMOKE-1, TS-06-SMOKE-2_

  - [ ] 6.4 Stub / dead-code audit
    - Search all files touched by this spec for: `return nil` on non-error
      returns, empty function bodies, `// TODO`, `// stub`,
      `panic("not implemented")`
    - Each hit must be either: (a) justified with a comment explaining why it
      is intentional, or (b) replaced with a real implementation
    - Document any intentional stubs here with rationale

  - [ ] 6.5 Cross-spec entry point verification
    - For each execution path whose entry point is owned by another spec
      (e.g., companion-app-cli sending REST commands to this gateway, or
      CLOUD_GATEWAY_CLIENT communicating via NATS), grep the codebase to
      confirm the entry point is actually called from production code -- not
      just from tests
    - If the upstream caller does not exist, either implement it within this
      spec or file an issue and remove the path from design.md
    - _Requirements: all_

  - [ ] 6.V Verify wiring group
    - [ ] All smoke tests pass
    - [ ] No unjustified stubs remain in touched files
    - [ ] All execution paths from design.md are live (traceable in code)
    - [ ] All cross-spec entry points are called from production code
    - [ ] All existing tests still pass: `cd backend && go test -v ./cloud-gateway/...`

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
| 06-REQ-1.1 | TS-06-1 | 4.2 | handler::TestCommandSubmissionSuccess |
| 06-REQ-1.2 | TS-06-2 | 4.1 | natsclient::TestNATSAuthorizationHeader |
| 06-REQ-1.3 | TS-06-3 | 3.1 | store::TestCommandTimeout |
| 06-REQ-1.E1 | TS-06-E1 | 4.2 | handler::TestInvalidCommandPayload |
| 06-REQ-1.E2 | TS-06-E2 | 4.2 | handler::TestInvalidCommandType |
| 06-REQ-2.1 | TS-06-4 | 4.2 | handler::TestCommandStatusQuerySuccess |
| 06-REQ-2.2 | TS-06-5 | 3.1 | store::TestResponseStoreThreadSafety |
| 06-REQ-2.E1 | TS-06-E3 | 4.2 | handler::TestCommandNotFound |
| 06-REQ-3.1 | TS-06-8 | 2.3 | auth::TestBearerTokenValidation |
| 06-REQ-3.2 | TS-06-9 | 2.3 | auth::TestVINAuthorizationCheck |
| 06-REQ-3.E1 | TS-06-E4, TS-06-E5 | 2.3 | auth::TestMissingAuthorizationHeader, auth::TestInvalidToken |
| 06-REQ-4.1 | TS-06-10 | 4.2 | handler::TestHealthCheck |
| 06-REQ-5.1 | TS-06-6 | 4.1 | natsclient::TestNATSResponseSubscription |
| 06-REQ-5.2 | TS-06-6 | 4.1 | natsclient::TestNATSResponseSubscription |
| 06-REQ-5.3 | TS-06-7 | 4.1 | natsclient::TestTelemetrySubscriptionLogging |
| 06-REQ-5.E1 | TS-06-E6 | 4.1 | natsclient::TestNATSConnectionRetryExhaustion |
| 06-REQ-5.E2 | TS-06-E10 | 4.1 | TS-06-E10 |
| 06-REQ-6.1 | TS-06-11 | 2.2 | config::TestLoadConfigFromFile |
| 06-REQ-6.2 | TS-06-11, TS-06-12 | 2.2 | config::TestConfigTokenVINLookup |
| 06-REQ-6.3 | TS-06-3 | 3.1 | store::TestCommandTimeout |
| 06-REQ-6.E1 | TS-06-E7, TS-06-E8 | 2.2 | config::TestConfigFileMissing, config::TestConfigFileInvalidJSON |
| 06-REQ-7.1 | TS-06-13 | 4.2 | handler::TestContentTypeHeader |
| 06-REQ-7.2 | TS-06-E9 | 4.2 | handler::TestErrorResponseFormat |
| 06-REQ-8.1 | TS-06-15 | 5.1 | main::TestStartupLogging |
| 06-REQ-8.2 | TS-06-14 | 5.1 | main::TestGracefulShutdown |
| Property 1 | TS-06-P1 | 2.3 | auth::TestPropertyTokenVINIsolation |
| Property 2 | TS-06-P2 | 3.1 | store::TestPropertyResponseStoreConsistency |
| Property 3 | TS-06-P3 | 3.1 | store::TestPropertyTimeoutCompleteness |
| Property 4 | TS-06-P4 | 2.3 | auth::TestPropertyAuthenticationGate |
| Property 5 | TS-06-P5 | 3.1 | store::TestPropertyTimeoutCancellation |
| Property 5 (NATS header) | TS-06-P6 | 1.5 | natsclient::TestPropertyNATSHeaderPropagation |
| Smoke 1 | TS-06-SMOKE-1 | 6.1 | smoke::TestEndToEndCommandFlow |
| Smoke 2 | TS-06-SMOKE-2 | 6.1 | smoke::TestCommandTimeoutEndToEnd |

## Notes

- The CLOUD_GATEWAY depends on `nats.go` (github.com/nats-io/nats.go) as its only external dependency. All other functionality uses Go standard library.
- Handler tests that verify NATS publishing use a mock/interface approach for the NATS client to avoid requiring a running NATS server for unit tests.
- Integration smoke tests (group 6) require a running NATS server. Use `docker compose up nats` from the project root (spec 01_project_setup group 7) before running.
- The auth middleware extracts the VIN from the URL path. It must handle both `/vehicles/{vin}/commands` and `/vehicles/{vin}/commands/{command_id}` patterns.
- The store package uses `sync.Mutex` (not `sync.RWMutex`) since timeout timer management requires write access on reads to cancel timers. Tests should run with `-race` flag.
- Go 1.22 `ServeMux` supports `POST /path/{param}` and `GET /path/{param}` pattern matching natively.
- Startup logging and graceful shutdown tests (TS-06-14, TS-06-15) may require starting the binary as a subprocess and capturing output/signals.
