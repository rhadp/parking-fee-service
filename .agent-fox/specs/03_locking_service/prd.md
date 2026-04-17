# PRD: LOCKING_SERVICE (Phase 2.1)

> Extracted from the master PRD at `.agent-fox/specs/prd.md`. This spec covers the LOCKING_SERVICE component of Phase 2.1: RHIVOS Safety Partition.

## Scope

Implement the LOCKING_SERVICE as an ASIL-B rated Rust service running in the RHIVOS safety partition. The service subscribes to command signals from DATA_BROKER for remote lock/unlock requests, validates safety constraints before executing commands, writes lock/unlock state back to DATA_BROKER, and writes command responses to DATA_BROKER. For this demo, the service focuses on stationary vehicle scenarios where velocity checks are trivial.

## Component Description

- Runs in the RHIVOS safety partition
- Subscribes to command signals from DATA_BROKER (`Vehicle.Command.Door.Lock`) for remote lock/unlock requests from CLOUD_GATEWAY_CLIENT
- Validates safety constraints (e.g., vehicle velocity, door ajar status) before executing lock/unlock commands
- Writes lock/unlock state to DATA_BROKER (`Vehicle.Cabin.Door.Row1.DriverSide.IsLocked`)
- Writes command response to DATA_BROKER (`Vehicle.Command.Door.Response`)
  - Success: `{"command_id": "<uuid>", "status": "success", "timestamp": <unix_ts>}`
  - Failure: `{"command_id": "<uuid>", "status": "failed", "reason": "<reason>", "timestamp": <unix_ts>}`
- Communicates with DATA_BROKER via gRPC over Unix Domain Sockets (same partition)
- Note: For this demo, focuses on stationary vehicle scenarios where velocity checks are trivial

## VSS Signals

### Signals Read by LOCKING_SERVICE

| Signal Path | Data Type | Source | Purpose |
|-------------|-----------|--------|---------|
| `Vehicle.Command.Door.Lock` | string (JSON) | CLOUD_GATEWAY_CLIENT | Lock/unlock command request |
| `Vehicle.Speed` | float | SPEED_SENSOR | Safety constraint: vehicle must be stationary |
| `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` | bool | DOOR_SENSOR | Safety constraint: door must not be ajar |

### Signals Written by LOCKING_SERVICE

| Signal Path | Data Type | Purpose |
|-------------|-----------|---------|
| `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` | bool | Current lock state |
| `Vehicle.Command.Door.Response` | string (JSON) | Command execution result |

### Command Payload Format

```json
{
  "command_id": "<uuid>",
  "action": "lock",
  "doors": ["driver"],
  "source": "companion_app",
  "vin": "<vin>",
  "timestamp": 1700000000
}
```

### Response Payload Format

Success:
```json
{
  "command_id": "<uuid>",
  "status": "success",
  "timestamp": 1700000001
}
```

Failure:
```json
{
  "command_id": "<uuid>",
  "status": "failed",
  "reason": "vehicle_moving",
  "timestamp": 1700000001
}
```

## Communication Protocols

| Source Component | Target Component | Protocol | Direction |
|-----------------|-----------------|----------|-----------|
| LOCKING_SERVICE | DATA_BROKER | gRPC (UDS) | Bidirectional (Write state, Read commands) |

## Mixed-Criticality Context

- LOCKING_SERVICE is an ASIL-B service publishing safety-relevant state (door lock/unlock) to DATA_BROKER
- LOCKING_SERVICE subscribes to command signals from DATA_BROKER (written by CLOUD_GATEWAY_CLIENT)
- QM adapters (e.g., PARKING_OPERATOR_ADAPTOR) subscribe to lock state signals from DATA_BROKER (read-only access)
- Same-partition communication uses Unix Domain Sockets/gRPC

## Dependencies

| Spec | From Group | To Group | Relationship |
|------|-----------|----------|--------------|
| 01_project_setup | 3 | 1 | Uses Rust workspace and locking-service skeleton from group 3 |
| 02_data_broker | 2 | 4 | Requires configured DATA_BROKER (compose.yml with dual listeners) for integration tests; group 2 produces the compose config |

## Clarifications

The following clarifications were resolved during requirements analysis.

- **C1 (Speed threshold):** "Stationary" means `Vehicle.Speed < 1.0` km/h. A small tolerance accounts for sensor noise. Any speed >= 1.0 km/h results in a "vehicle_moving" rejection.
- **C2 (Door ajar constraint):** Locking is rejected if `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen == true` (reason: "door_open"). Unlocking is always allowed regardless of door state — the door being open is not a safety concern for unlock operations.
- **C3 (DATA_BROKER connection):** The service connects to DATA_BROKER via a configurable endpoint. Default for local development: `http://localhost:55556` (TCP). Production: UDS path. Configured via environment variable `DATABROKER_ADDR`.
- **C4 (Supported doors):** For demo scope, only `"driver"` is supported in the `doors` array. Commands targeting any other door value are rejected with reason `"unsupported_door"`.
- **C5 (Payload validation):** Required fields: `command_id` (non-empty string), `action` ("lock" or "unlock"), `doors` (array containing "driver"). Missing or invalid fields result in a failure response with reason `"invalid_command"`. Fields `source`, `vin`, and `timestamp` are optional for the locking service (they are metadata for tracing, not used in decision logic).
- **C6 (Initial state):** The service starts with the door in unlocked state (`IsLocked = false`). On startup, it publishes this initial state to DATA_BROKER.
- **C7 (Command ordering):** Commands are processed sequentially. If multiple commands arrive while one is being processed, they are queued and handled in order.
- **C8 (Service lifecycle):** The service runs as a long-lived process. On startup, it subscribes to `Vehicle.Command.Door.Lock` and processes commands as they arrive. The service exits cleanly on SIGTERM/SIGINT.
- **C9 (Kuksa gRPC client):** Uses tonic-generated Rust client from the `kuksa.val.v1` proto definitions (Kuksa Databroker API). Proto files are vendored from the Eclipse Kuksa project.
- **C10 (Idempotent commands):** Locking an already-locked door or unlocking an already-unlocked door succeeds (returns "success") without changing state. This simplifies the companion app's retry logic.
