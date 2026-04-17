# Requirements Document

## Introduction

This document specifies the requirements for the CLOUD_GATEWAY component (Phase 2.2) of the SDV Parking Demo System. The CLOUD_GATEWAY is a cloud-based Go service with two interfaces: a REST API towards COMPANION_APPs for receiving lock/unlock commands and querying command status, and a NATS interface towards vehicles (CLOUD_GATEWAY_CLIENT) for relaying commands and receiving telemetry. It authenticates requests using bearer tokens mapped to VINs via a JSON configuration file, stores command responses in memory, and handles command timeouts.

## Glossary

- **CLOUD_GATEWAY:** A Go service that bridges REST (COMPANION_APP) and NATS (CLOUD_GATEWAY_CLIENT) protocols for vehicle command routing.
- **COMPANION_APP:** A mobile application paired with a specific VIN that sends lock/unlock commands and queries command status via REST.
- **CLOUD_GATEWAY_CLIENT:** A vehicle-side service that receives commands over NATS and publishes responses and telemetry.
- **VIN:** Vehicle Identification Number, used to route commands to the correct vehicle.
- **Bearer token:** A string used for authentication; each token is mapped to a specific VIN.
- **Command:** A lock or unlock request identified by a UUID (`command_id`), submitted by COMPANION_APP.
- **Command response:** A success, failed, or timeout result for a previously submitted command, received from CLOUD_GATEWAY_CLIENT via NATS or generated on timeout.
- **NATS:** A messaging system used for publish/subscribe communication between CLOUD_GATEWAY and vehicles.
- **Exponential backoff:** A retry strategy where wait times increase geometrically (1s, 2s, 4s) between connection attempts.
- **Command timeout:** A configurable duration (default 30s) after which a command with no response is marked as `"timeout"`.

## Requirements

### Requirement 1: Command Submission

**User Story:** As a COMPANION_APP, I want to submit lock/unlock commands for my vehicle so that I can remotely control the door locks.

#### Acceptance Criteria

1. [06-REQ-1.1] WHEN a POST request is made to `/vehicles/{vin}/commands` with a valid bearer token and a JSON body containing `command_id` (UUID), `type` ("lock" or "unlock"), and `doors` (string array), THE service SHALL publish the command to the NATS subject `vehicles.{vin}.commands` and return HTTP 202 with the command body echoed back.
2. [06-REQ-1.2] WHEN publishing a command to NATS, THE service SHALL include the bearer token as a NATS message header `Authorization: Bearer <token>`.
3. [06-REQ-1.3] WHEN a command is submitted, THE service SHALL start a timeout timer for the configured timeout duration (default 30 seconds) and, IF no response is received within that duration, SHALL set the command status to `{"command_id":"<id>","status":"timeout"}`.

#### Edge Cases

1. [06-REQ-1.E1] IF the request body is missing `command_id`, `type`, or `doors`, THEN THE service SHALL return HTTP 400 with `{"error":"invalid command payload"}`.
2. [06-REQ-1.E2] IF the `type` field is not "lock" or "unlock", THEN THE service SHALL return HTTP 400 with `{"error":"invalid command type"}`.

### Requirement 2: Command Status Query

**User Story:** As a COMPANION_APP, I want to query the status of a previously submitted command so that I can know whether the vehicle executed it.

#### Acceptance Criteria

1. [06-REQ-2.1] WHEN a GET request is made to `/vehicles/{vin}/commands/{command_id}` with a valid bearer token, THE service SHALL return HTTP 200 with the stored command response JSON containing `command_id` and `status` (and optional `reason`).
2. [06-REQ-2.2] THE service SHALL store command responses received from NATS subject `vehicles.{vin}.command_responses` in an in-memory map keyed by `command_id`, protected by a mutex.

#### Edge Cases

1. [06-REQ-2.E1] IF the `command_id` does not exist in the response store, THEN THE service SHALL return HTTP 404 with `{"error":"command not found"}`.

### Requirement 3: Authentication and Authorization

**User Story:** As a system operator, I want the gateway to authenticate and authorize requests so that only valid COMPANION_APPs can control their paired vehicles.

#### Acceptance Criteria

1. [06-REQ-3.1] WHEN a REST request is received, THE service SHALL extract the bearer token from the `Authorization` header and validate it against the configured token-VIN mappings.
2. [06-REQ-3.2] WHEN a valid token is presented but the token's VIN does not match the `{vin}` in the URL path, THE service SHALL return HTTP 403 with `{"error":"forbidden"}`.

#### Edge Cases

1. [06-REQ-3.E1] IF the `Authorization` header is missing or the token is not in the configuration, THEN THE service SHALL return HTTP 401 with `{"error":"unauthorized"}`.

### Requirement 4: Health Check

**User Story:** As an operator, I want a health check endpoint so that I can monitor service availability.

#### Acceptance Criteria

1. [06-REQ-4.1] WHEN a GET request is made to `/health`, THE service SHALL return HTTP 200 with `{"status":"ok"}`.

### Requirement 5: NATS Connectivity

**User Story:** As a system operator, I want the gateway to maintain a reliable NATS connection so that commands and responses flow between the cloud and vehicles.

#### Acceptance Criteria

1. [06-REQ-5.1] WHEN the service starts, THE service SHALL connect to the NATS server and subscribe to `vehicles.*.command_responses` and `vehicles.*.telemetry` subjects.
2. [06-REQ-5.2] WHEN a command response is received on NATS subject `vehicles.{vin}.command_responses`, THE service SHALL parse the JSON payload and store it in the in-memory response map keyed by `command_id`.
3. [06-REQ-5.3] WHEN telemetry is received on NATS subject `vehicles.{vin}.telemetry`, THE service SHALL log the telemetry data. No storage or aggregation SHALL be performed.

#### Edge Cases

1. [06-REQ-5.E1] IF the NATS server is unreachable at startup, THE service SHALL retry with exponential backoff (1s, 2s, 4s) up to 5 attempts, THEN exit with a non-zero code.
2. [06-REQ-5.E2] IF the NATS connection is lost at runtime, THE service SHALL rely on the nats.go client's automatic reconnection.

### Requirement 6: Configuration

**User Story:** As a developer, I want the service to load token-VIN mappings and settings from a JSON config file so that I can modify demo data without code changes.

#### Acceptance Criteria

1. [06-REQ-6.1] WHEN the service starts, THE service SHALL load configuration from the file path specified by the `CONFIG_PATH` environment variable, defaulting to `config.json` in the working directory.
2. [06-REQ-6.2] THE configuration SHALL include: NATS server URL, command timeout duration, server port, and a list of token-VIN pairs.
3. [06-REQ-6.3] THE service SHALL use the configured command timeout for all timeout timers.

#### Edge Cases

1. [06-REQ-6.E1] IF the configuration file does not exist or contains invalid JSON, THEN THE service SHALL exit with a non-zero code and log a descriptive error.

### Requirement 7: Response Format

**User Story:** As a COMPANION_APP developer, I want consistent JSON response formats so that I can reliably parse API responses.

#### Acceptance Criteria

1. [06-REQ-7.1] THE service SHALL set `Content-Type: application/json` on all REST responses.
2. [06-REQ-7.2] THE error responses SHALL use the format `{"error":"<message>"}`.

### Requirement 8: Graceful Lifecycle

**User Story:** As an operator, I want the service to start and stop cleanly.

#### Acceptance Criteria

1. [06-REQ-8.1] WHEN the service starts, THE service SHALL log its version, configured port, NATS URL, number of configured tokens, and a ready message.
2. [06-REQ-8.2] WHEN the service receives SIGTERM or SIGINT, THE service SHALL drain the NATS connection, gracefully shut down the HTTP server, and exit with code 0.
