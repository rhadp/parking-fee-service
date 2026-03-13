# Test Specification: UPDATE_SERVICE

## Overview

This test specification defines concrete test contracts for the UPDATE_SERVICE, a Rust gRPC service managing containerized adapter lifecycle. Tests are organized into unit tests (config, state, model, container modules using mock ContainerRuntime) and integration tests (gRPC client tests in `tests/update-service/`). Unit tests run via `cd rhivos && cargo test -p update-service`. The container module uses a trait-based abstraction (ContainerRuntime) with mock implementations for unit testing without podman.

## Test Cases

### TS-07-1: Install Adapter Happy Path

**Requirement:** 07-REQ-1.1
**Type:** unit
**Description:** InstallAdapter pulls image, verifies checksum, starts container, and returns job_id, adapter_id, and DOWNLOADING state.

**Preconditions:**
- Mock ContainerRuntime configured to succeed for pull, inspect (returns matching digest), and run.

**Input:**
- `image_ref: "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0"`
- `checksum_sha256: "sha256:abc123def456"`

**Expected:**
- Response contains non-empty `job_id`, `adapter_id == "parkhaus-munich-v1.0.0"`, `state == DOWNLOADING`.
- After completion: adapter state is RUNNING.

**Assertion pseudocode:**
```
resp = service.install_adapter(image_ref, checksum)
ASSERT resp.job_id != ""
ASSERT resp.adapter_id == "parkhaus-munich-v1.0.0"
ASSERT resp.state == DOWNLOADING
wait_for_state(adapter_id, RUNNING)
```

### TS-07-2: State Transitions During Install

**Requirement:** 07-REQ-1.2
**Type:** unit
**Description:** During successful installation, the adapter transitions through DOWNLOADING → INSTALLING → RUNNING.

**Preconditions:**
- Mock ContainerRuntime succeeds on all operations.
- WatchAdapterStates subscriber active.

**Input:**
- InstallAdapter call.

**Expected:**
- Three state events: (UNKNOWN→DOWNLOADING), (DOWNLOADING→INSTALLING), (INSTALLING→RUNNING).

**Assertion pseudocode:**
```
events = collect_events_during(install_adapter(image_ref, checksum))
ASSERT events[0] == (UNKNOWN, DOWNLOADING)
ASSERT events[1] == (DOWNLOADING, INSTALLING)
ASSERT events[2] == (INSTALLING, RUNNING)
```

### TS-07-3: Checksum Verification

**Requirement:** 07-REQ-1.3
**Type:** unit
**Description:** After pulling, the service compares the image digest against the provided checksum.

**Preconditions:**
- Mock ContainerRuntime: pull succeeds, inspect_digest returns "sha256:abc123def456".

**Input:**
- `checksum_sha256: "sha256:abc123def456"` (matching)

**Expected:**
- Verification passes, installation continues to INSTALLING.

**Assertion pseudocode:**
```
service.install_adapter(image_ref, "sha256:abc123def456")
adapter = state_manager.get("parkhaus-munich-v1.0.0")
ASSERT adapter.state != ERROR
```

### TS-07-4: Container Started with Host Networking

**Requirement:** 07-REQ-1.4
**Type:** unit
**Description:** The container is started with `--network=host`.

**Preconditions:**
- Mock ContainerRuntime recording run arguments.

**Input:**
- InstallAdapter call.

**Expected:**
- Container run was called (mock recorded the call with image_ref and adapter_id).

**Assertion pseudocode:**
```
service.install_adapter(image_ref, checksum)
ASSERT mock_runtime.run_called_with(image_ref, adapter_id)
// PodmanRuntime implementation verifies --network=host via integration test
```

### TS-07-5: Adapter ID Derivation

**Requirement:** 07-REQ-1.5
**Type:** unit
**Description:** adapter_id is derived from image_ref's last path segment and tag.

**Preconditions:**
- None.

**Input:**
- `"us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0"` → `"parkhaus-munich-v1.0.0"`
- `"registry.io/repo/my-adapter:latest"` → `"my-adapter-latest"`

**Expected:**
- Deterministic, human-readable adapter_id.

**Assertion pseudocode:**
```
ASSERT derive_adapter_id("us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0") == "parkhaus-munich-v1.0.0"
ASSERT derive_adapter_id("registry.io/repo/my-adapter:latest") == "my-adapter-latest"
```

### TS-07-6: Single Adapter Stops Running Before New Install

**Requirement:** 07-REQ-2.1
**Type:** unit
**Description:** When InstallAdapter is called while another adapter is RUNNING, the running adapter is stopped first.

**Preconditions:**
- Adapter "old-adapter" in RUNNING state.

**Input:**
- InstallAdapter with a new image_ref.

**Expected:**
- "old-adapter" transitions to STOPPED.
- New adapter installation proceeds.

**Assertion pseudocode:**
```
install_adapter("old-image:v1", checksum1)  // now RUNNING
install_adapter("new-image:v1", checksum2)
ASSERT state_manager.get("old-adapter").state == STOPPED
ASSERT state_manager.get("new-adapter").state == RUNNING
```

### TS-07-7: Previous Adapter Stopped State

**Requirement:** 07-REQ-2.2
**Type:** unit
**Description:** The previously running adapter transitions to STOPPED before the new one starts.

**Preconditions:**
- Adapter "adapter-a" in RUNNING state.

**Input:**
- InstallAdapter with new image.

**Expected:**
- State events show adapter-a transitioning to STOPPED before new adapter starts.

**Assertion pseudocode:**
```
events = collect_events_during(install_adapter("new-image:v1", checksum))
stop_event = find_event(events, adapter_id="adapter-a", new_state=STOPPED)
ASSERT stop_event EXISTS
ASSERT stop_event.timestamp <= first_event_for_new_adapter.timestamp
```

### TS-07-8: Watch Adapter States Stream

**Requirement:** 07-REQ-3.1
**Type:** unit
**Description:** WatchAdapterStates returns a stream of state events for subsequent transitions.

**Preconditions:**
- Active subscriber.

**Input:**
- Subscribe, then trigger an InstallAdapter.

**Expected:**
- Subscriber receives state transition events.

**Assertion pseudocode:**
```
rx = state_manager.subscribe()
install_adapter(image_ref, checksum)
event = rx.recv()
ASSERT event.adapter_id != ""
ASSERT event.new_state != UNKNOWN
```

### TS-07-9: Multiple Watch Subscribers

**Requirement:** 07-REQ-3.2
**Type:** unit
**Description:** Multiple WatchAdapterStates subscribers each receive all events.

**Preconditions:**
- Two active subscribers.

**Input:**
- Trigger a state transition.

**Expected:**
- Both subscribers receive the same event.

**Assertion pseudocode:**
```
rx1 = state_manager.subscribe()
rx2 = state_manager.subscribe()
state_manager.transition("adapter-1", RUNNING)
event1 = rx1.recv()
event2 = rx2.recv()
ASSERT event1.adapter_id == event2.adapter_id
ASSERT event1.new_state == event2.new_state
```

### TS-07-10: State Event Fields

**Requirement:** 07-REQ-3.3
**Type:** unit
**Description:** Each AdapterStateEvent includes adapter_id, old_state, new_state, and timestamp.

**Preconditions:**
- Subscriber active.

**Input:**
- Trigger a state transition.

**Expected:**
- Event has all four fields populated.

**Assertion pseudocode:**
```
rx = state_manager.subscribe()
state_manager.transition("adapter-1", INSTALLING)
event = rx.recv()
ASSERT event.adapter_id == "adapter-1"
ASSERT event.old_state == DOWNLOADING
ASSERT event.new_state == INSTALLING
ASSERT event.timestamp > 0
```

### TS-07-11: List Adapters

**Requirement:** 07-REQ-4.1
**Type:** unit
**Description:** ListAdapters returns all known adapters with current states.

**Preconditions:**
- Two adapters in state manager.

**Input:**
- ListAdapters call.

**Expected:**
- Returns 2 adapters with correct states.

**Assertion pseudocode:**
```
state_manager.create_adapter("a1", "img1", "chk1")
state_manager.create_adapter("a2", "img2", "chk2")
list = state_manager.list()
ASSERT len(list) == 2
```

### TS-07-12: Get Adapter Status

**Requirement:** 07-REQ-4.2
**Type:** unit
**Description:** GetAdapterStatus returns the current status of a specific adapter.

**Preconditions:**
- Adapter "a1" exists in RUNNING state.

**Input:**
- GetAdapterStatus("a1").

**Expected:**
- Returns adapter info with state=RUNNING, image_ref, timestamps.

**Assertion pseudocode:**
```
info = state_manager.get("a1")
ASSERT info.state == RUNNING
ASSERT info.image_ref != ""
ASSERT info.created_at > 0
```

### TS-07-13: Remove Adapter

**Requirement:** 07-REQ-5.1
**Type:** unit
**Description:** RemoveAdapter stops container, removes container and image, deletes from state.

**Preconditions:**
- Adapter "a1" in RUNNING state. Mock ContainerRuntime succeeds.

**Input:**
- RemoveAdapter("a1").

**Expected:**
- Mock runtime stop, remove, remove_image called.
- Adapter no longer in state manager.

**Assertion pseudocode:**
```
service.remove_adapter("a1")
ASSERT state_manager.get("a1") == None
ASSERT mock_runtime.stop_called
ASSERT mock_runtime.remove_called
ASSERT mock_runtime.remove_image_called
```

### TS-07-14: Remove Adapter State Transitions

**Requirement:** 07-REQ-5.2
**Type:** unit
**Description:** During removal, adapter transitions through STOPPED → OFFLOADING → (removed).

**Preconditions:**
- Adapter "a1" in RUNNING state.

**Input:**
- RemoveAdapter("a1").

**Expected:**
- Events: RUNNING→STOPPED, STOPPED→OFFLOADING.

**Assertion pseudocode:**
```
events = collect_events_during(remove_adapter("a1"))
ASSERT (RUNNING, STOPPED) IN events
ASSERT (STOPPED, OFFLOADING) IN events
```

### TS-07-15: Remove Adapter Events Emitted

**Requirement:** 07-REQ-5.3
**Type:** unit
**Description:** State transition events are emitted during adapter removal.

**Preconditions:**
- Subscriber active.

**Input:**
- RemoveAdapter on a running adapter.

**Expected:**
- Subscriber receives state transition events.

**Assertion pseudocode:**
```
rx = state_manager.subscribe()
remove_adapter("a1")
events = collect_all(rx)
ASSERT len(events) >= 2
```

### TS-07-16: Automatic Offloading

**Requirement:** 07-REQ-6.1
**Type:** unit
**Description:** Stopped adapters past the inactivity timeout are automatically offloaded.

**Preconditions:**
- Adapter "a1" in STOPPED state, stopped_at set to 25 hours ago. Inactivity timeout is 24 hours.

**Input:**
- Offload timer triggers check.

**Expected:**
- Adapter "a1" transitions to OFFLOADING, then removed.

**Assertion pseudocode:**
```
state_manager.create_adapter("a1", ...)
state_manager.transition("a1", STOPPED)
// Set stopped_at to 25 hours ago
expired = state_manager.get_stopped_expired(86400)
ASSERT "a1" IN expired
```

### TS-07-17: Configurable Inactivity Timeout

**Requirement:** 07-REQ-6.2
**Type:** unit
**Description:** The inactivity timeout is loaded from config.

**Preconditions:**
- Config file with `inactivity_timeout_secs: 3600`.

**Input:**
- LoadConfig.

**Expected:**
- Config.inactivity_timeout_secs == 3600.

**Assertion pseudocode:**
```
cfg = load_config(path_with_timeout_3600)
ASSERT cfg.inactivity_timeout_secs == 3600
```

### TS-07-18: Offloading Events Emitted

**Requirement:** 07-REQ-6.3
**Type:** unit
**Description:** State transition events are emitted during automatic offloading.

**Preconditions:**
- Subscriber active. Adapter past inactivity threshold.

**Input:**
- Offload timer triggers.

**Expected:**
- Events emitted for STOPPED→OFFLOADING.

**Assertion pseudocode:**
```
rx = state_manager.subscribe()
trigger_offload_check()
event = rx.recv()
ASSERT event.new_state == OFFLOADING
```

### TS-07-19: Config Loading

**Requirement:** 07-REQ-7.1
**Type:** unit
**Description:** Configuration is loaded from CONFIG_PATH env var.

**Preconditions:**
- Temp config file.

**Input:**
- load_config(path).

**Expected:**
- Config values match file.

**Assertion pseudocode:**
```
cfg = load_config("/tmp/test-config.json")
ASSERT cfg.grpc_port == 50053
```

### TS-07-20: Config Fields

**Requirement:** 07-REQ-7.2
**Type:** unit
**Description:** Config includes port, registry URL, inactivity timeout, and storage path.

**Preconditions:**
- Full config file.

**Input:**
- load_config.

**Expected:**
- All fields populated.

**Assertion pseudocode:**
```
cfg = load_config(full_config_path)
ASSERT cfg.grpc_port > 0
ASSERT cfg.registry_url != ""
ASSERT cfg.inactivity_timeout_secs > 0
ASSERT cfg.container_storage_path != ""
```

### TS-07-21: Config Defaults

**Requirement:** 07-REQ-7.3
**Type:** unit
**Description:** Missing fields use defaults.

**Preconditions:**
- Config file with empty JSON `{}`.

**Input:**
- load_config.

**Expected:**
- Defaults: port=50052, timeout=86400, storage=/var/lib/containers/adapters/.

**Assertion pseudocode:**
```
cfg = load_config(empty_config)
ASSERT cfg.grpc_port == 50052
ASSERT cfg.inactivity_timeout_secs == 86400
ASSERT cfg.container_storage_path == "/var/lib/containers/adapters/"
```

### TS-07-22: Startup Logging

**Requirement:** 07-REQ-8.1
**Type:** integration
**Description:** On startup, the service logs version, port, registry URL.

**Preconditions:**
- Service starts.

**Input:**
- Capture startup logs.

**Expected:**
- Logs contain port, registry URL.

**Assertion pseudocode:**
```
output = captureStartupLogs()
ASSERT "50052" IN output
ASSERT "registry" IN output
```

### TS-07-23: Graceful Shutdown

**Requirement:** 07-REQ-8.2
**Type:** integration
**Description:** SIGTERM stops running adapters and exits with code 0.

**Preconditions:**
- Service running.

**Input:**
- Send SIGTERM.

**Expected:**
- Service exits with code 0.

**Assertion pseudocode:**
```
proc = startService()
proc.Signal(SIGTERM)
exitCode = proc.Wait()
ASSERT exitCode == 0
```

## Edge Case Tests

### TS-07-E1: Empty Image Ref or Checksum

**Requirement:** 07-REQ-1.E1
**Type:** unit
**Description:** Empty image_ref or checksum returns INVALID_ARGUMENT.

**Preconditions:**
- None.

**Input:**
- InstallAdapter with empty image_ref.
- InstallAdapter with empty checksum.

**Expected:**
- gRPC INVALID_ARGUMENT error.

**Assertion pseudocode:**
```
err = service.install_adapter("", "sha256:abc")
ASSERT err.code == INVALID_ARGUMENT

err = service.install_adapter("image:v1", "")
ASSERT err.code == INVALID_ARGUMENT
```

### TS-07-E2: Image Pull Failure

**Requirement:** 07-REQ-1.E2
**Type:** unit
**Description:** Pull failure transitions adapter to ERROR and returns UNAVAILABLE.

**Preconditions:**
- Mock ContainerRuntime: pull returns error.

**Input:**
- InstallAdapter call.

**Expected:**
- Adapter state is ERROR.
- gRPC UNAVAILABLE error.

**Assertion pseudocode:**
```
mock_runtime.set_pull_error(true)
err = service.install_adapter(image_ref, checksum)
ASSERT err.code == UNAVAILABLE
adapter = state_manager.get(adapter_id)
ASSERT adapter.state == ERROR
```

### TS-07-E3: Checksum Mismatch

**Requirement:** 07-REQ-1.E3
**Type:** unit
**Description:** Checksum mismatch transitions to ERROR, removes image, returns FAILED_PRECONDITION.

**Preconditions:**
- Mock ContainerRuntime: pull succeeds, inspect_digest returns "sha256:wrong".

**Input:**
- InstallAdapter with checksum "sha256:expected".

**Expected:**
- Adapter state is ERROR.
- remove_image called on mock.
- gRPC FAILED_PRECONDITION error.

**Assertion pseudocode:**
```
mock_runtime.set_digest("sha256:wrong")
err = service.install_adapter(image_ref, "sha256:expected")
ASSERT err.code == FAILED_PRECONDITION
ASSERT mock_runtime.remove_image_called
adapter = state_manager.get(adapter_id)
ASSERT adapter.state == ERROR
```

### TS-07-E4: Container Start Failure

**Requirement:** 07-REQ-1.E4
**Type:** unit
**Description:** Container start failure transitions to ERROR and returns INTERNAL.

**Preconditions:**
- Mock ContainerRuntime: pull and inspect succeed, run returns error.

**Input:**
- InstallAdapter call.

**Expected:**
- Adapter state is ERROR.
- gRPC INTERNAL error.

**Assertion pseudocode:**
```
mock_runtime.set_run_error(true)
err = service.install_adapter(image_ref, checksum)
ASSERT err.code == INTERNAL
adapter = state_manager.get(adapter_id)
ASSERT adapter.state == ERROR
```

### TS-07-E5: Stop Running Adapter Fails

**Requirement:** 07-REQ-2.E1
**Type:** unit
**Description:** If stopping the running adapter fails, new install is aborted with INTERNAL.

**Preconditions:**
- Adapter "old" is RUNNING. Mock ContainerRuntime: stop returns error.

**Input:**
- InstallAdapter with new image.

**Expected:**
- gRPC INTERNAL error. New adapter not created.

**Assertion pseudocode:**
```
mock_runtime.set_stop_error(true)
err = service.install_adapter("new-image:v1", checksum)
ASSERT err.code == INTERNAL
ASSERT state_manager.get("new-adapter") == None
```

### TS-07-E6: Get Unknown Adapter

**Requirement:** 07-REQ-4.E1
**Type:** unit
**Description:** GetAdapterStatus with unknown adapter_id returns NOT_FOUND.

**Preconditions:**
- No adapters in state manager.

**Input:**
- GetAdapterStatus("nonexistent").

**Expected:**
- gRPC NOT_FOUND error.

**Assertion pseudocode:**
```
result = state_manager.get("nonexistent")
ASSERT result == None
```

### TS-07-E7: Remove Unknown Adapter

**Requirement:** 07-REQ-5.E1
**Type:** unit
**Description:** RemoveAdapter with unknown adapter_id returns NOT_FOUND.

**Preconditions:**
- No adapters in state manager.

**Input:**
- RemoveAdapter("nonexistent").

**Expected:**
- gRPC NOT_FOUND error.

**Assertion pseudocode:**
```
err = service.remove_adapter("nonexistent")
ASSERT err.code == NOT_FOUND
```

### TS-07-E8: Container Removal Failure

**Requirement:** 07-REQ-5.E2
**Type:** unit
**Description:** Container removal failure transitions to ERROR and returns INTERNAL.

**Preconditions:**
- Adapter "a1" in STOPPED state. Mock ContainerRuntime: remove returns error.

**Input:**
- RemoveAdapter("a1").

**Expected:**
- Adapter transitions to ERROR.
- gRPC INTERNAL error.

**Assertion pseudocode:**
```
mock_runtime.set_remove_error(true)
err = service.remove_adapter("a1")
ASSERT err.code == INTERNAL
adapter = state_manager.get("a1")
ASSERT adapter.state == ERROR
```

### TS-07-E9: Config File Missing

**Requirement:** 07-REQ-7.E1
**Type:** unit
**Description:** Missing config file uses defaults with warning.

**Preconditions:**
- No file at path.

**Input:**
- load_config("/nonexistent/config.json").

**Expected:**
- Returns default config. No error.

**Assertion pseudocode:**
```
cfg = load_config("/nonexistent/config.json")
ASSERT cfg.grpc_port == 50052
```

### TS-07-E10: Config Invalid JSON

**Requirement:** 07-REQ-7.E2
**Type:** unit
**Description:** Invalid JSON config returns error.

**Preconditions:**
- File containing `{invalid`.

**Input:**
- load_config(invalid_path).

**Expected:**
- Returns error.

**Assertion pseudocode:**
```
result = load_config(invalid_json_path)
ASSERT result.is_err()
```

## Property Test Cases

### TS-07-P1: State Machine Validity

**Property:** Property 1 from design.md
**Validates:** 07-REQ-1.2, 07-REQ-5.2
**Type:** property
**Description:** Only valid state transitions from the transition table are accepted.

**For any:** Random adapter state and random target state.
**Invariant:** The transition succeeds if and only if (from, to) is in the valid transition table.

**Assertion pseudocode:**
```
FOR ANY from_state IN all_states, to_state IN all_states:
    result = state_manager.transition(adapter, to_state)
    IF (from_state, to_state) IN valid_transitions:
        ASSERT result.is_ok()
    ELSE:
        ASSERT result.is_err()
```

### TS-07-P2: Single Adapter Constraint

**Property:** Property 2 from design.md
**Validates:** 07-REQ-2.1, 07-REQ-2.2
**Type:** property
**Description:** At most one adapter is in RUNNING state at any time.

**For any:** Random sequence of InstallAdapter calls.
**Invariant:** After each operation, at most one adapter is RUNNING.

**Assertion pseudocode:**
```
FOR ANY operations IN random_install_sequences:
    FOR op IN operations:
        service.install_adapter(op.image_ref, op.checksum)
    running = state_manager.list().filter(|a| a.state == RUNNING)
    ASSERT len(running) <= 1
```

### TS-07-P3: Checksum Gate

**Property:** Property 3 from design.md
**Validates:** 07-REQ-1.3, 07-REQ-1.E3
**Type:** property
**Description:** Mismatched checksums always result in ERROR, never RUNNING.

**For any:** Random image digest and random provided checksum where they differ.
**Invariant:** Adapter state is ERROR and container was not started.

**Assertion pseudocode:**
```
FOR ANY digest IN random_digests, checksum IN random_checksums:
    IF digest != checksum:
        mock_runtime.set_digest(digest)
        service.install_adapter(image_ref, checksum)
        ASSERT state_manager.get(adapter_id).state == ERROR
        ASSERT mock_runtime.run_not_called
```

### TS-07-P4: State Event Broadcasting

**Property:** Property 4 from design.md
**Validates:** 07-REQ-3.1, 07-REQ-3.2, 07-REQ-3.3
**Type:** property
**Description:** Every state transition emits an event with correct fields to all subscribers.

**For any:** Random number of subscribers (1-10) and random state transition.
**Invariant:** All subscribers receive the event with matching adapter_id, old_state, new_state, and non-zero timestamp.

**Assertion pseudocode:**
```
FOR ANY n_subs IN 1..10, transition IN valid_transitions:
    subscribers = (0..n_subs).map(|_| state_manager.subscribe())
    state_manager.transition(adapter, transition.to)
    FOR sub IN subscribers:
        event = sub.recv()
        ASSERT event.adapter_id == adapter
        ASSERT event.old_state == transition.from
        ASSERT event.new_state == transition.to
        ASSERT event.timestamp > 0
```

### TS-07-P5: Adapter ID Derivation

**Property:** Property 5 from design.md
**Validates:** 07-REQ-1.5
**Type:** property
**Description:** derive_adapter_id is deterministic and non-empty for valid image refs.

**For any:** Random valid OCI image references (registry/path:tag format).
**Invariant:** Result is non-empty, deterministic (same input → same output), and contains the image name.

**Assertion pseudocode:**
```
FOR ANY image_ref IN random_oci_refs:
    id1 = derive_adapter_id(image_ref)
    id2 = derive_adapter_id(image_ref)
    ASSERT id1 == id2
    ASSERT id1 != ""
```

### TS-07-P6: Inactivity Offloading

**Property:** Property 6 from design.md
**Validates:** 07-REQ-6.1, 07-REQ-6.2
**Type:** property
**Description:** Stopped adapters past the timeout appear in the expired list.

**For any:** Random timeout values and random stopped_at times.
**Invariant:** If now - stopped_at > timeout, the adapter is in the expired list.

**Assertion pseudocode:**
```
FOR ANY timeout IN random_durations, age IN random_ages:
    // Set adapter stopped_at to now - age
    expired = state_manager.get_stopped_expired(timeout)
    IF age > timeout:
        ASSERT adapter_id IN expired
    ELSE:
        ASSERT adapter_id NOT IN expired
```

### TS-07-P7: Config Defaults

**Property:** Property 7 from design.md
**Validates:** 07-REQ-7.1, 07-REQ-7.3, 07-REQ-7.E1
**Type:** property
**Description:** Missing config files always return valid defaults.

**For any:** Random nonexistent file paths.
**Invariant:** Config has port=50052, timeout=86400, valid storage path.

**Assertion pseudocode:**
```
FOR ANY path IN random_nonexistent_paths:
    cfg = load_config(path)
    ASSERT cfg.grpc_port == 50052
    ASSERT cfg.inactivity_timeout_secs == 86400
    ASSERT cfg.container_storage_path == "/var/lib/containers/adapters/"
```

## Coverage Matrix

| Requirement | Test Spec Entry | Type |
|-------------|-----------------|------|
| 07-REQ-1.1 | TS-07-1 | unit |
| 07-REQ-1.2 | TS-07-2 | unit |
| 07-REQ-1.3 | TS-07-3 | unit |
| 07-REQ-1.4 | TS-07-4 | unit |
| 07-REQ-1.5 | TS-07-5 | unit |
| 07-REQ-1.E1 | TS-07-E1 | unit |
| 07-REQ-1.E2 | TS-07-E2 | unit |
| 07-REQ-1.E3 | TS-07-E3 | unit |
| 07-REQ-1.E4 | TS-07-E4 | unit |
| 07-REQ-2.1 | TS-07-6 | unit |
| 07-REQ-2.2 | TS-07-7 | unit |
| 07-REQ-2.E1 | TS-07-E5 | unit |
| 07-REQ-3.1 | TS-07-8 | unit |
| 07-REQ-3.2 | TS-07-9 | unit |
| 07-REQ-3.3 | TS-07-10 | unit |
| 07-REQ-4.1 | TS-07-11 | unit |
| 07-REQ-4.2 | TS-07-12 | unit |
| 07-REQ-4.E1 | TS-07-E6 | unit |
| 07-REQ-5.1 | TS-07-13 | unit |
| 07-REQ-5.2 | TS-07-14 | unit |
| 07-REQ-5.3 | TS-07-15 | unit |
| 07-REQ-5.E1 | TS-07-E7 | unit |
| 07-REQ-5.E2 | TS-07-E8 | unit |
| 07-REQ-6.1 | TS-07-16 | unit |
| 07-REQ-6.2 | TS-07-17 | unit |
| 07-REQ-6.3 | TS-07-18 | unit |
| 07-REQ-7.1 | TS-07-19 | unit |
| 07-REQ-7.2 | TS-07-20 | unit |
| 07-REQ-7.3 | TS-07-21 | unit |
| 07-REQ-7.E1 | TS-07-E9 | unit |
| 07-REQ-7.E2 | TS-07-E10 | unit |
| 07-REQ-8.1 | TS-07-22 | integration |
| 07-REQ-8.2 | TS-07-23 | integration |
| Property 1 | TS-07-P1 | property |
| Property 2 | TS-07-P2 | property |
| Property 3 | TS-07-P3 | property |
| Property 4 | TS-07-P4 | property |
| Property 5 | TS-07-P5 | property |
| Property 6 | TS-07-P6 | property |
| Property 7 | TS-07-P7 | property |
