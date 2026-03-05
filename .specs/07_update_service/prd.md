# PRD: UPDATE_SERVICE (Phase 2.3)

> Extracted from the master PRD at `.specs/prd.md`. This spec covers the UPDATE_SERVICE component of Phase 2.3: RHIVOS QM Partition.

## Scope

Implement the UPDATE_SERVICE in Rust, running in the RHIVOS QM partition. This component manages the lifecycle of containerized PARKING_OPERATOR_ADAPTORs.

## Component Description

- Manages containerized adapter lifecycle in RHIVOS QM partition
- Pulls containers from REGISTRY on demand
- Handles installation, updates, and automatic offloading of unused adapters
- Offloading is triggered by a configurable inactivity timer OR when RHIVOS QM resources become scarce
- Local development port: 50051

## gRPC Interface

Defined in `proto/update_service.proto`:

- `InstallAdapter(image_ref, checksum_sha256)` — download and install adapter
  - Returns: `InstallAdapterResponse{job_id, adapter_id, state=DOWNLOADING}`
- `WatchAdapterStates()` — server-streaming RPC returning adapter state changes
  - Returns: stream of `AdapterStateEvent{adapter_id, old_state, new_state}`
- `ListAdapters()` — list all installed adapters
- `RemoveAdapter(adapter_id)` — remove an adapter
- `GetAdapterStatus(adapter_id)` — get current adapter status

## Adapter Lifecycle States

```
UNKNOWN → DOWNLOADING → INSTALLING → RUNNING → STOPPED → OFFLOADING
                                        ↓
                                      ERROR
```

States: UNKNOWN, DOWNLOADING, INSTALLING, RUNNING, STOPPED, ERROR, OFFLOADING

## Container Management

- Pulls OCI images from Google Artifact Registry
- Verifies SHA-256 checksum of OCI manifest digest
- Extracts container to `/var/lib/containers/adapters/`
- Starts container via podman/crun
- For demo scope: simplified container lifecycle without full OCI runtime

## Tech Stack

- Language: Rust
- gRPC: tonic
- Port: 50051 (network gRPC for cross-partition access)

## Dependencies

| Spec | From Group | To Group | Relationship |
|------|-----------|----------|--------------|
| 01_project_setup | 2 | 1 | Requires Rust workspace skeleton, proto definitions, and build system |
