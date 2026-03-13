# Requirements Document

## Introduction

This document specifies the requirements for the CLOUD_GATEWAY component (Phase 2.2) of the SDV Parking Demo System. The CLOUD_GATEWAY is a cloud-based Go service that bridges COMPANION_APPs (via REST API) and vehicles (via NATS), routing lock/unlock commands and receiving telemetry. It authenticates requests using bearer tokens and supports multiple vehicles simultaneously.

## Glossary

- **CLOUD_GATEWAY:** A Go service with dual interfaces (REST + NATS) that routes commands between COMPANION_APPs and vehicles.
- **COMPANION_APP:** A mobile application paired with a specific VIN that sends lock/unlock commands via REST.
- **CLOUD_GATEWAY_CLIENT:** A Rust service running on the vehicle that connects to CLOUD_GATEWAY via NATS.
- **VIN:** Vehicle Identification Number, a unique identifier for each vehicle.
- **Bearer token:** A string token used for authentication. Each token is associated with a specific VIN.
- **NATS:** A lightweight messaging system used for vehicle-to-cloud communication.
- **NATS subject:** A topic string used to route messages. Pattern: `vehicles.{vin}.{action}`.
- **Command:** A lock or unlock request sent by a COMPANION_APP to a vehicle.
- **Command response:** A status message (success/failed/timeout) returned after a command is processed by the vehicle.
- **Telemetry:** Vehicle state data (location, door status, parking state) published by CLOUD_GATEWAY_CLIENT.
- **Protocol translation:** Converting between REST request/response and NATS publish/subscribe patterns.
- **Command timeout:** The maximum time to wait for a vehicle to respond to a command before marking it as timed out.

## Requirements

### Requirement 1: Command Submission

**User Story:** As a COMPANION_APP, I want to send lock/unlock commands to my vehicle via REST, so that I can control my vehicle remotely.

#### Acceptance Criteria

1. [06-REQ-1.1] WHEN a POST request is made to `/vehicles/{vin}/commands` with a valid bearer token and command payload, THE service SHALL publish the command to NATS subject `vehicles.{vin}.commands` and return HTTP 202 with the command ID.
2. [06-REQ-1.2] THE command payload SHALL contain `command_id` (string), `type` ("lock" or "unlock"), and `doors` (array of strings).
3. [06-REQ-1.3] WHEN publishing to NATS, THE service SHALL include the bearer token as a NATS message header `Authorization: Bearer <token>`.
4. [06-REQ-1.4] THE service SHALL store the command with status `"pending"` in an in-memory map keyed by command ID.

#### Edge Cases

1. [06-REQ-1.E1] IF the `Authorization` header is missing or does not start with `Bearer `, THEN THE service SHALL return HTTP 401 with `{"error":"unauthorized"}`.
2. [06-REQ-1.E2] IF the bearer token is not associated with the VIN in the URL path, THEN THE service SHALL return HTTP 403 with `{"error":"forbidden"}`.
3. [06-REQ-1.E3] IF the request body is not valid JSON or is missing required fields (`command_id`, `type`, `doors`), THEN THE service SHALL return HTTP 400 with `{"error":"invalid command payload"}`.
4. [06-REQ-1.E4] IF the `type` field is not "lock" or "unlock", THEN THE service SHALL return HTTP 400 with `{"error":"invalid command payload"}`.

### Requirement 2: Command Status Query

**User Story:** As a COMPANION_APP, I want to query the status of a submitted command, so that I can know whether my lock/unlock request succeeded.

#### Acceptance Criteria

1. [06-REQ-2.1] WHEN a GET request is made to `/vehicles/{vin}/commands/{command_id}` with a valid bearer token, THE service SHALL return HTTP 200 with the command status JSON including `command_id`, `status`, and optionally `reason`.
2. [06-REQ-2.2] THE service SHALL return status `"pending"` for commands that have been submitted but not yet responded to.
3. [06-REQ-2.3] THE service SHALL return status `"success"` or `"failed"` for commands that have received a response from the vehicle.

#### Edge Cases

1. [06-REQ-2.E1] IF the `command_id` does not exist, THEN THE service SHALL return HTTP 404 with `{"error":"command not found"}`.
2. [06-REQ-2.E2] IF the bearer token is invalid or not authorized for the VIN, THEN THE service SHALL return HTTP 401 or HTTP 403 as appropriate.

### Requirement 3: Command Response Reception

**User Story:** As a CLOUD_GATEWAY operator, I want the service to receive command responses from vehicles via NATS, so that command statuses are updated automatically.

#### Acceptance Criteria

1. [06-REQ-3.1] WHEN the service starts, THE service SHALL subscribe to NATS subject `vehicles.*.command_responses` to receive command responses from all vehicles.
2. [06-REQ-3.2] WHEN a command response is received via NATS, THE service SHALL update the stored command status to the received status (`"success"` or `"failed"`) and optional reason.
3. [06-REQ-3.3] THE service SHALL parse the NATS response payload as JSON containing `command_id`, `status`, and optionally `reason`.

#### Edge Cases

1. [06-REQ-3.E1] IF a NATS response contains invalid JSON, THEN THE service SHALL log the error and discard the message.
2. [06-REQ-3.E2] IF a NATS response references a `command_id` not in the store, THEN THE service SHALL log a warning and discard the message.

### Requirement 4: Command Timeout

**User Story:** As a COMPANION_APP developer, I want commands to time out if the vehicle doesn't respond, so that I don't wait indefinitely.

#### Acceptance Criteria

1. [06-REQ-4.1] WHEN a command has been pending for longer than the configured timeout (default: 30 seconds), THE service SHALL update the command status to `"timeout"`.
2. [06-REQ-4.2] THE timeout duration SHALL be configurable via the configuration file.

### Requirement 5: Telemetry Reception

**User Story:** As a CLOUD_GATEWAY operator, I want the service to receive telemetry from vehicles, so that vehicle state is observable.

#### Acceptance Criteria

1. [06-REQ-5.1] WHEN the service starts, THE service SHALL subscribe to NATS subject `vehicles.*.telemetry` to receive telemetry from all vehicles.
2. [06-REQ-5.2] WHEN telemetry is received via NATS, THE service SHALL log the telemetry data including the VIN extracted from the subject.

#### Edge Cases

1. [06-REQ-5.E1] IF telemetry contains invalid JSON, THEN THE service SHALL log a warning and discard the message.

### Requirement 6: Authentication

**User Story:** As a security-conscious operator, I want all REST requests authenticated with bearer tokens, so that only authorized apps can control vehicles.

#### Acceptance Criteria

1. [06-REQ-6.1] THE service SHALL validate the `Authorization: Bearer <token>` header on every REST request to `/vehicles/{vin}/commands` and `/vehicles/{vin}/commands/{command_id}`.
2. [06-REQ-6.2] THE service SHALL load token-to-VIN mappings from a JSON configuration file at startup.
3. [06-REQ-6.3] THE service SHALL verify that the provided token is authorized for the VIN specified in the URL path.

### Requirement 7: Configuration

**User Story:** As a developer, I want the service to load configuration from a file, so that I can modify tokens, ports, and timeouts without code changes.

#### Acceptance Criteria

1. [06-REQ-7.1] WHEN the service starts, THE service SHALL load configuration from the file path specified by the `CONFIG_PATH` environment variable, defaulting to `config.json` in the working directory.
2. [06-REQ-7.2] THE configuration SHALL include: server port, NATS URL, command timeout, and token-to-VIN mappings.
3. [06-REQ-7.3] THE service SHALL use default values (port 8081, NATS URL `nats://localhost:4222`, timeout 30s) when fields are omitted from the config.

#### Edge Cases

1. [06-REQ-7.E1] IF the configuration file does not exist, THEN THE service SHALL start with built-in default configuration and log a warning.
2. [06-REQ-7.E2] IF the configuration file contains invalid JSON, THEN THE service SHALL exit with a non-zero code and log a descriptive error.

### Requirement 8: NATS Connection

**User Story:** As an operator, I want the service to connect to NATS reliably, so that command routing works consistently.

#### Acceptance Criteria

1. [06-REQ-8.1] WHEN the service starts, THE service SHALL connect to the configured NATS server URL.
2. [06-REQ-8.2] THE service SHALL subscribe to command response and telemetry subjects after connecting.

#### Edge Cases

1. [06-REQ-8.E1] IF the NATS server is unreachable at startup, THEN THE service SHALL retry with exponential backoff (1s, 2s, 4s) up to 5 attempts, then exit non-zero with a descriptive error.

### Requirement 9: Health Check and Lifecycle

**User Story:** As an operator, I want health check and graceful shutdown, so that I can monitor and manage the service.

#### Acceptance Criteria

1. [06-REQ-9.1] WHEN a GET request is made to `/health`, THE service SHALL return HTTP 200 with `{"status":"ok"}`.
2. [06-REQ-9.2] WHEN the service starts, THE service SHALL log its version, configured port, NATS URL, and number of configured tokens.
3. [06-REQ-9.3] WHEN the service receives SIGTERM or SIGINT, THE service SHALL drain the NATS connection, gracefully shut down the HTTP server, and exit with code 0.

### Requirement 10: Response Format

**User Story:** As a COMPANION_APP developer, I want consistent JSON response formats, so that I can reliably parse API responses.

#### Acceptance Criteria

1. [06-REQ-10.1] THE service SHALL set `Content-Type: application/json` on all responses.
2. [06-REQ-10.2] THE error responses SHALL use the format `{"error":"<message>"}`.
