# VSS Signal Definitions

This document describes the COVESA Vehicle Signal Specification (VSS) signals
used by the SDV Parking Demo System. The system uses Eclipse Kuksa Databroker
as the VSS signal broker running in the RHIVOS safety partition.

## Standard VSS Signals

These signals are part of the standard VSS model (v5.1) and are loaded by
Kuksa Databroker by default.

| Signal Path | Type | Datatype | Used By |
|-------------|------|----------|---------|
| `Vehicle.Speed` | sensor | float | LOCKING_SERVICE (read), mock-sensors (write) |
| `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` | actuator | boolean | LOCKING_SERVICE (write), integration tests (read) |
| `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` | sensor | boolean | LOCKING_SERVICE (read), mock-sensors (write) |
| `Vehicle.CurrentLocation.Latitude` | sensor | double | mock-sensors (write) |
| `Vehicle.CurrentLocation.Longitude` | sensor | double | mock-sensors (write) |

## Custom VSS Overlay Signals

These signals are defined in `infra/config/kuksa/vss_overlay.json` and extend
the standard VSS model with demo-specific signals.

### Vehicle.Command.Door.Lock

| Property | Value |
|----------|-------|
| **Path** | `Vehicle.Command.Door.Lock` |
| **Type** | actuator |
| **Datatype** | boolean |
| **Description** | Lock/unlock command request. `true` = lock, `false` = unlock. |
| **Written by** | mock-sensors (`lock-command`), companion-app (via CLOUD_GATEWAY_CLIENT) |
| **Read by** | LOCKING_SERVICE (subscription) |

### Vehicle.Command.Door.LockResult

| Property | Value |
|----------|-------|
| **Path** | `Vehicle.Command.Door.LockResult` |
| **Type** | sensor |
| **Datatype** | string |
| **Allowed values** | `SUCCESS`, `REJECTED_SPEED`, `REJECTED_DOOR_OPEN` |
| **Description** | Result of the last lock command processed by LOCKING_SERVICE. |
| **Written by** | LOCKING_SERVICE |
| **Read by** | CLOUD_GATEWAY_CLIENT (for reporting to cloud), integration tests |

### Vehicle.Parking.SessionActive

| Property | Value |
|----------|-------|
| **Path** | `Vehicle.Parking.SessionActive` |
| **Type** | sensor |
| **Datatype** | boolean |
| **Description** | Whether a parking session is currently active. |
| **Written by** | PARKING_OPERATOR_ADAPTOR |
| **Read by** | PARKING_APP (parking-app-cli), CLOUD_GATEWAY_CLIENT (telemetry) |

## LockResult Values

| Value | Meaning | Condition |
|-------|---------|-----------|
| `SUCCESS` | Command executed | Safety validation passed |
| `REJECTED_SPEED` | Command rejected | `Vehicle.Speed >= 1.0` km/h |
| `REJECTED_DOOR_OPEN` | Command rejected | Door is open during a lock command |

## Safety Validation Rules

The LOCKING_SERVICE applies the following safety checks before executing a
lock/unlock command:

1. **Speed check** (applies to both lock and unlock): If `Vehicle.Speed >= 1.0`
   km/h, the command is rejected with `REJECTED_SPEED`.
2. **Door-ajar check** (applies to lock only): If
   `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen = true` and the command is a lock
   request, the command is rejected with `REJECTED_DOOR_OPEN`.
3. **Safe defaults**: If a signal has not been set in the databroker, the
   service treats speed as `0.0` and door as closed (safe for the demo).

## Overlay File Location

The VSS overlay file is located at `infra/config/kuksa/vss_overlay.json`. It is
mounted into the Kuksa Databroker container via `infra/compose.yaml` and loaded
with the `--vss` flag.

## Signal Constants (Rust)

All signal paths are defined as constants in `rhivos/parking-proto/src/signals.rs`:

```rust
pub const DOOR_IS_LOCKED: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked";
pub const DOOR_IS_OPEN: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen";
pub const SPEED: &str = "Vehicle.Speed";
pub const LOCATION_LAT: &str = "Vehicle.CurrentLocation.Latitude";
pub const LOCATION_LON: &str = "Vehicle.CurrentLocation.Longitude";
pub const COMMAND_DOOR_LOCK: &str = "Vehicle.Command.Door.Lock";
pub const LOCK_RESULT: &str = "Vehicle.Command.Door.LockResult";
pub const PARKING_SESSION_ACTIVE: &str = "Vehicle.Parking.SessionActive";
```
