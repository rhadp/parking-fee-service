# Requirements Document

## Introduction

This document specifies the requirements for the Mock Apps (Phase 1.2 / 2.1 / 2.3) of the SDV Parking Demo System. The scope covers six on-demand mock tools that simulate real vehicle sensors, the PARKING_APP, the COMPANION_APP, and a PARKING_OPERATOR for testing backend services and RHIVOS components without real hardware or Android builds. The mock tools are split into two groups: Go CLI apps (`mock/`) and Rust CLI sensor tools (`rhivos/mock-sensors/`).

## Glossary

- **parking-app-cli:** Go CLI tool simulating the PARKING_APP on AAOS IVI. Queries PARKING_FEE_SERVICE (REST), triggers adapter install via UPDATE_SERVICE (gRPC), and overrides adapter sessions via PARKING_OPERATOR_ADAPTOR (gRPC).
- **companion-app-cli:** Go CLI tool simulating the COMPANION_APP on a mobile device. Sends lock/unlock commands via CLOUD_GATEWAY REST API.
- **parking-operator:** Go CLI tool simulating an external PARKING_OPERATOR. Long-lived REST server with in-memory session storage.
- **location-sensor:** Rust CLI tool that publishes mock lat/lon values to DATA_BROKER via gRPC.
- **speed-sensor:** Rust CLI tool that publishes mock speed values to DATA_BROKER via gRPC.
- **door-sensor:** Rust CLI tool that publishes mock door open/closed state to DATA_BROKER via gRPC.
- **DATA_BROKER:** Eclipse Kuksa Databroker providing VSS-compliant gRPC pub/sub for vehicle signals (port 55556).
- **PARKING_FEE_SERVICE:** Cloud-based REST service for parking operator discovery (port 8080).
- **CLOUD_GATEWAY:** Cloud-based REST service for remote lock/unlock commands (port 8081).
- **UPDATE_SERVICE:** gRPC service managing adapter lifecycle (port 50052).
- **PARKING_OPERATOR_ADAPTOR:** gRPC service bridging vehicle and parking operator (port 50053).
- **VSS:** Vehicle Signal Specification (COVESA v5.1).
- **Fire-and-forget:** A tool that publishes a value and exits immediately.

## Requirements

### Requirement 1: Location Sensor

**User Story:** As a developer, I want to send mock GPS coordinates to DATA_BROKER, so that location-dependent services can be tested without real hardware.

#### Acceptance Criteria

1. [09-REQ-1.1] WHEN the `location-sensor` binary is invoked with `--lat=<value> --lon=<value>`, THE tool SHALL publish `Vehicle.CurrentLocation.Latitude` (double) and `Vehicle.CurrentLocation.Longitude` (double) to DATA_BROKER via kuksa.val.v1 gRPC `Set` RPC and exit with code 0.
2. [09-REQ-1.2] THE tool SHALL connect to DATA_BROKER at the address specified by `--broker-addr` flag or `DATABROKER_ADDR` environment variable, with a default of `http://localhost:55556`.

#### Edge Cases

1. [09-REQ-1.E1] IF `--lat` or `--lon` is missing, THEN THE tool SHALL print a usage error to stderr and exit with code 1.
2. [09-REQ-1.E2] IF the DATA_BROKER is unreachable, THEN THE tool SHALL print an error to stderr and exit with code 1.

### Requirement 2: Speed Sensor

**User Story:** As a developer, I want to send mock vehicle speed to DATA_BROKER, so that speed-dependent safety checks can be tested.

#### Acceptance Criteria

1. [09-REQ-2.1] WHEN the `speed-sensor` binary is invoked with `--speed=<value>`, THE tool SHALL publish `Vehicle.Speed` (float) to DATA_BROKER via kuksa.val.v1 gRPC `Set` RPC and exit with code 0.
2. [09-REQ-2.2] THE tool SHALL connect to DATA_BROKER at the address specified by `--broker-addr` flag or `DATABROKER_ADDR` environment variable, with a default of `http://localhost:55556`.

#### Edge Cases

1. [09-REQ-2.E1] IF `--speed` is missing, THEN THE tool SHALL print a usage error to stderr and exit with code 1.
2. [09-REQ-2.E2] IF the DATA_BROKER is unreachable, THEN THE tool SHALL print an error to stderr and exit with code 1.

### Requirement 3: Door Sensor

**User Story:** As a developer, I want to send mock door open/closed state to DATA_BROKER, so that door-dependent safety checks can be tested.

#### Acceptance Criteria

1. [09-REQ-3.1] WHEN the `door-sensor` binary is invoked with `--open` or `--closed`, THE tool SHALL publish `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` (bool) to DATA_BROKER via kuksa.val.v1 gRPC `Set` RPC and exit with code 0: `--open` sets `true`, `--closed` sets `false`.
2. [09-REQ-3.2] THE tool SHALL connect to DATA_BROKER at the address specified by `--broker-addr` flag or `DATABROKER_ADDR` environment variable, with a default of `http://localhost:55556`.

#### Edge Cases

1. [09-REQ-3.E1] IF neither `--open` nor `--closed` is provided, THEN THE tool SHALL print a usage error to stderr and exit with code 1.
2. [09-REQ-3.E2] IF the DATA_BROKER is unreachable, THEN THE tool SHALL print an error to stderr and exit with code 1.
3. [09-REQ-3.E3] IF both `--open` and `--closed` are provided simultaneously, THEN THE tool SHALL print a usage error to stderr and exit with code 1.

### Requirement 4: Parking App CLI - Operator Discovery

**User Story:** As a developer, I want to query parking operators by location and retrieve adapter metadata, so that I can test PARKING_FEE_SERVICE without the real Android app.

#### Acceptance Criteria

1. [09-REQ-4.1] WHEN the `parking-app-cli lookup --lat=<lat> --lon=<lon>` subcommand is invoked, THE tool SHALL send `GET /operators?lat={lat}&lon={lon}` to PARKING_FEE_SERVICE and print the JSON response to stdout, then exit with code 0.
2. [09-REQ-4.2] WHEN the `parking-app-cli adapter-info --operator-id=<id>` subcommand is invoked, THE tool SHALL send `GET /operators/{id}/adapter` to PARKING_FEE_SERVICE and print the JSON response to stdout, then exit with code 0.
3. [09-REQ-4.3] THE tool SHALL connect to PARKING_FEE_SERVICE at the address specified by `--service-addr` flag or `PARKING_FEE_SERVICE_ADDR` environment variable, with a default of `http://localhost:8080`.

#### Edge Cases

1. [09-REQ-4.E1] IF required flags are missing, THEN THE tool SHALL print a usage error to stderr and exit with code 1.
2. [09-REQ-4.E2] IF the PARKING_FEE_SERVICE returns a non-2xx response, THEN THE tool SHALL print the HTTP status and body to stderr and exit with code 1.

### Requirement 5: Parking App CLI - Adapter Lifecycle

**User Story:** As a developer, I want to install, list, watch, status-check, and remove adapters via UPDATE_SERVICE, so that I can test the adapter lifecycle without the real Android app.

#### Acceptance Criteria

1. [09-REQ-5.1] WHEN `parking-app-cli install --image-ref=<ref> --checksum=<sha256>` is invoked, THE tool SHALL call `InstallAdapter` on UPDATE_SERVICE via gRPC and print the response (job_id, adapter_id, state) to stdout, then exit with code 0.
2. [09-REQ-5.2] WHEN `parking-app-cli list` is invoked, THE tool SHALL call `ListAdapters` on UPDATE_SERVICE via gRPC and print the response to stdout, then exit with code 0.
3. [09-REQ-5.3] WHEN `parking-app-cli watch` is invoked, THE tool SHALL call `WatchAdapterStates` on UPDATE_SERVICE via gRPC and print each `AdapterStateEvent` to stdout as it arrives. THE tool SHALL run until the stream closes or the user sends SIGINT.
4. [09-REQ-5.4] WHEN `parking-app-cli status --adapter-id=<id>` is invoked, THE tool SHALL call `GetAdapterStatus` on UPDATE_SERVICE via gRPC and print the response to stdout, then exit with code 0.
5. [09-REQ-5.5] WHEN `parking-app-cli remove --adapter-id=<id>` is invoked, THE tool SHALL call `RemoveAdapter` on UPDATE_SERVICE via gRPC and print the response to stdout, then exit with code 0.
6. [09-REQ-5.6] THE tool SHALL connect to UPDATE_SERVICE at the address specified by `--update-addr` flag or `UPDATE_SERVICE_ADDR` environment variable, with a default of `localhost:50052`.

#### Edge Cases

1. [09-REQ-5.E1] IF required flags are missing for a subcommand, THEN THE tool SHALL print a usage error to stderr and exit with code 1.
2. [09-REQ-5.E2] IF the gRPC call returns an error, THEN THE tool SHALL print the gRPC status code and message to stderr and exit with code 1.

### Requirement 6: Parking App CLI - Session Override

**User Story:** As a developer, I want to manually start and stop parking sessions via PARKING_OPERATOR_ADAPTOR, so that I can test session override behavior.

#### Acceptance Criteria

1. [09-REQ-6.1] WHEN `parking-app-cli start-session --zone-id=<zone>` is invoked, THE tool SHALL call `StartSession(zone_id)` on PARKING_OPERATOR_ADAPTOR via gRPC and print the response to stdout, then exit with code 0.
2. [09-REQ-6.2] WHEN `parking-app-cli stop-session` is invoked, THE tool SHALL call `StopSession()` on PARKING_OPERATOR_ADAPTOR via gRPC and print the response to stdout, then exit with code 0.
3. [09-REQ-6.3] THE tool SHALL connect to PARKING_OPERATOR_ADAPTOR at the address specified by `--adaptor-addr` flag or `ADAPTOR_ADDR` environment variable, with a default of `localhost:50053`.

#### Edge Cases

1. [09-REQ-6.E1] IF the gRPC call returns an error (e.g., ALREADY_EXISTS, NOT_FOUND), THEN THE tool SHALL print the gRPC status code and message to stderr and exit with code 1.

### Requirement 7: Companion App CLI

**User Story:** As a developer, I want to send lock/unlock commands and query command status via CLOUD_GATEWAY, so that I can test the remote command flow without the real mobile app.

#### Acceptance Criteria

1. [09-REQ-7.1] WHEN `companion-app-cli lock --vin=<vin>` is invoked, THE tool SHALL send `POST /vehicles/{vin}/commands` with `{"type": "lock", "doors": ["driver"]}` to CLOUD_GATEWAY and print the JSON response to stdout, then exit with code 0.
2. [09-REQ-7.2] WHEN `companion-app-cli unlock --vin=<vin>` is invoked, THE tool SHALL send `POST /vehicles/{vin}/commands` with `{"type": "unlock", "doors": ["driver"]}` to CLOUD_GATEWAY and print the JSON response to stdout, then exit with code 0.
3. [09-REQ-7.3] WHEN `companion-app-cli status --vin=<vin> --command-id=<id>` is invoked, THE tool SHALL send `GET /vehicles/{vin}/commands/{command_id}` to CLOUD_GATEWAY and print the JSON response to stdout, then exit with code 0.
4. [09-REQ-7.4] THE tool SHALL include an `Authorization: Bearer <token>` header, where the token is read from `--token=<token>` flag or `CLOUD_GATEWAY_TOKEN` environment variable.
5. [09-REQ-7.5] THE tool SHALL connect to CLOUD_GATEWAY at the address specified by `--gateway-addr` flag or `CLOUD_GATEWAY_ADDR` environment variable, with a default of `http://localhost:8081`.

#### Edge Cases

1. [09-REQ-7.E1] IF `--vin` is missing, THEN THE tool SHALL print a usage error to stderr and exit with code 1.
2. [09-REQ-7.E2] IF no bearer token is provided via flag or environment variable, THEN THE tool SHALL print an error mentioning "token" to stderr and exit with code 1.
3. [09-REQ-7.E3] IF the CLOUD_GATEWAY returns a non-2xx response, THEN THE tool SHALL print the HTTP status and body to stderr and exit with code 1.

### Requirement 8: Mock Parking Operator Server

**User Story:** As a developer, I want a mock PARKING_OPERATOR REST server, so that the PARKING_OPERATOR_ADAPTOR can be tested end-to-end without a real parking operator backend.

#### Acceptance Criteria

1. [09-REQ-8.1] WHEN `parking-operator serve` is invoked, THE tool SHALL start an HTTP server on the port specified by `--port` flag or `PORT` environment variable (default: 9090) and listen until SIGTERM or SIGINT is received.
2. [09-REQ-8.2] THE server SHALL handle `POST /parking/start` accepting `{"vehicle_id", "zone_id", "timestamp"}` (where `timestamp` is a Unix timestamp in seconds, int64) and returning `{"session_id": "<uuid>", "status": "active", "rate": {"rate_type": "per_hour", "amount": 2.50, "currency": "EUR"}}` with HTTP 200.
3. [09-REQ-8.3] THE server SHALL handle `POST /parking/stop` accepting `{"session_id", "timestamp"}` (where `timestamp` is a Unix timestamp in seconds, int64) and returning `{"session_id", "status": "stopped", "duration_seconds", "total_amount", "currency": "EUR"}` with HTTP 200, where `total_amount = rate * duration_hours`.
4. [09-REQ-8.4] THE server SHALL handle `GET /parking/status/{session_id}` and return the current session state as JSON with HTTP 200.
5. [09-REQ-8.5] THE server SHALL store sessions in memory, generating UUID-format `session_id` values and calculating duration on stop.

#### Edge Cases

1. [09-REQ-8.E1] IF `POST /parking/stop` references a non-existent `session_id`, THEN THE server SHALL return HTTP 404 with an error message.
2. [09-REQ-8.E2] IF `GET /parking/status/{session_id}` references a non-existent `session_id`, THEN THE server SHALL return HTTP 404 with an error message.
3. [09-REQ-8.E3] IF the request body for `POST /parking/start` or `POST /parking/stop` is malformed, THEN THE server SHALL return HTTP 400 with an error message.

### Requirement 9: Shared Error Behavior

**User Story:** As a developer, I want consistent error reporting across all mock tools, so that failures are easy to diagnose in scripts and CI.

#### Acceptance Criteria

1. [09-REQ-9.1] ALL mock tools SHALL print errors to stderr.
2. [09-REQ-9.2] ALL mock tools SHALL exit with code 1 on any failure (connection errors, invalid arguments, HTTP/gRPC errors).
3. [09-REQ-9.3] ALL mock tools SHALL exit with code 0 on success.

### Requirement 10: Mock Sensor Proto Vendoring

**User Story:** As a developer, I want mock sensors to use vendored kuksa.val.v1 proto files, so that they can communicate with DATA_BROKER without external proto dependencies.

#### Acceptance Criteria

1. [09-REQ-10.1] THE mock-sensors crate SHALL vendor kuksa.val.v1 proto files into `rhivos/mock-sensors/proto/` and use tonic-build for code generation.
2. [09-REQ-10.2] THE mock-sensors crate SHALL use a shared library module (`src/lib.rs`) providing a `publish_datapoint` helper that connects to DATA_BROKER and sets a single VSS signal value via kuksa.val.v1 `Set` RPC.
