# Requirements Document

## Introduction

This document defines the requirements for the UPDATE_SERVICE component of the SDV Parking Demo System. The UPDATE_SERVICE is a Rust service running in the RHIVOS QM partition that manages the lifecycle of containerized parking operator adapters.

The service receives adapter installation requests from the PARKING_APP via gRPC over TCP/TLS, pulls OCI container images from the REGISTRY, validates container manifests, manages adapter container lifecycle using podman/crun, and provides streaming state updates to clients.

## Glossary

- **UPDATE_SERVICE**: Container lifecycle management service running in the RHIVOS QM partition
- **PARKING_APP**: Android Automotive OS IVI application that requests adapter installations
- **PARKING_OPERATOR_ADAPTOR**: Dynamic containerized adapter for parking operator integration
- **REGISTRY**: OCI-compliant container registry (Google Artifact Registry) storing validated adapter images
- **OCI**: Open Container Initiative - standard for container image format and distribution
- **Adapter_State**: Current state of an adapter (DOWNLOADING, INSTALLING, RUNNING, STOPPED, ERROR)
- **Manifest**: Container image metadata including layers, config, and checksums
- **Checksum**: SHA256 digest used to verify container image integrity
- **podman**: Daemonless container engine for running OCI containers
- **crun**: Lightweight OCI runtime for container execution
- **gRPC**: High-performance RPC framework using Protocol Buffers
- **TLS**: Transport Layer Security for encrypted cross-domain communication
- **DATA_BROKER**: Eclipse Kuksa Databroker providing VSS-compliant signal interface

## Requirements

### Requirement 1: Adapter Installation

**User Story:** As the PARKING_APP, I want to request installation of a parking operator adapter, so that the vehicle can integrate with the local parking operator.

#### Acceptance Criteria

1. WHEN the UPDATE_SERVICE receives an InstallAdapter request with a valid adapter_id and registry_url THEN the UPDATE_SERVICE SHALL initiate the download and installation process
2. WHEN an adapter installation is initiated THEN the UPDATE_SERVICE SHALL return an InstallAdapterResponse with the initial state set to DOWNLOADING
3. WHEN an adapter with the same adapter_id is already installed and RUNNING THEN the UPDATE_SERVICE SHALL return success without re-downloading
4. WHEN an adapter is currently being installed (DOWNLOADING or INSTALLING state) THEN the UPDATE_SERVICE SHALL return the current state without starting a duplicate installation
5. IF the registry_url is malformed or unreachable THEN the UPDATE_SERVICE SHALL return an error indicating the registry is unavailable

### Requirement 2: Container Image Download

**User Story:** As a system operator, I want the UPDATE_SERVICE to download container images from the OCI registry, so that adapters can be installed on-demand.

#### Acceptance Criteria

1. WHEN downloading a container image THEN the UPDATE_SERVICE SHALL use HTTPS/OCI protocol to pull from the REGISTRY
2. WHEN a download starts THEN the UPDATE_SERVICE SHALL transition the adapter state to DOWNLOADING
3. WHEN a download completes successfully THEN the UPDATE_SERVICE SHALL transition the adapter state to INSTALLING
4. IF a download fails due to network error THEN the UPDATE_SERVICE SHALL retry up to 3 times with exponential backoff
5. IF all download retries fail THEN the UPDATE_SERVICE SHALL transition the adapter state to ERROR and include the failure reason

### Requirement 3: Container Manifest Validation

**User Story:** As a security engineer, I want container manifests to be validated before installation, so that only verified adapters are installed.

#### Acceptance Criteria

1. WHEN a container image is downloaded THEN the UPDATE_SERVICE SHALL verify the manifest checksum matches the expected SHA256 digest
2. IF the manifest checksum verification fails THEN the UPDATE_SERVICE SHALL reject the image, transition state to ERROR, and delete the downloaded content
3. THE UPDATE_SERVICE SHALL validate that the manifest contains required fields (config, layers, mediaType)
4. IF the manifest is missing required fields THEN the UPDATE_SERVICE SHALL reject the image and transition state to ERROR

### Requirement 4: Container Installation and Startup

**User Story:** As the PARKING_APP, I want adapters to be started automatically after installation, so that they are ready to handle parking operations.

#### Acceptance Criteria

1. WHEN manifest validation succeeds THEN the UPDATE_SERVICE SHALL install the container using podman
2. WHEN installation completes THEN the UPDATE_SERVICE SHALL start the adapter container
3. WHEN the container starts successfully THEN the UPDATE_SERVICE SHALL transition the adapter state to RUNNING
4. THE UPDATE_SERVICE SHALL configure the container with network access to DATA_BROKER
5. THE UPDATE_SERVICE SHALL store container data in /var/lib/containers/adapters/
6. IF container startup fails THEN the UPDATE_SERVICE SHALL transition state to ERROR and include the failure reason

### Requirement 5: Adapter State Tracking

**User Story:** As the PARKING_APP, I want to know the current state of all adapters, so that I can display status to the user and react to state changes.

#### Acceptance Criteria

1. THE UPDATE_SERVICE SHALL track the state of each adapter using the following states: UNKNOWN, DOWNLOADING, INSTALLING, RUNNING, STOPPED, ERROR
2. WHEN an adapter state changes THEN the UPDATE_SERVICE SHALL record the timestamp of the state change
3. THE UPDATE_SERVICE SHALL maintain state information including adapter_id, current_state, error_message (if applicable), and last_updated timestamp
4. WHEN the UPDATE_SERVICE restarts THEN it SHALL restore adapter state by querying podman for running containers

### Requirement 6: Streaming State Updates

**User Story:** As the PARKING_APP, I want to receive real-time state updates for adapters, so that I can update the UI immediately when state changes occur.

#### Acceptance Criteria

1. WHEN a client calls WatchAdapterStates THEN the UPDATE_SERVICE SHALL return a stream of AdapterStateEvent messages
2. WHEN an adapter state changes THEN the UPDATE_SERVICE SHALL emit an AdapterStateEvent to all active watchers
3. THE AdapterStateEvent SHALL include adapter_id, previous_state, new_state, timestamp, and error_message (if applicable)
4. WHEN a watcher disconnects THEN the UPDATE_SERVICE SHALL clean up the subscription without affecting other watchers
5. WHEN a new watcher connects THEN the UPDATE_SERVICE SHALL emit the current state of all adapters as initial events

### Requirement 7: Adapter Listing

**User Story:** As the PARKING_APP, I want to list all installed adapters, so that I can display the current adapter inventory.

#### Acceptance Criteria

1. WHEN the UPDATE_SERVICE receives a ListAdapters request THEN it SHALL return a list of all known adapters with their current states
2. THE ListAdaptersResponse SHALL include adapter_id, state, error_message, and last_updated for each adapter
3. WHEN no adapters are installed THEN the UPDATE_SERVICE SHALL return an empty list

### Requirement 8: Adapter Uninstallation

**User Story:** As the PARKING_APP, I want to uninstall adapters that are no longer needed, so that system resources are freed.

#### Acceptance Criteria

1. WHEN the UPDATE_SERVICE receives an UninstallAdapter request THEN it SHALL stop the adapter container if running
2. WHEN the container is stopped THEN the UPDATE_SERVICE SHALL remove the container and its associated storage
3. WHEN uninstallation completes THEN the UPDATE_SERVICE SHALL remove the adapter from the tracked state
4. IF the adapter_id does not exist THEN the UPDATE_SERVICE SHALL return an error indicating the adapter was not found
5. WHEN an adapter is uninstalled THEN the UPDATE_SERVICE SHALL emit a state change event to all watchers

### Requirement 9: Automatic Offloading

**User Story:** As a system operator, I want unused adapters to be automatically offloaded after 24 hours, so that system resources are not wasted on inactive adapters.

#### Acceptance Criteria

1. THE UPDATE_SERVICE SHALL track the last activity timestamp for each adapter
2. WHEN an adapter has been in STOPPED state for more than 24 hours THEN the UPDATE_SERVICE SHALL automatically uninstall it
3. WHEN an adapter is automatically offloaded THEN the UPDATE_SERVICE SHALL emit a state change event indicating automatic removal
4. THE UPDATE_SERVICE SHALL check for offload candidates periodically (every hour)

### Requirement 10: gRPC Service Interface

**User Story:** As a developer, I want a well-defined gRPC interface, so that I can integrate with the UPDATE_SERVICE from the PARKING_APP.

#### Acceptance Criteria

1. THE UPDATE_SERVICE SHALL expose an InstallAdapter RPC that accepts InstallAdapterRequest and returns InstallAdapterResponse
2. THE UPDATE_SERVICE SHALL expose an UninstallAdapter RPC that accepts UninstallAdapterRequest and returns UninstallAdapterResponse
3. THE UPDATE_SERVICE SHALL expose a ListAdapters RPC that accepts ListAdaptersRequest and returns ListAdaptersResponse
4. THE UPDATE_SERVICE SHALL expose a WatchAdapterStates RPC that accepts WatchAdapterStatesRequest and returns a stream of AdapterStateEvent
5. THE UPDATE_SERVICE SHALL listen on a TCP socket with TLS for gRPC connections from PARKING_APP
6. THE UPDATE_SERVICE SHALL use standard gRPC status codes with custom error details for domain-specific errors

### Requirement 11: OCI Registry Authentication

**User Story:** As a system operator, I want the UPDATE_SERVICE to authenticate with the OCI registry, so that it can pull container images from private registries securely.

#### Acceptance Criteria

1. THE UPDATE_SERVICE SHALL support Bearer token authentication for OCI registry requests
2. THE UPDATE_SERVICE SHALL obtain authentication tokens via the OCI token endpoint (GET /v2/token) when challenged with HTTP 401
3. THE UPDATE_SERVICE SHALL include the Authorization header with Bearer token in subsequent registry requests after authentication
4. THE UPDATE_SERVICE SHALL support configurable registry credentials via environment variables (REGISTRY_USERNAME, REGISTRY_PASSWORD)
5. THE UPDATE_SERVICE SHALL cache authentication tokens and refresh them before expiration
6. IF authentication fails THEN the UPDATE_SERVICE SHALL transition the adapter state to ERROR with a message indicating authentication failure
7. THE UPDATE_SERVICE SHALL support anonymous access for public registries when no credentials are configured

### Requirement 12: Operation Logging

**User Story:** As a system auditor, I want all adapter operations to be logged, so that I can trace operations for debugging and compliance.

#### Acceptance Criteria

1. THE UPDATE_SERVICE SHALL log every received request with timestamp, request type, and adapter_id
2. THE UPDATE_SERVICE SHALL log all state transitions with previous state, new state, and reason
3. THE UPDATE_SERVICE SHALL log container operations (pull, install, start, stop, remove) with outcomes
4. THE UPDATE_SERVICE SHALL include correlation identifiers in logs to enable end-to-end tracing

