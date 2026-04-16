# Requirements Document

## Introduction

This document specifies the requirements for the LOCKING_SERVICE component (Phase 2.1) of the SDV Parking Demo System. The scope covers implementing an ASIL-B rated Rust service that runs in the RHIVOS safety partition. The service subscribes to lock/unlock command signals from DATA_BROKER, validates safety constraints (vehicle speed, door ajar status) before executing commands, writes the resulting lock state to DATA_BROKER, and publishes command responses. Communication with DATA_BROKER uses gRPC over Unix Domain Sockets via the tonic-generated Rust client from kuksa.val.v1 proto definitions.

## Glossary

- **LOCKING_SERVICE:** An ASIL-B rated Rust service responsible for processing lock/unlock commands with safety validation.
- **DATA_BROKER:** Eclipse Kuksa Databroker providing VSS-compliant gRPC pub/sub for vehicle signals.
- **VSS:** Vehicle Signal Specification (COVESA standard) defining a taxonomy for vehicle data signals.
- **UDS:** Unix Domain Socket used for same-partition gRPC communication.
- **Safety constraint:** A condition that must be met before a lock command can be executed (vehicle stationary, door closed).
- **Command signal:** A JSON-encoded lock/unlock request published to `Vehicle.Command.Door.Lock` by CLOUD_GATEWAY_CLIENT.
- **Lock state signal:** The boolean signal `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` reflecting the current lock state.
- **Response signal:** A JSON-encoded command result published to `Vehicle.Command.Door.Response` by LOCKING_SERVICE.
- **Idempotent operation:** Locking an already-locked door or unlocking an already-unlocked door succeeds without changing state.
- **gRPC:** Google Remote Procedure Call protocol used for DATA_BROKER communication.

## Requirements

### Requirement 1: Command Subscription and Processing

**User Story:** As the LOCKING_SERVICE, I want to subscribe to the `Vehicle.Command.Door.Lock` signal from DATA_BROKER, so that I can receive and process remote lock/unlock commands from CLOUD_GATEWAY_CLIENT.

#### Acceptance Criteria

1. [03-REQ-1.1] WHEN the service starts, THE service SHALL subscribe to `Vehicle.Command.Door.Lock` via the kuksa.val.v1 gRPC Subscribe RPC on DATA_BROKER.
2. [03-REQ-1.2] WHEN a new value is published to `Vehicle.Command.Door.Lock`, THE service SHALL receive the JSON payload and begin processing it.
3. [03-REQ-1.3] THE service SHALL process commands sequentially, queuing any commands that arrive while one is being processed.

#### Edge Cases

1. [03-REQ-1.E1] IF the DATA_BROKER is unreachable at startup, THEN THE service SHALL retry connection with exponential backoff and exit with a non-zero code after exhausting retries.
2. [03-REQ-1.E2] IF the subscription stream is interrupted, THEN THE service SHALL attempt to resubscribe up to a maximum number of attempts before exiting.

### Requirement 2: Command Parsing and Validation

**User Story:** As the LOCKING_SERVICE, I want to validate incoming command payloads, so that only well-formed commands with supported parameters are processed.

#### Acceptance Criteria

1. [03-REQ-2.1] WHEN the service receives a command payload, THE service SHALL deserialize it as JSON with required fields: `command_id` (non-empty string), `action` ("lock" or "unlock"), and `doors` (array containing "driver").
2. [03-REQ-2.2] WHEN the `doors` array contains any value other than "driver", THE service SHALL respond with status "failed" and reason "unsupported_door".
3. [03-REQ-2.3] WHEN a required field is missing or invalid, THE service SHALL respond with status "failed" and reason "invalid_command".
4. [03-REQ-2.4] THE fields `source`, `vin`, and `timestamp` SHALL be treated as optional metadata and SHALL NOT affect command processing logic.

#### Edge Cases

1. [03-REQ-2.E1] IF the payload is not valid JSON, THEN THE service SHALL log a warning and discard the payload without publishing a response.
2. [03-REQ-2.E2] IF the payload is valid JSON but missing the `action` field, THEN THE service SHALL respond with reason "invalid_command" if a `command_id` can be extracted.
3. [03-REQ-2.E3] IF the `command_id` field is empty, THEN THE service SHALL respond with reason "invalid_command".

### Requirement 3: Safety Constraint Validation

**User Story:** As the LOCKING_SERVICE, I want to validate safety constraints before executing a lock command, so that the door is not locked while the vehicle is moving or the door is ajar.

#### Acceptance Criteria

1. [03-REQ-3.1] WHEN processing a lock command, THE service SHALL read `Vehicle.Speed` from DATA_BROKER and reject the command with reason "vehicle_moving" IF the speed is >= 1.0 km/h.
2. [03-REQ-3.2] WHEN processing a lock command, THE service SHALL read `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` from DATA_BROKER and reject the command with reason "door_open" IF the value is true.
3. [03-REQ-3.3] WHEN processing a lock command with speed < 1.0 km/h and door not ajar, THE service SHALL allow the command to proceed.
4. [03-REQ-3.4] WHEN processing an unlock command, THE service SHALL NOT check safety constraints and SHALL always allow the command to proceed.

#### Edge Cases

1. [03-REQ-3.E1] IF the `Vehicle.Speed` signal has no value (never set), THEN THE service SHALL treat speed as 0.0 (safe default).
2. [03-REQ-3.E2] IF the `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` signal has no value (never set), THEN THE service SHALL treat the door as closed (safe default).

### Requirement 4: Lock State Management

**User Story:** As the LOCKING_SERVICE, I want to manage and publish the door lock state, so that other services can observe the current lock status via DATA_BROKER.

#### Acceptance Criteria

1. [03-REQ-4.1] WHEN a lock command passes safety validation, THE service SHALL write `true` to `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` on DATA_BROKER.
2. [03-REQ-4.2] WHEN an unlock command is processed, THE service SHALL write `false` to `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` on DATA_BROKER.
3. [03-REQ-4.3] WHEN the service starts, THE service SHALL publish the initial lock state as `false` (unlocked) to DATA_BROKER.

#### Edge Cases

1. [03-REQ-4.E1] IF a lock command is received and the door is already locked, THEN THE service SHALL respond with "success" without writing to the lock state signal.
2. [03-REQ-4.E2] IF an unlock command is received and the door is already unlocked, THEN THE service SHALL respond with "success" without writing to the lock state signal.

### Requirement 5: Command Response Publishing

**User Story:** As the LOCKING_SERVICE, I want to publish command responses to DATA_BROKER, so that CLOUD_GATEWAY_CLIENT can relay results back to the COMPANION_APP.

#### Acceptance Criteria

1. [03-REQ-5.1] WHEN a command succeeds, THE service SHALL publish a JSON response to `Vehicle.Command.Door.Response` with fields: `command_id` (matching the request), `status` ("success"), and `timestamp` (current Unix timestamp in seconds).
2. [03-REQ-5.2] WHEN a command fails, THE service SHALL publish a JSON response to `Vehicle.Command.Door.Response` with fields: `command_id` (matching the request), `status` ("failed"), `reason` (one of "vehicle_moving", "door_open", "unsupported_door", "invalid_command"), and `timestamp` (current Unix timestamp in seconds).
3. [03-REQ-5.3] THE success response SHALL NOT include a `reason` field.

#### Edge Cases

1. [03-REQ-5.E1] IF the response publish to DATA_BROKER fails, THEN THE service SHALL log the error and continue processing subsequent commands.

### Requirement 6: Service Lifecycle

**User Story:** As a system operator, I want the LOCKING_SERVICE to start and stop cleanly, so that it integrates properly with the RHIVOS safety partition lifecycle.

#### Acceptance Criteria

1. [03-REQ-6.1] WHEN the service receives SIGTERM or SIGINT, THE service SHALL shut down gracefully and exit with code 0.
2. [03-REQ-6.2] WHEN the service starts, THE service SHALL log its version and the DATA_BROKER address it is connecting to.

#### Edge Cases

1. [03-REQ-6.E1] IF SIGTERM is received while a command is being processed, THEN THE service SHALL complete the current command before exiting.

### Requirement 7: Configurable DATA_BROKER Endpoint

**User Story:** As a developer, I want the DATA_BROKER address to be configurable, so that the service can connect to different environments (local dev, production).

#### Acceptance Criteria

1. [03-REQ-7.1] THE service SHALL read the DATA_BROKER gRPC address from the `DATABROKER_ADDR` environment variable.
2. [03-REQ-7.2] IF the `DATABROKER_ADDR` environment variable is not set, THEN THE service SHALL use the default address `http://localhost:55556`.
