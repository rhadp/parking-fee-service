# Requirements: Mock Apps (Spec 09)

## Introduction

This document specifies the requirements for all mock/demo tools: mock CLI apps (Go) for simulating the PARKING_APP and COMPANION_APP, a mock PARKING_OPERATOR REST server (Go), and mock sensor CLI tools (Rust) for injecting vehicle signals into DATA_BROKER. All tools are on-demand and share proto definitions with the real components they simulate.

## Glossary

| Term | Definition |
|------|-----------|
| Mock sensor | A Rust CLI tool that writes a single VSS signal value to DATA_BROKER via gRPC |
| parking-app-cli | Go CLI that simulates the AAOS PARKING_APP |
| companion-app-cli | Go CLI that simulates the mobile COMPANION_APP |
| parking-operator | Go REST server that simulates an external PARKING_OPERATOR |
| DATA_BROKER | Eclipse Kuksa Databroker instance (gRPC, port 55556 for network access) |
| On-demand | Tool publishes values only when triggered by CLI arguments, not on a schedule |
| VSS | COVESA Vehicle Signal Specification, version 5.1 |

## Notation

Requirements use the EARS (Easy Approach to Requirements Syntax) patterns:

- **Ubiquitous:** `The <system> shall <action>.`
- **Event-driven:** `When <trigger>, the <system> shall <action>.`
- **Unwanted behavior:** `If <condition>, then the <system> shall <action>.`

## Requirements

### Requirement 1: Mock Sensor -- Location

**User Story:** As a test engineer, I want to inject mock GPS coordinates into DATA_BROKER so that location-dependent components (PARKING_APP, PARKING_OPERATOR_ADAPTOR) can be tested without real GPS hardware.

#### Acceptance Criteria

1. **09-REQ-1.1** When a user invokes the location-sensor CLI with `--lat=<value> --lon=<value>`, the tool shall write the specified latitude to `Vehicle.CurrentLocation.Latitude` (double) and the specified longitude to `Vehicle.CurrentLocation.Longitude` (double) in DATA_BROKER via gRPC, then exit with code 0.

#### Edge Cases

1. **09-REQ-1.E1** If `--lat` or `--lon` is missing, then the location-sensor shall print a usage error to stderr and exit with a non-zero exit code without writing to DATA_BROKER.
2. **09-REQ-1.E2** If DATA_BROKER is unreachable, then the location-sensor shall print an error message including the target address to stderr and exit with a non-zero exit code.

---

### Requirement 2: Mock Sensor -- Speed

**User Story:** As a test engineer, I want to inject mock vehicle speed into DATA_BROKER so that safety-dependent components (LOCKING_SERVICE velocity check) can be tested without real vehicle hardware.

#### Acceptance Criteria

1. **09-REQ-2.1** When a user invokes the speed-sensor CLI with `--speed=<value>`, the tool shall write the specified speed to `Vehicle.Speed` (float) in DATA_BROKER via gRPC, then exit with code 0.

#### Edge Cases

1. **09-REQ-2.E1** If `--speed` is missing, then the speed-sensor shall print a usage error to stderr and exit with a non-zero exit code without writing to DATA_BROKER.
2. **09-REQ-2.E2** If DATA_BROKER is unreachable, then the speed-sensor shall print an error message including the target address to stderr and exit with a non-zero exit code.

---

### Requirement 3: Mock Sensor -- Door

**User Story:** As a test engineer, I want to inject mock door open/closed state into DATA_BROKER so that LOCKING_SERVICE can validate door-ajar safety constraints during testing.

#### Acceptance Criteria

1. **09-REQ-3.1** When a user invokes the door-sensor CLI with `--open`, the tool shall write `true` to `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` (bool) in DATA_BROKER via gRPC, then exit with code 0. When invoked with `--closed`, the tool shall write `false`.

#### Edge Cases

1. **09-REQ-3.E1** If neither `--open` nor `--closed` is provided, then the door-sensor shall print a usage error to stderr and exit with a non-zero exit code without writing to DATA_BROKER.
2. **09-REQ-3.E2** If DATA_BROKER is unreachable, then the door-sensor shall print an error message including the target address to stderr and exit with a non-zero exit code.

---

### Requirement 4: Mock PARKING_APP CLI

**User Story:** As a developer, I want a CLI tool that simulates the PARKING_APP so that I can integration-test PARKING_FEE_SERVICE, UPDATE_SERVICE, and PARKING_OPERATOR_ADAPTOR without a real Android build.

#### Acceptance Criteria

1. **09-REQ-4.1** When a user invokes the parking-app-cli with the `lookup --lat=<lat> --lon=<lon>` subcommand, the CLI shall send an HTTP GET request to `PARKING_FEE_SERVICE_URL/operators?lat=<lat>&lon=<lon>` and print the JSON response to stdout.
2. **09-REQ-4.2** When a user invokes the `adapter-info --operator-id=<id>` subcommand, the CLI shall send an HTTP GET to `PARKING_FEE_SERVICE_URL/operators/<id>/adapter` and print the JSON response to stdout.
3. **09-REQ-4.3** The parking-app-cli shall provide gRPC subcommands (`install`, `watch`, `list`, `remove`, `status`) that communicate with UPDATE_SERVICE, and gRPC subcommands (`start-session`, `stop-session`) that communicate with PARKING_OPERATOR_ADAPTOR, using the shared `.proto` definitions.

#### Edge Cases

1. **09-REQ-4.E1** If a required flag is missing from any subcommand, then the CLI shall print a usage error to stderr and exit with a non-zero exit code.
2. **09-REQ-4.E2** If a target service is unreachable, then the CLI shall print an error message including the target address or URL to stderr and exit with a non-zero exit code.

---

### Requirement 5: Mock COMPANION_APP CLI

**User Story:** As a developer, I want a CLI tool that simulates the COMPANION_APP so that I can integration-test CLOUD_GATEWAY without a real mobile app build.

#### Acceptance Criteria

1. **09-REQ-5.1** The companion-app-cli shall provide `lock --vin=<vin>`, `unlock --vin=<vin>`, and `status --vin=<vin>` subcommands that communicate with CLOUD_GATEWAY via REST, including a generated UUID `command_id` for lock/unlock commands and an `Authorization: Bearer <token>` header.

#### Edge Cases

1. **09-REQ-5.E1** If `--vin` is missing from any subcommand, then the CLI shall print a usage error to stderr and exit with a non-zero exit code.
2. **09-REQ-5.E2** If CLOUD_GATEWAY is unreachable or returns a non-2xx status, then the CLI shall print an error message to stderr and exit with a non-zero exit code.

---

### Requirement 6: Mock PARKING_OPERATOR REST Server

**User Story:** As a developer, I want a mock parking operator REST server so that the PARKING_OPERATOR_ADAPTOR can be tested end-to-end without a real external operator.

#### Acceptance Criteria

1. **09-REQ-6.1** The parking-operator shall expose `POST /parking/start` that accepts a JSON body with `vehicle_id`, `zone_id`, and `timestamp`, creates a parking session, and returns `{session_id, status: "active"}` with HTTP 200.
2. **09-REQ-6.2** The parking-operator shall expose `POST /parking/stop` that accepts a JSON body with `session_id`, calculates the parking duration and fee, and returns `{session_id, duration_seconds, fee, status: "completed"}` with HTTP 200.
3. **09-REQ-6.3** The parking-operator shall expose `GET /parking/status` that returns a list of all sessions (active and completed) as a JSON array.

#### Edge Cases

1. **09-REQ-6.E1** If `POST /parking/start` or `POST /parking/stop` receives a malformed JSON body, then the server shall return HTTP 400 with a JSON error message.
2. **09-REQ-6.E2** If `POST /parking/stop` receives a `session_id` that does not exist, then the server shall return HTTP 404 with a JSON error message.

---

### Requirement 7: On-Demand Operation

**User Story:** As a system architect, I want all mock tools to operate on-demand so that they do not interfere with normal system operation when not explicitly invoked.

#### Acceptance Criteria

1. **09-REQ-7.1** The mock sensor tools (location-sensor, speed-sensor, door-sensor) shall publish a single value per invocation and exit. They shall not run continuously or publish on a periodic schedule.
2. **09-REQ-7.2** The mock CLI apps (parking-app-cli, companion-app-cli) shall execute a single subcommand per invocation and exit (except for the `watch` subcommand, which streams until interrupted).

---

### Requirement 8: Shared Proto Definitions

**User Story:** As a developer, I want mock apps to use the same proto definitions as the real apps so that interface fidelity is guaranteed.

#### Acceptance Criteria

1. **09-REQ-8.1** The parking-app-cli shall use the generated gRPC stubs from the shared `proto/` directory for all gRPC communication with UPDATE_SERVICE and PARKING_OPERATOR_ADAPTOR.
2. **09-REQ-8.2** The mock sensor tools shall use the Kuksa Databroker gRPC API (kuksa.val.v1) for writing signals to DATA_BROKER.

---

## Traceability

| Requirement | PRD Section |
|-------------|-------------|
| 09-REQ-1 | Mock Sensors: LOCATION_SENSOR |
| 09-REQ-2 | Mock Sensors: SPEED_SENSOR |
| 09-REQ-3 | Mock Sensors: DOOR_SENSOR |
| 09-REQ-4 | Mock PARKING_APP CLI |
| 09-REQ-5 | Mock COMPANION_APP CLI |
| 09-REQ-6 | Mock PARKING_OPERATOR |
| 09-REQ-7 | Operational Model (on-demand) |
| 09-REQ-8 | Shared proto definitions |
