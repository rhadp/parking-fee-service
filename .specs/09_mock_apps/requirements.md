# Requirements: Mock CLI Apps (Spec 09)

> EARS-syntax requirements for the mock PARKING_APP CLI and mock COMPANION_APP CLI.
> Derived from the PRD at `.specs/09_mock_apps/prd.md` and the master PRD at `.specs/prd.md`.

## Notation

Requirements use the EARS (Easy Approach to Requirements Syntax) patterns:

- **Ubiquitous:** `The <system> shall <action>.`
- **Event-driven:** `When <trigger>, the <system> shall <action>.`
- **State-driven:** `While <state>, the <system> shall <action>.`
- **Unwanted behavior:** `If <condition>, then the <system> shall <action>.`
- **Option:** `Where <feature>, the <system> shall <action>.`

## Requirements

### 09-REQ-1.1: Mock PARKING_APP CLI -- REST Subcommands

When a user invokes the mock PARKING_APP CLI with the `lookup --lat=<lat> --lon=<lon>` subcommand, the CLI shall send an HTTP GET request to `PARKING_FEE_SERVICE_URL/operators?lat=<lat>&lon=<lon>` and print the JSON response to stdout. When a user invokes the `adapter-info --operator-id=<id>` subcommand, the CLI shall send an HTTP GET request to `PARKING_FEE_SERVICE_URL/operators/<id>/adapter` and print the JSON response to stdout.

**Rationale:** These subcommands simulate the PARKING_APP querying the PARKING_FEE_SERVICE for operator discovery and adapter metadata, enabling integration testing of the REST API without a real Android build.

**Acceptance criteria:**
- `lookup` sends GET `/operators?lat={lat}&lon={lon}` to the configured PARKING_FEE_SERVICE_URL.
- `adapter-info` sends GET `/operators/{id}/adapter` to the configured PARKING_FEE_SERVICE_URL.
- Both subcommands print the HTTP response body (JSON) to stdout and exit with code 0 on success.
- Both subcommands print the HTTP status code alongside the response when the status is not 2xx.

**Edge cases:**
- If `--lat` or `--lon` is missing from the `lookup` subcommand, then the CLI shall print a usage error to stderr and exit with a non-zero exit code.
- If `--operator-id` is missing from the `adapter-info` subcommand, then the CLI shall print a usage error to stderr and exit with a non-zero exit code.
- If the PARKING_FEE_SERVICE is unreachable, then the CLI shall print an error message including the target URL and the underlying connection error to stderr, and exit with a non-zero exit code.
- If the PARKING_FEE_SERVICE returns a non-2xx HTTP status, then the CLI shall print the status code and response body to stderr and exit with a non-zero exit code.

---

### 09-REQ-1.2: Mock PARKING_APP CLI -- gRPC Adapter Management Subcommands

The mock PARKING_APP CLI shall provide the following subcommands that communicate with the UPDATE_SERVICE via gRPC:

- `install --image-ref=<ref> --checksum=<sha256>` shall call `InstallAdapter` with the given image reference and SHA-256 checksum and print the response (job_id, adapter_id, state) to stdout.
- `watch` shall call `WatchAdapterStates` (server-streaming RPC) and print each `AdapterStateEvent` to stdout as it arrives, one event per line.
- `list` shall call `ListAdapters` and print the list of installed adapters to stdout.
- `remove --adapter-id=<id>` shall call `RemoveAdapter` with the given adapter ID and print the result to stdout.
- `status --adapter-id=<id>` shall call `GetAdapterStatus` with the given adapter ID and print the adapter status to stdout.

**Rationale:** These subcommands simulate the PARKING_APP managing adapter lifecycle through the UPDATE_SERVICE gRPC API, enabling integration testing of the gRPC interface without a real Android build.

**Acceptance criteria:**
- Each subcommand connects to the UPDATE_SERVICE at the address specified by `UPDATE_SERVICE_ADDR`.
- `install` sends the `image_ref` and `checksum_sha256` fields in the `InstallAdapterRequest` message.
- `watch` prints streaming events continuously until the stream ends or the user interrupts with Ctrl+C.
- `list`, `remove`, and `status` perform unary RPCs and print the response.

**Edge cases:**
- If `--image-ref` or `--checksum` is missing from the `install` subcommand, then the CLI shall print a usage error to stderr and exit with a non-zero exit code.
- If `--adapter-id` is missing from the `remove` or `status` subcommands, then the CLI shall print a usage error to stderr and exit with a non-zero exit code.
- If the UPDATE_SERVICE is unreachable, then the CLI shall print an error message including the target address and the underlying gRPC connection error to stderr, and exit with a non-zero exit code.
- If a gRPC call returns a non-OK status, then the CLI shall print the gRPC status code and message to stderr and exit with a non-zero exit code.
- If the `watch` stream is interrupted by the server (stream error or EOF), then the CLI shall print the error and exit with a non-zero exit code.

---

### 09-REQ-1.3: Mock PARKING_APP CLI -- gRPC Session Management Subcommands

The mock PARKING_APP CLI shall provide the following subcommands that communicate with the PARKING_OPERATOR_ADAPTOR via gRPC:

- `start-session --zone-id=<zone>` shall call `StartSession` with the given zone ID and print the session response (session_id, status) to stdout.
- `stop-session --session-id=<id>` shall call `StopSession` with the given session ID and print the result to stdout.

**Rationale:** These subcommands simulate the PARKING_APP overriding the autonomous session behavior of the PARKING_OPERATOR_ADAPTOR, enabling manual testing of session lifecycle without a real Android build.

**Acceptance criteria:**
- Each subcommand connects to the PARKING_OPERATOR_ADAPTOR at the address specified by `PARKING_ADAPTOR_ADDR`.
- `start-session` sends the `zone_id` field in the `StartSessionRequest` message.
- `stop-session` sends the `session_id` field in the `StopSessionRequest` message.
- Both subcommands print the gRPC response as JSON to stdout and exit with code 0 on success.

**Edge cases:**
- If `--zone-id` is missing from `start-session`, then the CLI shall print a usage error to stderr and exit with a non-zero exit code.
- If `--session-id` is missing from `stop-session`, then the CLI shall print a usage error to stderr and exit with a non-zero exit code.
- If the PARKING_OPERATOR_ADAPTOR is unreachable, then the CLI shall print an error message including the target address and the underlying gRPC connection error to stderr, and exit with a non-zero exit code.

---

### 09-REQ-2.1: Mock COMPANION_APP CLI Subcommands

The mock COMPANION_APP CLI shall provide three subcommands that communicate with the CLOUD_GATEWAY via REST:

- `lock --vin=<vin>` shall send a `POST /vehicles/{vin}/commands` request with body `{"command_id": "<generated-uuid>", "type": "lock", "doors": ["driver"]}` and print the response to stdout.
- `unlock --vin=<vin>` shall send a `POST /vehicles/{vin}/commands` request with body `{"command_id": "<generated-uuid>", "type": "unlock", "doors": ["driver"]}` and print the response to stdout.
- `status --vin=<vin>` shall send a `GET /vehicles/{vin}/status` request and print the response to stdout.

**Rationale:** These subcommands simulate the COMPANION_APP sending commands and querying status through the CLOUD_GATEWAY REST API, enabling integration testing without a real Android or Flutter build.

**Acceptance criteria:**
- `lock` and `unlock` send POST requests with a UUID `command_id`, the correct `type` field, and `doors: ["driver"]`.
- `status` sends a GET request to `/vehicles/{vin}/status`.
- All subcommands include an `Authorization: Bearer <token>` header using the token from `BEARER_TOKEN`.
- All subcommands print the HTTP response body (JSON) to stdout and exit with code 0 on success.

**Edge cases:**
- If `--vin` is missing from any subcommand, then the CLI shall print a usage error to stderr and exit with a non-zero exit code.
- If the CLOUD_GATEWAY is unreachable, then the CLI shall print an error message including the target URL and the underlying connection error to stderr, and exit with a non-zero exit code.
- If the CLOUD_GATEWAY returns a non-2xx HTTP status (e.g., 401, 400, 503), then the CLI shall print the status code and response body to stderr and exit with a non-zero exit code.
- If the `BEARER_TOKEN` environment variable is not set, then the CLI shall print a warning to stderr indicating that requests will be sent without authentication.

---

### 09-REQ-3.1: Connection Configuration

The mock PARKING_APP CLI shall read service connection addresses from the following environment variables, each with a default value:

- `PARKING_FEE_SERVICE_URL` (default: `http://localhost:8080`) -- base URL for REST calls to PARKING_FEE_SERVICE.
- `UPDATE_SERVICE_ADDR` (default: `localhost:50051`) -- gRPC address for UPDATE_SERVICE.
- `PARKING_ADAPTOR_ADDR` (default: `localhost:50052`) -- gRPC address for PARKING_OPERATOR_ADAPTOR.
- `DATA_BROKER_ADDR` (default: `localhost:55556`) -- gRPC address for DATA_BROKER (reserved for future use).

The mock COMPANION_APP CLI shall read service connection addresses from the following environment variables, each with a default value:

- `CLOUD_GATEWAY_URL` (default: `http://localhost:8081`) -- base URL for REST calls to CLOUD_GATEWAY.
- `BEARER_TOKEN` (default: empty) -- bearer token for authentication.

Both CLIs shall allow overriding these values via corresponding command-line flags, with flags taking precedence over environment variables.

**Rationale:** Environment variables and flags provide flexible configuration for running the mock CLIs against different environments (local development, CI, remote services).

**Acceptance criteria:**
- Environment variables are read at startup and used as defaults.
- Command-line flags override environment variable values when both are provided.
- Default values are used when neither the environment variable nor a flag is set.

**Edge cases:**
- If a URL environment variable contains an invalid URL (e.g., missing scheme), then the CLI shall print a configuration error to stderr and exit with a non-zero exit code before attempting any network call.

---

### 09-REQ-4.1: Error Handling and Output

The mock CLI applications shall display meaningful error messages for all failure scenarios, including connection failures, gRPC errors, HTTP errors, invalid arguments, and timeouts.

**Rationale:** Clear error messages are essential for debugging integration test failures and manual testing sessions.

**Acceptance criteria:**
- Connection failures include the target address/URL and the underlying error.
- gRPC errors include the gRPC status code name (e.g., UNAVAILABLE, NOT_FOUND) and the error message.
- HTTP errors include the HTTP status code, status text, and response body.
- Invalid argument errors include the flag name and expected format.
- All error messages are written to stderr; successful output is written to stdout.
- Successful responses are printed as formatted JSON (indented) to stdout.

**Edge cases:**
- If a gRPC call exceeds a 10-second deadline, then the CLI shall print a timeout error with the RPC method name and exit with a non-zero exit code.
- If an HTTP request exceeds a 10-second timeout, then the CLI shall print a timeout error with the request URL and exit with a non-zero exit code.
- If the server returns a response body that is not valid JSON, then the CLI shall print the raw response body to stdout without formatting.

---

## Traceability

| Requirement | PRD Section |
|-------------|-------------|
| 09-REQ-1.1 | Mock PARKING_APP CLI: lookup, adapter-info subcommands |
| 09-REQ-1.2 | Mock PARKING_APP CLI: install, watch, list, remove, status subcommands |
| 09-REQ-1.3 | Mock PARKING_APP CLI: start-session, stop-session subcommands |
| 09-REQ-2.1 | Mock COMPANION_APP CLI: lock, unlock, status subcommands |
| 09-REQ-3.1 | Connections: environment variables and connection defaults |
| 09-REQ-4.1 | Error handling and output formatting |
