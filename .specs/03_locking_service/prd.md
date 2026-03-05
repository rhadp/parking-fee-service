# PRD: LOCKING_SERVICE (Phase 2.1)

> Extracted from the master PRD at `.specs/prd.md`. This spec covers the LOCKING_SERVICE and mock sensor components of Phase 2.1.

## Scope

Implement the LOCKING_SERVICE (ASIL-B door locking service) in Rust, running in the RHIVOS safety partition. Also implement mock sensor CLI tools (DOOR_SENSOR, SPEED_SENSOR, LOCATION_SENSOR) used for testing.

## LOCKING_SERVICE

- Runs in the RHIVOS safety partition
- Subscribes to command signals from DATA_BROKER (`Vehicle.Command.Door.Lock`) for remote lock/unlock requests from CLOUD_GATEWAY_CLIENT
- Validates safety constraints (e.g., vehicle velocity, door ajar status) before executing lock/unlock commands
- Writes lock/unlock state to DATA_BROKER (`Vehicle.Cabin.Door.Row1.DriverSide.IsLocked`)
- Writes command response to DATA_BROKER (`Vehicle.Command.Door.Response`)
- For this demo, focuses on stationary vehicle scenarios where velocity checks are trivial
- Communicates with DATA_BROKER via gRPC over UDS (same partition)

### Safety Constraints

- Vehicle must be stationary (Vehicle.Speed == 0 or near-zero) to execute lock/unlock
- Door must not be ajar (Vehicle.Cabin.Door.Row1.DriverSide.IsOpen == false) to lock
- Unlock commands have no door-ajar constraint

## Mock Sensors

All mock services are on-demand CLI tools. They do not run continuously.

### LOCATION_SENSOR
- CLI tool that sends mock location data to DATA_BROKER via gRPC
- `Vehicle.CurrentLocation.Latitude` (double)
- `Vehicle.CurrentLocation.Longitude` (double)
- Values specified via CLI arguments

### SPEED_SENSOR
- CLI tool that sends mock velocity data to DATA_BROKER via gRPC
- `Vehicle.Speed` (float)
- Values specified via CLI arguments

### DOOR_SENSOR
- CLI tool that sends mock door open/closed data to DATA_BROKER via gRPC
- `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` (bool)
- Values specified via CLI arguments

## Tech Stack

- Language: Rust
- Communication: gRPC over UDS (kuksa-client or tonic with UDS support)

## Dependencies

| Spec | From Group | To Group | Relationship |
|------|-----------|----------|--------------|
| 01_project_setup | 2 | 1 | Requires Rust workspace skeleton and proto definitions |
| 02_data_broker | 2 | 1 | Requires running DATA_BROKER with VSS signals configured |
