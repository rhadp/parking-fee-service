# Design: UPDATE_SERVICE (Spec 07)

> Design document for the UPDATE_SERVICE, a Rust gRPC service managing containerized adapter lifecycle in the RHIVOS QM partition.

## References

- Master PRD: `.specs/prd.md`
- Component PRD: `.specs/07_update_service/prd.md`
- Requirements: `.specs/07_update_service/requirements.md`
- Test Specification: `.specs/07_update_service/test_spec.md`

## Architecture Overview

The UPDATE_SERVICE is a Rust async service built with `tonic` (gRPC) and `tokio` (async runtime). It manages the lifecycle of containerized PARKING_OPERATOR_ADAPTORs in the RHIVOS QM partition. The service exposes five gRPC RPCs on port 50051 for cross-partition access from the PARKING_APP.

For the demo scope, container operations (pull, install, start, stop) are simulated. The service maintains an in-memory adapter state store and drives a state machine for each adapter through its lifecycle.

```
+------------------------------------------------------------------+
|  RHIVOS QM Partition                                             |
|                                                                  |
|  +------------------------------------------------------------+ |
|  |  UPDATE_SERVICE (port 50051)                                | |
|  |                                                              | |
|  |  +------------------+  +-------------------+                 | |
|  |  | gRPC Service     |  | Event Broadcaster |                 | |
|  |  | (tonic)          |  | (tokio::broadcast) |                | |
|  |  +--------+---------+  +---------+---------+                 | |
|  |           |                      |                            | |
|  |  +--------v----------------------v---------+                 | |
|  |  |         Adapter State Store              |                 | |
|  |  |  (HashMap<AdapterId, AdapterRecord>)     |                 | |
|  |  |  - state machine enforcement             |                 | |
|  |  |  - last_activity tracking                |                 | |
|  |  +--------+--------------------------------+                 | |
|  |           |                                                   | |
|  |  +--------v--------------------------+                        | |
|  |  |   Container Manager (simulated)   |                        | |
|  |  |   - download simulation           |                        | |
|  |  |   - checksum verification         |                        | |
|  |  |   - install/start/stop simulation |                        | |
|  |  +-----------------------------------+                        | |
|  |                                                              | |
|  |  +-----------------------------------+                        | |
|  |  |   Offload Timer                   |                        | |
|  |  |   (tokio interval task)           |                        | |
|  |  +-----------------------------------+                        | |
|  +------------------------------------------------------------+ |
+------------------------------------------------------------------+
         |
         | gRPC (network TCP, port 50051)
         |
    PARKING_APP (AAOS)
```

## Technology Stack

| Technology | Version / Reference | Purpose |
|------------|-------------------|---------|
| Rust | Stable (edition 2021) | Implementation language |
| tonic | Latest stable | gRPC server framework |
| prost | Latest stable (via tonic) | Protocol buffer code generation |
| tokio | Latest stable (full features) | Async runtime |
| sha2 | Latest stable | SHA-256 checksum computation |
| uuid | Latest stable (v4 feature) | Job ID generation |
| tracing | Latest stable | Structured logging |

## Module Structure

```
rhivos/
  update-service/
    Cargo.toml
    build.rs                    # tonic-build for proto compilation
    src/
      main.rs                   # Entry point, server startup
      grpc.rs                   # tonic service implementation
      state.rs                  # AdapterState enum, state machine transitions
      store.rs                  # AdapterStore (in-memory state store)
      container.rs              # Container manager (simulated lifecycle)
      offload.rs                # Inactivity timer and offloading logic
      error.rs                  # Error types and gRPC status mapping
    tests/
      integration_tests.rs      # Integration tests
```

## gRPC Service Definition

The service is defined in `proto/update_service.proto`:

```protobuf
syntax = "proto3";

package update_service;

enum AdapterState {
  UNKNOWN = 0;
  DOWNLOADING = 1;
  INSTALLING = 2;
  RUNNING = 3;
  STOPPED = 4;
  ERROR = 5;
  OFFLOADING = 6;
}

message InstallAdapterRequest {
  string image_ref = 1;
  string checksum_sha256 = 2;
}

message InstallAdapterResponse {
  string job_id = 1;
  string adapter_id = 2;
  AdapterState state = 3;
}

message WatchAdapterStatesRequest {}

message AdapterStateEvent {
  string adapter_id = 1;
  AdapterState old_state = 2;
  AdapterState new_state = 3;
}

message ListAdaptersRequest {}

message AdapterInfo {
  string adapter_id = 1;
  string image_ref = 2;
  AdapterState state = 3;
}

message ListAdaptersResponse {
  repeated AdapterInfo adapters = 1;
}

message RemoveAdapterRequest {
  string adapter_id = 1;
}

message RemoveAdapterResponse {}

message GetAdapterStatusRequest {
  string adapter_id = 1;
}

message GetAdapterStatusResponse {
  string adapter_id = 1;
  string image_ref = 2;
  AdapterState state = 3;
}

service UpdateService {
  rpc InstallAdapter(InstallAdapterRequest) returns (InstallAdapterResponse);
  rpc WatchAdapterStates(WatchAdapterStatesRequest) returns (stream AdapterStateEvent);
  rpc ListAdapters(ListAdaptersRequest) returns (ListAdaptersResponse);
  rpc RemoveAdapter(RemoveAdapterRequest) returns (RemoveAdapterResponse);
  rpc GetAdapterStatus(GetAdapterStatusRequest) returns (GetAdapterStatusResponse);
}
```

## Adapter State Machine

### States

| State | Description |
|-------|-------------|
| `UNKNOWN` | Default/initial state; adapter not tracked in the system |
| `DOWNLOADING` | OCI image pull in progress |
| `INSTALLING` | Image downloaded and verified; container being set up |
| `RUNNING` | Container is running and serving requests |
| `STOPPED` | Container has been stopped (explicit or pre-removal) |
| `ERROR` | An error occurred during any lifecycle phase |
| `OFFLOADING` | Adapter is being removed from the system |

### Valid Transitions

```
UNKNOWN -----> DOWNLOADING          (InstallAdapter called)
DOWNLOADING -> INSTALLING           (download complete, checksum verified)
DOWNLOADING -> ERROR                (download failed or checksum mismatch)
INSTALLING --> RUNNING              (container started successfully)
INSTALLING --> ERROR                (container start failed)
RUNNING -----> STOPPED              (explicit stop or pre-removal)
RUNNING -----> ERROR                (runtime failure)
STOPPED -----> OFFLOADING           (inactivity timer or explicit remove)
STOPPED -----> RUNNING              (adapter restarted)
ERROR -------> OFFLOADING           (cleanup)
OFFLOADING --> (removed from store) (adapter fully removed)
```

### State Machine Implementation

The `AdapterState` enum and a `transition(from, to) -> Result<(), InvalidTransition>` function enforce that only valid transitions occur. Any attempt to perform an invalid transition returns an error and is logged.

```rust
impl AdapterState {
    pub fn can_transition_to(&self, target: &AdapterState) -> bool {
        matches!(
            (self, target),
            (Self::Unknown, Self::Downloading)
                | (Self::Downloading, Self::Installing)
                | (Self::Downloading, Self::Error)
                | (Self::Installing, Self::Running)
                | (Self::Installing, Self::Error)
                | (Self::Running, Self::Stopped)
                | (Self::Running, Self::Error)
                | (Self::Stopped, Self::Offloading)
                | (Self::Stopped, Self::Running)
                | (Self::Error, Self::Offloading)
        )
    }
}
```

## Adapter State Store

The `AdapterStore` is a thread-safe in-memory store wrapping a `HashMap<String, AdapterRecord>` behind an `Arc<RwLock<...>>` (using `tokio::sync::RwLock`).

### AdapterRecord

```rust
struct AdapterRecord {
    adapter_id: String,
    image_ref: String,
    checksum_sha256: String,
    state: AdapterState,
    job_id: String,
    last_activity: Instant,
}
```

### Key Operations

| Operation | Description |
|-----------|-------------|
| `insert(record)` | Adds a new adapter record to the store |
| `get(adapter_id)` | Returns a clone of the adapter record |
| `list()` | Returns all adapter records (excluding fully removed) |
| `transition(adapter_id, new_state)` | Validates and applies a state transition, updates `last_activity`, broadcasts event |
| `remove(adapter_id)` | Removes the adapter record from the store |

The `adapter_id` is derived deterministically from the `image_ref` (e.g., via a stable hash or by extracting the image name/tag). This ensures that repeated installs of the same image reference produce the same `adapter_id`, enabling duplicate detection.

## Container Manager (Simulated)

For the demo scope, the container manager simulates OCI image operations without a full container runtime. Each operation introduces a short delay (configurable, default 500ms) to simulate realistic timing.

### Simulated Operations

| Operation | Simulation | Duration |
|-----------|-----------|----------|
| Download | `tokio::time::sleep` + checksum verification | 1-2 seconds |
| Install | `tokio::time::sleep` | 500ms |
| Start | `tokio::time::sleep`, mark as running | 200ms |
| Stop | `tokio::time::sleep`, mark as stopped | 200ms |

### Download Simulation Flow

1. Receive `image_ref` and `checksum_sha256`.
2. Simulate download delay.
3. Generate a simulated manifest digest (for demo: use a known test digest, or derive from `image_ref`).
4. Compute SHA-256 of the simulated manifest digest.
5. Compare against `checksum_sha256`.
6. If match: transition `DOWNLOADING -> INSTALLING`.
7. If mismatch: transition `DOWNLOADING -> ERROR`.

### Checksum Verification

```rust
use sha2::{Sha256, Digest};

fn verify_checksum(manifest_digest: &[u8], expected: &str) -> bool {
    let mut hasher = Sha256::new();
    hasher.update(manifest_digest);
    let actual = format!("{:x}", hasher.finalize());
    actual == expected
}
```

For the demo, the simulated download produces a known manifest digest. The test fixtures and PARKING_FEE_SERVICE provide matching checksums. To test checksum mismatch, a deliberately wrong checksum is supplied.

## Event Broadcasting

State transition events are broadcast to all `WatchAdapterStates` subscribers using a `tokio::sync::broadcast` channel.

### Flow

1. When `AdapterStore::transition()` succeeds, it sends an `AdapterStateEvent` to the broadcast channel.
2. Each `WatchAdapterStates` RPC call creates a new receiver from the broadcast channel.
3. The gRPC streaming response reads from the receiver and yields events to the client.
4. If a receiver falls behind (buffer full), lagged events are skipped and a warning is logged.

### Broadcast Channel Configuration

- Buffer size: 64 events (sufficient for demo; adapters rarely exceed single digits).

## Offloading Logic

A background `tokio` task runs on a configurable interval (default: 60 seconds for the check interval) and inspects all adapters in `STOPPED` or `ERROR` state.

### Algorithm

```
every CHECK_INTERVAL:
    for each adapter in store:
        if adapter.state == STOPPED and (now - adapter.last_activity) > OFFLOAD_TIMEOUT:
            transition(adapter_id, OFFLOADING)
            remove(adapter_id)
        if adapter.state == ERROR and (now - adapter.last_activity) > OFFLOAD_TIMEOUT:
            transition(adapter_id, OFFLOADING)
            remove(adapter_id)
```

### Configuration

| Parameter | Environment Variable | Default |
|-----------|---------------------|---------|
| Offload timeout | `UPDATE_SERVICE_OFFLOAD_TIMEOUT_SECS` | 86400 (24 hours) |
| Check interval | `UPDATE_SERVICE_OFFLOAD_CHECK_INTERVAL_SECS` | 60 |

For integration tests, the offload timeout can be set to a few seconds to verify the behavior without long waits.

## Correctness Properties

### CP-1: State Machine Integrity

Every adapter state transition SHALL pass through the `can_transition_to` validation. No adapter may skip states or enter an invalid transition. The store rejects and logs any invalid transition attempt.

### CP-2: Checksum Verification Before Installation

The UPDATE_SERVICE SHALL NOT transition an adapter from `DOWNLOADING` to `INSTALLING` unless the SHA-256 checksum of the OCI manifest digest matches the expected `checksum_sha256`. A mismatch always results in a `DOWNLOADING -> ERROR` transition.

### CP-3: Event Delivery Completeness

Every successful state transition SHALL produce exactly one `AdapterStateEvent` on the broadcast channel. Subscribers who are connected at the time of the transition receive the event. No event is emitted for failed (rejected) transitions.

### CP-4: Deterministic Adapter ID

Given the same `image_ref`, the UPDATE_SERVICE SHALL produce the same `adapter_id`. This enables duplicate detection: if an adapter with a matching `adapter_id` already exists and is not in a terminal state (`UNKNOWN`), the `InstallAdapter` RPC returns `ALREADY_EXISTS`.

### CP-5: Idempotent Removal

Calling `RemoveAdapter` for an adapter that is already in `OFFLOADING` state or has been fully removed SHALL return `NOT_FOUND` (the adapter is no longer tracked). Multiple concurrent `RemoveAdapter` calls for the same adapter SHALL NOT cause panics or inconsistent state.

### CP-6: Offload Timer Reset

When an adapter transitions from `STOPPED` back to `RUNNING`, the inactivity timer for that adapter SHALL be reset. The adapter is not eligible for offloading while it is in `RUNNING` state.

### CP-7: Concurrent Access Safety

All operations on the adapter state store SHALL be safe under concurrent access from multiple gRPC handler tasks and the offload background task. The `tokio::sync::RwLock` ensures this property.

## Error Handling

| Error Scenario | gRPC Status Code | Details |
|---------------|-----------------|---------|
| `InstallAdapter` with empty `image_ref` | `INVALID_ARGUMENT` | `"image_ref must not be empty"` |
| `InstallAdapter` with empty `checksum_sha256` | `INVALID_ARGUMENT` | `"checksum_sha256 must not be empty"` |
| `InstallAdapter` for already installed/in-progress adapter | `ALREADY_EXISTS` | Includes existing `adapter_id` and current state |
| `GetAdapterStatus` with unknown `adapter_id` | `NOT_FOUND` | `"adapter not found: <adapter_id>"` |
| `RemoveAdapter` with unknown `adapter_id` | `NOT_FOUND` | `"adapter not found: <adapter_id>"` |
| Download failure (simulated) | N/A (state -> ERROR) | Event broadcast with `DOWNLOADING -> ERROR` |
| Checksum mismatch | N/A (state -> ERROR) | Event broadcast with `DOWNLOADING -> ERROR`; error details include expected/actual checksums |
| Container start failure (simulated) | N/A (state -> ERROR) | Event broadcast with `INSTALLING -> ERROR` |
| Internal error (store lock poisoned, broadcast send failure) | `INTERNAL` | Logged with tracing; generic error returned to client |

## Definition of Done

1. The `update-service` crate compiles without errors: `cd rhivos && cargo build -p update-service`.
2. All unit tests pass: `cd rhivos && cargo test -p update-service`.
3. No clippy warnings: `cd rhivos && cargo clippy -p update-service -- -D warnings`.
4. The gRPC server starts and listens on port 50051.
5. `InstallAdapter` returns a valid response with `state=DOWNLOADING` and triggers the simulated lifecycle.
6. `WatchAdapterStates` streams state transition events to connected clients.
7. `ListAdapters` returns all tracked adapters.
8. `GetAdapterStatus` returns the current state of a specific adapter.
9. `RemoveAdapter` stops and removes an adapter.
10. Checksum verification rejects mismatched images with an `ERROR` state transition.
11. The offload timer removes inactive `STOPPED` adapters after the configured timeout.
12. All test specifications in `test_spec.md` are covered by passing tests.

## Testing Strategy

### Unit Tests

Unit tests are co-located with the source modules in `rhivos/update-service/src/`. They cover:

- State machine transition validation (all valid transitions succeed, invalid transitions are rejected).
- Adapter ID determinism (same `image_ref` produces same `adapter_id`).
- Checksum verification (matching and non-matching digests).
- Store operations (insert, get, list, transition, remove).

### Integration Tests

Integration tests in `rhivos/update-service/tests/` start the gRPC server in-process using `tonic` and exercise the RPCs end-to-end:

- Install an adapter and verify the response.
- Watch state transitions during installation.
- List adapters and verify completeness.
- Get status of a specific adapter.
- Remove an adapter and verify it disappears.
- Duplicate install returns `ALREADY_EXISTS`.
- Checksum mismatch transitions to `ERROR`.
- Offloading after inactivity (with short timer).

### Test Commands

- **Unit tests:** `cd rhivos && cargo test -p update-service`
- **Lint:** `cd rhivos && cargo clippy -p update-service -- -D warnings`
- **Format check:** `cd rhivos && cargo fmt -p update-service -- --check`
