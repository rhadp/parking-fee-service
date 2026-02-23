# PRD: RHIVOS Safety Partition (Phase 2.1)

## Overview

This specification covers Phase 2.1 of the SDV Parking Demo System:
implementation of the RHIVOS safety-partition services. The safety partition
hosts ASIL-B components responsible for vehicle door locking, vehicle signal
brokering, and cloud connectivity. This phase delivers the foundational vehicle
services that all subsequent phases depend on.

The four deliverables are:

1. **DATA_BROKER** — Deploy and configure Eclipse Kuksa Databroker with custom
   VSS signals, access control, and dual endpoints (UDS + network gRPC).
2. **LOCKING_SERVICE** (Rust) — Subscribe to command signals from DATA_BROKER,
   validate safety constraints, write lock/unlock state and command responses.
3. **CLOUD_GATEWAY_CLIENT** (Rust) — MQTT client connecting to CLOUD_GATEWAY
   (Eclipse Mosquitto for local dev), relaying commands and telemetry through
   DATA_BROKER.
4. **Mock Sensor Services** (Rust CLI tools) — LOCATION_SENSOR, SPEED_SENSOR,
   DOOR_SENSOR providing on-demand VSS signal injection via CLI.

## Dependencies

| Spec | Relationship | Notes |
|------|-------------|-------|
| 01_project_setup | Depends on | Rust workspace, proto definitions, local infra (Mosquitto + Kuksa), Makefile targets |

## User Stories

### US-1: Vehicle Signal Brokering

As a vehicle service developer, I want a running DATA_BROKER with all required
VSS signals configured, so that safety-partition services can publish and
subscribe to vehicle state and command signals without custom signal broker code.

### US-2: Remote Door Locking

As a vehicle owner using the COMPANION_APP, I want the LOCKING_SERVICE to
receive and execute lock/unlock commands that arrive through the DATA_BROKER,
so that I can remotely control my vehicle's door locks when safety constraints
are satisfied.

### US-3: Cloud Command Relay

As a cloud service, I want the CLOUD_GATEWAY_CLIENT to receive MQTT commands,
validate them, and write them to DATA_BROKER as command signals, so that the
LOCKING_SERVICE can process commands without direct cloud coupling.

### US-4: Telemetry Publishing

As a fleet operator, I want the CLOUD_GATEWAY_CLIENT to subscribe to vehicle
state from DATA_BROKER and publish telemetry to the CLOUD_GATEWAY via MQTT, so
that I can monitor vehicle status from the cloud.

### US-5: Simulated Sensor Input

As a developer or tester, I want CLI tools that can set vehicle sensor values
(location, speed, door status) in the DATA_BROKER on demand, so that I can
test safety-partition services without real vehicle hardware.

## Scope

### In Scope

- Kuksa Databroker configuration: VSS overlay file for custom signals,
  bearer-token access control, UDS endpoint for same-partition services,
  network gRPC endpoint for cross-partition consumers.
- LOCKING_SERVICE: command subscription, safety constraint validation
  (velocity check, door ajar check), lock/unlock state publication, command
  response publication.
- CLOUD_GATEWAY_CLIENT: MQTT connection to Mosquitto, command reception and
  validation, command signal publication to DATA_BROKER, vehicle state
  subscription, telemetry publication via MQTT, command response relay.
- Mock sensor CLI tools: LOCATION_SENSOR, SPEED_SENSOR, DOOR_SENSOR.
- Integration tests verifying end-to-end command flow through DATA_BROKER.

### Out of Scope

- RHIVOS QM partition services (Phase 2.3).
- CLOUD_GATEWAY backend service (Phase 2.2).
- Real hardware integration or GPS.
- Production TLS configuration (demo-grade bearer tokens only).
- PARKING_OPERATOR_ADAPTOR or UPDATE_SERVICE functionality.

## VSS Signals

### State Signals (standard VSS)

| Signal Path | Type | Written By | Read By |
|-------------|------|-----------|---------|
| Vehicle.Cabin.Door.Row1.DriverSide.IsLocked | bool | LOCKING_SERVICE | CLOUD_GATEWAY_CLIENT, PARKING_OPERATOR_ADAPTOR (Phase 2.3) |
| Vehicle.Cabin.Door.Row1.DriverSide.IsOpen | bool | DOOR_SENSOR | LOCKING_SERVICE |
| Vehicle.CurrentLocation.Latitude | double | LOCATION_SENSOR | CLOUD_GATEWAY_CLIENT |
| Vehicle.CurrentLocation.Longitude | double | LOCATION_SENSOR | CLOUD_GATEWAY_CLIENT |
| Vehicle.Speed | float | SPEED_SENSOR | LOCKING_SERVICE |

### Custom Signals

| Signal Path | Type | Written By | Read By | Payload |
|-------------|------|-----------|---------|---------|
| Vehicle.Command.Door.Lock | string (JSON) | CLOUD_GATEWAY_CLIENT | LOCKING_SERVICE | `{"command_id": "<uuid>", "action": "lock"\|"unlock", "doors": ["driver"], "source": "companion_app", "vin": "<vin>", "timestamp": <unix_ts>}` |
| Vehicle.Command.Door.Response | string (JSON) | LOCKING_SERVICE | CLOUD_GATEWAY_CLIENT | `{"command_id": "<uuid>", "status": "success"\|"failed", "reason": "<optional>", "timestamp": <unix_ts>}` |

## MQTT Topics

| Topic | Publisher | Subscriber | Payload |
|-------|----------|-----------|---------|
| `vehicles/{vin}/commands` | CLOUD_GATEWAY | CLOUD_GATEWAY_CLIENT | Lock/unlock command JSON |
| `vehicles/{vin}/command_responses` | CLOUD_GATEWAY_CLIENT | CLOUD_GATEWAY | Command result JSON |
| `vehicles/{vin}/telemetry` | CLOUD_GATEWAY_CLIENT | CLOUD_GATEWAY | Vehicle state JSON |

## Message Flows

### Flow 1: Remote Lock Command (Happy Path)

```
1. CLOUD_GATEWAY_CLIENT receives MQTT message on vehicles/{vin}/commands
2. CLOUD_GATEWAY_CLIENT validates command structure and bearer token
3. CLOUD_GATEWAY_CLIENT writes Vehicle.Command.Door.Lock to DATA_BROKER (gRPC, UDS)
4. LOCKING_SERVICE receives command via DATA_BROKER subscription (gRPC, UDS)
5. LOCKING_SERVICE reads Vehicle.Speed and Vehicle.Cabin.Door.Row1.DriverSide.IsOpen
6. LOCKING_SERVICE validates: speed == 0, door is not open
7. LOCKING_SERVICE writes Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = true (gRPC, UDS)
8. LOCKING_SERVICE writes Vehicle.Command.Door.Response = {status: "success"} (gRPC, UDS)
9. CLOUD_GATEWAY_CLIENT receives response via DATA_BROKER subscription
10. CLOUD_GATEWAY_CLIENT publishes response to vehicles/{vin}/command_responses (MQTT)
```

### Flow 2: Lock Command Rejected (Safety Constraint)

```
1-5. Same as Flow 1
6. LOCKING_SERVICE detects speed > 0 OR door is open
7. LOCKING_SERVICE writes Vehicle.Command.Door.Response = {status: "failed", reason: "vehicle_moving"} (gRPC, UDS)
8. CLOUD_GATEWAY_CLIENT receives and relays failure response via MQTT
```

## Success Criteria

1. DATA_BROKER starts with all VSS signals configured and both endpoints available.
2. Lock command succeeds when vehicle is stationary and door is closed.
3. Lock command fails with reason when vehicle is moving or door is open.
4. Unlock command succeeds when vehicle is stationary.
5. CLOUD_GATEWAY_CLIENT relays commands and responses between MQTT and DATA_BROKER.
6. Mock sensors can set arbitrary signal values in DATA_BROKER.
7. All communication within the safety partition uses gRPC over UDS.
