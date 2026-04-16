# Requirements Document

## Introduction

This document specifies the requirements for the UPDATE_SERVICE component (Phase 2.3) of the SDV Parking Demo System. The UPDATE_SERVICE is a Rust gRPC service running in the RHIVOS QM partition that manages the lifecycle of containerized PARKING_OPERATOR_ADAPTORs: pulling OCI images from a registry, verifying integrity via SHA-256 checksum, installing and running containers via podman, monitoring adapter states, and automatically offloading unused adapters after a configurable inactivity period.

## Glossary

- **UPDATE_SERVICE:** A Rust gRPC service that manages the lifecycle of containerized PARKING_OPERATOR_ADAPTORs in the RHIVOS QM partition.
- **PARKING_OPERATOR_ADAPTOR:** A containerized application that interfaces between the vehicle and a specific parking operator.
- **Adapter:** Short for PARKING_OPERATOR_ADAPTOR container instance managed by UPDATE_SERVICE.
- **Adapter ID:** A deterministic, human-readable identifier derived from the OCI image reference by extracting the last path segment and tag (e.g., `us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0` becomes `parkhaus-munich-v1.0.0`).
- **Job ID:** A UUID v4 generated per `InstallAdapter` call, used to track the installation operation.
- **OCI image reference:** A container image address in the format `registry/repository/name:tag`.
- **Checksum verification:** Comparing the SHA-256 digest of the pulled OCI image manifest (obtained via `podman image inspect --format '{{.Digest}}'`) against the checksum provided by the caller. The checksum includes the `sha256:` prefix.
- **Adapter state:** One of: UNKNOWN, DOWNLOADING, INSTALLING, RUNNING, STOPPED, ERROR, OFFLOADING.
- **Offloading:** The process of removing both the container and image of an unused adapter to free storage and system resources.
- **Inactivity timer:** A configurable duration (default 24 hours) after which a STOPPED adapter is automatically offloaded.
- **podman:** A daemonless container engine used to pull, run, stop, and remove OCI containers.
- **tonic:** A Rust gRPC framework used for the UPDATE_SERVICE server implementation.
- **tokio:** An asynchronous Rust runtime used for concurrent operations and process spawning.

## Requirements

### Requirement 1: Install Adapter

**User Story:** As a PARKING_APP, I want to install and start a parking operator adapter by providing an image reference and checksum, so that the adapter is available for parking sessions.

#### Acceptance Criteria

1. [07-REQ-1.1] WHEN an `InstallAdapter(image_ref, checksum_sha256)` RPC is received, THE service SHALL return an `InstallAdapterResponse` containing a `job_id` (UUID v4), the derived `adapter_id`, and the initial state `DOWNLOADING`.
2. [07-REQ-1.2] THE service SHALL pull the OCI image by executing `podman pull <image_ref>` via `tokio::process::Command`.
3. [07-REQ-1.3] AFTER a successful pull, THE service SHALL verify the image digest by executing `podman image inspect --format '{{.Digest}}' <image_ref>` and comparing the output to the provided `checksum_sha256`.
4. [07-REQ-1.4] WHEN the checksum matches, THE service SHALL transition the adapter state from DOWNLOADING to INSTALLING, then start the container by executing `podman run -d --name <adapter_id> --network=host <image_ref>`.
5. [07-REQ-1.5] WHEN the container starts successfully, THE service SHALL transition the adapter state from INSTALLING to RUNNING.
6. [07-REQ-1.6] THE service SHALL derive the `adapter_id` from `image_ref` by extracting the last path segment and replacing the colon with a hyphen (e.g., `parkhaus-munich:v1.0.0` becomes `parkhaus-munich-v1.0.0`).

#### Edge Cases

1. [07-REQ-1.E1] IF `image_ref` is empty, THEN THE service SHALL return gRPC status `INVALID_ARGUMENT` with message `"image_ref is required"`.
2. [07-REQ-1.E2] IF `checksum_sha256` is empty, THEN THE service SHALL return gRPC status `INVALID_ARGUMENT` with message `"checksum_sha256 is required"`.
3. [07-REQ-1.E3] IF `podman pull` fails (non-zero exit code), THEN THE service SHALL transition the adapter state to ERROR and emit an error state event with the podman stderr output.
4. [07-REQ-1.E4] IF the checksum does not match, THEN THE service SHALL transition the adapter state to ERROR, remove the pulled image via `podman rmi <image_ref>`, and emit an error state event with reason `"checksum_mismatch"`.
5. [07-REQ-1.E5] IF `podman run` fails (non-zero exit code), THEN THE service SHALL transition the adapter state to ERROR and emit an error state event with the podman stderr output.

### Requirement 2: Single Adapter Constraint

**User Story:** As a vehicle system, I want only one adapter running at a time, so that resources are not wasted on multiple concurrent adapters (a vehicle parks at one location at a time).

#### Acceptance Criteria

1. [07-REQ-2.1] WHEN `InstallAdapter` is called while another adapter is in state RUNNING, THE service SHALL stop the currently running adapter by executing `podman stop <adapter_id>` before starting the new one.
2. [07-REQ-2.2] AFTER stopping the previously running adapter, THE service SHALL transition it to state STOPPED and emit a state event before proceeding with the new installation.

#### Edge Cases

1. [07-REQ-2.E1] IF stopping the currently running adapter fails (podman stop returns non-zero), THEN THE service SHALL transition the old adapter to ERROR, emit an error state event, and still proceed with installing the new adapter.

### Requirement 3: Watch Adapter States

**User Story:** As a PARKING_APP, I want to subscribe to a stream of adapter state transitions, so that I can display real-time installation progress and adapter status.

#### Acceptance Criteria

1. [07-REQ-3.1] WHEN a `WatchAdapterStates()` RPC is received, THE service SHALL return a server-streaming response of `AdapterStateEvent` messages.
2. [07-REQ-3.2] EACH `AdapterStateEvent` SHALL contain `adapter_id`, `old_state`, `new_state`, and `timestamp`.
3. [07-REQ-3.3] THE service SHALL emit a state event for every adapter state transition to all active subscribers.
4. [07-REQ-3.4] THE stream SHALL send only new state transitions occurring after the subscription starts; it SHALL NOT replay historical events.

#### Edge Cases

1. [07-REQ-3.E1] IF a subscriber disconnects, THEN THE service SHALL remove the subscriber from the active list without affecting other subscribers or service operation.

### Requirement 4: List and Query Adapters

**User Story:** As a PARKING_APP, I want to list all known adapters and query individual adapter status, so that I can display current adapter information.

#### Acceptance Criteria

1. [07-REQ-4.1] WHEN a `ListAdapters()` RPC is received, THE service SHALL return a list of all known adapters with their current states and adapter IDs.
2. [07-REQ-4.2] WHEN a `GetAdapterStatus(adapter_id)` RPC is received, THE service SHALL return the current state of the specified adapter.

#### Edge Cases

1. [07-REQ-4.E1] IF `GetAdapterStatus` is called with an unknown `adapter_id`, THEN THE service SHALL return gRPC status `NOT_FOUND` with message `"adapter not found"`.
2. [07-REQ-4.E2] IF no adapters have been installed, THEN `ListAdapters` SHALL return an empty list.

### Requirement 5: Remove Adapter

**User Story:** As a PARKING_APP, I want to explicitly remove an adapter, so that I can free resources when an adapter is no longer needed.

#### Acceptance Criteria

1. [07-REQ-5.1] WHEN a `RemoveAdapter(adapter_id)` RPC is received, THE service SHALL stop the adapter container (if running) via `podman stop`, remove the container via `podman rm`, and remove the image via `podman rmi`.
2. [07-REQ-5.2] AFTER removal, THE service SHALL remove the adapter from in-memory state entirely and return a success response.

#### Edge Cases

1. [07-REQ-5.E1] IF `RemoveAdapter` is called with an unknown `adapter_id`, THEN THE service SHALL return gRPC status `NOT_FOUND` with message `"adapter not found"`.
2. [07-REQ-5.E2] IF any podman removal command fails, THEN THE service SHALL transition the adapter to ERROR and return gRPC status `INTERNAL` with the podman error details.

### Requirement 6: Automatic Offloading

**User Story:** As a vehicle system, I want unused adapters to be automatically offloaded after a configurable period of inactivity, so that storage and system resources are freed.

#### Acceptance Criteria

1. [07-REQ-6.1] WHEN an adapter has been in state STOPPED for longer than the configured inactivity timeout (default 24 hours), THE service SHALL transition the adapter to OFFLOADING and begin cleanup.
2. [07-REQ-6.2] DURING offloading, THE service SHALL remove the container via `podman rm` and the image via `podman rmi`.
3. [07-REQ-6.3] AFTER offloading completes, THE service SHALL remove the adapter from in-memory state entirely.
4. [07-REQ-6.4] THE service SHALL emit state events for the STOPPED to OFFLOADING transition to all active WatchAdapterStates subscribers.

#### Edge Cases

1. [07-REQ-6.E1] IF offloading cleanup fails (podman rm or rmi returns non-zero), THEN THE service SHALL transition the adapter to ERROR and emit an error state event.

### Requirement 7: Configuration

**User Story:** As a deployer, I want to configure the service via a JSON file, so that I can adjust settings without code changes.

#### Acceptance Criteria

1. [07-REQ-7.1] WHEN the service starts, THE service SHALL load configuration from the file path specified by the `CONFIG_PATH` environment variable, defaulting to `config.json` in the working directory.
2. [07-REQ-7.2] THE configuration SHALL support the following fields: `grpc_port` (default 50052), `registry_url` (string), `inactivity_timeout_secs` (default 86400), and `container_storage_path` (default `/var/lib/containers/adapters/`).
3. [07-REQ-7.3] THE service SHALL listen for gRPC connections on `0.0.0.0:<grpc_port>`.

#### Edge Cases

1. [07-REQ-7.E1] IF the configuration file does not exist, THEN THE service SHALL start with built-in defaults and log a warning.
2. [07-REQ-7.E2] IF the configuration file contains invalid JSON, THEN THE service SHALL exit with a non-zero code and log a descriptive error.

### Requirement 8: State Event Emission

**User Story:** As a PARKING_APP subscriber, I want to receive state events for all adapter transitions, so that I can track the complete lifecycle of each adapter.

#### Acceptance Criteria

1. [07-REQ-8.1] THE service SHALL emit an `AdapterStateEvent` for every state transition: UNKNOWN to DOWNLOADING, DOWNLOADING to INSTALLING, INSTALLING to RUNNING, RUNNING to STOPPED, STOPPED to OFFLOADING, and any transition to ERROR.
2. [07-REQ-8.2] EACH emitted event SHALL contain a Unix timestamp (seconds since epoch) indicating when the transition occurred.
3. [07-REQ-8.3] THE service SHALL support multiple simultaneous WatchAdapterStates subscribers, delivering the same events to all.

#### Edge Cases

1. [07-REQ-8.E1] IF no subscribers are active when a state transition occurs, THE service SHALL still update the in-memory state but discard the event (no buffering).

### Requirement 9: Container Exit Monitoring

**User Story:** As a vehicle system, I want the service to detect when an adapter container exits unexpectedly, so that the adapter state reflects reality.

#### Acceptance Criteria

1. [07-REQ-9.1] WHEN a RUNNING adapter's container exits with a non-zero exit code, THE service SHALL transition the adapter to ERROR and emit a state event.
2. [07-REQ-9.2] WHEN a RUNNING adapter's container exits with exit code 0, THE service SHALL transition the adapter to STOPPED and emit a state event.

#### Edge Cases

1. [07-REQ-9.E1] IF `podman wait` or container status polling fails, THEN THE service SHALL transition the adapter to ERROR and emit a state event.

### Requirement 10: Graceful Lifecycle

**User Story:** As an operator, I want the service to start and stop cleanly.

#### Acceptance Criteria

1. [07-REQ-10.1] WHEN the service starts, THE service SHALL log its configuration (port, inactivity timeout) and a ready message.
2. [07-REQ-10.2] WHEN the service receives SIGTERM or SIGINT, THE service SHALL stop accepting new RPCs, complete in-flight RPCs, and exit with code 0.

#### Edge Cases

1. [07-REQ-10.E1] IF in-flight RPCs do not complete within 10 seconds of receiving a shutdown signal, THEN THE service SHALL force-terminate and exit with code 0.
