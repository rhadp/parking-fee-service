# Requirements: LOCKING_SERVICE (Spec 03)

## Introduction

This document specifies the requirements for the LOCKING_SERVICE, an ASIL-B rated Rust service running in the RHIVOS safety partition. The service receives remote lock/unlock commands via DATA_BROKER, validates safety constraints, executes the lock/unlock operation, and reports the result. Communication with DATA_BROKER uses gRPC over Unix Domain Sockets.

## Glossary

| Term | Definition |
|------|-----------|
| LOCKING_SERVICE | ASIL-B Rust service responsible for executing door lock/unlock commands in the RHIVOS safety partition |
| DATA_BROKER | Eclipse Kuksa Databroker instance deployed in the RHIVOS safety partition |
| VSS | COVESA Vehicle Signal Specification, version 5.1 |
| UDS | Unix Domain Socket, used for same-partition gRPC communication |
| Command signal | A JSON string written to `Vehicle.Command.Door.Lock` in DATA_BROKER |
| Response signal | A JSON string written to `Vehicle.Command.Door.Response` in DATA_BROKER |
| Safety constraint | A condition that must be satisfied before a lock/unlock command can be executed |

## Requirements

### Requirement 1: Startup and DATA_BROKER Connection

**User Story:** As a vehicle platform engineer, I want the LOCKING_SERVICE to connect to DATA_BROKER via UDS at startup and subscribe to command signals, so that the service is ready to process remote lock/unlock requests.

#### Acceptance Criteria

1. **03-REQ-1.1** WHEN the LOCKING_SERVICE starts, THE LOCKING_SERVICE SHALL connect to DATA_BROKER via gRPC over Unix Domain Socket at a configurable path and subscribe to `Vehicle.Command.Door.Lock` signals.
2. **03-REQ-1.2** IF the DATA_BROKER is unreachable at startup, THEN THE LOCKING_SERVICE SHALL retry connection with exponential backoff (1s, 2s, 4s, ..., max 30s) until a connection is established.

### Requirement 2: Command JSON Validation

**User Story:** As a safety engineer, I want the LOCKING_SERVICE to validate the structure of incoming command payloads, so that only well-formed commands are processed.

#### Acceptance Criteria

1. **03-REQ-2.1** WHEN the LOCKING_SERVICE receives a signal update on `Vehicle.Command.Door.Lock`, THE LOCKING_SERVICE SHALL parse the JSON payload and validate that all required fields are present: `command_id`, `action`, `doors`, `source`, `vin`, `timestamp`.
2. **03-REQ-2.2** IF the command JSON is malformed or missing required fields, THEN THE LOCKING_SERVICE SHALL discard the command, log a warning, and write a failure response to `Vehicle.Command.Door.Response` with reason `"invalid_command"`.

### Requirement 3: Safety Constraint Validation

**User Story:** As a safety engineer, I want the LOCKING_SERVICE to check that the vehicle is stationary and the door is not ajar before executing a lock/unlock command, so that unsafe operations are prevented.

#### Acceptance Criteria

1. **03-REQ-3.1** BEFORE executing a lock or unlock command, THE LOCKING_SERVICE SHALL read `Vehicle.Speed` from DATA_BROKER and verify that the vehicle speed is zero or near-zero (below 1.0 km/h).
2. **03-REQ-3.2** BEFORE executing a lock or unlock command, THE LOCKING_SERVICE SHALL read `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` from DATA_BROKER and verify that the door is not ajar (value is `false`).
3. **03-REQ-3.3** IF any safety constraint is violated, THEN THE LOCKING_SERVICE SHALL reject the command and write a failure response to `Vehicle.Command.Door.Response` with the appropriate reason (`"vehicle_moving"` or `"door_ajar"`).

### Requirement 4: Lock/Unlock Execution

**User Story:** As a vehicle user, I want the LOCKING_SERVICE to execute valid lock/unlock commands and update the vehicle state, so that the door lock state reflects the requested action.

#### Acceptance Criteria

1. **03-REQ-4.1** WHEN a command with `action` `"lock"` passes all validation and safety checks, THE LOCKING_SERVICE SHALL write `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = true` to DATA_BROKER.
2. **03-REQ-4.2** WHEN a command with `action` `"unlock"` passes all validation and safety checks, THE LOCKING_SERVICE SHALL write `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = false` to DATA_BROKER.

### Requirement 5: Command Response

**User Story:** As a system integrator, I want the LOCKING_SERVICE to always write a response for every command received, so that upstream components can track command outcomes.

#### Acceptance Criteria

1. **03-REQ-5.1** AFTER successfully executing a lock or unlock command, THE LOCKING_SERVICE SHALL write a success response to `Vehicle.Command.Door.Response` containing `command_id`, `status` (`"success"`), and `timestamp`.
2. **03-REQ-5.2** AFTER rejecting a command due to validation failure or safety constraint violation, THE LOCKING_SERVICE SHALL write a failure response to `Vehicle.Command.Door.Response` containing `command_id`, `status` (`"failed"`), `reason`, and `timestamp`.

### Requirement 6: Invalid Action Handling

**User Story:** As a safety engineer, I want the LOCKING_SERVICE to reject commands with unrecognized action values, so that only known operations are executed.

#### Acceptance Criteria

1. **03-REQ-6.1** IF the `action` field in a command is not `"lock"` or `"unlock"`, THEN THE LOCKING_SERVICE SHALL discard the command and write a failure response to `Vehicle.Command.Door.Response` with reason `"invalid_action"`.

### Requirement 7: Graceful Shutdown

**User Story:** As a platform engineer, I want the LOCKING_SERVICE to disconnect cleanly from DATA_BROKER on shutdown, so that resources are properly released.

#### Acceptance Criteria

1. **03-REQ-7.1** WHEN the LOCKING_SERVICE receives a termination signal (SIGTERM or SIGINT), THE LOCKING_SERVICE SHALL cancel active subscriptions, close the gRPC connection to DATA_BROKER, and exit with code 0.

### Requirement 8: DATA_BROKER Connection Recovery

**User Story:** As a platform engineer, I want the LOCKING_SERVICE to recover from DATA_BROKER connection losses during operation, so that the service remains available after transient failures.

#### Acceptance Criteria

1. **03-REQ-8.1** IF the DATA_BROKER connection is lost during operation, THEN THE LOCKING_SERVICE SHALL detect the broken connection, log an error, and retry with exponential backoff until the connection is re-established and subscriptions are restored.
