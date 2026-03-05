# PRD: DATA_BROKER (Phase 2.1)

> Extracted from the master PRD at `.specs/prd.md`. This spec covers the DATA_BROKER component of Phase 2.1: RHIVOS Safety Partition.

## Scope

Deploy and configure Eclipse Kuksa Databroker as the DATA_BROKER in the RHIVOS safety partition. Configure VSS standard and custom signals required by the parking demo system.

## Component Description

- Eclipse Kuksa Databroker, deployed as a pre-built binary (no wrapper or reimplementation)
- Runs in the RHIVOS safety partition
- VSS-compliant gRPC pub/sub interface for vehicle signals
- Manages signal state and enforces read/write access control
- Same-partition consumers use Unix Domain Sockets (UDS)
- Cross-partition and cross-domain consumers use network TCP (gRPC over HTTP/2)
- Local development port: 55556

## VSS Signals

### State Signals (standard VSS v5.1)

- `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` (bool) — current lock state, written by LOCKING_SERVICE
- `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` (bool) — door ajar detection, written by DOOR_SENSOR
- `Vehicle.CurrentLocation.Latitude` (double) — vehicle latitude, written by LOCATION_SENSOR
- `Vehicle.CurrentLocation.Longitude` (double) — vehicle longitude, written by LOCATION_SENSOR
- `Vehicle.Speed` (float) — vehicle speed, written by SPEED_SENSOR

### Custom Signals

- `Vehicle.Parking.SessionActive` (bool) — adapter-managed parking state, written by PARKING_OPERATOR_ADAPTOR
- `Vehicle.Command.Door.Lock` (string, JSON) — lock/unlock command request, written by CLOUD_GATEWAY_CLIENT
  - Payload: `{"command_id": "<uuid>", "action": "lock"|"unlock", "doors": ["driver"], "source": "companion_app", "vin": "<vin>", "timestamp": <unix_ts>}`
- `Vehicle.Command.Door.Response` (string, JSON) — command execution result, written by LOCKING_SERVICE
  - Payload: `{"command_id": "<uuid>", "status": "success"|"failed", "reason": "<optional>", "timestamp": <unix_ts>}`

## Dependencies

| Spec | From Group | To Group | Relationship |
|------|-----------|----------|--------------|
| 01_project_setup | 2 | 1 | Requires repo structure and local infra (docker-compose with Kuksa) |
