# PRD: Mock Apps (Phase 1.2 / 2.1 / 2.3)

> Extracted from the master PRD at `.agent-fox/specs/prd.md`. This spec covers all mock/demo tools: mock CLI apps (Go) and mock sensors (Rust).

## Scope

Implement six on-demand mock tools that simulate real vehicle sensors, the PARKING_APP, the COMPANION_APP, and a PARKING_OPERATOR for testing backend services and RHIVOS components without real hardware or Android builds.

### Mock CLI Apps (Go, in `mock/` directory)

1. **parking-app-cli** -- Mock PARKING_APP. Queries PARKING_FEE_SERVICE for operators, triggers adapter install via UPDATE_SERVICE (gRPC), and can override adapter session behavior via PARKING_OPERATOR_ADAPTOR (gRPC).
2. **companion-app-cli** -- Mock COMPANION_APP. Sends lock/unlock commands via CLOUD_GATEWAY REST API and queries command status.
3. **parking-operator** -- Mock PARKING_OPERATOR. REST server that receives start/stop parking events from PARKING_OPERATOR_ADAPTOR and exposes a status endpoint.

### Mock Sensors (Rust CLI tools, in `rhivos/` directory)

4. **location-sensor** -- CLI tool that sends mock lat/lon to DATA_BROKER via gRPC (Vehicle.CurrentLocation.Latitude, Vehicle.CurrentLocation.Longitude).
5. **speed-sensor** -- CLI tool that sends mock speed to DATA_BROKER via gRPC (Vehicle.Speed).
6. **door-sensor** -- CLI tool that sends mock door state to DATA_BROKER via gRPC (Vehicle.Cabin.Door.Row1.DriverSide.IsOpen).

## Operational Model

All mock sensors are **on-demand**: they publish values when triggered by CLI arguments, then exit. The mock PARKING_OPERATOR runs as a long-lived HTTP server (started via `serve` subcommand).

## Rationale

The two Android applications (PARKING_APP on AAOS and COMPANION_APP on mobile Android) differ fundamentally from the rest of the codebase. To avoid coupling their development lifecycle to the backend-services and RHIVOS components:

- Mock apps expose the same gRPC interface stubs and follow the same messaging protocols as the real apps.
- Can be run interactively for manual testing or scripted for automated integration tests.
- Written in Go (same language as backend services) for mock CLI apps and Rust (same language as RHIVOS services) for mock sensors, to minimize toolchain overhead.
- Share the same `.proto` definitions and message schemas as the real Android apps.

Mock sensors provide the vehicle signal inputs that the real safety-partition services (LOCKING_SERVICE, CLOUD_GATEWAY_CLIENT, PARKING_OPERATOR_ADAPTOR) depend on, enabling integration testing without physical hardware.

The mock PARKING_OPERATOR simulates an external parking operator's REST API so that the PARKING_OPERATOR_ADAPTOR can be tested end-to-end.

## Mock PARKING_APP CLI

Simulates the PARKING_APP running on AAOS IVI. Subcommands:

- `lookup --lat=<lat> --lon=<lon>` -- query PARKING_FEE_SERVICE for operators at location
- `adapter-info --operator-id=<id>` -- get adapter metadata from PARKING_FEE_SERVICE
- `install --image-ref=<ref> --checksum=<sha256>` -- trigger adapter install via UPDATE_SERVICE (gRPC)
- `watch` -- watch adapter state changes via UPDATE_SERVICE (gRPC streaming)
- `list` -- list installed adapters via UPDATE_SERVICE
- `remove --adapter-id=<id>` -- remove adapter via UPDATE_SERVICE
- `status --adapter-id=<id>` -- get adapter status via UPDATE_SERVICE
- `start-session --zone-id=<zone>` -- manually start parking session via PARKING_OPERATOR_ADAPTOR (gRPC)
- `stop-session` -- manually stop parking session via PARKING_OPERATOR_ADAPTOR (gRPC)

### Connections

- PARKING_FEE_SERVICE: REST (HTTP, port 8080)
- UPDATE_SERVICE: gRPC (network TCP, port 50052)
- PARKING_OPERATOR_ADAPTOR: gRPC (network TCP, port 50053)
- DATA_BROKER: gRPC (network TCP, port 55556) -- reserved for future use

## Mock COMPANION_APP CLI

Simulates the COMPANION_APP on a mobile device. Subcommands:

- `lock --vin=<vin>` -- send lock command via CLOUD_GATEWAY REST API
- `unlock --vin=<vin>` -- send unlock command via CLOUD_GATEWAY REST API
- `status --vin=<vin> --command-id=<id>` -- query command status via CLOUD_GATEWAY REST API

### Connections

- CLOUD_GATEWAY: REST (HTTP, port 8081)
- Uses bearer token authentication

## Mock PARKING_OPERATOR

Simulates an external parking operator's REST API. Endpoints:

- `POST /parking/start` -- start a parking session
- `POST /parking/stop` -- stop a parking session
- `GET /parking/status/{session_id}` -- query session status

### Connections

- Receives requests from PARKING_OPERATOR_ADAPTOR
- Default port: 8080

## Mock Sensors

### LOCATION_SENSOR

- CLI tool that sends mock location data to DATA_BROKER via gRPC
- VSS Vehicle.CurrentLocation.Latitude (double) and Vehicle.CurrentLocation.Longitude (double)
- Values are specified via CLI arguments: `--lat=<value> --lon=<value>`

### SPEED_SENSOR

- CLI tool that sends mock velocity data to DATA_BROKER via gRPC
- VSS Vehicle.Speed (float)
- Value is specified via CLI argument: `--speed=<value>`

### DOOR_SENSOR

- CLI tool that sends mock door open/closed data to DATA_BROKER via gRPC
- VSS Vehicle.Cabin.Door.Row1.DriverSide.IsOpen (bool)
- Value is specified via CLI argument: `--open` or `--closed`

### Sensor Connections

- DATA_BROKER: gRPC (network TCP, port 55556)

## Tech Stack

| Component | Language | Location |
|-----------|----------|----------|
| parking-app-cli | Go 1.22+ | `mock/parking-app-cli/` |
| companion-app-cli | Go 1.22+ | `mock/companion-app-cli/` |
| parking-operator | Go 1.22+ | `mock/parking-operator/` |
| location-sensor | Rust (edition 2021) | `rhivos/mock-sensors/` (binary target) |
| speed-sensor | Rust (edition 2021) | `rhivos/mock-sensors/` (binary target) |
| door-sensor | Rust (edition 2021) | `rhivos/mock-sensors/` (binary target) |

## VSS Signals Written by Mock Sensors

| Signal Path | Data Type | Written By |
|-------------|-----------|------------|
| `Vehicle.CurrentLocation.Latitude` | double | location-sensor |
| `Vehicle.CurrentLocation.Longitude` | double | location-sensor |
| `Vehicle.Speed` | float | speed-sensor |
| `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` | bool | door-sensor |

## Dependencies

| Spec | From Group | To Group | Relationship |
|------|-----------|----------|--------------|
| 01_project_setup | 3 | 2 | Uses Rust workspace and mock-sensors crate skeleton from group 3; group 3 creates the Cargo workspace with mock-sensors binary targets |
| 01_project_setup | 4 | 3 | Uses Go workspace and mock CLI skeletons from group 4; group 4 creates mock/ Go module with skeleton binaries |
| 01_project_setup | 6 | 3 | Uses proto definitions from group 6; group 6 creates proto files for UpdateService and ParkingAdaptor gRPC RPCs |

## Clarifications

- **C1 (Ports):** UPDATE_SERVICE gRPC port is 50052 (per spec 07). PARKING_OPERATOR_ADAPTOR gRPC port is 50053 (per spec 08). CLOUD_GATEWAY REST port is 8081 (per spec 06). PARKING_FEE_SERVICE REST port is 8080 (per spec 05).
- **C2 (stop-session):** `stop-session` takes no `--session-id` argument; the `StopSession()` gRPC RPC is parameterless per spec 08. It stops whichever session is currently active.
- **C3 (PARKING_OPERATOR status endpoint):** `GET /parking/status/{session_id}` includes session_id in the URL path, matching the contract in spec 08 design.md.
- **C4 (PARKING_OPERATOR storage):** The mock PARKING_OPERATOR stores sessions in memory. It generates session_id values (UUID format), calculates duration on stop, and returns JSON responses matching the contract defined in spec 08: start returns `{session_id, status, rate}`, stop returns `{session_id, status, duration_seconds, total_amount, currency}`.
- **C5 (PARKING_OPERATOR port):** The mock PARKING_OPERATOR default port is 8080, matching spec 08's default `PARKING_OPERATOR_URL` (`http://localhost:8080`).
- **C6 (companion-app status):** Since CLOUD_GATEWAY (spec 06) has no general vehicle status endpoint, the `status` subcommand queries command status: `status --vin=<vin> --command-id=<id>` maps to `GET /vehicles/{vin}/commands/{command_id}`.
- **C7 (Error behavior):** All mock tools print errors to stderr and exit with code 1 on connection failures, invalid arguments, or HTTP/gRPC errors. Success exits with code 0.
- **C8 (Sensor proto vendoring):** Mock sensors vendor kuksa.val.v1 proto files into `rhivos/mock-sensors/proto/` for DATA_BROKER gRPC communication, following the per-crate vendoring pattern used by other Rust services.
- **C9 (PARKING_OPERATOR rate):** The mock PARKING_OPERATOR returns a hardcoded rate of `{rate_type: "per_hour", amount: 2.50, currency: "EUR"}` for all start responses. The total_amount on stop is calculated as `rate * duration_hours`.
- **C10 (companion-app auth):** The mock COMPANION_APP CLI reads the bearer token from `--token=<token>` flag or `CLOUD_GATEWAY_TOKEN` environment variable.
- **C11 (parking-app-cli adaptor port):** The `start-session` and `stop-session` subcommands connect to PARKING_OPERATOR_ADAPTOR at the address specified by `--adaptor-addr` flag or `ADAPTOR_ADDR` environment variable (default: `localhost:50053`).
- **C12 (Mock PARKING_OPERATOR server mode):** Unlike mock sensors (fire-and-forget), the mock PARKING_OPERATOR runs as a long-lived HTTP server. It starts with `parking-operator serve` and shuts down on SIGTERM/SIGINT.

## Out-of-Scope

- Real Android PARKING_APP or COMPANION_APP UI
- Persistent session storage in mock PARKING_OPERATOR
- Real GPS or hardware integration
- Payment processing
- TLS/mTLS for mock connections
