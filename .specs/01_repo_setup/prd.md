# PRD: Repository Setup (Phase 1.2)

> Extracted from the [main PRD](../prd.md). This spec covers Phase 1.2: project
> structure, build system, proto definitions, local infrastructure, and mock CLI
> apps.

## Scope

From the main PRD, Phase 1.2:

- Setup the code repo, with dedicated sub-folders for each type of code:
  - RHIVOS services: Rust
  - Android Automotive OS app: Kotlin (placeholder only)
  - Android app: Flutter/Dart (placeholder only)
  - Backend-services: Golang
- Create skeleton implementations for each component, except for the Android
  Automotive OS and Android app. Just keep placeholder directories.
- Create mock CLI apps for PARKING_APP and COMPANION_APP (see *Android
  Development Separation* in the main PRD) so that backend-services and RHIVOS
  components can be integration-tested without real Android builds.
- Create local build capabilities for each toolchain using make.
- Setup local infrastructure, used for local unit and integration testing.
- Setup local unit and integration testing capabilities.

### Components to scaffold

| Component                | Domain              | Language | Skeleton? |
|--------------------------|---------------------|----------|-----------|
| LOCKING_SERVICE          | RHIVOS Safety       | Rust     | Yes       |
| DATA_BROKER              | RHIVOS Safety       | N/A      | Config only (Kuksa binary) |
| CLOUD_GATEWAY_CLIENT     | RHIVOS Safety       | Rust     | Yes       |
| PARKING_OPERATOR_ADAPTOR | RHIVOS QM           | Rust     | Yes       |
| UPDATE_SERVICE           | RHIVOS QM           | Rust     | Yes       |
| PARKING_FEE_SERVICE      | Backend             | Go       | Yes       |
| CLOUD_GATEWAY            | Backend             | Go       | Yes       |
| Mock PARKING_APP CLI     | Mock                | Go       | Yes       |
| Mock COMPANION_APP CLI   | Mock                | Go       | Yes       |
| Mock sensors             | Mock                | Rust     | Yes       |
| PARKING_APP              | AAOS                | Kotlin   | Placeholder dir only |
| COMPANION_APP            | Android             | Flutter  | Placeholder dir only |

### Proto definitions to create

| Proto file               | Purpose                                         |
|--------------------------|-------------------------------------------------|
| update_service.proto     | UPDATE_SERVICE gRPC interface                   |
| parking_adapter.proto    | PARKING_OPERATOR_ADAPTOR common gRPC interface  |
| common.proto             | Shared message types (VIN, location, errors)    |

DATA_BROKER (Kuksa) and CLOUD_GATEWAY/PARKING_FEE_SERVICE (REST) do not need
custom proto definitions. Kuksa's proto is consumed as-is; REST services use
OpenAPI or handler definitions.

### Local infrastructure

| Service             | Image                  | Purpose                          |
|---------------------|------------------------|----------------------------------|
| Eclipse Kuksa       | ghcr.io/eclipse-kuksa  | DATA_BROKER (VSS databroker)     |
| Eclipse Mosquitto   | eclipse-mosquitto      | MQTT broker for CLOUD_GATEWAY    |

### Communication clarifications

- Same-partition services (within RHIVOS Safety or within RHIVOS QM) use gRPC
  over Unix Domain Sockets (UDS).
- Cross-partition services (e.g., PARKING_OPERATOR_ADAPTOR in QM to DATA_BROKER
  in Safety) use network gRPC (TCP/HTTP2, TLS in production, plaintext for local
  dev).
- PARKING_APP (AAOS) to RHIVOS services use network gRPC.
- PARKING_APP to PARKING_FEE_SERVICE uses HTTPS/REST.
- COMPANION_APP to CLOUD_GATEWAY uses HTTPS/REST.
- CLOUD_GATEWAY to CLOUD_GATEWAY_CLIENT uses MQTT over TLS (via Mosquitto).

## Dependencies

This is the first spec. No dependencies on other specs.

| Spec | Relationship | Notes |
|------|-------------|-------|
| (none) | — | Foundation for all subsequent specs |

## Clarifications

The following clarifications were obtained from the product owner and apply
globally across all specs derived from the main PRD.

### Architecture

- **A1 (COMPANION_APP protocol):** CLOUD_GATEWAY exposes two interfaces: REST
  towards mobile apps (COMPANION_APP) and MQTT towards vehicle
  (CLOUD_GATEWAY_CLIENT). The COMPANION_APP uses REST exclusively.

- **A2 (Command flow to LOCKING_SERVICE):** CLOUD_GATEWAY_CLIENT publishes
  validated commands to DATA_BROKER. LOCKING_SERVICE subscribes to DATA_BROKER
  for command signals. There are no direct service calls between
  CLOUD_GATEWAY_CLIENT and LOCKING_SERVICE.

- **A3 (PARKING_FEE_SERVICE ↔ REGISTRY):** PARKING_FEE_SERVICE acts as a
  gatekeeper for an external OCI registry (Google Artifact Registry). It does
  not run its own registry.

- **A4 (Adapter discovery flow):** PARKING_FEE_SERVICE exposes a location-based
  lookup endpoint. The PARKING_APP sends the vehicle's location and receives a
  list of feasible parking operators. Once the user/PARKING_APP selects an
  operator, the PARKING_APP requests adapter metadata (image ref, checksum)
  needed for secure installation via UPDATE_SERVICE.

- **A5 (Adapter offloading):** Offloading is triggered by either a configurable
  inactivity timer OR when RHIVOS QM resources become scarce.

### Protocols

- **I1 (COMPANION_APP):** Confirmed: COMPANION_APP uses REST towards
  CLOUD_GATEWAY.

- **I2 (Cross-partition transport):** Confirmed: cross-partition gRPC uses
  network TCP (not UDS).

- **I3 (DATA_BROKER):** Confirmed: DATA_BROKER IS Eclipse Kuksa Databroker,
  deployed as a pre-built binary. No wrapper or reimplementation.

### Interfaces

- **U1 (PARKING_FEE_SERVICE REST API):** Endpoints include: lookup operator by
  location, get adapter info, start session, stop session, get fee, health
  check.

- **U2 (Adapter common interface):** gRPC methods: StartSession, StopSession,
  GetStatus, GetRate.

- **U3 (Zone definition):** Zones are geofence polygons defined by lat/lon
  coordinates. Lookups allow fuzziness — a vehicle "near" a parking zone is a
  potential match. Secondary option (future): address-based lookup with reverse
  geo-lookup.

- **U4 (Authentication):** Bearer tokens for the demo.

- **U5 (Adapter lifecycle states):** Full state machine includes: UNKNOWN,
  DOWNLOADING, INSTALLING, RUNNING, STOPPED, ERROR, OFFLOADING.

- **U7 (UPDATE_SERVICE interface):** Full interface includes: InstallAdapter,
  WatchAdapterStates, ListAdapters, RemoveAdapter, GetAdapterStatus.

### Implementation

- **U6 (Mock services):** CLI tools that simulate sensor data. They connect to
  DATA_BROKER via gRPC. Android HAL integration is a later-stage concern.

- **U8 (Demo adapters):** Pre-built container images pushed manually to the
  registry. No automated approval workflow.

### Deployment

- **IA2 (MQTT):** Use containerized Eclipse Mosquitto for local development.
  Production MQTT setup is deferred.

- **IA4 (Zones):** Hardcoded but realistic geofence polygons. Specific zone data
  to be defined later.

- **IA5 (Container runtime):** Podman on RHIVOS; OpenShift for cloud.

- **IA6 (Vehicle identity):** VINs are self-created. Vehicles self-register on
  startup. The demo must support many virtual devices/cars simultaneously.
