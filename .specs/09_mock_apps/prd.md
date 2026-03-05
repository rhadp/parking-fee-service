# PRD: Mock Apps (Phase 1.2 / 2.1 / 2.3)

> Extracted from the master PRD at `.specs/prd.md`. This spec covers all mock/demo tools: mock CLI apps (Go) and mock sensors (Rust).

## Scope

Implement six on-demand mock tools that simulate real vehicle sensors, the PARKING_APP, the COMPANION_APP, and a PARKING_OPERATOR for testing backend services and RHIVOS components without real hardware or Android builds.

### Mock CLI Apps (Go, in `mock/` directory)

1. **parking-app-cli** -- Mock PARKING_APP. Queries PARKING_FEE_SERVICE for operators, triggers adapter install via UPDATE_SERVICE (gRPC), and can override adapter session behavior via PARKING_OPERATOR_ADAPTOR (gRPC).
2. **companion-app-cli** -- Mock COMPANION_APP. Sends lock/unlock commands via CLOUD_GATEWAY REST API and queries vehicle status.
3. **parking-operator** -- Mock PARKING_OPERATOR. REST server that receives start/stop parking events from PARKING_OPERATOR_ADAPTOR and exposes a status endpoint.

### Mock Sensors (Rust CLI tools, in `rhivos/` directory)

4. **location-sensor** -- CLI tool that sends mock lat/lon to DATA_BROKER via gRPC (Vehicle.CurrentLocation.Latitude, Vehicle.CurrentLocation.Longitude).
5. **speed-sensor** -- CLI tool that sends mock speed to DATA_BROKER via gRPC (Vehicle.Speed).
6. **door-sensor** -- CLI tool that sends mock door state to DATA_BROKER via gRPC (Vehicle.Cabin.Door.Row1.DriverSide.IsOpen).

## Operational Model

All mock tools are **on-demand**: they publish values when triggered by CLI arguments or test harness commands. They do NOT run continuously or publish on a periodic schedule.

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
- `stop-session --session-id=<id>` -- manually stop parking session via PARKING_OPERATOR_ADAPTOR (gRPC)

### Connections

- PARKING_FEE_SERVICE: REST (HTTP, port 8080)
- UPDATE_SERVICE: gRPC (network TCP, port 50051)
- PARKING_OPERATOR_ADAPTOR: gRPC (network TCP, port 50052)
- DATA_BROKER: gRPC (network TCP, port 55556) -- reserved for future use

## Mock COMPANION_APP CLI

Simulates the COMPANION_APP on a mobile device. Subcommands:

- `lock --vin=<vin>` -- send lock command via CLOUD_GATEWAY REST API
- `unlock --vin=<vin>` -- send unlock command via CLOUD_GATEWAY REST API
- `status --vin=<vin>` -- query vehicle status via CLOUD_GATEWAY REST API

### Connections

- CLOUD_GATEWAY: REST (HTTP, port 8081)
- Uses bearer token authentication

## Mock PARKING_OPERATOR

Simulates an external parking operator's REST API. Endpoints:

- `POST /parking/start` -- start a parking session
- `POST /parking/stop` -- stop a parking session
- `GET /parking/status` -- query session status

### Connections

- Receives requests from PARKING_OPERATOR_ADAPTOR
- Default port: 9090

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
| location-sensor | Rust (edition 2021) | `rhivos/location-sensor/` |
| speed-sensor | Rust (edition 2021) | `rhivos/speed-sensor/` |
| door-sensor | Rust (edition 2021) | `rhivos/door-sensor/` |

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
| 01_project_setup | 2 | 1 | Uses repo structure from group 2 |
| 02_data_broker | 3 | 1 | Mock sensors write to DATA_BROKER |
| 05_parking_fee_service | 2 | 1 | Mock PARKING_APP CLI calls PARKING_FEE_SERVICE REST API |
| 06_cloud_gateway | 2 | 1 | Mock COMPANION_APP CLI calls CLOUD_GATEWAY REST API |
| 07_update_service | 2 | 1 | Mock PARKING_APP CLI calls UPDATE_SERVICE gRPC API |
| 08_parking_operator_adaptor | 2 | 1 | Mock PARKING_APP CLI calls PARKING_OPERATOR_ADAPTOR gRPC API; mock PARKING_OPERATOR receives calls from PARKING_OPERATOR_ADAPTOR |
