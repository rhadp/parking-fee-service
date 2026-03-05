# Requirements: UPDATE_SERVICE (Spec 07)

> EARS-syntax requirements for the UPDATE_SERVICE managing containerized adapter lifecycle in the RHIVOS QM partition.
> Derived from the PRD at `.specs/07_update_service/prd.md` and the master PRD at `.specs/prd.md`.

## Notation

Requirements use the EARS (Easy Approach to Requirements Syntax) patterns:

- **Ubiquitous:** `The <system> shall <action>.`
- **Event-driven:** `When <trigger>, the <system> shall <action>.`
- **State-driven:** `While <state>, the <system> shall <action>.`
- **Unwanted behavior:** `If <condition>, then the <system> shall <action>.`
- **Option:** `Where <feature>, the <system> shall <action>.`

## UPDATE_SERVICE Requirements

### 07-REQ-1.1: InstallAdapter RPC

When the UPDATE_SERVICE receives an `InstallAdapter` request containing an `image_ref` and `checksum_sha256`, the UPDATE_SERVICE shall initiate an asynchronous download of the referenced OCI image and return an `InstallAdapterResponse` containing a unique `job_id`, a unique `adapter_id`, and `state` set to `DOWNLOADING`.

**Rationale:** The PARKING_APP triggers adapter installation by providing an image reference and its expected checksum. The response must be immediate (non-blocking) so the caller can track progress via `WatchAdapterStates`. The `job_id` identifies the installation job and the `adapter_id` identifies the resulting adapter instance.

**Acceptance criteria:**
- The response contains a non-empty `job_id` (UUID format).
- The response contains a non-empty `adapter_id` derived deterministically from `image_ref`.
- The response `state` field is `DOWNLOADING`.
- If the adapter identified by `image_ref` is already installed and in state `RUNNING`, the RPC returns gRPC status `ALREADY_EXISTS` with the existing `adapter_id` in the error details.
- If the adapter is currently being downloaded or installed (states `DOWNLOADING` or `INSTALLING`), the RPC returns gRPC status `ALREADY_EXISTS` with the existing `adapter_id` and current state.

---

### 07-REQ-2.1: Adapter Lifecycle State Machine

The UPDATE_SERVICE shall maintain the following state machine for each adapter, permitting only these transitions:

- `UNKNOWN -> DOWNLOADING` (install initiated)
- `DOWNLOADING -> INSTALLING` (download complete, checksum verified)
- `INSTALLING -> RUNNING` (container started successfully)
- `INSTALLING -> ERROR` (container start failed)
- `DOWNLOADING -> ERROR` (download failed or checksum mismatch)
- `RUNNING -> STOPPED` (adapter explicitly stopped or removed)
- `RUNNING -> ERROR` (runtime failure detected)
- `STOPPED -> OFFLOADING` (inactivity timer expired or resource constraint triggered)
- `STOPPED -> RUNNING` (adapter restarted)
- `ERROR -> OFFLOADING` (cleanup of failed adapter)
- `OFFLOADING -> UNKNOWN` (adapter fully removed from the system)

**Rationale:** A well-defined state machine prevents invalid transitions, ensures deterministic lifecycle behavior, and allows watchers to reason about adapter progress.

**Acceptance criteria:**
- Every adapter has exactly one current state at any point in time.
- Only the transitions listed above are permitted; any other transition is rejected.
- Each state transition is recorded and made available to `WatchAdapterStates` subscribers.

---

### 07-REQ-3.1: WatchAdapterStates Server-Streaming RPC

When a client calls `WatchAdapterStates`, the UPDATE_SERVICE shall open a server-streaming RPC connection and emit an `AdapterStateEvent` message for every adapter state transition that occurs, containing `adapter_id`, `old_state`, and `new_state`.

**Rationale:** The PARKING_APP uses this stream to track adapter installation progress and react to lifecycle changes (e.g., display "Parking active" when the adapter reaches `RUNNING`).

**Acceptance criteria:**
- The stream emits events for all adapters, not just a specific one.
- Each event contains the correct `adapter_id`, `old_state`, and `new_state`.
- Multiple clients can subscribe concurrently and each receives all events.
- The stream remains open until the client disconnects or the server shuts down.

---

### 07-REQ-4.1: ListAdapters and GetAdapterStatus RPCs

The UPDATE_SERVICE shall provide a `ListAdapters` RPC that returns all currently tracked adapters with their `adapter_id`, `image_ref`, and current `state`, and a `GetAdapterStatus` RPC that accepts an `adapter_id` and returns the adapter's current `state` and `image_ref`.

**Rationale:** The PARKING_APP needs to query which adapters are installed and their current status, both as a batch listing and for individual lookup.

**Acceptance criteria:**
- `ListAdapters` returns an empty list when no adapters are installed.
- `ListAdapters` returns all tracked adapters regardless of their current state (including `ERROR` and `STOPPED`).
- Adapters in state `OFFLOADING` that have completed removal (transitioned to `UNKNOWN`) are not returned by `ListAdapters`.
- `GetAdapterStatus` returns the current state for a valid `adapter_id`.
- If `GetAdapterStatus` is called with an `adapter_id` that does not exist, the RPC returns gRPC status `NOT_FOUND`.

---

### 07-REQ-5.1: RemoveAdapter RPC

When the UPDATE_SERVICE receives a `RemoveAdapter` request with an `adapter_id`, the UPDATE_SERVICE shall stop the adapter if it is in state `RUNNING`, transition it through `STOPPED` to `OFFLOADING`, remove it from the system, and return a success response.

**Rationale:** The PARKING_APP or system administrator must be able to explicitly remove an adapter that is no longer needed, reclaiming system resources.

**Acceptance criteria:**
- A `RUNNING` adapter is stopped before removal (transitions `RUNNING -> STOPPED -> OFFLOADING`).
- A `STOPPED` or `ERROR` adapter transitions directly to `OFFLOADING`.
- An adapter in state `DOWNLOADING` or `INSTALLING` is cancelled, transitioned to `ERROR`, then to `OFFLOADING`.
- If `adapter_id` does not exist, the RPC returns gRPC status `NOT_FOUND`.
- After successful removal, the adapter no longer appears in `ListAdapters` responses.

---

### 07-REQ-6.1: Checksum Verification

When the UPDATE_SERVICE completes downloading an OCI image, the UPDATE_SERVICE shall compute the SHA-256 digest of the OCI manifest and compare it against the `checksum_sha256` provided in the `InstallAdapter` request. If the checksums do not match, the UPDATE_SERVICE shall transition the adapter to `ERROR` state and shall not proceed with installation.

**Rationale:** Checksum verification ensures that the downloaded image has not been tampered with or corrupted during transit. The checksum is sourced from the PARKING_FEE_SERVICE, which acts as a trusted authority.

**Acceptance criteria:**
- When checksums match, the adapter transitions from `DOWNLOADING` to `INSTALLING`.
- When checksums do not match, the adapter transitions from `DOWNLOADING` to `ERROR`.
- The error details include the expected and actual checksum values.
- A `WatchAdapterStates` event is emitted for the `DOWNLOADING -> ERROR` transition on mismatch.

---

### 07-REQ-7.1: Automatic Offloading

While an adapter is in state `STOPPED`, if the configurable inactivity timer expires (default: 24 hours), then the UPDATE_SERVICE shall transition the adapter to `OFFLOADING` state and remove it from the system.

**Rationale:** Unused adapters consume storage and system resources. Automatic offloading ensures the RHIVOS QM partition reclaims resources for adapters that are no longer needed, as specified in the master PRD.

**Acceptance criteria:**
- The inactivity timeout is configurable via an environment variable or configuration parameter (default: 24 hours).
- An adapter in `STOPPED` state that remains inactive beyond the timeout transitions to `OFFLOADING`.
- An adapter that is restarted (transitions back to `RUNNING`) before the timer expires does not get offloaded.
- A `WatchAdapterStates` event is emitted for the `STOPPED -> OFFLOADING` transition.
- For demo/testing purposes, the timer value can be set to a short duration (e.g., seconds).

---

### 07-REQ-8.1: gRPC Server Configuration

The UPDATE_SERVICE shall listen for gRPC connections on port `50051` (network TCP) and serve all RPCs (`InstallAdapter`, `WatchAdapterStates`, `ListAdapters`, `RemoveAdapter`, `GetAdapterStatus`) on that port.

**Rationale:** Port 50051 is designated for cross-partition gRPC access from the PARKING_APP running on AAOS. Network TCP is required because the PARKING_APP runs in a different partition (AAOS) than the UPDATE_SERVICE (RHIVOS QM).

**Acceptance criteria:**
- The server binds to `0.0.0.0:50051` on startup.
- All five RPCs are accessible via gRPC on port 50051.
- The server handles multiple concurrent client connections.
