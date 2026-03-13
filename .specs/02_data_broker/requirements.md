# Requirements Document

## Introduction

This document specifies the requirements for the DATA_BROKER component (Phase 2.1) of the SDV Parking Demo System. The scope covers configuring Eclipse Kuksa Databroker for dual-listener operation (TCP and UDS), validating the VSS overlay for custom signals, pinning the container image version, and verifying signal accessibility via integration tests. No wrapper code or reimplementation is written — this is purely configuration and validation of a pre-built component.

## Glossary

- **DATA_BROKER:** Eclipse Kuksa Databroker — a VSS-compliant vehicle signal broker providing gRPC pub/sub for vehicle signals. Deployed as a pre-built container image.
- **VSS:** Vehicle Signal Specification (COVESA standard) — a taxonomy for vehicle data signals.
- **VSS overlay:** A JSON file that extends the standard VSS tree with custom signals not in the default specification (e.g., `Vehicle.Parking.*`, `Vehicle.Command.*`).
- **UDS:** Unix Domain Socket — an IPC mechanism for same-host communication, used by same-partition consumers of the databroker.
- **TCP listener:** A network TCP socket (gRPC over HTTP/2) used by cross-partition and cross-domain consumers.
- **Dual listener:** Running the databroker with both a UDS and a TCP listener simultaneously.
- **gRPC:** Google Remote Procedure Call — the protocol used by Kuksa Databroker for signal pub/sub.
- **Kuksa Databroker:** The Eclipse Kuksa project's VSS-compliant data broker, providing get/set/subscribe operations on vehicle signals.
- **Permissive mode:** Running Kuksa Databroker without token-based authorization, allowing any client to read/write any signal.
- **Custom signal:** A VSS signal not in the standard VSS v5.1 tree, defined via the VSS overlay file.
- **Standard signal:** A VSS signal included in the default Kuksa Databroker image (VSS v5.1).

## Requirements

### Requirement 1: Dual Listener Configuration

**User Story:** As a service developer, I want the DATA_BROKER to listen on both TCP and UDS simultaneously, so that same-partition services use UDS for low-latency IPC and cross-partition services use TCP.

#### Acceptance Criteria

1. [02-REQ-1.1] THE `deployments/compose.yml` databroker service SHALL configure the Kuksa Databroker with a TCP listener on address `0.0.0.0:55555` (mapped to host port 55556).
2. [02-REQ-1.2] THE `deployments/compose.yml` databroker service SHALL configure the Kuksa Databroker with a UDS listener at path `/tmp/kuksa-databroker.sock`.
3. [02-REQ-1.3] THE `deployments/compose.yml` databroker service SHALL expose the UDS socket to the host via a volume mount so that same-host consumers can connect.
4. [02-REQ-1.4] WHEN the databroker container is running, THE databroker SHALL accept gRPC connections on both the TCP port (55556 from host) and the UDS socket simultaneously.

#### Edge Cases

1. [02-REQ-1.E1] IF the UDS socket file already exists from a previous run, THEN THE databroker SHALL overwrite it and start successfully.
2. [02-REQ-1.E2] IF a client connects to the TCP listener while another client is connected via UDS, THEN THE databroker SHALL serve both connections independently.

### Requirement 2: Container Image Version Pinning

**User Story:** As a developer, I want the Kuksa Databroker container image pinned to a specific version, so that builds are reproducible and not affected by upstream changes.

#### Acceptance Criteria

1. [02-REQ-2.1] THE `deployments/compose.yml` databroker service SHALL use a version-pinned container image (`ghcr.io/eclipse-kuksa/kuksa-databroker:0.5.1`) instead of `latest`.

#### Edge Cases

1. [02-REQ-2.E1] IF the pinned image version is not available in the registry, THEN THE `make infra-up` command SHALL fail with a clear image-pull error from Podman.

### Requirement 3: VSS Overlay Validation

**User Story:** As a service developer, I want the custom VSS signals to be accessible in the running databroker, so that downstream services can publish and subscribe to parking and command signals.

#### Acceptance Criteria

1. [02-REQ-3.1] WHEN the databroker is running with the VSS overlay, THE databroker SHALL expose the custom signal `Vehicle.Parking.SessionActive` with datatype `boolean`.
2. [02-REQ-3.2] WHEN the databroker is running with the VSS overlay, THE databroker SHALL expose the custom signal `Vehicle.Command.Door.Lock` with datatype `string`.
3. [02-REQ-3.3] WHEN the databroker is running with the VSS overlay, THE databroker SHALL expose the custom signal `Vehicle.Command.Door.Response` with datatype `string`.
4. [02-REQ-3.4] WHEN a client sets a value for a custom signal via gRPC, THE databroker SHALL store the value and return it on subsequent get requests.

#### Edge Cases

1. [02-REQ-3.E1] IF the VSS overlay file is malformed JSON, THEN THE databroker container SHALL fail to start and log an error.
2. [02-REQ-3.E2] IF a client attempts to get a custom signal that has never been set, THEN THE databroker SHALL return a response with no value (not an error).

### Requirement 4: Standard VSS Signal Availability

**User Story:** As a service developer, I want the standard VSS v5.1 signals to be available in the databroker, so that vehicle state signals (door lock, location, speed) can be used by all services.

#### Acceptance Criteria

1. [02-REQ-4.1] WHEN the databroker is running, THE databroker SHALL expose the standard signal `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` with datatype `boolean`.
2. [02-REQ-4.2] WHEN the databroker is running, THE databroker SHALL expose the standard signal `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` with datatype `boolean`.
3. [02-REQ-4.3] WHEN the databroker is running, THE databroker SHALL expose the standard signal `Vehicle.CurrentLocation.Latitude` with datatype `double`.
4. [02-REQ-4.4] WHEN the databroker is running, THE databroker SHALL expose the standard signal `Vehicle.CurrentLocation.Longitude` with datatype `double`.
5. [02-REQ-4.5] WHEN the databroker is running, THE databroker SHALL expose the standard signal `Vehicle.Speed` with datatype `float`.

#### Edge Cases

1. [02-REQ-4.E1] IF a client queries a signal path that does not exist in the VSS tree, THEN THE databroker SHALL return a NOT_FOUND error.

### Requirement 5: Signal Pub/Sub Verification

**User Story:** As a service developer, I want to verify that signals can be published and subscribed to via gRPC, so that I know the databroker is functional for downstream component integration.

#### Acceptance Criteria

1. [02-REQ-5.1] WHEN a client subscribes to a signal via gRPC and another client sets a value for that signal, THE subscribing client SHALL receive a notification with the updated value.
2. [02-REQ-5.2] WHEN a client sets a boolean signal to `true` and then reads it back, THE databroker SHALL return `true`.
3. [02-REQ-5.3] WHEN a client sets a string signal to a JSON payload and then reads it back, THE databroker SHALL return the exact same JSON string.

#### Edge Cases

1. [02-REQ-5.E1] IF a subscriber disconnects and reconnects, THEN THE databroker SHALL allow the new subscription without error.
