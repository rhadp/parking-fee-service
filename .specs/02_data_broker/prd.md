# PRD: DATA_BROKER (Phase 2.1)

> Extracted from the master PRD at `.specs/prd.md`. This spec covers the DATA_BROKER component of Phase 2.1: RHIVOS Safety Partition.

## Scope

Deploy and configure Eclipse Kuksa Databroker as the DATA_BROKER in the RHIVOS safety partition. This component is a pre-built binary -- no wrapper code or reimplementation is written. The work consists of downloading the binary, creating a VSS overlay file for custom signals, and configuring dual listeners (UDS and TCP).

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

- `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` (bool) -- current lock state, written by LOCKING_SERVICE
- `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` (bool) -- door ajar detection, written by DOOR_SENSOR
- `Vehicle.CurrentLocation.Latitude` (double) -- vehicle latitude, written by LOCATION_SENSOR
- `Vehicle.CurrentLocation.Longitude` (double) -- vehicle longitude, written by LOCATION_SENSOR
- `Vehicle.Speed` (float) -- vehicle speed, written by SPEED_SENSOR

### Custom Signals

Custom signals are defined in a VSS overlay file shared across all vehicle services (see AC7 in master PRD).

- `Vehicle.Parking.SessionActive` (bool) -- adapter-managed parking state, written by PARKING_OPERATOR_ADAPTOR
- `Vehicle.Command.Door.Lock` (string, JSON) -- lock/unlock command request, written by CLOUD_GATEWAY_CLIENT
  - Payload: `{"command_id": "<uuid>", "action": "lock"|"unlock", "doors": ["driver"], "source": "companion_app", "vin": "<vin>", "timestamp": <unix_ts>}`
- `Vehicle.Command.Door.Response` (string, JSON) -- command execution result, written by LOCKING_SERVICE
  - Payload: `{"command_id": "<uuid>", "status": "success"|"failed", "reason": "<optional>", "timestamp": <unix_ts>}`

## Communication Protocols

| Source Component | Target Component | Protocol | Direction |
|-----------------|-----------------|----------|-----------|
| LOCKING_SERVICE | DATA_BROKER | gRPC (UDS) | Bidirectional (Write state, Read commands) |
| CLOUD_GATEWAY_CLIENT | DATA_BROKER | gRPC (UDS) | Bidirectional (Write commands, Read state) |
| PARKING_APP | DATA_BROKER | Network gRPC | Read |
| PARKING_OPERATOR_ADAPTOR | DATA_BROKER | Network gRPC | Read + Write (SessionActive) |

## Mixed-Criticality Communication Pattern

- ASIL-B services publish safety-relevant state (door lock/unlock) to DATA_BROKER
- ASIL-B services subscribe to command signals from DATA_BROKER (written by CLOUD_GATEWAY_CLIENT)
- QM adapters subscribe to state signals from DATA_BROKER (read-only access to safety-relevant state)
- Cross-partition communication uses network TCP/gRPC; same-partition uses Unix Domain Sockets/gRPC

## Dependencies

| Spec | From Group | To Group | Relationship |
|------|-----------|----------|--------------|
| 01_project_setup | 7 | 1 | Uses compose.yml and VSS overlay from group 7 (infrastructure) |

## Clarifications

The following clarifications were resolved during requirements analysis.

- **C1 (Container, not native binary):** For local development, DATA_BROKER runs as the Kuksa Databroker container already defined in `deployments/compose.yml` by spec 01. The PRD's "pre-built binary" language means no custom wrapper code is written — the container image is used as-is. This spec configures that container correctly for dual listeners and validates signal availability.
- **C2 (Scope vs. spec 01):** Spec 01 created a skeleton compose.yml with a basic Kuksa container (TCP only, port 55556). This spec's deliverables are: (a) update compose.yml to enable UDS listener alongside TCP, (b) validate and complete the VSS overlay with correct metadata for all 8 signals, (c) pin the Kuksa image to a specific version for reproducibility, (d) create integration tests that verify databroker connectivity and signal read/write via both TCP and UDS.
- **C3 (UDS socket path):** The UDS socket path is `/tmp/kuksa-databroker.sock`, exposed to same-partition consumers via a shared volume mount in compose.yml.
- **C4 (Dual listener configuration):** Kuksa Databroker supports dual listeners via CLI flags: `--address 0.0.0.0 --port 55555` (TCP, mapped to host 55556) and `--unix-socket /tmp/kuksa-databroker.sock` (UDS). Both are configured as command args in compose.yml.
- **C5 (Access control):** For the demo scope, Kuksa Databroker runs in permissive mode (no token-based authorization). The "access control" mentioned in the PRD is architectural — enforced by partition isolation, not by databroker config. Token-based auth is out of scope.
- **C6 (Standard VSS signals):** Standard VSS v5.1 signals (IsLocked, IsOpen, Latitude, Longitude, Speed) are included in the default Kuksa Databroker image. Only the 3 custom signals (Vehicle.Parking.SessionActive, Vehicle.Command.Door.Lock, Vehicle.Command.Door.Response) require overlay entries.
- **C7 (Image version):** Pin to `ghcr.io/eclipse-kuksa/kuksa-databroker:0.5.1` for reproducibility. The version can be updated via compose.yml as needed.
- **C8 (Testing approach):** Integration tests start the databroker container, connect via gRPC (both TCP and UDS), verify custom signals can be set/get, and verify standard VSS signals are present in the metadata. Tests use the `kuksa-client` CLI or direct gRPC calls.
- **C9 (VSS overlay completeness):** The overlay created by spec 01 contains the correct 3 custom signals. This spec validates the overlay is loaded correctly by the databroker and signals are accessible. No structural changes to the overlay file are needed unless testing reveals issues.
