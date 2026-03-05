# PRD: UPDATE_SERVICE (Phase 2.3)

> Extracted from the master PRD at `.specs/prd.md`. This spec covers the UPDATE_SERVICE component of Phase 2.3: RHIVOS QM Partition.

## Scope

Implement the UPDATE_SERVICE as a Rust gRPC service running in the RHIVOS QM partition. It manages the lifecycle of containerized PARKING_OPERATOR_ADAPTORs: pulling OCI images from Google Artifact Registry, verifying integrity, installing, running, monitoring, and offloading containers.

## Component Description

- Rust service running in the RHIVOS QM partition
- Manages containerized adapter lifecycle (download, install, run, stop, remove, offload)
- Pulls containers from OCI Registry (Google Artifact Registry) on demand
- Handles installation, updates, and automatic offloading of unused adapters
- Offloading is triggered by a configurable inactivity timer OR when RHIVOS QM resources become scarce
- Only one adapter can run at a time per vehicle (enforced by UPDATE_SERVICE)
- Verifies SHA-256 checksum of OCI manifest digest before installing
- Container runtime: podman/crun
- Static gRPC port assignment from config

## gRPC Interface

The UPDATE_SERVICE exposes a gRPC service over network TCP (for cross-domain access from PARKING_APP on AAOS):

- `InstallAdapter(image_ref, checksum_sha256)` -- Pull image from registry, verify checksum, install and start the container. Returns `InstallAdapterResponse{job_id, adapter_id, state}`.
- `WatchAdapterStates()` -- Server-streaming RPC returning a stream of `AdapterStateEvent` for all adapter state transitions.
- `ListAdapters()` -- Returns a list of all known adapters with their current states.
- `RemoveAdapter(adapter_id)` -- Stops and removes the specified adapter.
- `GetAdapterStatus(adapter_id)` -- Returns the current status of a specific adapter.

## Adapter Lifecycle States

```
UNKNOWN -> DOWNLOADING -> INSTALLING -> RUNNING -> STOPPED -> OFFLOADING -> (removed)
                                    \-> ERROR
                          RUNNING -> ERROR
                          RUNNING -> STOPPED
                          STOPPED -> RUNNING (restart)
```

Full set of states: `UNKNOWN`, `DOWNLOADING`, `INSTALLING`, `RUNNING`, `STOPPED`, `ERROR`, `OFFLOADING`.

## Adapter Download and Installation Flow

1. PARKING_APP calls `InstallAdapter(image_ref, checksum_sha256)` via gRPC.
2. UPDATE_SERVICE pulls the OCI image from the registry.
3. UPDATE_SERVICE verifies the SHA-256 checksum of the OCI manifest digest against the provided checksum.
4. If checksum matches, UPDATE_SERVICE installs and starts the container via podman/crun.
5. State transitions are emitted to all `WatchAdapterStates` subscribers.

## Single Adapter Constraint

Only one PARKING_OPERATOR_ADAPTOR can run at a time per vehicle, since a vehicle can only park at one location at a time. When `InstallAdapter` is called while another adapter is already RUNNING, the UPDATE_SERVICE must stop the currently running adapter before starting the new one.

## Automatic Offloading

Unused adapters are automatically offloaded to free up storage and system resources. Offloading is triggered by:

- **Inactivity timer:** A configurable period (default: 24 hours) after the adapter was last stopped. When the timer expires, the adapter transitions to OFFLOADING and is removed.
- **Resource pressure:** When RHIVOS QM resources become scarce (out of scope for initial implementation; placeholder for future).

## Configuration

- Registry URL (Google Artifact Registry base URL)
- gRPC listen port (static assignment)
- Inactivity timeout (default: 24 hours)
- Container storage path (default: `/var/lib/containers/adapters/`)

## Tech Stack

- Language: Rust (edition 2021)
- gRPC framework: tonic
- Async runtime: tokio
- Container management: podman CLI (shelling out)
- Serialization: prost (protobuf)

## Dependencies

| Spec | From Group | To Group | Relationship |
|------|-----------|----------|--------------|
| 01_project_setup | 2 | 1 | Uses repo structure and Rust project skeleton from group 2 |
