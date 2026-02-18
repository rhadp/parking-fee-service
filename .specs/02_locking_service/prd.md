# PRD: DATA_BROKER + LOCKING_SERVICE + Mock Sensors (Phase 2.1)

> Extracted from the [main PRD](../prd.md). This spec covers Phase 2.1: the
> RHIVOS safety-partition core — Kuksa DATA_BROKER configuration,
> LOCKING_SERVICE implementation, and mock sensor tooling.

## Scope

From the main PRD, Phase 2.1 (partial — CLOUD_GATEWAY_CLIENT deferred to spec
03):

- Configure Eclipse Kuksa Databroker (DATA_BROKER) with custom VSS signals
  needed by the demo.
- Implement LOCKING_SERVICE: subscribe to lock/unlock commands via DATA_BROKER,
  validate safety constraints, execute lock/unlock, report results.
- Implement mock sensor CLI tools with real Kuksa connectivity (extending the
  spec 01 skeleton).
- Integration test of the LOCKING_SERVICE ↔ DATA_BROKER flow using mock
  sensors.

### Components in scope

| Component | Work | Language |
|-----------|------|----------|
| DATA_BROKER (Kuksa) | Custom VSS overlay, infrastructure config | Config (JSON, YAML) |
| LOCKING_SERVICE | Full implementation | Rust |
| Mock sensors | Real implementation (replace spec 01 skeleton) | Rust |

### VSS signals used

**Standard VSS 5.1:**

| Signal | Type | Used by |
|--------|------|---------|
| Vehicle.Cabin.Door.Row1.DriverSide.IsLocked | bool | Written by LOCKING_SERVICE |
| Vehicle.Cabin.Door.Row1.DriverSide.IsOpen | bool | Read by LOCKING_SERVICE |
| Vehicle.CurrentLocation.Latitude | double | Written by mock sensors |
| Vehicle.CurrentLocation.Longitude | double | Written by mock sensors |
| Vehicle.Speed | float | Read by LOCKING_SERVICE |

**Custom signals (VSS overlay):**

| Signal | Type | Used by |
|--------|------|---------|
| Vehicle.Command.Door.Lock | bool | Written by CLOUD_GATEWAY_CLIENT (spec 03) or mock sensors; read by LOCKING_SERVICE |
| Vehicle.Command.Door.LockResult | string | Written by LOCKING_SERVICE; read by CLOUD_GATEWAY_CLIENT (spec 03) |
| Vehicle.Parking.SessionActive | bool | Written by PARKING_OPERATOR_ADAPTOR (spec 04); defined here for completeness |

### Command flow

```
mock-sensors lock-command --lock
        │
        ▼
DATA_BROKER (Vehicle.Command.Door.Lock = true)
        │
        ▼ (subscription stream)
LOCKING_SERVICE
   ├─ Read Vehicle.Speed
   ├─ Read Vehicle.Cabin.Door.Row1.DriverSide.IsOpen
   ├─ Validate: speed < 1.0 AND (lock → door closed)
   │
   ├─ IF valid:
   │   ├─ Write Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = true
   │   └─ Write Vehicle.Command.Door.LockResult = "SUCCESS"
   │
   └─ IF invalid:
       ├─ Do NOT change IsLocked
       └─ Write Vehicle.Command.Door.LockResult = "REJECTED_SPEED"
          or "REJECTED_DOOR_OPEN"
```

### Safety validation rules (demo)

1. **Speed check:** `Vehicle.Speed` must be < 1.0 km/h.
2. **Door-ajar check (lock only):** `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen`
   must be `false` to lock. No constraint for unlocking.
3. These are the only two safety checks for the demo.

### Out-of-scope for this spec

- CLOUD_GATEWAY_CLIENT (spec 03)
- PARKING_OPERATOR_ADAPTOR subscribing to lock events (spec 04)
- Kuksa token-based access control (deferred; anonymous read/write for demo)
- Multi-door support (driver side only)
- Physical key fob interaction

## Dependencies

| Spec | Relationship | Notes |
|------|-------------|-------|
| 01_repo_setup | Depends on | Rust workspace, parking-proto crate, infra compose, mock-sensors skeleton |

## Clarifications

### Architecture

- **A1 (Lock command signal):** Custom VSS signal `Vehicle.Command.Door.Lock`
  (bool) is used for lock/unlock command requests. LOCKING_SERVICE subscribes
  to this signal. It is separate from the `IsLocked` result signal.

- **A2 (Rejected command reporting):** LOCKING_SERVICE writes a result to custom
  signal `Vehicle.Command.Door.LockResult` (string) for every command
  processed. Values: `"SUCCESS"`, `"REJECTED_SPEED"`, `"REJECTED_DOOR_OPEN"`.
  This propagates back to CLOUD_GATEWAY_CLIENT (spec 03) via DATA_BROKER
  subscription.

### Implementation

- **U1 (Safety rules):** Speed < 1.0 km/h and door closed (for lock only).
  These are the only two checks.

- **U2 (Door scope):** Driver side only for the demo.

- **U3 (Kuksa access control):** Anonymous read/write for the demo. Recorded
  as an area of improvement for hardening.

- **U4 (VSS overlay location):** Custom VSS overlay lives in
  `infra/config/kuksa/` and is mounted into the Kuksa container.

- **U5 (Mock sensor lock-command):** A `lock-command` subcommand is added to
  mock-sensors to write `Vehicle.Command.Door.Lock` for testing without
  CLOUD_GATEWAY_CLIENT.
