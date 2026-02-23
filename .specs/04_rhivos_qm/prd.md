# PRD: RHIVOS QM Partition (Phase 2.3)

> Extracted from the main PRD (`.specs/prd.md`) — Phase 2.3: RHIVOS QM Partition.

## Context

This specification covers the implementation of the RHIVOS QM partition
services for the SDV Parking Demo System. The QM partition hosts
non-safety-critical services that handle cloud connectivity, parking operator
integration, and dynamic adapter lifecycle management. These services
communicate with the safety partition's DATA_BROKER over network gRPC (TCP)
for cross-partition access to vehicle signals such as lock/unlock state and
location.

The key architectural pattern is "feature-on-demand": containerized
PARKING_OPERATOR_ADAPTORs are downloaded, installed, and run dynamically based
on the vehicle's location. Once running, an adaptor autonomously manages
parking sessions by subscribing to lock/unlock events from DATA_BROKER and
communicating with the external PARKING_OPERATOR's REST API.

## Scope

From the main PRD, Phase 2.3:

- Implementation of the RHIVOS QM services:
  - Generic PARKING_OPERATOR_ADAPTOR (with autonomous session management via
    DATA_BROKER events)
  - UPDATE_SERVICE (adapter lifecycle management, OCI container pulling)
  - Mock PARKING_OPERATOR to test the PARKING_OPERATOR_ADAPTOR
  - Mock PARKING_APP CLI enhancements to test UPDATE_SERVICE and
    PARKING_OPERATOR_ADAPTOR
  - Integration test of DATA_BROKER to PARKING_OPERATOR_ADAPTOR communication

## Components

### PARKING_OPERATOR_ADAPTOR (Rust)

- Containerized application running in the RHIVOS QM partition
- Implements the common gRPC interface towards PARKING_APP:
  - `StartSession` — manually start a parking session (override)
  - `StopSession` — manually stop a parking session (override)
  - `GetStatus` — query current session status
  - `GetRate` — query parking rate for a zone
- Subscribes to lock/unlock state signals from DATA_BROKER
  (`Vehicle.Cabin.Door.Row1.DriverSide.IsLocked`) via cross-partition network
  gRPC
- Autonomously starts parking sessions when a lock event is detected, and stops
  sessions on unlock events (session ownership per PRD clarification A6)
- The PARKING_APP can override the autonomous session behavior (e.g., manual
  stop, prevent auto-start)
- Writes parking session state (`Vehicle.Parking.SessionActive`) to DATA_BROKER
- Reads vehicle location from DATA_BROKER for zone context
  (`Vehicle.CurrentLocation.Latitude`, `Vehicle.CurrentLocation.Longitude`)
- REST client towards PARKING_OPERATOR:
  - `POST /parking/start` — start a parking session
  - `POST /parking/stop` — stop a parking session

### UPDATE_SERVICE (Rust)

- Manages containerized adapter lifecycle in RHIVOS QM partition
- gRPC interface:
  - `InstallAdapter` — pull and install an adapter from REGISTRY
  - `WatchAdapterStates` — server-streaming of adapter state change events
  - `ListAdapters` — list all known adapters and their states
  - `RemoveAdapter` — remove an installed adapter
  - `GetAdapterStatus` — get status of a specific adapter
- Pulls OCI containers from REGISTRY on demand
- SHA-256 checksum verification of OCI manifest digest
- Adapter lifecycle states: UNKNOWN, DOWNLOADING, INSTALLING, RUNNING, STOPPED,
  ERROR, OFFLOADING
- Automatic offloading after configurable inactivity timer or resource pressure
- Container management via podman/crun

### Mock PARKING_OPERATOR (Go)

- Simulates a real parking operator's REST API
- Endpoints:
  - `POST /parking/start` — returns session_id and status
  - `POST /parking/stop` — returns fee calculation (duration-based)
  - `GET /parking/{session_id}/status` — returns session status
  - `GET /rate/{zone_id}` — returns parking rate for a zone
- Stateful: tracks active sessions in memory
- Calculates fees based on elapsed time and zone rate

### Mock PARKING_APP CLI Enhancements (Go)

Extends the skeleton mock PARKING_APP CLI from Phase 1.2 with real
functionality:

- `install` — calls UPDATE_SERVICE `InstallAdapter` with image_ref and checksum
- `watch` — calls UPDATE_SERVICE `WatchAdapterStates` (streaming)
- `list` — calls UPDATE_SERVICE `ListAdapters`
- `start-session` — calls PARKING_OPERATOR_ADAPTOR `StartSession`
- `stop-session` — calls PARKING_OPERATOR_ADAPTOR `StopSession`

### Integration Tests

- DATA_BROKER lock event triggers PARKING_OPERATOR_ADAPTOR auto-start session
- PARKING_APP CLI triggers UPDATE_SERVICE adapter lifecycle operations
- PARKING_OPERATOR_ADAPTOR communicates with Mock PARKING_OPERATOR via REST

## Communication Protocols

| Source                   | Target             | Protocol       | Direction          |
|--------------------------|--------------------|----------------|--------------------|
| PARKING_OPERATOR_ADAPTOR | DATA_BROKER        | Network gRPC   | Read (subscribe)   |
| PARKING_OPERATOR_ADAPTOR | DATA_BROKER        | Network gRPC   | Write (session)    |
| PARKING_OPERATOR_ADAPTOR | PARKING_OPERATOR   | HTTPS/REST     | Request/Response   |
| PARKING_APP (mock CLI)   | UPDATE_SERVICE     | Network gRPC   | Request/Response   |
| PARKING_APP (mock CLI)   | PARKING_OPERATOR_ADAPTOR | Network gRPC | Request/Response |
| UPDATE_SERVICE           | REGISTRY           | HTTPS/OCI      | Pull only          |

## VSS Signals Used

| Signal | Type | Direction | Component |
|--------|------|-----------|-----------|
| Vehicle.Cabin.Door.Row1.DriverSide.IsLocked | bool | Read | PARKING_OPERATOR_ADAPTOR |
| Vehicle.Parking.SessionActive | bool | Write | PARKING_OPERATOR_ADAPTOR |
| Vehicle.CurrentLocation.Latitude | double | Read | PARKING_OPERATOR_ADAPTOR |
| Vehicle.CurrentLocation.Longitude | double | Read | PARKING_OPERATOR_ADAPTOR |

## Port Assignments (Local Development)

| Service                    | Protocol | Port  |
|----------------------------|----------|-------|
| Eclipse Kuksa Databroker   | gRPC     | 55556 |
| UPDATE_SERVICE             | gRPC     | 50051 |
| PARKING_OPERATOR_ADAPTOR   | gRPC     | 50052 |
| Mock PARKING_OPERATOR      | HTTP     | 8090  |

## Technology Stack

- PARKING_OPERATOR_ADAPTOR: Rust (tonic, reqwest, tokio)
- UPDATE_SERVICE: Rust (tonic, tokio, oci-distribution or manual OCI pull)
- Mock PARKING_OPERATOR: Go (net/http, standard library)
- Mock PARKING_APP CLI: Go (cobra, gRPC client)
- Container management: podman/crun CLI invocations
- Proto definitions: Reused from Phase 1.2 (`proto/update_service.proto`,
  `proto/parking_adaptor.proto`, `proto/common.proto`)

## Out-of-Scope for This Spec

- PARKING_FEE_SERVICE implementation (Phase 2.4)
- Real OCI registry setup (uses mock/local registry for testing)
- Production container security (image signing, trust chains)
- Android PARKING_APP implementation (Phase 2.5)
- LOCKING_SERVICE and CLOUD_GATEWAY_CLIENT implementation (Phase 2.1)
- DATA_BROKER deployment (Phase 2.1, reused as local infrastructure)
- Real payment processing
- Production-grade TLS configuration

## Dependencies

| Spec | Relationship | Notes |
|------|-------------|-------|
| 01_project_setup | Depends on | Rust workspace, proto definitions, Makefile, mock CLI skeleton |
| 02_rhivos_safety | Depends on | DATA_BROKER running for lock event subscription, LOCKING_SERVICE writes lock state |
