# Requirements Document

## Introduction

This document specifies the requirements for the UPDATE_SERVICE component (Phase 2.3) of the SDV Parking Demo System. The UPDATE_SERVICE is a Rust gRPC service running in the RHIVOS QM partition that manages the lifecycle of containerized PARKING_OPERATOR_ADAPTORs: pulling OCI images, verifying integrity, installing, running, monitoring, and offloading containers via podman.

## Glossary

- **UPDATE_SERVICE:** A Rust gRPC service managing containerized adapter lifecycle in the RHIVOS QM partition.
- **PARKING_OPERATOR_ADAPTOR:** A containerized application that interfaces between the vehicle and a specific parking operator.
- **PARKING_APP:** An Android Automotive OS application that requests adapter installation and manages the parking workflow.
- **OCI image:** An Open Container Initiative image, a standard container image format.
- **OCI manifest digest:** A SHA-256 hash uniquely identifying a container image manifest.
- **Adapter ID:** A human-readable identifier derived from the image reference (last path segment + tag).
- **Job ID:** A UUID (v4) generated per InstallAdapter call for tracking the installation operation.
- **Adapter state:** One of: UNKNOWN, DOWNLOADING, INSTALLING, RUNNING, STOPPED, ERROR, OFFLOADING.
- **State transition:** A change from one adapter state to another, emitted to WatchAdapterStates subscribers.
- **Offloading:** The process of removing an unused adapter's container and image to free resources.
- **Inactivity timer:** A configurable period after which a stopped adapter is automatically offloaded.
- **Single adapter constraint:** Only one PARKING_OPERATOR_ADAPTOR can run at a time per vehicle.
- **Google Artifact Registry:** The OCI-compliant container registry used to store adapter images.
- **podman:** A daemonless container runtime used for pulling, running, and managing containers.

## Requirements

### Requirement 1: Adapter Installation

**User Story:** As a PARKING_APP, I want to install a parking operator adapter by providing an image reference and checksum, so that the adapter runs on the vehicle and can manage parking sessions.

#### Acceptance Criteria

1. [07-REQ-1.1] WHEN an `InstallAdapter` gRPC request is received with `image_ref` and `checksum_sha256`, THE service SHALL pull the OCI image using `podman pull`, verify the digest, install and start the container, and return an `InstallAdapterResponse` with `job_id`, `adapter_id`, and initial state `DOWNLOADING`.
2. [07-REQ-1.2] THE service SHALL transition the adapter through states DOWNLOADING → INSTALLING → RUNNING during a successful installation.
3. [07-REQ-1.3] THE service SHALL verify the OCI manifest digest against the provided `checksum_sha256` after pulling the image, before installing.
4. [07-REQ-1.4] THE service SHALL start the container with `--network=host` so it can reach DATA_BROKER via network TCP.
5. [07-REQ-1.5] THE service SHALL derive the `adapter_id` from the image reference by extracting the last path segment and tag.

#### Edge Cases

1. [07-REQ-1.E1] IF the `image_ref` is empty or the `checksum_sha256` is empty, THEN THE service SHALL return gRPC `INVALID_ARGUMENT` with a descriptive error message.
2. [07-REQ-1.E2] IF the image pull fails (registry unreachable, image not found), THEN THE service SHALL transition the adapter to ERROR state and return gRPC `UNAVAILABLE` with the error details.
3. [07-REQ-1.E3] IF the checksum verification fails (digest does not match), THEN THE service SHALL transition the adapter to ERROR state, remove the pulled image, and return gRPC `FAILED_PRECONDITION` with a descriptive error message.
4. [07-REQ-1.E4] IF the container fails to start, THEN THE service SHALL transition the adapter to ERROR state and return gRPC `INTERNAL` with the error details.

### Requirement 2: Single Adapter Constraint

**User Story:** As the system, I want only one adapter running at a time, so that the vehicle's resources are not overloaded and parking sessions don't conflict.

#### Acceptance Criteria

1. [07-REQ-2.1] WHEN `InstallAdapter` is called while another adapter is in RUNNING state, THE service SHALL stop the currently running adapter before starting the new one.
2. [07-REQ-2.2] THE service SHALL transition the previously running adapter to STOPPED state before proceeding with the new installation.

#### Edge Cases

1. [07-REQ-2.E1] IF stopping the currently running adapter fails, THEN THE service SHALL return gRPC `INTERNAL` with the error details and not proceed with the new installation.

### Requirement 3: Adapter State Watching

**User Story:** As a PARKING_APP, I want to watch adapter state transitions in real-time, so that I can update the UI to reflect the adapter's lifecycle.

#### Acceptance Criteria

1. [07-REQ-3.1] WHEN a `WatchAdapterStates` gRPC request is received, THE service SHALL return a server-streaming response of `AdapterStateEvent` messages for all subsequent state transitions.
2. [07-REQ-3.2] THE service SHALL support multiple concurrent `WatchAdapterStates` subscribers.
3. [07-REQ-3.3] EACH `AdapterStateEvent` SHALL include `adapter_id`, `old_state`, `new_state`, and a Unix timestamp.

### Requirement 4: Adapter Listing and Status

**User Story:** As a PARKING_APP, I want to list all known adapters and query individual adapter status, so that I can display adapter information to the user.

#### Acceptance Criteria

1. [07-REQ-4.1] WHEN a `ListAdapters` gRPC request is received, THE service SHALL return a list of all known adapters with their current states and metadata.
2. [07-REQ-4.2] WHEN a `GetAdapterStatus` gRPC request is received with an `adapter_id`, THE service SHALL return the current status of the specified adapter including state, image reference, and timestamps.

#### Edge Cases

1. [07-REQ-4.E1] IF `GetAdapterStatus` is called with an unknown `adapter_id`, THEN THE service SHALL return gRPC `NOT_FOUND` with a descriptive error message.

### Requirement 5: Adapter Removal

**User Story:** As a PARKING_APP, I want to explicitly remove an adapter, so that resources are freed immediately.

#### Acceptance Criteria

1. [07-REQ-5.1] WHEN a `RemoveAdapter` gRPC request is received with an `adapter_id`, THE service SHALL stop the container (if running), remove the container, remove the image, and delete the adapter from in-memory state.
2. [07-REQ-5.2] THE service SHALL transition the adapter through appropriate states: current → STOPPED (if running) → OFFLOADING → (removed).
3. [07-REQ-5.3] THE service SHALL emit state transition events for each state change during removal.

#### Edge Cases

1. [07-REQ-5.E1] IF `RemoveAdapter` is called with an unknown `adapter_id`, THEN THE service SHALL return gRPC `NOT_FOUND` with a descriptive error message.
2. [07-REQ-5.E2] IF container removal fails, THEN THE service SHALL transition the adapter to ERROR state and return gRPC `INTERNAL` with the error details.

### Requirement 6: Automatic Offloading

**User Story:** As the system, I want unused adapters to be automatically offloaded after a configurable inactivity period, so that vehicle resources are kept available.

#### Acceptance Criteria

1. [07-REQ-6.1] WHEN an adapter has been in STOPPED state for longer than the configured inactivity timeout (default: 24 hours), THE service SHALL automatically transition the adapter to OFFLOADING, remove the container and image, and delete it from in-memory state.
2. [07-REQ-6.2] THE inactivity timeout SHALL be configurable via the configuration file.
3. [07-REQ-6.3] THE service SHALL emit state transition events during automatic offloading.

### Requirement 7: Configuration

**User Story:** As a developer, I want the service to load configuration from a file, so that I can modify settings without code changes.

#### Acceptance Criteria

1. [07-REQ-7.1] WHEN the service starts, THE service SHALL load configuration from the file path specified by the `CONFIG_PATH` environment variable, defaulting to `config.json` in the working directory.
2. [07-REQ-7.2] THE configuration SHALL include: gRPC listen port, registry URL, inactivity timeout, and container storage path.
3. [07-REQ-7.3] THE service SHALL use default values (port 50052, inactivity timeout 24h, storage path `/var/lib/containers/adapters/`) when fields are omitted from the config.

#### Edge Cases

1. [07-REQ-7.E1] IF the configuration file does not exist, THEN THE service SHALL start with built-in default configuration and log a warning.
2. [07-REQ-7.E2] IF the configuration file contains invalid JSON, THEN THE service SHALL exit with a non-zero code and log a descriptive error.

### Requirement 8: Graceful Lifecycle

**User Story:** As an operator, I want the service to start and stop cleanly.

#### Acceptance Criteria

1. [07-REQ-8.1] WHEN the service starts, THE service SHALL log its version, configured port, registry URL, and a ready message.
2. [07-REQ-8.2] WHEN the service receives SIGTERM or SIGINT, THE service SHALL stop all running adapters, shut down the gRPC server gracefully, and exit with code 0.
