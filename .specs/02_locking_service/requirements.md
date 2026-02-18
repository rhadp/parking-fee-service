# Requirements Document: LOCKING_SERVICE + DATA_BROKER + Mock Sensors

## Introduction

This specification defines the behavior of the LOCKING_SERVICE, the Kuksa
DATA_BROKER configuration (custom VSS signals), and the mock sensor CLI tools
for the SDV Parking Demo System. Together these form the RHIVOS safety-partition
core that processes lock/unlock commands with safety validation.

## Glossary

| Term | Definition |
|------|-----------|
| DATA_BROKER | Eclipse Kuksa Databroker — VSS-compliant signal broker running in the RHIVOS safety partition |
| IsLocked | VSS signal `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` — the current lock state of the driver door |
| IsOpen | VSS signal `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` — whether the driver door is physically open |
| Kuksa | Eclipse Kuksa project — open-source vehicle data management |
| Lock command | A request to lock or unlock the vehicle, represented by the `Vehicle.Command.Door.Lock` signal |
| LockResult | VSS signal `Vehicle.Command.Door.LockResult` — the outcome of the last lock command |
| LOCKING_SERVICE | Rust service in the RHIVOS safety partition that processes lock/unlock commands |
| Mock sensors | CLI tools that publish simulated vehicle signal values to DATA_BROKER |
| Safety validation | Pre-execution checks on vehicle state (speed, door position) before allowing a lock/unlock |
| VSS | COVESA Vehicle Signal Specification, version 5.1 |
| VSS overlay | A JSON file extending the standard VSS model with custom signal definitions |

## Requirements

### Requirement 1: Kuksa VSS Overlay

**User Story:** As a developer, I want DATA_BROKER to support custom VSS
signals required by the demo, so that services can communicate lock commands,
results, and parking state through the standard signal broker.

#### Acceptance Criteria

1. **02-REQ-1.1** THE VSS overlay SHALL define `Vehicle.Command.Door.Lock`
   as a boolean actuator signal.

2. **02-REQ-1.2** THE VSS overlay SHALL define `Vehicle.Command.Door.LockResult`
   as a string sensor signal with allowed values: `"SUCCESS"`,
   `"REJECTED_SPEED"`, `"REJECTED_DOOR_OPEN"`.

3. **02-REQ-1.3** THE VSS overlay SHALL define `Vehicle.Parking.SessionActive`
   as a boolean sensor signal.

4. **02-REQ-1.4** WHEN `make infra-up` is run, THE Kuksa Databroker SHALL load
   the custom VSS overlay so that all custom signals are accessible via gRPC.

5. **02-REQ-1.5** THE VSS overlay file SHALL reside at
   `infra/config/kuksa/vss_overlay.json`.

#### Edge Cases

1. **02-REQ-1.E1** IF the VSS overlay file is malformed, THEN Kuksa Databroker
   SHALL fail to start with a clear error message.

---

### Requirement 2: LOCKING_SERVICE Command Subscription

**User Story:** As a safety system, I want LOCKING_SERVICE to continuously
monitor lock/unlock command requests via DATA_BROKER, so that commands are
processed promptly.

#### Acceptance Criteria

1. **02-REQ-2.1** WHEN LOCKING_SERVICE starts, THE service SHALL connect to
   DATA_BROKER and subscribe to `Vehicle.Command.Door.Lock` via gRPC streaming.

2. **02-REQ-2.2** WHILE LOCKING_SERVICE is running and subscribed, THE service
   SHALL process each `Vehicle.Command.Door.Lock` signal change within the
   same subscription stream.

3. **02-REQ-2.3** THE LOCKING_SERVICE SHALL accept a DATA_BROKER address via
   `--databroker-addr` flag or `DATABROKER_ADDR` environment variable, defaulting
   to `localhost:55555`.

#### Edge Cases

1. **02-REQ-2.E1** IF the DATA_BROKER is unreachable at startup, THEN
   LOCKING_SERVICE SHALL retry connection with exponential backoff, logging each
   attempt.

2. **02-REQ-2.E2** IF the subscription stream is interrupted, THEN
   LOCKING_SERVICE SHALL re-subscribe automatically.

---

### Requirement 3: Safety Validation

**User Story:** As a safety system, I want LOCKING_SERVICE to validate vehicle
state before executing a lock/unlock command, so that unsafe operations are
prevented.

#### Acceptance Criteria

1. **02-REQ-3.1** WHEN a lock command (`Vehicle.Command.Door.Lock = true`) is
   received AND `Vehicle.Speed >= 1.0` km/h, THEN LOCKING_SERVICE SHALL reject
   the command.

2. **02-REQ-3.2** WHEN a lock command (`Vehicle.Command.Door.Lock = true`) is
   received AND `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen = true`, THEN
   LOCKING_SERVICE SHALL reject the command.

3. **02-REQ-3.3** WHEN an unlock command (`Vehicle.Command.Door.Lock = false`)
   is received AND `Vehicle.Speed >= 1.0` km/h, THEN LOCKING_SERVICE SHALL
   reject the command.

4. **02-REQ-3.4** WHEN an unlock command is received, THE LOCKING_SERVICE SHALL
   NOT check `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` (no door-ajar
   constraint for unlocking).

#### Edge Cases

1. **02-REQ-3.E1** IF `Vehicle.Speed` has no value in DATA_BROKER (signal not
   yet set), THEN LOCKING_SERVICE SHALL treat speed as 0.0 (safe) for the
   demo.

2. **02-REQ-3.E2** IF `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` has no
   value in DATA_BROKER, THEN LOCKING_SERVICE SHALL treat the door as closed
   (safe) for the demo.

---

### Requirement 4: Lock Execution

**User Story:** As a safety system, I want LOCKING_SERVICE to update the
vehicle lock state in DATA_BROKER when a command passes safety validation, so
that downstream services can react to lock/unlock events.

#### Acceptance Criteria

1. **02-REQ-4.1** WHEN a lock command passes safety validation, THE
   LOCKING_SERVICE SHALL write `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked`
   to DATA_BROKER with the commanded value (`true` for lock, `false` for
   unlock).

2. **02-REQ-4.2** WHEN a command is rejected by safety validation, THE
   LOCKING_SERVICE SHALL NOT modify `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked`.

#### Edge Cases

1. **02-REQ-4.E1** IF writing `IsLocked` to DATA_BROKER fails, THEN
   LOCKING_SERVICE SHALL log the error and write `LockResult` with an
   appropriate error indication.

---

### Requirement 5: Result Reporting

**User Story:** As a connected vehicle system, I want LOCKING_SERVICE to
report the outcome of every lock command via DATA_BROKER, so that
CLOUD_GATEWAY_CLIENT (spec 03) can propagate results back to the user.

#### Acceptance Criteria

1. **02-REQ-5.1** WHEN a lock command is successfully executed, THE
   LOCKING_SERVICE SHALL write `Vehicle.Command.Door.LockResult = "SUCCESS"`
   to DATA_BROKER.

2. **02-REQ-5.2** WHEN a lock command is rejected due to speed, THE
   LOCKING_SERVICE SHALL write
   `Vehicle.Command.Door.LockResult = "REJECTED_SPEED"` to DATA_BROKER.

3. **02-REQ-5.3** WHEN a lock command is rejected due to door ajar, THE
   LOCKING_SERVICE SHALL write
   `Vehicle.Command.Door.LockResult = "REJECTED_DOOR_OPEN"` to DATA_BROKER.

4. **02-REQ-5.4** THE LOCKING_SERVICE SHALL write exactly one `LockResult`
   for every `Vehicle.Command.Door.Lock` signal change it processes.

#### Edge Cases

1. **02-REQ-5.E1** IF writing `LockResult` to DATA_BROKER fails, THEN
   LOCKING_SERVICE SHALL log the error (best-effort reporting).

---

### Requirement 6: Mock Sensors Implementation

**User Story:** As a developer, I want mock sensor CLI tools that write
realistic vehicle signal values to DATA_BROKER, so that I can test
LOCKING_SERVICE and other services without real hardware.

#### Acceptance Criteria

1. **02-REQ-6.1** WHEN `mock-sensors set-location <lat> <lon>` is run, THE
   tool SHALL write the given latitude and longitude to DATA_BROKER at
   `Vehicle.CurrentLocation.Latitude` and
   `Vehicle.CurrentLocation.Longitude`.

2. **02-REQ-6.2** WHEN `mock-sensors set-speed <value>` is run, THE tool
   SHALL write the given speed (km/h) to DATA_BROKER at `Vehicle.Speed`.

3. **02-REQ-6.3** WHEN `mock-sensors set-door <open|closed>` is run, THE
   tool SHALL write the corresponding boolean to DATA_BROKER at
   `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen`.

4. **02-REQ-6.4** WHEN `mock-sensors lock-command <lock|unlock>` is run, THE
   tool SHALL write the corresponding boolean to DATA_BROKER at
   `Vehicle.Command.Door.Lock`.

5. **02-REQ-6.5** THE mock-sensors tool SHALL connect to DATA_BROKER via gRPC
   using the address from `--databroker-addr` flag or `DATABROKER_ADDR`
   environment variable, defaulting to `localhost:55555`.

#### Edge Cases

1. **02-REQ-6.E1** IF DATA_BROKER is unreachable, THEN mock-sensors SHALL
   print an error message and exit with a non-zero exit code.

---

### Requirement 7: Integration Verification

**User Story:** As a developer, I want an automated integration test that
exercises the full lock/unlock flow (mock-sensors → DATA_BROKER →
LOCKING_SERVICE → DATA_BROKER), so that I can verify the safety-partition
core works end-to-end.

#### Acceptance Criteria

1. **02-REQ-7.1** THE integration test SHALL verify: given safe vehicle
   conditions (speed = 0, door closed), a lock command results in
   `IsLocked = true` and `LockResult = "SUCCESS"`.

2. **02-REQ-7.2** THE integration test SHALL verify: given unsafe speed
   (speed >= 1.0), a lock command does NOT change `IsLocked` and
   `LockResult = "REJECTED_SPEED"`.

3. **02-REQ-7.3** THE integration test SHALL verify: given door ajar, a lock
   command does NOT change `IsLocked` and
   `LockResult = "REJECTED_DOOR_OPEN"`.

4. **02-REQ-7.4** THE integration test SHALL verify: given safe conditions,
   an unlock command results in `IsLocked = false` and
   `LockResult = "SUCCESS"`.

#### Edge Cases

1. **02-REQ-7.E1** IF the integration test infrastructure (DATA_BROKER) is
   unavailable, THEN the test SHALL skip with a clear message rather than
   fail.
