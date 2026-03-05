# Tasks: Mock CLI Apps (Spec 09)

> Implementation tasks for the mock PARKING_APP CLI and mock COMPANION_APP CLI.

## Dependencies

| Spec | Relationship |
|------|-------------|
| 01_project_setup | Requires Go module structure, proto definitions, generated gRPC code, Go workspace (`go.work`) |
| 05_parking_fee_service | Mock PARKING_APP CLI calls PARKING_FEE_SERVICE REST API (GET /operators, GET /operators/{id}/adapter) |
| 06_cloud_gateway | Mock COMPANION_APP CLI calls CLOUD_GATEWAY REST API (POST /vehicles/{vin}/commands, GET /vehicles/{vin}/status) |
| 07_update_service | Mock PARKING_APP CLI calls UPDATE_SERVICE gRPC API (InstallAdapter, WatchAdapterStates, ListAdapters, RemoveAdapter, GetAdapterStatus) |
| 08_parking_operator_adaptor | Mock PARKING_APP CLI calls PARKING_OPERATOR_ADAPTOR gRPC API (StartSession, StopSession) |

## Test Commands

| Action | Command |
|--------|---------|
| Unit tests (parking-app-cli) | `cd mock/parking-app-cli && go test ./... -v` |
| Unit tests (companion-app-cli) | `cd mock/companion-app-cli && go test ./... -v` |
| Build (parking-app-cli) | `go build ./mock/parking-app-cli/...` |
| Build (companion-app-cli) | `go build ./mock/companion-app-cli/...` |
| Lint (parking-app-cli) | `cd mock/parking-app-cli && go vet ./...` |
| Lint (companion-app-cli) | `cd mock/companion-app-cli && go vet ./...` |

---

## Group 1: Write Failing Spec Tests

Write Go tests that validate the requirements. All tests should fail initially (red phase) since no implementation exists yet. Tests use `net/http/httptest` for REST subcommand testing and mock gRPC servers for gRPC subcommand testing.

### Task 1.1: Create test scaffolding for parking-app-cli

Create `mock/parking-app-cli/cmd/cmd_test.go` (or per-subcommand test files) with the following test cases:

- `TestSubcommandDispatch_UnknownCommand` -- verifies unknown subcommand produces usage error on stderr and non-zero exit (TS-09-3).
- `TestSubcommandDispatch_NoArguments` -- verifies no arguments produces usage message on stderr (TS-09-4).
- `TestLookup_MissingFlags` -- verifies `lookup` without `--lat`/`--lon` produces usage error (TS-09-5).
- `TestAdapterInfo_MissingFlags` -- verifies `adapter-info` without `--operator-id` produces usage error (TS-09-5).
- `TestInstall_MissingFlags` -- verifies `install` without `--image-ref`/`--checksum` produces usage error (TS-09-5).
- `TestRemove_MissingFlags` -- verifies `remove` without `--adapter-id` produces usage error (TS-09-5).
- `TestStatus_MissingFlags` -- verifies `status` without `--adapter-id` produces usage error (TS-09-5).
- `TestStartSession_MissingFlags` -- verifies `start-session` without `--zone-id` produces usage error (TS-09-5).
- `TestStopSession_MissingFlags` -- verifies `stop-session` without `--session-id` produces usage error (TS-09-5).
- `TestLookup_CorrectRESTEndpoint` -- uses httptest server to verify GET `/operators?lat=...&lon=...` (TS-09-P1).
- `TestAdapterInfo_CorrectRESTEndpoint` -- uses httptest server to verify GET `/operators/{id}/adapter` (TS-09-P2).
- `TestInstall_CorrectGRPCMethod` -- uses mock gRPC server to verify `InstallAdapter` call (TS-09-P3).
- `TestWatch_StreamingResponses` -- uses mock gRPC server to verify streaming event reception (TS-09-P4).
- `TestServiceUnreachable_REST` -- verifies error message on connection failure (TS-09-E1).
- `TestServiceUnreachable_GRPC` -- verifies error message on gRPC connection failure (TS-09-E2).
- `TestHTTPNon2xx_ErrorDisplay` -- verifies HTTP error display (TS-09-E3).
- `TestGRPCError_ErrorDisplay` -- verifies gRPC error display (TS-09-E4).

**Verify:** `cd mock/parking-app-cli && go test ./... -v` -- tests compile but fail.

### Task 1.2: Create test scaffolding for companion-app-cli

Create `mock/companion-app-cli/cmd/cmd_test.go` with the following test cases:

- `TestSubcommandDispatch_UnknownCommand` -- verifies unknown subcommand produces usage error (TS-09-3).
- `TestSubcommandDispatch_NoArguments` -- verifies no arguments produces usage message (TS-09-4).
- `TestLock_MissingFlags` -- verifies `lock` without `--vin` produces usage error (TS-09-5).
- `TestUnlock_MissingFlags` -- verifies `unlock` without `--vin` produces usage error (TS-09-5).
- `TestStatus_MissingFlags` -- verifies `status` without `--vin` produces usage error (TS-09-5).
- `TestLock_CorrectPayload` -- uses httptest server to verify POST body and headers (TS-09-P6).
- `TestUnlock_CorrectPayload` -- uses httptest server to verify POST body and headers (TS-09-P7).
- `TestStatus_CorrectEndpoint` -- uses httptest server to verify GET endpoint (TS-09-P8).
- `TestBearerToken_IncludedInRequests` -- verifies Authorization header (TS-09-P9).
- `TestMissingBearerToken_Warning` -- verifies warning on stderr (TS-09-E5).
- `TestServiceUnreachable_REST` -- verifies error message on connection failure (TS-09-E8).

**Verify:** `cd mock/companion-app-cli && go test ./... -v` -- tests compile but fail.

---

## Group 2: CLI Scaffolding

Set up the Go module structure, entry points, and subcommand dispatch for both CLIs.

### Task 2.1: Create parking-app-cli Go module and entry point

Create the following files:

- `mock/parking-app-cli/go.mod` -- Go module declaration (`module github.com/rhadp/parking-fee-service/mock/parking-app-cli` or matching workspace convention), requiring `google.golang.org/grpc`, `google.golang.org/protobuf`, and workspace references to `gen/go`.
- `mock/parking-app-cli/main.go` -- Entry point with subcommand dispatch. Parse `os.Args[1]` and route to the appropriate handler function. Print usage and exit with code 1 for unknown subcommands or no arguments.

The subcommand handlers can be stubs (printing "not implemented" and exiting) at this stage.

**Verify:** `go build ./mock/parking-app-cli/...` succeeds. Running the binary with no args or an unknown subcommand prints usage to stderr and exits with code 1.

### Task 2.2: Create companion-app-cli Go module and entry point

Create the following files:

- `mock/companion-app-cli/go.mod` -- Go module declaration, requiring `github.com/google/uuid`.
- `mock/companion-app-cli/main.go` -- Entry point with subcommand dispatch for `lock`, `unlock`, `status`.

**Verify:** `go build ./mock/companion-app-cli/...` succeeds. Running the binary with no args or an unknown subcommand prints usage to stderr and exits with code 1.

### Task 2.3: Create shared internal packages

For `mock/parking-app-cli/internal/`:

- `config/config.go` -- Read environment variables (`PARKING_FEE_SERVICE_URL`, `UPDATE_SERVICE_ADDR`, `PARKING_ADAPTOR_ADDR`, `DATA_BROKER_ADDR`) with defaults. Provide a `Config` struct.
- `output/output.go` -- `PrintJSON(v interface{})` for indented JSON to stdout. `PrintError(format string, args ...interface{})` for error messages to stderr.
- `restclient/client.go` -- HTTP client wrapper with 10-second timeout. `Get(url string) ([]byte, int, error)` method.
- `grpcclient/client.go` -- `Dial(addr string) (*grpc.ClientConn, error)` helper with insecure credentials and 10-second timeout.

For `mock/companion-app-cli/internal/`:

- `config/config.go` -- Read `CLOUD_GATEWAY_URL`, `BEARER_TOKEN` with defaults.
- `output/output.go` -- Same as above.
- `restclient/client.go` -- HTTP client wrapper with bearer token support. `Get(url string, token string)` and `Post(url string, body []byte, token string)` methods.

**Verify:** Both modules build. `go vet ./...` passes for both.

### Task 2.4: Update go.work to include new modules

Add `mock/parking-app-cli` and `mock/companion-app-cli` to the root `go.work` file if not already present.

**Verify:** `go work sync` succeeds.

---

## Group 3: Mock PARKING_APP CLI -- REST Subcommands

Implement the two REST-based subcommands for the PARKING_APP CLI.

### Task 3.1: Implement lookup subcommand

Create `mock/parking-app-cli/cmd/lookup.go`:

- Parse `--lat` and `--lon` flags (both required, float64).
- Validate that both flags are provided; print usage error if missing.
- Construct URL: `{PARKING_FEE_SERVICE_URL}/operators?lat={lat}&lon={lon}`.
- Send HTTP GET request using the REST client.
- On 2xx: print response body as indented JSON to stdout, exit 0.
- On non-2xx: print status code and body to stderr, exit 1.
- On connection error: print error to stderr, exit 1.

**Verify:** `cd mock/parking-app-cli && go test -run TestLookup -v` -- related tests pass.

### Task 3.2: Implement adapter-info subcommand

Create `mock/parking-app-cli/cmd/adapter_info.go`:

- Parse `--operator-id` flag (required, string).
- Validate that the flag is provided.
- Construct URL: `{PARKING_FEE_SERVICE_URL}/operators/{operator_id}/adapter`.
- Send HTTP GET request.
- Handle response same as lookup.

**Verify:** `cd mock/parking-app-cli && go test -run TestAdapterInfo -v` -- related tests pass.

---

## Group 4: Mock PARKING_APP CLI -- gRPC Subcommands

Implement the seven gRPC-based subcommands for the PARKING_APP CLI.

### Task 4.1: Implement install subcommand

Create `mock/parking-app-cli/cmd/install.go`:

- Parse `--image-ref` and `--checksum` flags (both required, string).
- Dial UPDATE_SERVICE using the gRPC client helper.
- Create `InstallAdapterRequest{ImageRef: imageRef, ChecksumSha256: checksum}`.
- Call `InstallAdapter` with a 10-second context deadline.
- Print response as JSON to stdout on success.
- Print gRPC status code and message to stderr on error.

**Verify:** `cd mock/parking-app-cli && go test -run TestInstall -v` -- related tests pass.

### Task 4.2: Implement watch subcommand

Create `mock/parking-app-cli/cmd/watch.go`:

- Dial UPDATE_SERVICE.
- Call `WatchAdapterStates` (server-streaming RPC).
- Loop on `stream.Recv()`, printing each event as JSON to stdout.
- Handle `io.EOF` (stream ended): exit 0.
- Handle stream errors: print to stderr, exit 1.
- Handle `SIGINT`: cancel context, exit 0.

**Verify:** `cd mock/parking-app-cli && go test -run TestWatch -v` -- related tests pass.

### Task 4.3: Implement list subcommand

Create `mock/parking-app-cli/cmd/list.go`:

- Dial UPDATE_SERVICE.
- Call `ListAdapters` with a 10-second context deadline.
- Print response as JSON to stdout.

**Verify:** `cd mock/parking-app-cli && go test -run TestList -v` -- related tests pass.

### Task 4.4: Implement remove subcommand

Create `mock/parking-app-cli/cmd/remove.go`:

- Parse `--adapter-id` flag (required, string).
- Dial UPDATE_SERVICE.
- Call `RemoveAdapter` with the adapter ID.
- Print response as JSON to stdout.

**Verify:** `cd mock/parking-app-cli && go test -run TestRemove -v` -- related tests pass.

### Task 4.5: Implement status subcommand (adapter)

Create `mock/parking-app-cli/cmd/status.go`:

- Parse `--adapter-id` flag (required, string).
- Dial UPDATE_SERVICE.
- Call `GetAdapterStatus` with the adapter ID.
- Print response as JSON to stdout.

**Verify:** `cd mock/parking-app-cli && go test -run TestStatus -v` -- related tests pass.

### Task 4.6: Implement start-session subcommand

Create `mock/parking-app-cli/cmd/start_session.go`:

- Parse `--zone-id` flag (required, string).
- Dial PARKING_OPERATOR_ADAPTOR using the gRPC client helper.
- Create `StartSessionRequest{ZoneId: zoneId}`.
- Call `StartSession` with a 10-second context deadline.
- Print response as JSON to stdout.

**Verify:** `cd mock/parking-app-cli && go test -run TestStartSession -v` -- related tests pass.

### Task 4.7: Implement stop-session subcommand

Create `mock/parking-app-cli/cmd/stop_session.go`:

- Parse `--session-id` flag (required, string).
- Dial PARKING_OPERATOR_ADAPTOR.
- Call `StopSession` with the session ID.
- Print response as JSON to stdout.

**Verify:** `cd mock/parking-app-cli && go test -run TestStopSession -v` -- related tests pass.

---

## Group 5: Mock COMPANION_APP CLI -- REST Subcommands

Implement the three REST-based subcommands for the COMPANION_APP CLI.

### Task 5.1: Implement lock subcommand

Create `mock/companion-app-cli/cmd/lock.go`:

- Parse `--vin` flag (required, string).
- Generate a UUID for `command_id` using `github.com/google/uuid`.
- Construct JSON body: `{"command_id": "<uuid>", "type": "lock", "doors": ["driver"]}`.
- Send POST to `{CLOUD_GATEWAY_URL}/vehicles/{vin}/commands` with `Content-Type: application/json` and `Authorization: Bearer {token}`.
- If `BEARER_TOKEN` is empty, print warning to stderr but proceed.
- Print response as JSON to stdout on success; print error to stderr on failure.

**Verify:** `cd mock/companion-app-cli && go test -run TestLock -v` -- related tests pass.

### Task 5.2: Implement unlock subcommand

Create `mock/companion-app-cli/cmd/unlock.go`:

- Same structure as lock but with `"type": "unlock"` in the JSON body.

**Verify:** `cd mock/companion-app-cli && go test -run TestUnlock -v` -- related tests pass.

### Task 5.3: Implement status subcommand (vehicle)

Create `mock/companion-app-cli/cmd/status.go`:

- Parse `--vin` flag (required, string).
- Send GET to `{CLOUD_GATEWAY_URL}/vehicles/{vin}/status` with `Authorization: Bearer {token}`.
- Print response as JSON to stdout on success; print error to stderr on failure.

**Verify:** `cd mock/companion-app-cli && go test -run TestStatus -v` -- related tests pass.

---

## Group 6: Configuration and Error Handling

Finalize configuration, error handling, and output formatting.

### Task 6.1: Implement flag-overrides-env precedence

For both CLIs, ensure that command-line flags (e.g., `--parking-fee-service-url`, `--update-service-addr`, `--cloud-gateway-url`, `--bearer-token`) override environment variable values when both are provided.

- Add global flags to each subcommand's `flag.FlagSet` for service addresses.
- In the config resolution logic, apply: default -> env var -> flag.

**Verify:** `cd mock/parking-app-cli && go test -run TestConfig -v` and `cd mock/companion-app-cli && go test -run TestConfig -v` -- configuration precedence tests pass. TS-09-E6 passes.

### Task 6.2: Implement timeout handling

Ensure all HTTP requests use a 10-second `http.Client.Timeout` and all gRPC calls use a 10-second `context.WithTimeout` deadline.

- Verify timeout errors produce messages mentioning the target URL or RPC method.

**Verify:** TS-09-E7 passes.

### Task 6.3: Final error message polish

Review all error paths and ensure:

- Connection errors include the target address/URL.
- gRPC errors include the status code name (UNAVAILABLE, NOT_FOUND, etc.) and message.
- HTTP errors include the status code, status text, and response body.
- Argument errors include the flag name and expected format.
- All errors go to stderr; all success output goes to stdout.

**Verify:** All TS-09-E* tests pass. `go vet ./...` passes for both modules.

---

## Group 7: Checkpoint

Final verification that all requirements are met.

### Task 7.1: Full test suite pass

Run the complete test suite for both CLIs and verify all tests pass.

**Verify:**
- `cd mock/parking-app-cli && go test ./... -v` -- all tests pass.
- `cd mock/companion-app-cli && go test ./... -v` -- all tests pass.
- `go build ./mock/parking-app-cli/...` -- builds successfully.
- `go build ./mock/companion-app-cli/...` -- builds successfully.
- `cd mock/parking-app-cli && go vet ./...` -- no issues.
- `cd mock/companion-app-cli && go vet ./...` -- no issues.

### Task 7.2: Manual smoke test

Run each subcommand with `--help` or missing flags and verify usage output is clear and accurate.

- `./parking-app-cli` (no args) -- prints usage with all 9 subcommands.
- `./parking-app-cli lookup` (missing flags) -- prints usage error for --lat and --lon.
- `./companion-app-cli` (no args) -- prints usage with all 3 subcommands.
- `./companion-app-cli lock` (missing flags) -- prints usage error for --vin.

**Verify:** Usage output is correct and human-readable.

---

## Traceability

| Task | Requirement | Test Spec |
|------|-------------|-----------|
| 1.1 | 09-REQ-1.1, 09-REQ-1.2, 09-REQ-1.3, 09-REQ-4.1 | TS-09-1, TS-09-3, TS-09-4, TS-09-5, TS-09-P1 through TS-09-P5, TS-09-E1 through TS-09-E4 |
| 1.2 | 09-REQ-2.1, 09-REQ-3.1, 09-REQ-4.1 | TS-09-2, TS-09-3, TS-09-4, TS-09-5, TS-09-P6 through TS-09-P9, TS-09-E5, TS-09-E8 |
| 2.1 | 09-REQ-1.1, 09-REQ-1.2, 09-REQ-1.3 | TS-09-1, TS-09-3, TS-09-4 |
| 2.2 | 09-REQ-2.1 | TS-09-2, TS-09-3, TS-09-4 |
| 2.3 | 09-REQ-3.1, 09-REQ-4.1 | TS-09-E1, TS-09-E2 |
| 2.4 | 09-REQ-3.1 | -- |
| 3.1 | 09-REQ-1.1 | TS-09-P1, TS-09-E1, TS-09-E3 |
| 3.2 | 09-REQ-1.1 | TS-09-P2, TS-09-E3 |
| 4.1 | 09-REQ-1.2 | TS-09-P3, TS-09-E2 |
| 4.2 | 09-REQ-1.2 | TS-09-P4 |
| 4.3 | 09-REQ-1.2 | TS-09-P5 |
| 4.4 | 09-REQ-1.2 | TS-09-E4 |
| 4.5 | 09-REQ-1.2 | TS-09-E4 |
| 4.6 | 09-REQ-1.3 | TS-09-P10 |
| 4.7 | 09-REQ-1.3 | TS-09-P11 |
| 5.1 | 09-REQ-2.1 | TS-09-P6, TS-09-P9, TS-09-E5 |
| 5.2 | 09-REQ-2.1 | TS-09-P7 |
| 5.3 | 09-REQ-2.1 | TS-09-P8 |
| 6.1 | 09-REQ-3.1 | TS-09-E6 |
| 6.2 | 09-REQ-4.1 | TS-09-E7 |
| 6.3 | 09-REQ-4.1 | TS-09-E1 through TS-09-E8 |
| 7.1 | All | All |
| 7.2 | 09-REQ-4.1 | TS-09-3, TS-09-4, TS-09-5 |
