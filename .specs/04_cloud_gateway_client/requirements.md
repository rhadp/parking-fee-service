# Requirements: CLOUD_GATEWAY_CLIENT (Spec 04)

> EARS-syntax requirements for the CLOUD_GATEWAY_CLIENT component.
> Derived from the PRD at `.specs/04_cloud_gateway_client/prd.md` and the master PRD at `.specs/prd.md`.

## Notation

Requirements use EARS (Easy Approach to Requirements Syntax) patterns:

| Pattern | Template |
|---------|----------|
| Ubiquitous | The system SHALL ... |
| Event-driven | WHEN [event], the system SHALL ... |
| State-driven | WHILE [state], the system SHALL ... |
| Conditional | IF [condition], THEN the system SHALL ... |
| Complex | WHEN [event] AND [condition], the system SHALL ... |

## Glossary

| Term | Definition |
|------|------------|
| CLOUD_GATEWAY_CLIENT | Rust service running in the RHIVOS safety partition that bridges DATA_BROKER and CLOUD_GATEWAY via NATS |
| DATA_BROKER | Eclipse Kuksa Databroker instance deployed in the RHIVOS safety partition |
| CLOUD_GATEWAY | Cloud-based service that relays commands and telemetry between COMPANION_APPs and vehicles via NATS |
| NATS | Lightweight messaging system used for vehicle-to-cloud communication |
| VIN | Vehicle Identification Number, used to namespace NATS subjects |
| UDS | Unix Domain Socket, used for same-partition gRPC communication with DATA_BROKER |
| Bearer token | Authentication credential included in command payloads for validation |

## Requirements

### 04-REQ-1.1: NATS Connection and Command Subscription

**User Story:** As a vehicle system, I want the CLOUD_GATEWAY_CLIENT to connect to the NATS server and subscribe to my VIN-specific command subject, so that remote commands can reach the vehicle.

#### Acceptance Criteria

1. WHEN the CLOUD_GATEWAY_CLIENT starts, it SHALL connect to the NATS server at the URL specified by the `NATS_URL` environment variable (default: `nats://localhost:4222`) and subscribe to the subject `vehicles.{VIN}.commands`, where `{VIN}` is configured via the `VIN` environment variable.
2. WHEN TLS is configured (via `NATS_TLS_ENABLED=true`), the CLOUD_GATEWAY_CLIENT SHALL use TLS for the NATS connection. WHEN TLS is not configured, it SHALL use a plain NATS connection.

#### Edge Cases

1. IF the `VIN` environment variable is not set, THEN the CLOUD_GATEWAY_CLIENT SHALL exit with a non-zero exit code and a descriptive error message.
2. IF the NATS server is unreachable at startup, THEN the CLOUD_GATEWAY_CLIENT SHALL retry using the async-nats built-in reconnection mechanism and log each attempt.

---

### 04-REQ-1.2: NATS Reconnection

**User Story:** As a vehicle operator, I want the CLOUD_GATEWAY_CLIENT to automatically recover from NATS disconnections, so that cloud communication resumes without manual intervention.

#### Acceptance Criteria

1. IF the NATS connection is lost during operation, THEN the CLOUD_GATEWAY_CLIENT SHALL attempt to reconnect using the async-nats built-in reconnection mechanism and SHALL resume all subscriptions once reconnected.

#### Edge Cases

1. WHILE the NATS connection is unavailable, commands arriving from other sources SHALL NOT cause the service to crash.

---

### 04-REQ-2.1: Command Validation and DATA_BROKER Write

**User Story:** As a safety system, I want incoming commands to be validated before reaching DATA_BROKER, so that only well-formed commands are processed by the LOCKING_SERVICE.

#### Acceptance Criteria

1. WHEN a message is received on `vehicles.{VIN}.commands`, the CLOUD_GATEWAY_CLIENT SHALL parse the JSON payload and validate that it contains the required fields: `command_id` (UUID string), `action` (`"lock"` or `"unlock"`), `doors` (string array), `source` (string), `vin` (string), and `timestamp` (integer).
2. WHEN a command passes validation, the CLOUD_GATEWAY_CLIENT SHALL write the command JSON to `Vehicle.Command.Door.Lock` on DATA_BROKER via gRPC over UDS, preserving all fields including `command_id`.

#### Edge Cases

1. IF the received JSON cannot be parsed or is missing required fields, THEN the CLOUD_GATEWAY_CLIENT SHALL log a warning with details of the validation failure and discard the message without writing to DATA_BROKER.
2. IF the `action` field is not `"lock"` or `"unlock"`, THEN the CLOUD_GATEWAY_CLIENT SHALL reject the command, log a warning, and discard the message.

---

### 04-REQ-3.1: Command Response Relay

**User Story:** As a COMPANION_APP user, I want the result of my lock/unlock command relayed back through the cloud, so that I know whether it succeeded.

#### Acceptance Criteria

1. WHEN `Vehicle.Command.Door.Response` changes on DATA_BROKER, the CLOUD_GATEWAY_CLIENT SHALL read the response JSON and publish it to the NATS subject `vehicles.{VIN}.command_responses`, preserving the `command_id` and `status` fields.

#### Edge Cases

1. IF the response JSON from DATA_BROKER cannot be parsed, THEN the CLOUD_GATEWAY_CLIENT SHALL log a warning and skip the relay without crashing.

---

### 04-REQ-4.1: Telemetry Publishing

**User Story:** As a fleet operator, I want vehicle state changes published to the cloud, so that the vehicle's current status is visible remotely.

#### Acceptance Criteria

1. WHILE the CLOUD_GATEWAY_CLIENT is connected to both NATS and DATA_BROKER, it SHALL subscribe to the following DATA_BROKER signals: `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked`, `Vehicle.CurrentLocation.Latitude`, `Vehicle.CurrentLocation.Longitude`, and `Vehicle.Parking.SessionActive`.
2. WHEN any subscribed signal value changes, the CLOUD_GATEWAY_CLIENT SHALL publish a JSON telemetry message to `vehicles.{VIN}.telemetry` containing the signal name, value, VIN, and a timestamp.

#### Edge Cases

1. Telemetry SHALL only be published when signal values actually change, not on a periodic schedule.

---

### 04-REQ-5.1: DATA_BROKER Connectivity

**User Story:** As a vehicle system, I want the CLOUD_GATEWAY_CLIENT to connect to DATA_BROKER via gRPC over UDS, so that commands and signals flow within the safety partition.

#### Acceptance Criteria

1. WHEN the CLOUD_GATEWAY_CLIENT starts, it SHALL connect to DATA_BROKER via gRPC over UDS at the socket path specified by the `DATABROKER_UDS_PATH` environment variable (default: `/tmp/kuksa/databroker.sock`).

#### Edge Cases

1. IF DATA_BROKER is unreachable at startup, THEN the CLOUD_GATEWAY_CLIENT SHALL retry the connection with exponential backoff (max 30s) and log each failed attempt.
2. IF DATA_BROKER becomes unreachable during operation, THEN the CLOUD_GATEWAY_CLIENT SHALL detect the broken stream, log an error, and retry the connection and subscriptions.
3. IF a command is received via NATS while DATA_BROKER is unreachable, THEN the command SHALL be logged and discarded (not silently lost).

---

### 04-REQ-6.1: VIN-Based Subject Addressing

**User Story:** As a multi-vehicle deployment, I want each CLOUD_GATEWAY_CLIENT instance to operate only on its own VIN namespace, so that vehicles do not interfere with each other.

#### Acceptance Criteria

1. THE CLOUD_GATEWAY_CLIENT SHALL use the configured VIN to construct all NATS subjects: `vehicles.{VIN}.commands` (subscribe), `vehicles.{VIN}.command_responses` (publish), and `vehicles.{VIN}.telemetry` (publish).
2. THE CLOUD_GATEWAY_CLIENT SHALL NOT subscribe to or publish on NATS subjects belonging to other VINs.

---

### 04-REQ-7.1: Graceful Startup and Shutdown

**User Story:** As an operator, I want the service to start up and shut down cleanly, so that no resources are leaked and connections are properly closed.

#### Acceptance Criteria

1. WHEN the CLOUD_GATEWAY_CLIENT starts, it SHALL establish connections to NATS and DATA_BROKER, spawn concurrent tasks for command processing, response relay, and telemetry publishing, and log a startup confirmation message.
2. WHEN the CLOUD_GATEWAY_CLIENT receives a termination signal (SIGTERM, SIGINT), it SHALL close the NATS connection, close the DATA_BROKER gRPC channel, and exit cleanly.

#### Edge Cases

1. IF any spawned task exits with an error during operation, THEN the CLOUD_GATEWAY_CLIENT SHALL log the error and attempt to restart the failed task.

## Traceability

| Requirement | PRD Section |
|-------------|-------------|
| 04-REQ-1.1 | NATS connectivity, Vehicle Identity, TLS |
| 04-REQ-1.2 | NATS connectivity (resilience) |
| 04-REQ-2.1 | Command reception, Command Payload Format, validation |
| 04-REQ-3.1 | Command response relay, VSS signals |
| 04-REQ-4.1 | Telemetry publishing, VSS signals |
| 04-REQ-5.1 | DATA_BROKER communication via gRPC over UDS |
| 04-REQ-6.1 | VIN-based subject addressing, Vehicle Identity |
| 04-REQ-7.1 | Startup/shutdown behavior |
