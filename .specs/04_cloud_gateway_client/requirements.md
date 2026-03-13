# Requirements Document

## Introduction

This document specifies the requirements for the CLOUD_GATEWAY_CLIENT component (Phase 2.1) of the SDV Parking Demo System. The CLOUD_GATEWAY_CLIENT is a Rust service running in the RHIVOS safety partition that bridges the vehicle's DATA_BROKER with the cloud-based CLOUD_GATEWAY via NATS messaging. It receives and validates lock/unlock commands, forwards them to DATA_BROKER, and relays command responses and vehicle telemetry back to NATS.

## Glossary

- **CLOUD_GATEWAY_CLIENT:** A Rust service bridging DATA_BROKER (vehicle-side) and CLOUD_GATEWAY (cloud-side) via NATS.
- **CLOUD_GATEWAY:** Cloud-based service with REST (towards apps) and NATS (towards vehicles) interfaces.
- **DATA_BROKER:** Eclipse Kuksa Databroker — VSS-compliant vehicle signal broker, accessed via gRPC.
- **NATS:** Lightweight messaging system used for vehicle-to-cloud communication.
- **NATS subject:** A hierarchical topic string for message routing (e.g., `vehicles.{VIN}.commands`).
- **Bearer token:** A simple authentication mechanism where a token string is passed in NATS message headers.
- **VIN:** Vehicle Identification Number — a unique identifier for each vehicle instance.
- **VSS:** Vehicle Signal Specification (COVESA standard) — a taxonomy for vehicle data signals.
- **Telemetry:** Aggregated vehicle state (lock status, location, parking state) published to NATS.
- **Command signal:** A VSS string signal (`Vehicle.Command.Door.Lock`) carrying a JSON-encoded lock/unlock request.
- **Response signal:** A VSS string signal (`Vehicle.Command.Door.Response`) carrying a JSON-encoded command result.
- **Self-registration:** Publishing an online status message to NATS on startup.
- **UDS:** Unix Domain Socket — IPC mechanism for same-partition gRPC communication.
- **async-nats:** The Rust async NATS client crate.

## Requirements

### Requirement 1: NATS Command Subscription

**User Story:** As a cloud gateway, I want the CLOUD_GATEWAY_CLIENT to subscribe to commands on NATS, so that lock/unlock requests from the companion app reach the vehicle.

#### Acceptance Criteria

1. [04-REQ-1.1] WHEN the service starts, THE service SHALL connect to NATS at the address specified by the `NATS_URL` environment variable, defaulting to `nats://localhost:4222` if not set.
2. [04-REQ-1.2] WHEN connected to NATS, THE service SHALL subscribe to the subject `vehicles.{VIN}.commands` where `{VIN}` is the configured vehicle identification number.
3. [04-REQ-1.3] WHEN a message is received on `vehicles.{VIN}.commands`, THE service SHALL extract the bearer token from the `Authorization` NATS message header and validate it against the configured token.

#### Edge Cases

1. [04-REQ-1.E1] IF NATS is unreachable on startup, THEN THE service SHALL retry connection with exponential backoff (1s, 2s, 4s) up to 5 attempts, then exit with a non-zero code.
2. [04-REQ-1.E2] IF the NATS connection is lost after initial connection, THEN THE service SHALL attempt to reconnect up to 3 times before exiting with a non-zero code.

### Requirement 2: Command Validation

**User Story:** As a safety engineer, I want incoming commands to be validated before reaching DATA_BROKER, so that only well-formed, authenticated commands are forwarded.

#### Acceptance Criteria

1. [04-REQ-2.1] WHEN a command message is received, THE service SHALL validate that the `Authorization` header contains a bearer token matching the configured `BEARER_TOKEN` environment variable (default: `demo-token`).
2. [04-REQ-2.2] WHEN a command message passes token validation, THE service SHALL validate that the payload is valid JSON containing `command_id` (non-empty string) and `action` (`"lock"` or `"unlock"`).
3. [04-REQ-2.3] WHEN a command passes all validation, THE service SHALL publish the command JSON to DATA_BROKER by setting the `Vehicle.Command.Door.Lock` signal.

#### Edge Cases

1. [04-REQ-2.E1] IF the bearer token is missing or does not match, THEN THE service SHALL log the rejection and discard the command without forwarding to DATA_BROKER.
2. [04-REQ-2.E2] IF the command payload is not valid JSON, THEN THE service SHALL log the error and discard the command.
3. [04-REQ-2.E3] IF a required field (`command_id` or `action`) is missing or invalid, THEN THE service SHALL log the error and discard the command.

### Requirement 3: Command Response Relay

**User Story:** As a companion app user, I want command responses relayed back to the cloud, so that I know whether my lock/unlock command succeeded.

#### Acceptance Criteria

1. [04-REQ-3.1] WHEN the service starts, THE service SHALL subscribe to the `Vehicle.Command.Door.Response` signal in DATA_BROKER.
2. [04-REQ-3.2] WHEN `Vehicle.Command.Door.Response` changes in DATA_BROKER, THE service SHALL publish the response JSON verbatim to the NATS subject `vehicles.{VIN}.command_responses`.

#### Edge Cases

1. [04-REQ-3.E1] IF publishing the response to NATS fails, THEN THE service SHALL log the error and continue processing.

### Requirement 4: Telemetry Publishing

**User Story:** As a fleet operator, I want the vehicle to publish telemetry to the cloud, so that I can monitor vehicle state remotely.

#### Acceptance Criteria

1. [04-REQ-4.1] WHEN the service starts, THE service SHALL subscribe to DATA_BROKER signals: `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked`, `Vehicle.CurrentLocation.Latitude`, `Vehicle.CurrentLocation.Longitude`, `Vehicle.Parking.SessionActive`.
2. [04-REQ-4.2] WHEN any subscribed telemetry signal changes in DATA_BROKER, THE service SHALL publish an aggregated JSON telemetry message to the NATS subject `vehicles.{VIN}.telemetry`.
3. [04-REQ-4.3] THE telemetry message SHALL contain the VIN, current values of all subscribed signals, and a Unix timestamp.

#### Edge Cases

1. [04-REQ-4.E1] IF a telemetry signal has never been set in DATA_BROKER, THEN THE service SHALL omit that field from the telemetry payload.
2. [04-REQ-4.E2] IF publishing telemetry to NATS fails, THEN THE service SHALL log the error and continue processing.

### Requirement 5: DATA_BROKER Connection

**User Story:** As a vehicle system, I want the CLOUD_GATEWAY_CLIENT to connect to DATA_BROKER, so that it can read/write vehicle signals.

#### Acceptance Criteria

1. [04-REQ-5.1] WHEN the service starts, THE service SHALL connect to DATA_BROKER at the address specified by the `DATABROKER_ADDR` environment variable, defaulting to `http://localhost:55556` if not set.
2. [04-REQ-5.2] WHEN connected to DATA_BROKER, THE service SHALL be able to set string signals and subscribe to signal changes via gRPC.

#### Edge Cases

1. [04-REQ-5.E1] IF DATA_BROKER is unreachable on startup, THEN THE service SHALL retry connection with exponential backoff (1s, 2s, 4s) up to 5 attempts, then exit with a non-zero code.

### Requirement 6: Vehicle Identity and Registration

**User Story:** As a fleet operator, I want each vehicle to register with a unique VIN, so that commands and telemetry are correctly routed.

#### Acceptance Criteria

1. [04-REQ-6.1] THE service SHALL read the vehicle identification number from the `VIN` environment variable.
2. [04-REQ-6.2] WHEN the service starts and connects to NATS, THE service SHALL publish a registration message to `vehicles.{VIN}.status` with `{"vin":"<vin>","status":"online","timestamp":<unix_ts>}`.

#### Edge Cases

1. [04-REQ-6.E1] IF the `VIN` environment variable is not set, THEN THE service SHALL exit with code 1 and log a descriptive error message.

### Requirement 7: Graceful Lifecycle

**User Story:** As an operator, I want the CLOUD_GATEWAY_CLIENT to start and stop cleanly, so that it integrates well with systemd and container orchestration.

#### Acceptance Criteria

1. [04-REQ-7.1] WHEN the service receives SIGTERM or SIGINT, THE service SHALL close NATS and DATA_BROKER connections, log a shutdown message, and exit with code 0.
2. [04-REQ-7.2] WHEN the service starts successfully, THE service SHALL log its version, VIN, NATS URL, DATA_BROKER address, and a ready message.

#### Edge Cases

1. [04-REQ-7.E1] IF a command is being processed when SIGTERM is received, THEN THE service SHALL complete the in-flight command before shutting down.
