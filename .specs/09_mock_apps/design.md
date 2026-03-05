# Design: Mock Apps (Spec 09)

> Design document for all mock/demo tools: mock CLI apps (Go), mock PARKING_OPERATOR (Go), and mock sensors (Rust).
> Implements requirements from `.specs/09_mock_apps/requirements.md`.

## References

- Master PRD: `.specs/prd.md`
- Component PRD: `.specs/09_mock_apps/prd.md`
- Requirements: `.specs/09_mock_apps/requirements.md`
- DATA_BROKER Design: `.specs/02_data_broker/design.md`

## Architecture Overview

Six standalone tools simulate real vehicle sensors, Android applications, and an external parking operator for testing the parking fee demo system without real hardware or Android builds.

```
+------------------------------+    +------------------------------+
| Mock CLI Apps (Go)           |    | Mock Sensors (Rust)          |
|                              |    |                              |
| parking-app-cli              |    | location-sensor              |
|   -> PARKING_FEE_SERVICE     |    |   -> DATA_BROKER (lat/lon)   |
|   -> UPDATE_SERVICE          |    |                              |
|   -> PARKING_OPERATOR_ADAPTOR|    | speed-sensor                 |
|                              |    |   -> DATA_BROKER (speed)     |
| companion-app-cli            |    |                              |
|   -> CLOUD_GATEWAY           |    | door-sensor                  |
|                              |    |   -> DATA_BROKER (door open) |
| parking-operator             |    |                              |
|   <- PARKING_OPERATOR_ADAPTOR|    |                              |
+------------------------------+    +------------------------------+
```

## Module Structure

```
mock/
  parking-app-cli/
    main.go                         # Entry point, subcommand dispatch
    cmd/
      lookup.go                     # lookup subcommand (REST)
      adapter_info.go               # adapter-info subcommand (REST)
      install.go                    # install subcommand (gRPC)
      watch.go                      # watch subcommand (gRPC streaming)
      list.go                       # list subcommand (gRPC)
      remove.go                     # remove subcommand (gRPC)
      status.go                     # status subcommand (gRPC)
      start_session.go              # start-session subcommand (gRPC)
      stop_session.go               # stop-session subcommand (gRPC)
    internal/
      config/
        config.go                   # Environment variable and flag parsing
      restclient/
        client.go                   # HTTP client wrapper for REST calls
      grpcclient/
        client.go                   # gRPC connection helpers
      output/
        output.go                   # JSON formatting and error display
    go.mod
    go.sum

  companion-app-cli/
    main.go                         # Entry point, subcommand dispatch
    cmd/
      lock.go                       # lock subcommand (REST)
      unlock.go                     # unlock subcommand (REST)
      status.go                     # status subcommand (REST)
    internal/
      config/
        config.go                   # Environment variable and flag parsing
      restclient/
        client.go                   # HTTP client wrapper with bearer token
      output/
        output.go                   # JSON formatting and error display
    go.mod
    go.sum

  parking-operator/
    main.go                         # HTTP server entry point
    handler.go                      # Route handlers (start/stop/status)
    handler_test.go                 # Handler tests
    session.go                      # In-memory session store
    models.go                       # Request/response types
    go.mod
    go.sum

rhivos/
  location-sensor/
    Cargo.toml
    src/
      main.rs                       # CLI entry point, DATA_BROKER write

  speed-sensor/
    Cargo.toml
    src/
      main.rs                       # CLI entry point, DATA_BROKER write

  door-sensor/
    Cargo.toml
    src/
      main.rs                       # CLI entry point, DATA_BROKER write
```

## Technology Stack

| Technology | Version / Reference | Purpose |
|------------|-------------------|---------|
| Go | 1.22+ | Mock CLI apps and parking-operator |
| Rust | Stable (edition 2021) | Mock sensor tools |
| `flag` (Go stdlib) | -- | CLI argument parsing |
| `net/http` (Go stdlib) | -- | HTTP client (mock CLIs) and server (parking-operator) |
| `google.golang.org/grpc` | Latest stable | gRPC client for UPDATE_SERVICE and PARKING_OPERATOR_ADAPTOR |
| `google.golang.org/protobuf` | Latest stable | Protocol buffer runtime (Go) |
| `github.com/google/uuid` | Latest stable | UUID generation for command IDs |
| `encoding/json` (Go stdlib) | -- | JSON marshalling/unmarshalling |
| `clap` | Latest stable | Rust CLI argument parsing |
| `tonic` | Latest stable | Rust gRPC client for DATA_BROKER |
| `tokio` | Latest stable | Rust async runtime |
| `kuksa-client` or raw `tonic` | -- | Kuksa Databroker gRPC client (kuksa.val.v1) |

## CLI Interface Design

### location-sensor (Rust)

```
USAGE:
    location-sensor --lat=<LATITUDE> --lon=<LONGITUDE> [--broker-addr=<ADDR>]

FLAGS:
    --lat <LATITUDE>        Latitude value (double, required)
    --lon <LONGITUDE>       Longitude value (double, required)
    --broker-addr <ADDR>    DATA_BROKER gRPC address (default: http://localhost:55556)

EXAMPLES:
    location-sensor --lat=48.1351 --lon=11.5820
    location-sensor --lat=48.3570 --lon=11.7950 --broker-addr=http://192.168.1.10:55556
```

**Behavior:** Connects to DATA_BROKER via gRPC. Sends a `SetRequest` for `Vehicle.CurrentLocation.Latitude` and `Vehicle.CurrentLocation.Longitude` with the specified values. Prints confirmation to stdout and exits.

### speed-sensor (Rust)

```
USAGE:
    speed-sensor --speed=<SPEED> [--broker-addr=<ADDR>]

FLAGS:
    --speed <SPEED>         Speed value in km/h (float, required)
    --broker-addr <ADDR>    DATA_BROKER gRPC address (default: http://localhost:55556)

EXAMPLES:
    speed-sensor --speed=0.0
    speed-sensor --speed=50.5
```

**Behavior:** Connects to DATA_BROKER via gRPC. Sends a `SetRequest` for `Vehicle.Speed` with the specified value. Prints confirmation to stdout and exits.

### door-sensor (Rust)

```
USAGE:
    door-sensor <--open|--closed> [--broker-addr=<ADDR>]

FLAGS:
    --open                  Set door state to open (true)
    --closed                Set door state to closed (false)
    --broker-addr <ADDR>    DATA_BROKER gRPC address (default: http://localhost:55556)

EXAMPLES:
    door-sensor --open
    door-sensor --closed
```

**Behavior:** Connects to DATA_BROKER via gRPC. Sends a `SetRequest` for `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` with `true` (--open) or `false` (--closed). Prints confirmation to stdout and exits.

### parking-app-cli (Go)

```
USAGE:
    parking-app-cli <subcommand> [flags]

SUBCOMMANDS:
    lookup          Query operators by location
    adapter-info    Get adapter metadata for an operator
    install         Install an adapter via UPDATE_SERVICE
    watch           Watch adapter state changes (streaming)
    list            List installed adapters
    remove          Remove an installed adapter
    status          Get adapter status
    start-session   Manually start a parking session
    stop-session    Manually stop a parking session

ENVIRONMENT VARIABLES:
    PARKING_FEE_SERVICE_URL    (default: http://localhost:8080)
    UPDATE_SERVICE_ADDR        (default: localhost:50051)
    PARKING_ADAPTOR_ADDR       (default: localhost:50052)
    DATA_BROKER_ADDR           (default: localhost:55556)
```

See the subcommand table below for details on each subcommand's flags and target service.

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

### companion-app-cli (Go)

```
USAGE:
    companion-app-cli <subcommand> [flags]

SUBCOMMANDS:
    lock       Send lock command to vehicle
    unlock     Send unlock command to vehicle
    status     Query vehicle status

ENVIRONMENT VARIABLES:
    CLOUD_GATEWAY_URL    (default: http://localhost:8081)
    BEARER_TOKEN         (default: empty)
```

| Subcommand | Flags | Target Service | Protocol | Endpoint |
|------------|-------|---------------|----------|----------|
| `lock` | `--vin` | CLOUD_GATEWAY | REST | POST `/vehicles/{vin}/commands` |
| `unlock` | `--vin` | CLOUD_GATEWAY | REST | POST `/vehicles/{vin}/commands` |
| `status` | `--vin` | CLOUD_GATEWAY | REST | GET `/vehicles/{vin}/status` |

## parking-operator REST API Design

The mock PARKING_OPERATOR is a minimal Go HTTP server that simulates a real operator's REST API.

### Default Port

Port 9090. Configurable via `PORT` environment variable or `-port` CLI flag.

### POST /parking/start

Starts a new parking session.

**Request:**

```json
{
  "vehicle_id": "VIN12345",
  "zone_id": "muc-central",
  "timestamp": 1709640000
}
```

**Response (200 OK):**

```json
{
  "session_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "status": "active"
}
```

**Error Responses:**

| Status | Condition |
|--------|-----------|
| 400 | Malformed JSON body or missing required fields |

### POST /parking/stop

Stops an active parking session and calculates the fee.

**Request:**

```json
{
  "session_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890"
}
```

**Response (200 OK):**

```json
{
  "session_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "duration_seconds": 3600,
  "fee": 2.50,
  "status": "completed"
}
```

**Error Responses:**

| Status | Condition |
|--------|-----------|
| 400 | Malformed JSON body or missing session_id |
| 404 | Unknown session_id |

### GET /parking/status

Returns all sessions (active and completed).

**Response (200 OK):**

```json
[
  {
    "session_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
    "vehicle_id": "VIN12345",
    "zone_id": "muc-central",
    "start_time": "2024-03-05T10:00:00Z",
    "status": "active"
  }
]
```

Returns an empty array `[]` if no sessions exist.

### In-Memory Session Store

Sessions are stored in a `sync.Map` keyed by `session_id`. Each session record contains:

```go
type Session struct {
    SessionID  string    `json:"session_id"`
    VehicleID  string    `json:"vehicle_id"`
    ZoneID     string    `json:"zone_id"`
    StartTime  time.Time `json:"start_time"`
    Status     string    `json:"status"`  // "active" or "completed"
}
```

Fee calculation uses a hardcoded rate of 2.50 EUR/hour (0.0417 EUR/minute).

## Shared Proto Usage

The mock CLI apps use the same `.proto` definitions as the real components:

- `proto/update_service.proto` -- Used by parking-app-cli for UPDATE_SERVICE gRPC calls
- `proto/parking_adaptor.proto` -- Used by parking-app-cli for PARKING_OPERATOR_ADAPTOR gRPC calls

The mock sensors use the Kuksa Databroker gRPC API (`kuksa.val.v1`) for writing signals to DATA_BROKER. The proto definitions come from the Eclipse Kuksa project.

## Correctness Properties

| ID | Property | Description |
|----|----------|-------------|
| CP-1 | Sensor signal fidelity | Each mock sensor shall write exactly the VSS signal path and data type specified in the PRD. No additional signals shall be written. |
| CP-2 | On-demand execution | Mock sensors shall publish exactly one value per invocation and exit. They shall not run as daemons or publish periodically. |
| CP-3 | Subcommand routing | Each CLI binary shall dispatch to the correct handler based on the subcommand name. Unknown subcommands produce usage errors. |
| CP-4 | Proto interface fidelity | Mock CLI apps shall use the generated gRPC stubs from the shared `proto/` directory. Request and response messages shall match the proto definitions exactly. |
| CP-5 | REST request correctness | REST subcommands shall produce HTTP requests conforming to the target service's API contract: correct method, URL path, query parameters, JSON body, and headers. |
| CP-6 | Session store consistency | The parking-operator shall maintain consistent session state: a started session can be stopped exactly once, producing correct duration and fee calculations. |
| CP-7 | Error display fidelity | All error conditions shall produce distinct, human-readable error messages on stderr that include enough context to diagnose the issue. |

## Error Handling

### Mock Sensors

| Error Scenario | Behavior |
|---------------|----------|
| Missing required CLI argument | Print usage error to stderr; exit code 1 |
| DATA_BROKER unreachable | Print connection error with address to stderr; exit code 1 |
| DATA_BROKER returns gRPC error | Print gRPC status code and message to stderr; exit code 1 |

### Mock CLI Apps (parking-app-cli, companion-app-cli)

| Error Scenario | Behavior |
|---------------|----------|
| Missing required flag | Print usage error to stderr; exit code 1 |
| Unknown subcommand | Print usage listing to stderr; exit code 1 |
| No arguments | Print usage listing to stderr; exit code 1 |
| Target service unreachable (REST) | Print connection error with URL to stderr; exit code 1 |
| Target service unreachable (gRPC) | Print connection error with address to stderr; exit code 1 |
| HTTP non-2xx response | Print status code and body to stderr; exit code 1 |
| gRPC non-OK status | Print gRPC status code and message to stderr; exit code 1 |
| Request timeout (10s) | Print timeout error to stderr; exit code 1 |
| BEARER_TOKEN not set (companion-app-cli) | Print warning to stderr; proceed without auth header |
| Ctrl+C during watch | Cancel stream context; exit code 0 |

### Mock PARKING_OPERATOR

| Error Scenario | Behavior |
|---------------|----------|
| Malformed JSON in POST body | Return HTTP 400 with `{"error": "..."}` |
| Unknown session_id in POST /parking/stop | Return HTTP 404 with `{"error": "..."}` |
| Undefined route | Return HTTP 404 with `{"error": "not found"}` |

## Testing Strategy

### What We Test

1. **Mock sensors** -- Correct VSS signal path and value written to DATA_BROKER; argument validation; error handling for unreachable broker.
2. **parking-app-cli** -- Subcommand dispatch; argument parsing; REST request construction (URL, method, headers); gRPC request construction (correct RPC method, message fields); response output formatting; error output.
3. **companion-app-cli** -- Subcommand dispatch; argument parsing; REST request construction (URL, method, headers, bearer token); response output formatting; error output.
4. **parking-operator** -- POST /parking/start creates a session; POST /parking/stop completes a session with correct duration/fee; GET /parking/status returns all sessions; error responses for invalid input.

### What We Do Not Test

- Backend service correctness (covered by specs 05, 06, 07, 08).
- Network-level behavior (TLS, retries, connection pooling).
- End-to-end flows involving multiple services (covered by integration test specs).

### Test Implementation

- **Go mock apps:** `cd mock/parking-app-cli && go test ./... -v`, `cd mock/companion-app-cli && go test ./... -v`, `cd mock/parking-operator && go test ./... -v`
- **Rust sensors:** `cd rhivos && cargo test -p location-sensor -p speed-sensor -p door-sensor`
- **Lint (Go):** `go vet ./...` for each Go module
- **Lint (Rust):** `cd rhivos && cargo clippy -p location-sensor -p speed-sensor -p door-sensor`

## Definition of Done

1. All three mock sensor binaries build: `cd rhivos && cargo build -p location-sensor -p speed-sensor -p door-sensor`.
2. Each sensor writes the correct VSS signal to DATA_BROKER when invoked with valid arguments.
3. Each sensor prints a usage error and exits with code 1 when invoked with missing arguments.
4. The mock parking-operator builds and handles POST /parking/start, POST /parking/stop, and GET /parking/status correctly.
5. The parking-app-cli builds and supports all 9 subcommands.
6. The companion-app-cli builds and supports all 3 subcommands.
7. All unit tests pass for Go and Rust components.
8. `go vet` and `cargo clippy` report no issues.
9. Successful responses are printed as formatted JSON to stdout; errors are printed to stderr.
