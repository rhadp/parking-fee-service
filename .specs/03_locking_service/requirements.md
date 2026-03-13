# Requirements Document

## Introduction

This document specifies the requirements for the LOCKING_SERVICE component (Phase 2.1) of the SDV Parking Demo System. The LOCKING_SERVICE is an ASIL-B rated Rust service running in the RHIVOS safety partition that processes remote lock/unlock commands, enforces safety constraints, and publishes lock state and command responses to DATA_BROKER via gRPC.

## Glossary

- **LOCKING_SERVICE:** An ASIL-B rated Rust service that manages the vehicle's driver-side door lock, enforcing safety constraints before executing lock/unlock commands.
- **DATA_BROKER:** Eclipse Kuksa Databroker — a VSS-compliant vehicle signal broker. The LOCKING_SERVICE communicates with it via gRPC.
- **VSS:** Vehicle Signal Specification (COVESA standard) — a taxonomy for vehicle data signals.
- **ASIL-B:** Automotive Safety Integrity Level B — an ISO 26262 classification for safety-relevant components.
- **UDS:** Unix Domain Socket — IPC mechanism used for same-partition gRPC communication.
- **Command signal:** A VSS string signal (`Vehicle.Command.Door.Lock`) carrying a JSON-encoded lock/unlock request.
- **Response signal:** A VSS string signal (`Vehicle.Command.Door.Response`) carrying a JSON-encoded command result.
- **Lock state signal:** A VSS boolean signal (`Vehicle.Cabin.Door.Row1.DriverSide.IsLocked`) representing the current lock state.
- **Safety constraint:** A precondition (vehicle stationary, door closed) that must be satisfied before a lock command can execute.
- **Stationary:** Vehicle speed below 1.0 km/h, as reported by `Vehicle.Speed`.
- **Door ajar:** Door physically open, as reported by `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen == true`.
- **Command payload:** JSON object containing `command_id`, `action`, `doors`, and optional fields (`source`, `vin`, `timestamp`).
- **tonic:** A Rust gRPC framework used to generate the Kuksa Databroker client from proto definitions.

## Requirements

### Requirement 1: Command Subscription

**User Story:** As a vehicle system, I want the LOCKING_SERVICE to subscribe to lock/unlock commands from DATA_BROKER, so that remote commands from the companion app are received and processed.

#### Acceptance Criteria

1. [03-REQ-1.1] WHEN the LOCKING_SERVICE starts, THE service SHALL subscribe to the `Vehicle.Command.Door.Lock` signal on DATA_BROKER via gRPC.
2. [03-REQ-1.2] WHEN a new value is published to `Vehicle.Command.Door.Lock`, THE service SHALL deserialize the JSON payload and begin command processing.
3. [03-REQ-1.3] THE service SHALL connect to DATA_BROKER at the address specified by the `DATABROKER_ADDR` environment variable, defaulting to `http://localhost:55556` if not set.

#### Edge Cases

1. [03-REQ-1.E1] IF the DATA_BROKER is unreachable on startup, THEN THE service SHALL retry connection with exponential backoff (1s, 2s, 4s) up to 5 attempts, then exit with a non-zero code.
2. [03-REQ-1.E2] IF the subscription stream is interrupted after initial connection, THEN THE service SHALL attempt to resubscribe up to 3 times before exiting with a non-zero code.

### Requirement 2: Command Payload Validation

**User Story:** As a safety engineer, I want invalid commands to be rejected with clear error reasons, so that only well-formed commands reach the safety logic.

#### Acceptance Criteria

1. [03-REQ-2.1] WHEN a command payload is received, THE service SHALL validate that `command_id` is a non-empty string.
2. [03-REQ-2.2] WHEN a command payload is received, THE service SHALL validate that `action` is either `"lock"` or `"unlock"`.
3. [03-REQ-2.3] WHEN a command payload is received, THE service SHALL validate that `doors` is an array containing `"driver"`.

#### Edge Cases

1. [03-REQ-2.E1] IF the command payload is not valid JSON, THEN THE service SHALL log the error and discard the command without publishing a response.
2. [03-REQ-2.E2] IF any required field (`command_id`, `action`, `doors`) is missing or invalid, THEN THE service SHALL publish a failure response with reason `"invalid_command"`.
3. [03-REQ-2.E3] IF the `doors` array contains a value other than `"driver"`, THEN THE service SHALL publish a failure response with reason `"unsupported_door"`.

### Requirement 3: Safety Constraint Validation

**User Story:** As a safety engineer, I want the LOCKING_SERVICE to check vehicle speed and door state before locking, so that lock commands are only executed when safe.

#### Acceptance Criteria

1. [03-REQ-3.1] WHEN a valid lock command is received AND `Vehicle.Speed` >= 1.0, THEN THE service SHALL publish a failure response with reason `"vehicle_moving"`.
2. [03-REQ-3.2] WHEN a valid lock command is received AND `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` is `true`, THEN THE service SHALL publish a failure response with reason `"door_open"`.
3. [03-REQ-3.3] WHEN a valid lock command is received AND `Vehicle.Speed` < 1.0 AND `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` is `false`, THE service SHALL proceed to execute the lock.
4. [03-REQ-3.4] WHEN a valid unlock command is received, THE service SHALL execute the unlock without checking speed or door state.

#### Edge Cases

1. [03-REQ-3.E1] IF `Vehicle.Speed` has never been set (no value in DATA_BROKER), THEN THE service SHALL treat the speed as 0.0 (stationary) and allow the lock.
2. [03-REQ-3.E2] IF `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` has never been set, THEN THE service SHALL treat the door as closed (false) and allow the lock.

### Requirement 4: Lock State Management

**User Story:** As a vehicle system, I want the LOCKING_SERVICE to update the lock state in DATA_BROKER, so that other services can observe the current door lock status.

#### Acceptance Criteria

1. [03-REQ-4.1] WHEN a lock command passes safety validation, THE service SHALL set `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` to `true` in DATA_BROKER.
2. [03-REQ-4.2] WHEN an unlock command is executed, THE service SHALL set `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` to `false` in DATA_BROKER.
3. [03-REQ-4.3] WHEN the service starts, THE service SHALL publish initial lock state `false` (unlocked) to DATA_BROKER.

#### Edge Cases

1. [03-REQ-4.E1] WHEN a lock command is received AND the door is already locked, THEN THE service SHALL publish a success response without changing state (idempotent).
2. [03-REQ-4.E2] WHEN an unlock command is received AND the door is already unlocked, THEN THE service SHALL publish a success response without changing state (idempotent).

### Requirement 5: Command Response Publishing

**User Story:** As a cloud gateway, I want to receive command execution results from DATA_BROKER, so that I can relay status back to the companion app.

#### Acceptance Criteria

1. [03-REQ-5.1] WHEN a command is executed successfully, THE service SHALL publish a JSON response to `Vehicle.Command.Door.Response` with `status` set to `"success"` and the original `command_id`.
2. [03-REQ-5.2] WHEN a command fails validation or safety checks, THE service SHALL publish a JSON response to `Vehicle.Command.Door.Response` with `status` set to `"failed"`, the original `command_id`, and a `reason` field.
3. [03-REQ-5.3] THE response payload SHALL include a `timestamp` field set to the current Unix timestamp in seconds.

#### Edge Cases

1. [03-REQ-5.E1] IF publishing the response to DATA_BROKER fails, THEN THE service SHALL log the error and continue processing subsequent commands.

### Requirement 6: Graceful Lifecycle

**User Story:** As an operator, I want the LOCKING_SERVICE to start and stop cleanly, so that it integrates well with systemd and container orchestration.

#### Acceptance Criteria

1. [03-REQ-6.1] WHEN the service receives SIGTERM or SIGINT, THE service SHALL cancel the command subscription, log a shutdown message, and exit with code 0.
2. [03-REQ-6.2] WHEN the service starts successfully, THE service SHALL log its version, DATA_BROKER address, and a ready message.

#### Edge Cases

1. [03-REQ-6.E1] IF a command is being processed when SIGTERM is received, THEN THE service SHALL complete the in-flight command before shutting down.
