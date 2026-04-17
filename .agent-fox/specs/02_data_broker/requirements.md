# Requirements: DATA_BROKER (Spec 02)

## Introduction

This specification defines the requirements for configuring Eclipse Kuksa Databroker as the DATA_BROKER component in the RHIVOS safety partition. The DATA_BROKER is a pre-built binary (no custom wrapper code) that provides a VSS-compliant gRPC pub/sub interface for vehicle signals. This spec configures the existing container from spec 01 for dual listeners (TCP and UDS), validates the VSS overlay for all 8 signals, pins the Kuksa image version, and creates integration tests verifying databroker connectivity and signal operations.

## Glossary

| Term | Definition |
|------|-----------|
| DATA_BROKER | Eclipse Kuksa Databroker instance serving as the central vehicle signal hub |
| VSS | COVESA Vehicle Signal Specification, version 5.1 |
| UDS | Unix Domain Socket, used for same-partition gRPC communication |
| TCP | Network TCP transport for cross-partition gRPC (HTTP/2) communication |
| Overlay | A YAML file defining custom VSS signals not present in the standard specification |
| Permissive mode | Kuksa Databroker running without token-based authorization |
| Signal | A named data point in the VSS tree (e.g., Vehicle.Speed) with type, access, and metadata |

## Requirements

### 02-REQ-1: Kuksa Databroker image pinning

**User Story:** As a developer, I want the Kuksa Databroker container image pinned to a specific version so that builds are reproducible across environments.

**Requirements:**

- [02-REQ-1.1] The compose.yml SHALL specify the Kuksa Databroker image as `ghcr.io/eclipse-kuksa/kuksa-databroker:0.5.0`.
- [02-REQ-1.2] WHEN a developer runs `podman compose up`, the DATA_BROKER container SHALL start using the pinned image version.

**Acceptance Criteria:**

1. The compose.yml SHALL contain an image reference of exactly `ghcr.io/eclipse-kuksa/kuksa-databroker:0.5.0` for the databroker service.
2. The running container SHALL report version 0.5.0 in its startup logs.

**Edge Cases:**

- [02-REQ-1.E1] IF the pinned image is not available in the local cache or remote registry, THEN `podman compose up` SHALL fail with a clear image-pull error rather than falling back to another version.

---

### 02-REQ-2: TCP listener configuration

**User Story:** As a cross-partition service, I want to connect to the DATA_BROKER over network TCP so that I can read and write vehicle signals from outside the safety partition.

**Requirements:**

- [02-REQ-2.1] The DATA_BROKER container SHALL listen on TCP address `0.0.0.0:55555` inside the container.
- [02-REQ-2.2] The compose.yml SHALL map container port 55555 to host port 55556.

**Acceptance Criteria:**

1. A gRPC client connecting to `localhost:55556` from the host SHALL receive a successful gRPC channel connection.
2. The compose.yml SHALL contain port mapping `55556:55555` for the databroker service.
3. The DATA_BROKER command args SHALL include `--address 0.0.0.0 --port 55555`.

**Edge Cases:**

- [02-REQ-2.E1] IF port 55556 is already in use on the host, THEN `podman compose up` SHALL fail with a port-conflict error.

---

### 02-REQ-3: UDS listener configuration

**User Story:** As a same-partition service (LOCKING_SERVICE, CLOUD_GATEWAY_CLIENT), I want to connect to the DATA_BROKER via Unix Domain Socket so that I can achieve low-latency, partition-local communication.

**Requirements:**

- [02-REQ-3.1] The DATA_BROKER container SHALL listen on UDS path `/tmp/kuksa-databroker.sock`.
- [02-REQ-3.2] The compose.yml SHALL configure a shared volume mount exposing the UDS socket directory to co-located containers.

**Acceptance Criteria:**

1. The DATA_BROKER command args SHALL include `--unix-socket /tmp/kuksa-databroker.sock`.
2. The compose.yml SHALL define a named volume or bind mount that makes the UDS socket accessible to same-partition consumer containers.
3. A gRPC client connecting via `unix:///tmp/kuksa-databroker.sock` from a co-located container SHALL receive a successful gRPC channel connection.

**Edge Cases:**

- [02-REQ-3.E1] IF the UDS socket file does not exist at startup (first run), THEN the DATA_BROKER SHALL create it automatically.
- [02-REQ-3.E2] IF a stale UDS socket file exists from a previous run, THEN the DATA_BROKER SHALL replace it and accept new connections.

---

### 02-REQ-4: Dual listener operation

**User Story:** As a developer, I want the DATA_BROKER to serve both TCP and UDS listeners simultaneously so that both cross-partition and same-partition consumers can connect concurrently.

**Requirements:**

- [02-REQ-4.1] WHEN the DATA_BROKER container starts, it SHALL activate both the TCP listener (port 55555) and the UDS listener (`/tmp/kuksa-databroker.sock`) simultaneously.

**Acceptance Criteria:**

1. A gRPC client connecting via TCP (`localhost:55556`) and a gRPC client connecting via UDS (`unix:///tmp/kuksa-databroker.sock`) SHALL both be able to read and write signals concurrently.
2. A signal written via TCP SHALL be readable via UDS, and vice versa.

**Edge Cases:**

- [02-REQ-4.E1] IF one listener fails to bind, THEN the DATA_BROKER SHALL log the error and exit with a non-zero status code rather than running with only one listener.

---

### 02-REQ-5: Standard VSS signal availability

**User Story:** As a vehicle service developer, I want the 5 standard VSS v5.1 signals to be available in the DATA_BROKER so that I can read and write vehicle state data.

**Requirements:**

- [02-REQ-5.1] The DATA_BROKER SHALL include the following standard VSS v5.1 signals in its metadata: `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` (bool), `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` (bool), `Vehicle.CurrentLocation.Latitude` (double), `Vehicle.CurrentLocation.Longitude` (double), `Vehicle.Speed` (float).
- [02-REQ-5.2] Each standard signal SHALL be retrievable via gRPC GetMetadata or equivalent introspection call.
- [02-REQ-5.3] The compose.yml SHALL load the bundled standard VSS release tree (e.g., `vss_release_4.0.json`) via the `--vss` CLI flag alongside the custom overlay, since standard signals are not available by default without an explicit VSS tree.

**Acceptance Criteria:**

1. A metadata query for each of the 5 standard signals SHALL return a valid entry with the correct VSS data type.
2. Each standard signal SHALL support set and get operations via gRPC.

**Edge Cases:**

- [02-REQ-5.E1] IF a standard signal is missing from the Kuksa Databroker's built-in VSS tree, THEN the integration tests SHALL fail and report the missing signal by name.

---

### 02-REQ-6: Custom VSS signal availability via overlay

**User Story:** As a vehicle service developer, I want the 3 custom VSS signals defined in the overlay to be available in the DATA_BROKER so that parking and command workflows can operate.

**Requirements:**

- [02-REQ-6.1] The VSS overlay file SHALL define `Vehicle.Parking.SessionActive` as type `boolean`.
- [02-REQ-6.2] The VSS overlay file SHALL define `Vehicle.Command.Door.Lock` as type `string`.
- [02-REQ-6.3] The VSS overlay file SHALL define `Vehicle.Command.Door.Response` as type `string`.
- [02-REQ-6.4] The DATA_BROKER SHALL load the VSS overlay file at startup via the appropriate CLI flag or configuration.
- [02-REQ-6.5] The VSS overlay file SHALL include intermediate branch nodes (`Vehicle.Parking`, `Vehicle.Command`, `Vehicle.Command.Door`) with `type: "branch"`, as required by the Kuksa Databroker flat JSON format for custom signal paths not present in the standard VSS tree.

**Acceptance Criteria:**

1. A metadata query for `Vehicle.Parking.SessionActive` SHALL return type `boolean`.
2. A metadata query for `Vehicle.Command.Door.Lock` SHALL return type `string`.
3. A metadata query for `Vehicle.Command.Door.Response` SHALL return type `string`.
4. The compose.yml SHALL mount the overlay file into the container and reference it in the databroker command args.

**Edge Cases:**

- [02-REQ-6.E1] IF the overlay file contains a syntax error, THEN the DATA_BROKER SHALL fail to start and log a parse error.
- [02-REQ-6.E2] IF the overlay file path is missing or inaccessible, THEN the DATA_BROKER SHALL fail to start and log a file-not-found error.

---

### 02-REQ-7: Permissive mode operation

**User Story:** As a demo developer, I want the DATA_BROKER to run without token-based authorization so that integration testing is simplified.

**Requirements:**

- [02-REQ-7.1] The DATA_BROKER SHALL run in permissive mode with no token authentication required.

**Acceptance Criteria:**

1. A gRPC client connecting without any authorization token SHALL be able to read and write all signals.
2. The DATA_BROKER command args SHALL NOT include any token or authorization flags.

**Edge Cases:**

- [02-REQ-7.E1] IF a client sends a request with an arbitrary or invalid token, THEN the DATA_BROKER in permissive mode SHALL still accept the request.

---

### 02-REQ-8: Signal read/write operations via TCP

**User Story:** As a cross-partition consumer, I want to set and get signal values via the TCP gRPC interface so that I can interact with vehicle data remotely.

**Requirements:**

- [02-REQ-8.1] WHEN a client sets a signal value via TCP gRPC, THEN the DATA_BROKER SHALL store the value and return it on subsequent get requests.
- [02-REQ-8.2] WHEN a client sets `Vehicle.Parking.SessionActive` to `true` via TCP, THEN a subsequent get request via TCP SHALL return `true`.

**Acceptance Criteria:**

1. Setting `Vehicle.Parking.SessionActive` to `true` via TCP gRPC and then getting it SHALL return `true`.
2. Setting `Vehicle.Command.Door.Lock` to a JSON string via TCP gRPC and then getting it SHALL return the same JSON string.
3. Setting `Vehicle.Speed` to `50.0` via TCP gRPC and then getting it SHALL return `50.0`.

**Edge Cases:**

- [02-REQ-8.E1] IF a client attempts to set a signal that does not exist in the VSS tree, THEN the DATA_BROKER SHALL return a gRPC NOT_FOUND error.

---

### 02-REQ-9: Signal read/write operations via UDS

**User Story:** As a same-partition consumer, I want to set and get signal values via the UDS gRPC interface so that I can interact with vehicle data with minimal latency.

**Requirements:**

- [02-REQ-9.1] WHEN a client sets a signal value via UDS gRPC, THEN the DATA_BROKER SHALL store the value and return it on subsequent get requests.
- [02-REQ-9.2] Signal values written via UDS SHALL be readable via TCP, and vice versa.

**Acceptance Criteria:**

1. Setting `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` to `true` via UDS gRPC and then getting it via UDS SHALL return `true`.
2. Setting a signal via UDS and getting it via TCP SHALL return the same value.
3. Setting a signal via TCP and getting it via UDS SHALL return the same value.

**Edge Cases:**

- [02-REQ-9.E1] IF the UDS socket is disconnected mid-operation, THEN the client SHALL receive a gRPC UNAVAILABLE error.

---

### 02-REQ-10: Signal subscription notifications

**User Story:** As a vehicle service, I want to subscribe to signal changes so that I receive real-time notifications when signal values are updated.

**Requirements:**

- [02-REQ-10.1] WHEN a client subscribes to a signal via gRPC (TCP or UDS), THEN the DATA_BROKER SHALL deliver update notifications when the signal value changes.

**Acceptance Criteria:**

1. A client subscribing to `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` via TCP SHALL receive a notification when the signal is set to `true` by another client.
2. A client subscribing to `Vehicle.Parking.SessionActive` via UDS SHALL receive a notification when the signal is set by another client via TCP.

**Edge Cases:**

- [02-REQ-10.E1] IF a subscriber disconnects and reconnects, THEN the subscriber SHALL be able to re-subscribe and receive subsequent updates without missing the current value.
