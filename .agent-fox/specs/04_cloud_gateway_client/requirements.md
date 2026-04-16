# Requirements: CLOUD_GATEWAY_CLIENT

## Introduction

This document specifies the requirements for the CLOUD_GATEWAY_CLIENT component, a Rust service running in the RHIVOS safety partition. The service bridges the vehicle's DATA_BROKER (Eclipse Kuksa Databroker) with the cloud-based CLOUD_GATEWAY via NATS messaging. It receives authenticated lock/unlock commands, validates them, writes them to DATA_BROKER as command signals, and relays command responses and vehicle telemetry back to the cloud.

Requirements use EARS (Easy Approach to Requirements Syntax) notation. Every acceptance criterion contains the keyword SHALL. Requirement IDs follow the pattern `[04-REQ-N.C]` for acceptance criteria and `[04-REQ-N.EN]` for edge cases.

## Glossary

| Term | Definition |
|------|-----------|
| NATS | Lightweight messaging system used for vehicle-to-cloud communication |
| DATA_BROKER | Eclipse Kuksa Databroker providing VSS-compliant gRPC pub/sub for vehicle signals |
| VIN | Vehicle Identification Number, used to scope NATS subjects |
| VSS | Vehicle Signal Specification (COVESA), version 5.1 |
| UDS | Unix Domain Socket, used for same-partition gRPC communication |
| Bearer token | Simple authentication token passed in NATS message headers |
| Telemetry | Aggregated vehicle state (lock status, location, parking state) published to NATS |
| Command signal | JSON-encoded lock/unlock request written to DATA_BROKER (`Vehicle.Command.Door.Lock`) |
| Command response | JSON-encoded result from LOCKING_SERVICE observed via DATA_BROKER (`Vehicle.Command.Door.Response`) |

## Requirements

### REQ-1: Service Configuration

**User Story:** As a platform operator, I want the service to be configurable via environment variables so that I can deploy it in different environments without code changes.

**Acceptance Criteria:**

- `[04-REQ-1.1]` WHEN the `VIN` environment variable is set, the system SHALL use its value as the vehicle identifier in all NATS subject paths.
- `[04-REQ-1.2]` WHEN the `NATS_URL` environment variable is set, the system SHALL connect to NATS at the specified URL. WHEN `NATS_URL` is not set, the system SHALL default to `nats://localhost:4222`.
- `[04-REQ-1.3]` WHEN the `DATABROKER_ADDR` environment variable is set, the system SHALL connect to DATA_BROKER at the specified address. WHEN `DATABROKER_ADDR` is not set, the system SHALL default to `http://localhost:55556`.
- `[04-REQ-1.4]` WHEN the `BEARER_TOKEN` environment variable is set, the system SHALL use its value for command authentication. WHEN `BEARER_TOKEN` is not set, the system SHALL default to `demo-token`.

**Edge Cases:**

- `[04-REQ-1.E1]` WHEN the `VIN` environment variable is not set, the system SHALL exit with code 1 and log a descriptive error message.

---

### REQ-2: NATS Connection

**User Story:** As a platform operator, I want the service to establish and maintain a connection to the NATS server with automatic reconnection so that transient network issues do not require manual intervention.

**Acceptance Criteria:**

- `[04-REQ-2.1]` WHEN the service starts, the system SHALL connect to the NATS server at the configured URL.
- `[04-REQ-2.2]` WHEN the initial NATS connection fails, the system SHALL retry with exponential backoff (1s, 2s, 4s) for up to 5 attempts.
- `[04-REQ-2.3]` WHEN connected to NATS, the system SHALL subscribe to the subject `vehicles.{VIN}.commands` to receive incoming commands.

**Edge Cases:**

- `[04-REQ-2.E1]` WHEN all 5 NATS connection retry attempts are exhausted, the system SHALL exit with code 1 and log an error message indicating the NATS server is unreachable.

---

### REQ-3: DATA_BROKER Connection

**User Story:** As a platform operator, I want the service to connect to the DATA_BROKER via gRPC so that it can read vehicle state and write command signals.

**Acceptance Criteria:**

- `[04-REQ-3.1]` WHEN the service starts, the system SHALL establish a gRPC connection to the DATA_BROKER at the configured address.
- `[04-REQ-3.2]` WHEN connected to DATA_BROKER, the system SHALL subscribe to the following VSS signals for telemetry: `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked`, `Vehicle.CurrentLocation.Latitude`, `Vehicle.CurrentLocation.Longitude`, `Vehicle.Parking.SessionActive`.
- `[04-REQ-3.3]` WHEN connected to DATA_BROKER, the system SHALL subscribe to `Vehicle.Command.Door.Response` to observe command results from LOCKING_SERVICE.

**Edge Cases:**

- `[04-REQ-3.E1]` WHEN the DATA_BROKER connection cannot be established at startup, the system SHALL exit with code 1 and log an error message.

---

### REQ-4: Self-Registration

**User Story:** As a cloud operator, I want each vehicle to announce itself on startup so that the CLOUD_GATEWAY knows which vehicles are online.

**Acceptance Criteria:**

- `[04-REQ-4.1]` WHEN the service has connected to NATS, the system SHALL publish a registration message to `vehicles.{VIN}.status` with the payload `{"vin":"<vin>","status":"online","timestamp":<unix_ts>}`.
- `[04-REQ-4.2]` The registration message SHALL be fire-and-forget; the system SHALL NOT wait for an acknowledgment.

---

### REQ-5: Command Authentication

**User Story:** As a security engineer, I want commands to be validated against a bearer token so that only authorized sources can issue lock/unlock commands.

**Acceptance Criteria:**

- `[04-REQ-5.1]` WHEN a message is received on `vehicles.{VIN}.commands`, the system SHALL extract the `Authorization` header from the NATS message headers.
- `[04-REQ-5.2]` WHEN the `Authorization` header is present and its value matches `Bearer <configured_token>`, the system SHALL proceed with command validation.

**Edge Cases:**

- `[04-REQ-5.E1]` WHEN the `Authorization` header is missing, the system SHALL discard the message, log a warning, and SHALL NOT publish any response to DATA_BROKER.
- `[04-REQ-5.E2]` WHEN the `Authorization` header is present but the token does not match the configured value, the system SHALL discard the message, log a warning, and SHALL NOT publish any response to DATA_BROKER.

---

### REQ-6: Command Validation

**User Story:** As a system architect, I want incoming commands to be structurally validated before being forwarded to DATA_BROKER so that only well-formed commands reach LOCKING_SERVICE.

**Acceptance Criteria:**

- `[04-REQ-6.1]` WHEN a command passes bearer token authentication, the system SHALL validate the payload is valid JSON.
- `[04-REQ-6.2]` WHEN the payload is valid JSON, the system SHALL validate the presence of: `command_id` (non-empty string), `action` (one of `"lock"` or `"unlock"`), and `doors` (array).
- `[04-REQ-6.3]` WHEN the command passes all validation checks, the system SHALL write the command payload as-is to `Vehicle.Command.Door.Lock` in DATA_BROKER via gRPC SetRequest.
- `[04-REQ-6.4]` The system SHALL NOT validate individual door values in the `doors` array; that responsibility belongs to LOCKING_SERVICE.

**Edge Cases:**

- `[04-REQ-6.E1]` WHEN the payload is not valid JSON, the system SHALL discard the message and log a warning. The system SHALL NOT publish any response to DATA_BROKER.
- `[04-REQ-6.E2]` WHEN a required field (`command_id`, `action`, `doors`) is missing or has an invalid type, the system SHALL discard the message and log a warning. The system SHALL NOT publish any response to DATA_BROKER.
- `[04-REQ-6.E3]` WHEN `action` is present but is not `"lock"` or `"unlock"`, the system SHALL discard the message and log a warning.

---

### REQ-7: Command Response Relay

**User Story:** As a companion app user, I want to receive the result of my lock/unlock command so that I know whether it succeeded.

**Acceptance Criteria:**

- `[04-REQ-7.1]` WHEN `Vehicle.Command.Door.Response` changes in DATA_BROKER, the system SHALL read the JSON value and publish it verbatim to `vehicles.{VIN}.command_responses` on NATS.
- `[04-REQ-7.2]` The response payload published to NATS SHALL contain `command_id`, `status`, and `timestamp` fields as received from DATA_BROKER.

**Edge Cases:**

- `[04-REQ-7.E1]` WHEN the response value from DATA_BROKER is not valid JSON, the system SHALL log an error and SHALL NOT publish to NATS.

---

### REQ-8: Telemetry Publishing

**User Story:** As a fleet operator, I want the cloud to receive updated vehicle telemetry whenever relevant state changes so that I have near-real-time visibility into the fleet.

**Acceptance Criteria:**

- `[04-REQ-8.1]` WHEN any subscribed DATA_BROKER signal changes (IsLocked, Latitude, Longitude, SessionActive), the system SHALL publish an aggregated telemetry message to `vehicles.{VIN}.telemetry` on NATS.
- `[04-REQ-8.2]` The telemetry payload SHALL be JSON with the format: `{"vin":"<vin>","is_locked":bool,"latitude":double,"longitude":double,"parking_active":bool,"timestamp":<unix_ts>}`.
- `[04-REQ-8.3]` WHEN a signal has never been set in DATA_BROKER, the corresponding field SHALL be omitted from the telemetry payload.

---

### REQ-9: Graceful Startup Sequencing

**User Story:** As a platform operator, I want the service to start up in a deterministic order so that all connections are established before processing begins.

**Acceptance Criteria:**

- `[04-REQ-9.1]` WHEN the service starts, the system SHALL perform initialization in this order: (1) read and validate environment variables, (2) connect to NATS, (3) connect to DATA_BROKER, (4) publish self-registration, (5) begin processing commands and telemetry.
- `[04-REQ-9.2]` WHEN any step in the startup sequence fails, the system SHALL log the failure and exit with code 1 without proceeding to subsequent steps.

---

### REQ-10: Logging and Observability

**User Story:** As a developer, I want structured logging so that I can diagnose issues in development and production.

**Acceptance Criteria:**

- `[04-REQ-10.1]` The system SHALL use the `tracing` crate for structured logging.
- `[04-REQ-10.2]` The system SHALL log at INFO level: successful NATS connection, successful DATA_BROKER connection, self-registration published, each validated command forwarded, each command response relayed, each telemetry message published.
- `[04-REQ-10.3]` The system SHALL log at WARN level: authentication failures, validation failures, discarded messages.
- `[04-REQ-10.4]` The system SHALL log at ERROR level: connection failures, unexpected DATA_BROKER errors, NATS publish failures.
