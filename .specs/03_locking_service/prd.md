# PRD: LOCKING_SERVICE (Phase 2.1)

> Extracted from the master PRD at `.specs/prd.md`. This spec covers the LOCKING_SERVICE component of Phase 2.1: RHIVOS Safety Partition.

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
| 01_project_setup | 2 | 1 | Uses repo structure and Rust project skeleton from group 2 |
| 02_data_broker | 3 | 1 | Requires DATA_BROKER with VSS overlay for command/state signals |
