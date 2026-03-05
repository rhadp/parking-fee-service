# Requirements: UPDATE_SERVICE (Spec 07)

> EARS-syntax requirements for the UPDATE_SERVICE adapter lifecycle manager.
> Derived from the PRD at `.specs/07_update_service/prd.md` and the master PRD at `.specs/prd.md`.

## Introduction

The UPDATE_SERVICE is a Rust gRPC service running in the RHIVOS QM partition that manages the full lifecycle of containerized PARKING_OPERATOR_ADAPTORs. It pulls OCI images from Google Artifact Registry, verifies integrity via SHA-256 checksums, installs and runs containers using podman, streams state transitions to clients, enforces a single-running-adapter constraint, and automatically offloads unused adapters after a configurable inactivity period.

## Glossary

| Term | Definition |
|------|-----------|
| Adapter | A containerized PARKING_OPERATOR_ADAPTOR managed by UPDATE_SERVICE |
| OCI image | A container image conforming to the Open Container Initiative specification |
| image_ref | A fully qualified OCI image reference (registry/repository:tag) |
| checksum_sha256 | SHA-256 hash of the OCI manifest digest, used for integrity verification |
| Adapter state | One of: UNKNOWN, DOWNLOADING, INSTALLING, RUNNING, STOPPED, ERROR, OFFLOADING |
| Offloading | The process of stopping and removing an unused adapter to reclaim resources |
| Inactivity timer | A configurable duration after which a stopped adapter is automatically offloaded |
| podman | A daemonless container engine used to manage adapter containers |
| tonic | A Rust gRPC framework built on tokio |

## Requirements

### Requirement 1: Install Adapter

**User Story:** As a PARKING_APP, I want to install and start a parking operator adapter by providing an OCI image reference and checksum, so that the adapter is ready to manage parking sessions.

#### Acceptance Criteria

1. **07-REQ-1.1** WHEN a client calls `InstallAdapter(image_ref, checksum_sha256)`, THE UPDATE_SERVICE SHALL pull the OCI image from the registry, verify the checksum, install the container, start it, and return an `InstallAdapterResponse` containing `job_id`, `adapter_id`, and the initial `state` (DOWNLOADING).
2. **07-REQ-1.2** WHEN the adapter image is successfully pulled and verified, THE UPDATE_SERVICE SHALL transition the adapter state through DOWNLOADING, INSTALLING, and RUNNING in sequence, emitting a state event for each transition.
3. **07-REQ-1.3** WHEN `InstallAdapter` is called with an `image_ref` that is already installed and in RUNNING state, THE UPDATE_SERVICE SHALL return ALREADY_EXISTS with the existing adapter's ID and state.

#### Edge Cases

1. **07-REQ-1.E1** IF the OCI registry is unreachable during an `InstallAdapter` call, THEN THE UPDATE_SERVICE SHALL transition the adapter to ERROR state and return a gRPC UNAVAILABLE status with a descriptive error message.
2. **07-REQ-1.E2** IF the container fails to start after installation, THEN THE UPDATE_SERVICE SHALL transition the adapter to ERROR state and return a gRPC INTERNAL status with details about the failure.

### Requirement 2: Checksum Verification

**User Story:** As a PARKING_APP, I want the UPDATE_SERVICE to verify the integrity of downloaded adapter images, so that only trusted adapters are installed on the vehicle.

#### Acceptance Criteria

1. **07-REQ-2.1** AFTER pulling an OCI image, THE UPDATE_SERVICE SHALL compute the SHA-256 hash of the OCI manifest digest and compare it to the `checksum_sha256` value provided in the `InstallAdapter` request.
2. **07-REQ-2.2** WHEN the computed checksum matches the provided checksum, THE UPDATE_SERVICE SHALL proceed with installation.

#### Edge Cases

1. **07-REQ-2.E1** IF the computed checksum does not match the provided checksum, THEN THE UPDATE_SERVICE SHALL transition the adapter to ERROR state, discard the downloaded image, and return a gRPC INVALID_ARGUMENT status with a message indicating checksum mismatch.

### Requirement 3: Watch Adapter States

**User Story:** As a PARKING_APP, I want to subscribe to a stream of adapter state transitions, so that I can display real-time installation progress and adapter status to the driver.

#### Acceptance Criteria

1. **07-REQ-3.1** WHEN a client calls `WatchAdapterStates()`, THE UPDATE_SERVICE SHALL return a server-streaming gRPC response that emits an `AdapterStateEvent` for every adapter state transition that occurs while the stream is open.
2. **07-REQ-3.2** EACH `AdapterStateEvent` SHALL contain `adapter_id`, `old_state`, `new_state`, and `timestamp`.

### Requirement 4: List and Query Adapters

**User Story:** As a PARKING_APP, I want to list all adapters and query individual adapter status, so that I can determine which adapters are available.

#### Acceptance Criteria

1. **07-REQ-4.1** WHEN a client calls `ListAdapters()`, THE UPDATE_SERVICE SHALL return a list of all known adapters with their current state, adapter_id, and image_ref.
2. **07-REQ-4.2** WHEN a client calls `GetAdapterStatus(adapter_id)`, THE UPDATE_SERVICE SHALL return the current state and metadata of the specified adapter.

#### Edge Cases

1. **07-REQ-4.E1** IF `GetAdapterStatus` is called with an unknown `adapter_id`, THEN THE UPDATE_SERVICE SHALL return a gRPC NOT_FOUND status.

### Requirement 5: Remove Adapter

**User Story:** As a PARKING_APP, I want to explicitly remove an adapter, so that I can free up resources or prepare for a different adapter.

#### Acceptance Criteria

1. **07-REQ-5.1** WHEN a client calls `RemoveAdapter(adapter_id)` for an adapter in RUNNING or STOPPED state, THE UPDATE_SERVICE SHALL stop the container (if running), remove it, and transition the adapter to OFFLOADING then remove it from the adapter list.

#### Edge Cases

1. **07-REQ-5.E1** IF `RemoveAdapter` is called with an unknown `adapter_id`, THEN THE UPDATE_SERVICE SHALL return a gRPC NOT_FOUND status.

### Requirement 6: Adapter Lifecycle State Machine

**User Story:** As a system operator, I want adapter state transitions to follow a well-defined state machine, so that the system behavior is predictable and debuggable.

#### Acceptance Criteria

1. **07-REQ-6.1** THE UPDATE_SERVICE SHALL enforce the following valid state transitions: UNKNOWN->DOWNLOADING, DOWNLOADING->INSTALLING, DOWNLOADING->ERROR, INSTALLING->RUNNING, INSTALLING->ERROR, RUNNING->STOPPED, RUNNING->ERROR, STOPPED->RUNNING, STOPPED->OFFLOADING.
2. **07-REQ-6.2** THE UPDATE_SERVICE SHALL reject any state transition not listed in 07-REQ-6.1 and log a warning.

### Requirement 7: Single Adapter Constraint

**User Story:** As a system designer, I want only one adapter running at a time per vehicle, so that resource usage is bounded and parking session conflicts are avoided.

#### Acceptance Criteria

1. **07-REQ-7.1** WHEN `InstallAdapter` is called while another adapter is in RUNNING state, THE UPDATE_SERVICE SHALL stop the currently running adapter (transitioning it to STOPPED) before starting the new adapter.
2. **07-REQ-7.2** AT NO TIME SHALL more than one adapter be in the RUNNING state simultaneously.

### Requirement 8: Automatic Offloading

**User Story:** As a system operator, I want unused adapters to be automatically removed after a period of inactivity, so that vehicle storage and resources are not consumed indefinitely.

#### Acceptance Criteria

1. **07-REQ-8.1** WHEN an adapter has been in STOPPED state for longer than the configured inactivity timeout (default: 24 hours), THE UPDATE_SERVICE SHALL automatically transition it to OFFLOADING and remove it.
2. **07-REQ-8.2** THE UPDATE_SERVICE SHALL load the inactivity timeout from configuration, expressed in seconds.

### Requirement 9: Configuration

**User Story:** As a deployer, I want to configure the UPDATE_SERVICE via a configuration file, so that I can adapt it to different environments without code changes.

#### Acceptance Criteria

1. **07-REQ-9.1** WHEN the service starts, THE UPDATE_SERVICE SHALL load configuration from a file or environment variables, including: gRPC listen port, registry base URL, inactivity timeout, and container storage path.
2. **07-REQ-9.2** THE UPDATE_SERVICE SHALL use sensible defaults if configuration values are not provided: gRPC port 50060, inactivity timeout 86400 seconds, storage path `/var/lib/containers/adapters/`.

### Requirement 10: Error Handling

**User Story:** As a PARKING_APP developer, I want consistent gRPC error responses from UPDATE_SERVICE, so that I can handle failures programmatically.

#### Acceptance Criteria

1. **07-REQ-10.1** THE UPDATE_SERVICE SHALL use standard gRPC status codes for all error responses: UNAVAILABLE for registry connectivity issues, INVALID_ARGUMENT for checksum mismatches, NOT_FOUND for unknown adapters, ALREADY_EXISTS for duplicate installs, and INTERNAL for unexpected failures.
2. **07-REQ-10.2** THE UPDATE_SERVICE SHALL include descriptive error messages in gRPC status details for all error responses.

## Traceability

| Requirement | PRD Section |
|-------------|-------------|
| 07-REQ-1 | gRPC Interface: InstallAdapter; Adapter Download and Installation Flow |
| 07-REQ-2 | Adapter Download and Installation Flow: checksum verification |
| 07-REQ-3 | gRPC Interface: WatchAdapterStates |
| 07-REQ-4 | gRPC Interface: ListAdapters, GetAdapterStatus |
| 07-REQ-5 | gRPC Interface: RemoveAdapter |
| 07-REQ-6 | Adapter Lifecycle States |
| 07-REQ-7 | Single Adapter Constraint |
| 07-REQ-8 | Automatic Offloading |
| 07-REQ-9 | Configuration |
| 07-REQ-10 | Error Handling (master PRD) |
