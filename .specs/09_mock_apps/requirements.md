# Requirements Document

## Introduction

This document specifies the requirements for the Mock Apps component of the SDV Parking Demo System. The mock apps are six on-demand tools (three Rust CLI sensors and three Go CLI/server apps) that simulate real vehicle sensors, the PARKING_APP, the COMPANION_APP, and a PARKING_OPERATOR for testing backend services and RHIVOS components without real hardware or Android builds.

## Glossary

- **Mock sensor:** A Rust CLI tool that writes a single VSS signal value to DATA_BROKER via gRPC and exits.
- **DATA_BROKER:** Eclipse Kuksa Databroker providing VSS-compliant gRPC pub/sub for vehicle signals.
- **kuksa.val.v1:** The gRPC service API exposed by DATA_BROKER for reading and writing vehicle signals.
- **VSS:** Vehicle Signal Specification (COVESA), defining standardized signal paths and data types.
- **PARKING_FEE_SERVICE:** Cloud-based Go HTTP service providing operator discovery and adapter metadata.
- **UPDATE_SERVICE:** Rust gRPC service managing containerized adapter lifecycle via podman.
- **PARKING_OPERATOR_ADAPTOR:** Containerized Rust application bridging PARKING_APP (gRPC) with a PARKING_OPERATOR (REST).
- **CLOUD_GATEWAY:** Go HTTP/NATS service routing commands between COMPANION_APPs and vehicles.
- **Mock PARKING_OPERATOR:** A Go HTTP server simulating an external parking operator's REST API.
- **Subcommand:** A positional argument that selects the operation a CLI tool performs (e.g., `lock`, `lookup`, `serve`).
- **Bearer token:** An opaque string passed in the HTTP `Authorization: Bearer <token>` header for authentication.
- **Session:** A parking session tracked by the mock PARKING_OPERATOR, identified by a UUID session_id.

## Requirements

### Requirement 1: Mock Sensor Signal Publishing

**User Story:** As a developer, I want CLI tools that write mock vehicle signal values to DATA_BROKER, so that I can integration-test RHIVOS services without real hardware.

#### Acceptance Criteria

1. [09-REQ-1.1] WHEN `location-sensor` is invoked with `--lat=<value> --lon=<value>`, THE tool SHALL write `Vehicle.CurrentLocation.Latitude` and `Vehicle.CurrentLocation.Longitude` to DATA_BROKER via kuksa.val.v1 gRPC and exit with code 0.
2. [09-REQ-1.2] WHEN `speed-sensor` is invoked with `--speed=<value>`, THE tool SHALL write `Vehicle.Speed` to DATA_BROKER via kuksa.val.v1 gRPC and exit with code 0.
3. [09-REQ-1.3] WHEN `door-sensor` is invoked with `--open`, THE tool SHALL write `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen = true` to DATA_BROKER and exit with code 0. WHEN invoked with `--closed`, THE tool SHALL write `false`.

#### Edge Cases

1. [09-REQ-1.E1] IF a mock sensor is invoked with no arguments or invalid arguments, THEN THE tool SHALL print a usage message to stderr and exit with code 1.
2. [09-REQ-1.E2] IF DATA_BROKER is unreachable, THEN THE mock sensor SHALL print an error message to stderr and exit with code 1.

### Requirement 2: Mock PARKING_OPERATOR REST Server

**User Story:** As a developer, I want a mock parking operator HTTP server, so that I can test the PARKING_OPERATOR_ADAPTOR end-to-end without a real parking backend.

#### Acceptance Criteria

1. [09-REQ-2.1] WHEN `parking-operator serve` is invoked, THE server SHALL listen on the configured port and log a ready message including the port number.
2. [09-REQ-2.2] WHEN a `POST /parking/start` request is received with `{vehicle_id, zone_id, timestamp}`, THE server SHALL create a session with a generated UUID session_id, store it in memory, and return HTTP 200 with `{session_id, status: "active", rate: {rate_type: "per_hour", amount: 2.50, currency: "EUR"}}`.
3. [09-REQ-2.3] WHEN a `POST /parking/stop` request is received with `{session_id, timestamp}`, THE server SHALL find the session, calculate `duration_seconds` from start to stop timestamp, compute `total_amount` as `rate * duration_hours`, and return HTTP 200 with `{session_id, status: "stopped", duration_seconds, total_amount, currency}`.
4. [09-REQ-2.4] WHEN a `GET /parking/status/{session_id}` request is received, THE server SHALL return HTTP 200 with the session's current state as JSON.
5. [09-REQ-2.5] WHEN the server receives SIGTERM or SIGINT, THE server SHALL shut down gracefully and exit with code 0.

#### Edge Cases

1. [09-REQ-2.E1] IF `POST /parking/stop` is called with an unknown session_id, THEN THE server SHALL return HTTP 404 with `{"error": "session not found"}`.
2. [09-REQ-2.E2] IF `GET /parking/status/{session_id}` is called with an unknown session_id, THEN THE server SHALL return HTTP 404 with `{"error": "session not found"}`.
3. [09-REQ-2.E3] IF `POST /parking/start` or `POST /parking/stop` receives invalid JSON, THEN THE server SHALL return HTTP 400 with `{"error": "invalid request body"}`.

### Requirement 3: Mock COMPANION_APP CLI

**User Story:** As a developer, I want a CLI that simulates the COMPANION_APP, so that I can test CLOUD_GATEWAY lock/unlock command flow without a real mobile app.

#### Acceptance Criteria

1. [09-REQ-3.1] WHEN `companion-app-cli lock --vin=<vin>` is invoked with a valid bearer token, THE tool SHALL send a `POST /vehicles/{vin}/commands` request to CLOUD_GATEWAY with `{command_id: "<uuid>", type: "lock", doors: ["driver"]}` and print the JSON response to stdout.
2. [09-REQ-3.2] WHEN `companion-app-cli unlock --vin=<vin>` is invoked with a valid bearer token, THE tool SHALL send a `POST /vehicles/{vin}/commands` request to CLOUD_GATEWAY with `{command_id: "<uuid>", type: "unlock", doors: ["driver"]}` and print the JSON response to stdout.
3. [09-REQ-3.3] WHEN `companion-app-cli status --vin=<vin> --command-id=<id>` is invoked, THE tool SHALL send a `GET /vehicles/{vin}/commands/{command_id}` request to CLOUD_GATEWAY and print the JSON response to stdout.

#### Edge Cases

1. [09-REQ-3.E1] IF no bearer token is provided via `--token` flag or `CLOUD_GATEWAY_TOKEN` environment variable, THEN THE tool SHALL print an error to stderr and exit with code 1.
2. [09-REQ-3.E2] IF CLOUD_GATEWAY is unreachable, THEN THE tool SHALL print an error to stderr and exit with code 1.

### Requirement 4: Mock PARKING_APP CLI

**User Story:** As a developer, I want a CLI that simulates the PARKING_APP, so that I can test PARKING_FEE_SERVICE, UPDATE_SERVICE, and PARKING_OPERATOR_ADAPTOR without a real AAOS app.

#### Acceptance Criteria

1. [09-REQ-4.1] WHEN `parking-app-cli lookup --lat=<lat> --lon=<lon>` is invoked, THE tool SHALL send `GET /operators?lat={lat}&lon={lon}` to PARKING_FEE_SERVICE and print the JSON response to stdout.
2. [09-REQ-4.2] WHEN `parking-app-cli adapter-info --operator-id=<id>` is invoked, THE tool SHALL send `GET /operators/{id}/adapter` to PARKING_FEE_SERVICE and print the JSON response to stdout.
3. [09-REQ-4.3] WHEN `parking-app-cli install --image-ref=<ref> --checksum=<sha256>` is invoked, THE tool SHALL call UPDATE_SERVICE `InstallAdapter` gRPC and print the response to stdout.
4. [09-REQ-4.4] WHEN `parking-app-cli watch` is invoked, THE tool SHALL call UPDATE_SERVICE `WatchAdapterStates` gRPC and stream each `AdapterStateEvent` to stdout as it arrives.
5. [09-REQ-4.5] WHEN `parking-app-cli list` is invoked, THE tool SHALL call UPDATE_SERVICE `ListAdapters` gRPC and print the adapter list to stdout.
6. [09-REQ-4.6] WHEN `parking-app-cli remove --adapter-id=<id>` is invoked, THE tool SHALL call UPDATE_SERVICE `RemoveAdapter` gRPC and print the response to stdout.
7. [09-REQ-4.7] WHEN `parking-app-cli status --adapter-id=<id>` is invoked, THE tool SHALL call UPDATE_SERVICE `GetAdapterStatus` gRPC and print the response to stdout.
8. [09-REQ-4.8] WHEN `parking-app-cli start-session --zone-id=<zone>` is invoked, THE tool SHALL call PARKING_OPERATOR_ADAPTOR `StartSession` gRPC and print the response to stdout.
9. [09-REQ-4.9] WHEN `parking-app-cli stop-session` is invoked, THE tool SHALL call PARKING_OPERATOR_ADAPTOR `StopSession` gRPC and print the response to stdout.

#### Edge Cases

1. [09-REQ-4.E1] IF `parking-app-cli` is invoked with no subcommand or an unknown subcommand, THEN THE tool SHALL print usage to stderr and exit with code 1.
2. [09-REQ-4.E2] IF a required flag is missing for a subcommand (e.g., `install` without `--image-ref`), THEN THE tool SHALL print an error to stderr and exit with code 1.
3. [09-REQ-4.E3] IF the upstream service (PARKING_FEE_SERVICE, UPDATE_SERVICE, or PARKING_OPERATOR_ADAPTOR) is unreachable, THEN THE tool SHALL print an error to stderr and exit with code 1.

### Requirement 5: Configuration

**User Story:** As a developer, I want all mock tools to be configurable via environment variables and flags, so that I can point them at different service endpoints.

#### Acceptance Criteria

1. [09-REQ-5.1] THE mock sensors SHALL read the DATA_BROKER address from `DATA_BROKER_ADDR` environment variable with a default of `http://localhost:55556`.
2. [09-REQ-5.2] THE mock COMPANION_APP CLI SHALL read the CLOUD_GATEWAY address from `CLOUD_GATEWAY_URL` environment variable or `--gateway-url` flag with a default of `http://localhost:8081`.
3. [09-REQ-5.3] THE mock PARKING_APP CLI SHALL read service addresses from environment variables: `PARKING_FEE_SERVICE_URL` (default `http://localhost:8080`), `UPDATE_SERVICE_ADDR` (default `localhost:50052`), `ADAPTOR_ADDR` (default `localhost:50053`).
4. [09-REQ-5.4] THE mock PARKING_OPERATOR SHALL read the listen port from `PORT` environment variable or `--port` flag with a default of `8080`.

### Requirement 6: Error Handling and Usage

**User Story:** As a developer, I want consistent error reporting across all mock tools, so that I can quickly diagnose issues.

#### Acceptance Criteria

1. [09-REQ-6.1] WHEN any mock tool is invoked with `--help`, THE tool SHALL print a usage message describing available subcommands and flags, and exit with code 0.
2. [09-REQ-6.2] WHEN any mock tool encounters a connection error, THE tool SHALL print the error to stderr with the target address and exit with code 1.
3. [09-REQ-6.3] WHEN any mock tool receives an error response from an upstream service, THE tool SHALL print the error details to stderr and exit with code 1.
