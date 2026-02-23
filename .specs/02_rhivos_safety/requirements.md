# Requirements Document: RHIVOS Safety Partition (Phase 2.1)

## Introduction

This document specifies the requirements for the RHIVOS safety-partition
services of the SDV Parking Demo System. The safety partition hosts ASIL-B
components: DATA_BROKER (Eclipse Kuksa Databroker), LOCKING_SERVICE,
CLOUD_GATEWAY_CLIENT, and mock sensor CLI tools. These services form the
foundation for all vehicle-side functionality in the demo.

## Glossary

| Term | Definition |
|------|-----------|
| DATA_BROKER | Eclipse Kuksa Databroker — a pre-built VSS-compliant gRPC signal broker. Deployed as-is with custom configuration. |
| VSS | Vehicle Signal Specification (COVESA, version 5.1) — a standardized taxonomy for vehicle signals. |
| VSS overlay | A JSON or YAML file that extends the standard VSS tree with custom signal definitions. |
| UDS | Unix Domain Socket — local IPC transport for same-partition gRPC communication. |
| Safety constraint | A precondition that must be satisfied before a lock/unlock command can be executed (e.g., vehicle stationary, door closed). |
| Bearer token | A simple authentication credential passed in the MQTT payload or gRPC metadata. Demo-scope only. |
| Command signal | A custom VSS signal carrying a structured JSON command (e.g., Vehicle.Command.Door.Lock). |
| Telemetry | Vehicle state data published to the cloud at intervals or on change. |

## Requirements

### Requirement 1: DATA_BROKER VSS Configuration

**User Story:** As a vehicle service developer, I want DATA_BROKER to serve
all required standard and custom VSS signals, so that safety-partition
services can read and write vehicle state and command signals.

#### Acceptance Criteria

1. THE DATA_BROKER configuration SHALL include a VSS overlay file that defines
   custom signals Vehicle.Command.Door.Lock (string) and
   Vehicle.Command.Door.Response (string). `02-REQ-1.1`
2. THE DATA_BROKER configuration SHALL support standard VSS signals:
   Vehicle.Cabin.Door.Row1.DriverSide.IsLocked (bool),
   Vehicle.Cabin.Door.Row1.DriverSide.IsOpen (bool),
   Vehicle.CurrentLocation.Latitude (double),
   Vehicle.CurrentLocation.Longitude (double), and
   Vehicle.Speed (float). `02-REQ-1.2`
3. WHEN DATA_BROKER starts, THE gRPC interface SHALL be reachable on a Unix
   Domain Socket path for same-partition consumers. `02-REQ-1.3`
4. WHEN DATA_BROKER starts, THE gRPC interface SHALL be reachable on a network
   TCP port for cross-partition and cross-domain consumers. `02-REQ-1.4`
5. THE DATA_BROKER configuration SHALL enforce write-access control via bearer
   tokens so that only authorized services can write to specific signals.
   `02-REQ-1.5`

#### Edge Cases

1. IF DATA_BROKER receives a write request for a signal that does not exist in
   the VSS tree, THEN it SHALL return an error indicating the signal is unknown.
   `02-REQ-1.E1`
2. IF DATA_BROKER receives a write request without a valid bearer token, THEN
   it SHALL reject the request with a permission denied error. `02-REQ-1.E2`

---

### Requirement 2: LOCKING_SERVICE Command Subscription

**User Story:** As a vehicle locking system, I want to subscribe to door
lock/unlock command signals from DATA_BROKER, so that I can receive remote
commands without direct coupling to the cloud gateway.

#### Acceptance Criteria

1. WHEN LOCKING_SERVICE starts, THE service SHALL subscribe to
   Vehicle.Command.Door.Lock signals from DATA_BROKER via gRPC over UDS.
   `02-REQ-2.1`
2. WHEN a Vehicle.Command.Door.Lock signal is received, THE LOCKING_SERVICE
   SHALL parse the JSON payload and extract the command_id, action, and doors
   fields. `02-REQ-2.2`
3. WHEN the action field is "lock", THE LOCKING_SERVICE SHALL attempt to lock
   the specified doors subject to safety constraints. `02-REQ-2.3`
4. WHEN the action field is "unlock", THE LOCKING_SERVICE SHALL attempt to
   unlock the specified doors subject to safety constraints. `02-REQ-2.4`

#### Edge Cases

1. IF the Vehicle.Command.Door.Lock payload is not valid JSON, THEN
   LOCKING_SERVICE SHALL write a response with status "failed" and reason
   "invalid_payload". `02-REQ-2.E1`
2. IF the action field contains an unrecognized value (neither "lock" nor
   "unlock"), THEN LOCKING_SERVICE SHALL write a response with status "failed"
   and reason "unknown_action". `02-REQ-2.E2`
3. IF the command payload is missing required fields (command_id or action),
   THEN LOCKING_SERVICE SHALL write a response with status "failed" and reason
   "missing_fields". `02-REQ-2.E3`

---

### Requirement 3: LOCKING_SERVICE Safety Constraints

**User Story:** As a safety engineer, I want the LOCKING_SERVICE to validate
safety constraints before executing any lock/unlock command, so that the
vehicle cannot be locked while moving or while a door is ajar.

#### Acceptance Criteria

1. BEFORE executing a lock command, THE LOCKING_SERVICE SHALL read
   Vehicle.Speed from DATA_BROKER and reject the command IF the speed
   value is greater than zero. `02-REQ-3.1`
2. BEFORE executing a lock command, THE LOCKING_SERVICE SHALL read
   Vehicle.Cabin.Door.Row1.DriverSide.IsOpen from DATA_BROKER and reject
   the command IF the door is open (IsOpen == true). `02-REQ-3.2`
3. BEFORE executing an unlock command, THE LOCKING_SERVICE SHALL read
   Vehicle.Speed from DATA_BROKER and reject the command IF the speed
   value is greater than zero. `02-REQ-3.3`
4. WHEN a command is rejected due to a safety constraint violation, THE
   LOCKING_SERVICE SHALL write a Vehicle.Command.Door.Response with status
   "failed" and a reason indicating the specific constraint violated
   ("vehicle_moving" or "door_open"). `02-REQ-3.4`
5. WHEN a command passes all safety constraint checks, THE LOCKING_SERVICE
   SHALL write the updated lock state to
   Vehicle.Cabin.Door.Row1.DriverSide.IsLocked and write a
   Vehicle.Command.Door.Response with status "success". `02-REQ-3.5`

#### Edge Cases

1. IF Vehicle.Speed has not been set in DATA_BROKER (no value available), THEN
   LOCKING_SERVICE SHALL treat the speed as zero (safe to proceed) for demo
   purposes. `02-REQ-3.E1`
2. IF Vehicle.Cabin.Door.Row1.DriverSide.IsOpen has not been set, THEN
   LOCKING_SERVICE SHALL treat the door as closed (safe to proceed) for demo
   purposes. `02-REQ-3.E2`

---

### Requirement 4: CLOUD_GATEWAY_CLIENT MQTT Connectivity

**User Story:** As a cloud integration developer, I want the
CLOUD_GATEWAY_CLIENT to maintain an MQTT connection to the CLOUD_GATEWAY
broker, so that it can receive commands and publish telemetry.

#### Acceptance Criteria

1. WHEN CLOUD_GATEWAY_CLIENT starts, THE service SHALL connect to the MQTT
   broker at a configurable address (default: localhost:1883). `02-REQ-4.1`
2. WHEN connected to the MQTT broker, THE CLOUD_GATEWAY_CLIENT SHALL subscribe
   to the topic `vehicles/{vin}/commands` where {vin} is a configurable
   vehicle identification number. `02-REQ-4.2`
3. WHEN an MQTT message is received on the commands topic, THE
   CLOUD_GATEWAY_CLIENT SHALL validate the command JSON structure and write
   it to Vehicle.Command.Door.Lock in DATA_BROKER via gRPC over UDS.
   `02-REQ-4.3`
4. WHEN CLOUD_GATEWAY_CLIENT observes a Vehicle.Command.Door.Response signal
   change in DATA_BROKER, THE service SHALL publish the response to the MQTT
   topic `vehicles/{vin}/command_responses`. `02-REQ-4.4`

#### Edge Cases

1. IF the MQTT broker is unreachable at startup, THEN CLOUD_GATEWAY_CLIENT
   SHALL retry connecting with exponential backoff and log each retry attempt.
   `02-REQ-4.E1`
2. IF the MQTT connection is lost during operation, THEN CLOUD_GATEWAY_CLIENT
   SHALL attempt reconnection with exponential backoff while continuing to
   serve DATA_BROKER subscriptions. `02-REQ-4.E2`
3. IF an MQTT message on the commands topic contains invalid JSON, THEN
   CLOUD_GATEWAY_CLIENT SHALL log the error and discard the message without
   writing to DATA_BROKER. `02-REQ-4.E3`

---

### Requirement 5: CLOUD_GATEWAY_CLIENT Telemetry Publishing

**User Story:** As a fleet operations system, I want the CLOUD_GATEWAY_CLIENT
to publish vehicle telemetry to the cloud, so that I can monitor vehicle
status remotely.

#### Acceptance Criteria

1. WHEN CLOUD_GATEWAY_CLIENT starts, THE service SHALL subscribe to vehicle
   state signals in DATA_BROKER: IsLocked, IsOpen, Latitude, Longitude, and
   Speed. `02-REQ-5.1`
2. WHEN a subscribed vehicle state signal changes, THE CLOUD_GATEWAY_CLIENT
   SHALL publish a telemetry message to the MQTT topic
   `vehicles/{vin}/telemetry` containing the signal path, value, and
   timestamp. `02-REQ-5.2`

#### Edge Cases

1. IF DATA_BROKER is unreachable when subscribing, THEN CLOUD_GATEWAY_CLIENT
   SHALL retry with exponential backoff and log each retry attempt.
   `02-REQ-5.E1`

---

### Requirement 6: Mock Sensor Services

**User Story:** As a tester, I want CLI tools that can inject sensor values
into DATA_BROKER on demand, so that I can set up test scenarios for
LOCKING_SERVICE and CLOUD_GATEWAY_CLIENT without real vehicle hardware.

#### Acceptance Criteria

1. THE repository SHALL contain a LOCATION_SENSOR CLI tool that accepts
   latitude (double) and longitude (double) as CLI arguments and writes
   Vehicle.CurrentLocation.Latitude and Vehicle.CurrentLocation.Longitude
   to DATA_BROKER via gRPC. `02-REQ-6.1`
2. THE repository SHALL contain a SPEED_SENSOR CLI tool that accepts a speed
   value (float) as a CLI argument and writes Vehicle.Speed to DATA_BROKER
   via gRPC. `02-REQ-6.2`
3. THE repository SHALL contain a DOOR_SENSOR CLI tool that accepts an
   is_open flag (bool) as a CLI argument and writes
   Vehicle.Cabin.Door.Row1.DriverSide.IsOpen to DATA_BROKER via gRPC.
   `02-REQ-6.3`
4. WHEN run with valid arguments, each mock sensor tool SHALL write the
   specified value to DATA_BROKER and exit with code 0. `02-REQ-6.4`
5. WHEN run without required arguments, each mock sensor tool SHALL display
   a usage message and exit with a non-zero exit code. `02-REQ-6.5`

#### Edge Cases

1. IF DATA_BROKER is unreachable when a mock sensor tool is invoked, THEN
   the tool SHALL print an error message indicating the connection failure
   and exit with a non-zero exit code. `02-REQ-6.E1`
2. IF a mock sensor tool is invoked with an invalid value (e.g., non-numeric
   speed), THEN it SHALL print an error message and exit with a non-zero
   exit code. `02-REQ-6.E2`

---

### Requirement 7: Safety Partition Communication

**User Story:** As a system architect, I want all same-partition services to
communicate via gRPC over Unix Domain Sockets, so that the safety partition
maintains communication isolation from other partitions.

#### Acceptance Criteria

1. THE LOCKING_SERVICE SHALL connect to DATA_BROKER exclusively via gRPC
   over UDS. `02-REQ-7.1`
2. THE CLOUD_GATEWAY_CLIENT SHALL connect to DATA_BROKER exclusively via
   gRPC over UDS. `02-REQ-7.2`
3. THE mock sensor services SHALL connect to DATA_BROKER via gRPC using a
   configurable endpoint (defaulting to UDS for same-partition use, with an
   option to use network gRPC for cross-partition testing). `02-REQ-7.3`
4. THE DATA_BROKER UDS socket path SHALL be configurable via environment
   variable (default: `/tmp/kuksa-databroker.sock`). `02-REQ-7.4`

#### Edge Cases

1. IF the UDS socket file does not exist at the configured path, THEN
   services attempting to connect SHALL log an error indicating the socket
   path and fail gracefully. `02-REQ-7.E1`

---

### Requirement 8: Integration Test Support

**User Story:** As a developer, I want integration tests that verify the
end-to-end command flow through DATA_BROKER, so that I can validate the
safety partition services work together correctly.

#### Acceptance Criteria

1. THE repository SHALL contain integration tests that verify the lock
   command flow: CLOUD_GATEWAY_CLIENT writes a command signal, LOCKING_SERVICE
   processes it, and the correct lock state and command response are written
   to DATA_BROKER. `02-REQ-8.1`
2. THE integration tests SHALL be runnable with `cargo test --test integration`
   or a similar single command. `02-REQ-8.2`
3. THE integration tests SHALL require DATA_BROKER and MQTT broker to be
   running (via `make infra-up`). `02-REQ-8.3`

#### Edge Cases

1. IF the required infrastructure is not running, THEN integration tests
   SHALL fail with a clear error message indicating which service is
   unreachable. `02-REQ-8.E1`
