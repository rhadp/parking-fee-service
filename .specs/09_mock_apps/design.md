# Design: Mock CLI Apps (Spec 09)

> Design document for the mock PARKING_APP CLI and mock COMPANION_APP CLI applications.

## References

- Master PRD: `.specs/prd.md`
- Component PRD: `.specs/09_mock_apps/prd.md`
- Requirements: `.specs/09_mock_apps/requirements.md`

## Architecture Overview

Two standalone Go CLI binaries simulate the Android applications for testing backend services and RHIVOS components without real Android builds:

1. **mock PARKING_APP CLI** (`mock/parking-app-cli/`) -- simulates the AAOS PARKING_APP with 9 subcommands targeting PARKING_FEE_SERVICE (REST), UPDATE_SERVICE (gRPC), and PARKING_OPERATOR_ADAPTOR (gRPC).
2. **mock COMPANION_APP CLI** (`mock/companion-app-cli/`) -- simulates the mobile COMPANION_APP with 3 subcommands targeting CLOUD_GATEWAY (REST).

```
mock/
  parking-app-cli/
    main.go               # Entry point, subcommand dispatch
    cmd/
      lookup.go           # lookup subcommand (REST)
      adapter_info.go     # adapter-info subcommand (REST)
      install.go          # install subcommand (gRPC)
      watch.go            # watch subcommand (gRPC streaming)
      list.go             # list subcommand (gRPC)
      remove.go           # remove subcommand (gRPC)
      status.go           # status subcommand (gRPC)
      start_session.go    # start-session subcommand (gRPC)
      stop_session.go     # stop-session subcommand (gRPC)
    internal/
      config/
        config.go         # Environment variable and flag parsing
      restclient/
        client.go         # HTTP client wrapper for REST calls
      grpcclient/
        client.go         # gRPC connection helpers
      output/
        output.go         # JSON formatting and error display
    go.mod
    go.sum

  companion-app-cli/
    main.go               # Entry point, subcommand dispatch
    cmd/
      lock.go             # lock subcommand (REST)
      unlock.go           # unlock subcommand (REST)
      status.go           # status subcommand (REST)
    internal/
      config/
        config.go         # Environment variable and flag parsing
      restclient/
        client.go         # HTTP client wrapper for REST calls
      output/
        output.go         # JSON formatting and error display
    go.mod
    go.sum
```

## Technology Stack

| Technology | Version / Reference | Purpose |
|------------|-------------------|---------|
| Go | 1.22+ | Implementation language |
| `flag` (stdlib) | -- | CLI argument parsing |
| `net/http` (stdlib) | -- | HTTP client for REST calls |
| `google.golang.org/grpc` | Latest stable | gRPC client for UPDATE_SERVICE and PARKING_OPERATOR_ADAPTOR |
| `google.golang.org/protobuf` | Latest stable | Protocol buffer runtime |
| `gen/go/updateservicepb` | Workspace module | Generated gRPC stubs for UPDATE_SERVICE |
| `gen/go/parkingadaptorpb` | Workspace module | Generated gRPC stubs for PARKING_OPERATOR_ADAPTOR |
| `github.com/google/uuid` | Latest stable | UUID generation for command IDs |
| `encoding/json` (stdlib) | -- | JSON marshalling/unmarshalling |

## CLI Framework

Both CLIs use Go's standard `flag` package with a subcommand dispatch pattern. The `main.go` file uses `os.Args[1]` to determine the subcommand and delegates to the appropriate handler, each implemented with its own `flag.FlagSet`.

```go
// Subcommand dispatch pattern (pseudocode)
func main() {
    if len(os.Args) < 2 {
        printUsage()
        os.Exit(1)
    }

    switch os.Args[1] {
    case "lookup":
        runLookup(os.Args[2:])
    case "adapter-info":
        runAdapterInfo(os.Args[2:])
    // ... other subcommands
    default:
        fmt.Fprintf(os.Stderr, "unknown subcommand: %s\n", os.Args[1])
        printUsage()
        os.Exit(1)
    }
}
```

## Subcommand Structure

### Mock PARKING_APP CLI Subcommands

| Subcommand | Flags | Target Service | Protocol | RPC/Endpoint |
|------------|-------|---------------|----------|--------------|
| `lookup` | `--lat`, `--lon` | PARKING_FEE_SERVICE | REST | GET `/operators?lat={lat}&lon={lon}` |
| `adapter-info` | `--operator-id` | PARKING_FEE_SERVICE | REST | GET `/operators/{id}/adapter` |
| `install` | `--image-ref`, `--checksum` | UPDATE_SERVICE | gRPC | `InstallAdapter` |
| `watch` | (none) | UPDATE_SERVICE | gRPC | `WatchAdapterStates` (streaming) |
| `list` | (none) | UPDATE_SERVICE | gRPC | `ListAdapters` |
| `remove` | `--adapter-id` | UPDATE_SERVICE | gRPC | `RemoveAdapter` |
| `status` | `--adapter-id` | UPDATE_SERVICE | gRPC | `GetAdapterStatus` |
| `start-session` | `--zone-id` | PARKING_OPERATOR_ADAPTOR | gRPC | `StartSession` |
| `stop-session` | `--session-id` | PARKING_OPERATOR_ADAPTOR | gRPC | `StopSession` |

### Mock COMPANION_APP CLI Subcommands

| Subcommand | Flags | Target Service | Protocol | Endpoint |
|------------|-------|---------------|----------|----------|
| `lock` | `--vin` | CLOUD_GATEWAY | REST | POST `/vehicles/{vin}/commands` |
| `unlock` | `--vin` | CLOUD_GATEWAY | REST | POST `/vehicles/{vin}/commands` |
| `status` | `--vin` | CLOUD_GATEWAY | REST | GET `/vehicles/{vin}/status` |

## gRPC Client Setup

gRPC clients are created per-subcommand invocation. Each subcommand that requires gRPC:

1. Reads the target address from the configuration (environment variable or flag).
2. Creates a `grpc.ClientConn` with `grpc.NewClient` using `grpc.WithTransportCredentials(insecure.NewCredentials())` (no TLS for local development).
3. Creates the typed service client from the generated code (e.g., `updateservicepb.NewUpdateServiceClient(conn)`).
4. Sets a 10-second context deadline on the call via `context.WithTimeout`.
5. Executes the RPC and handles the response or error.
6. Closes the connection with `conn.Close()`.

For the `watch` subcommand (server-streaming RPC):
1. Calls `WatchAdapterStates` which returns a stream client.
2. Loops calling `stream.Recv()` until `io.EOF` or an error occurs.
3. Handles `SIGINT` (Ctrl+C) by cancelling the context and cleanly closing the stream.

```go
// gRPC connection helper (pseudocode)
func dialGRPC(addr string) (*grpc.ClientConn, error) {
    ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
    defer cancel()
    return grpc.DialContext(ctx, addr,
        grpc.WithTransportCredentials(insecure.NewCredentials()),
        grpc.WithBlock(),
    )
}
```

## HTTP Client Setup

REST clients use Go's standard `net/http` package:

1. Create an `http.Client` with a 10-second timeout.
2. Construct the request URL from the base URL (environment variable or flag) and the endpoint path.
3. For POST requests (lock/unlock), marshal the JSON body and set `Content-Type: application/json`.
4. For authenticated requests (COMPANION_APP), set the `Authorization: Bearer <token>` header.
5. Execute the request and read the response body.
6. Print formatted JSON on success; print error details on failure.

```go
// HTTP client setup (pseudocode)
func newHTTPClient() *http.Client {
    return &http.Client{
        Timeout: 10 * time.Second,
    }
}
```

## Configuration

### Environment Variables

| Variable | Used By | Default | Description |
|----------|---------|---------|-------------|
| `PARKING_FEE_SERVICE_URL` | parking-app-cli | `http://localhost:8080` | PARKING_FEE_SERVICE base URL |
| `UPDATE_SERVICE_ADDR` | parking-app-cli | `localhost:50051` | UPDATE_SERVICE gRPC address |
| `PARKING_ADAPTOR_ADDR` | parking-app-cli | `localhost:50052` | PARKING_OPERATOR_ADAPTOR gRPC address |
| `DATA_BROKER_ADDR` | parking-app-cli | `localhost:55556` | DATA_BROKER gRPC address (reserved) |
| `CLOUD_GATEWAY_URL` | companion-app-cli | `http://localhost:8081` | CLOUD_GATEWAY base URL |
| `BEARER_TOKEN` | companion-app-cli | (empty) | Bearer token for authentication |

### Flag Precedence

Command-line flags override environment variables. Environment variables override built-in defaults. The configuration is resolved in this order:

1. Built-in default value.
2. Override with environment variable, if set.
3. Override with command-line flag, if provided.

## Output Formatting

- **Successful responses:** Print as indented JSON to stdout (using `json.MarshalIndent` with 2-space indentation).
- **Error messages:** Print to stderr with a prefix indicating the error type:
  - `error: connection failed: <details>` for connection errors.
  - `error: HTTP <status_code>: <body>` for HTTP errors.
  - `error: gRPC <status_code>: <message>` for gRPC errors.
  - `error: <flag>: <reason>` for argument validation errors.
- **Raw responses:** If a response body is not valid JSON, print it as-is to stdout.

## Correctness Properties

### CP-1: Subcommand Routing

Each CLI binary shall dispatch to the correct handler based on the subcommand name. An unknown subcommand shall produce a usage error listing all available subcommands. Invoking the binary with no arguments shall print the usage message.

### CP-2: gRPC Client Connectivity

Each gRPC subcommand shall successfully connect to and invoke the correct RPC method on the target service when the service is running and reachable. The generated protobuf stubs shall be used for type-safe message construction and response parsing.

### CP-3: REST Client Request Formatting

Each REST subcommand shall produce HTTP requests that conform to the target service's API contract: correct HTTP method (GET or POST), correct URL path with interpolated parameters, correct query parameters, correct JSON body structure, and correct headers (Content-Type, Authorization).

### CP-4: Error Display Fidelity

All error conditions (connection failures, HTTP error responses, gRPC error statuses, argument validation failures, timeouts) shall produce distinct, human-readable error messages on stderr that include enough context to diagnose the issue without inspecting source code or network traces.

### CP-5: Streaming Event Delivery

The `watch` subcommand shall print each streaming event as it arrives from the server without buffering. When the stream ends or is interrupted, the subcommand shall exit cleanly with an appropriate message.

### CP-6: Configuration Consistency

The resolved configuration (after merging defaults, environment variables, and flags) shall be used consistently across all network calls within a single CLI invocation. No subcommand shall use hardcoded addresses that bypass the configuration system.

## Error Handling

| Error Scenario | CLI Behavior |
|---------------|-------------|
| Missing required flag | Print usage error to stderr; exit code 1 |
| Unknown subcommand | Print usage listing to stderr; exit code 1 |
| Invalid URL in configuration | Print configuration error to stderr; exit code 1 |
| Target service unreachable (REST) | Print connection error with URL to stderr; exit code 1 |
| Target service unreachable (gRPC) | Print connection error with address to stderr; exit code 1 |
| HTTP non-2xx response | Print status code and body to stderr; exit code 1 |
| gRPC non-OK status | Print gRPC status code and message to stderr; exit code 1 |
| HTTP request timeout (10s) | Print timeout error with URL to stderr; exit code 1 |
| gRPC call timeout (10s) | Print timeout error with RPC method to stderr; exit code 1 |
| gRPC stream error (watch) | Print stream error to stderr; exit code 1 |
| Response body not valid JSON | Print raw body to stdout; exit code 0 |
| BEARER_TOKEN not set (companion-app-cli) | Print warning to stderr; proceed without auth header |
| Ctrl+C during watch | Cancel stream context; exit code 0 |

## Testing Strategy

### What We Test

1. **Subcommand dispatch** -- each subcommand name routes to the correct handler; unknown subcommands produce usage errors.
2. **Argument parsing** -- required flags are validated; missing flags produce usage errors; flag values are correctly passed to handlers.
3. **REST request construction** -- correct URL, method, headers, query parameters, and body.
4. **gRPC request construction** -- correct RPC method, request message fields.
5. **Response output** -- successful responses are formatted as indented JSON on stdout.
6. **Error output** -- error scenarios produce appropriate messages on stderr with non-zero exit codes.

### What We Do Not Test

- Backend service correctness (covered by specs 05, 06, 07, 08).
- Network-level behavior (TLS, retries, connection pooling).
- End-to-end flows involving multiple services (covered by integration test specs).

### Test Implementation

Tests are written in Go and located alongside the source code:

- **Unit tests:** `cd mock/parking-app-cli && go test ./... -v` and `cd mock/companion-app-cli && go test ./... -v`
- **Build verification:** `go build ./mock/parking-app-cli/...` and `go build ./mock/companion-app-cli/...`
- **Lint:** `cd mock/parking-app-cli && go vet ./...` and `cd mock/companion-app-cli && go vet ./...`

See `test_spec.md` for detailed test specifications.

## Definition of Done

1. Both CLIs build successfully: `go build ./mock/parking-app-cli/...` and `go build ./mock/companion-app-cli/...`.
2. The mock PARKING_APP CLI supports all 9 subcommands: `lookup`, `adapter-info`, `install`, `watch`, `list`, `remove`, `status`, `start-session`, `stop-session`.
3. The mock COMPANION_APP CLI supports all 3 subcommands: `lock`, `unlock`, `status`.
4. Each subcommand correctly constructs and sends the appropriate REST or gRPC request.
5. Missing required flags produce usage errors on stderr with a non-zero exit code.
6. Service-unreachable errors produce meaningful error messages on stderr.
7. Successful responses are printed as indented JSON to stdout.
8. All unit tests pass: `cd mock/parking-app-cli && go test ./... -v` and `cd mock/companion-app-cli && go test ./... -v`.
9. Go vet reports no issues: `cd mock/parking-app-cli && go vet ./...` and `cd mock/companion-app-cli && go vet ./...`.
