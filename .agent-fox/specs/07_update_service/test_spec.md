# Test Specification: UPDATE_SERVICE

## Overview

This test specification defines concrete test contracts for the UPDATE_SERVICE, a Rust gRPC server managing the lifecycle of containerized PARKING_OPERATOR_ADAPTORs. Tests are organized into unit tests (config, adapter, state, podman modules), integration tests (gRPC client against running service with mock podman), and property tests (proptest). All Rust tests run via `cd rhivos && cargo test -p update-service`. Integration tests run via `cd tests/update-service && go test -v ./...`.

## Test Cases

### TS-07-1: InstallAdapter Returns Response Immediately

**Requirement:** 07-REQ-1.1
**Type:** unit
**Description:** An `InstallAdapter` call returns an `InstallAdapterResponse` with a UUID v4 `job_id`, the derived `adapter_id`, and initial state DOWNLOADING.

**Preconditions:**
- State manager is empty (no adapters installed).
- Mock podman executor configured (pull succeeds after delay).

**Input:**
- `InstallAdapter(image_ref: "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0", checksum_sha256: "sha256:abc123")`

**Expected:**
- Response contains non-empty `job_id` matching UUID v4 format.
- Response contains `adapter_id` equal to `"parkhaus-munich-v1.0.0"`.
- Response contains `state` equal to `DOWNLOADING`.

**Assertion pseudocode:**
```
mock_podman = MockPodmanExecutor::new()
mock_podman.set_pull_result(Ok(()))
mock_podman.set_inspect_result(Ok("sha256:abc123"))
state_mgr = StateManager::new(broadcast_tx)
resp = grpc_service.install_adapter("us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0", "sha256:abc123")
ASSERT resp.job_id matches UUID_V4_REGEX
ASSERT resp.adapter_id == "parkhaus-munich-v1.0.0"
ASSERT resp.state == DOWNLOADING
```

### TS-07-2: Podman Pull Executed on Install

**Requirement:** 07-REQ-1.2
**Type:** unit
**Description:** After `InstallAdapter`, the service executes `podman pull` with the provided image_ref.

**Preconditions:**
- Mock podman executor tracks calls.

**Input:**
- `InstallAdapter(image_ref: "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0", checksum_sha256: "sha256:abc123")`

**Expected:**
- Mock podman executor received a `pull("us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0")` call.

**Assertion pseudocode:**
```
mock_podman = MockPodmanExecutor::new()
mock_podman.set_pull_result(Ok(()))
mock_podman.set_inspect_result(Ok("sha256:abc123"))
mock_podman.set_run_result(Ok(()))
grpc_service.install_adapter("us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0", "sha256:abc123")
tokio::time::sleep(100ms).await  // allow async task to run
ASSERT mock_podman.pull_calls() == ["us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0"]
```

### TS-07-3: Checksum Verification After Pull

**Requirement:** 07-REQ-1.3
**Type:** unit
**Description:** After pulling, the service inspects the image digest and compares it with the provided checksum.

**Preconditions:**
- Mock podman: pull succeeds, inspect returns `"sha256:abc123"`.

**Input:**
- `InstallAdapter(image_ref: "...", checksum_sha256: "sha256:abc123")`

**Expected:**
- Mock podman received an `inspect_digest` call.
- Adapter transitions to INSTALLING (checksum matches).

**Assertion pseudocode:**
```
mock_podman = MockPodmanExecutor::new()
mock_podman.set_pull_result(Ok(()))
mock_podman.set_inspect_result(Ok("sha256:abc123"))
mock_podman.set_run_result(Ok(()))
grpc_service.install_adapter(image_ref, "sha256:abc123")
tokio::time::sleep(100ms).await
ASSERT mock_podman.inspect_digest_calls().len() == 1
adapter = state_mgr.get_adapter("parkhaus-munich-v1.0.0")
ASSERT adapter.state IN [Installing, Running]  // may have progressed
```

### TS-07-4: Container Started With Network Host

**Requirement:** 07-REQ-1.4
**Type:** unit
**Description:** On checksum match, the service starts the container with `--network=host` and the derived adapter_id as the container name.

**Preconditions:**
- Mock podman: pull succeeds, inspect returns matching checksum, run succeeds.

**Input:**
- `InstallAdapter(image_ref: "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0", checksum_sha256: "sha256:abc123")`

**Expected:**
- Mock podman received a `run("parkhaus-munich-v1.0.0", "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0")` call.

**Assertion pseudocode:**
```
mock_podman = MockPodmanExecutor::new()
mock_podman.set_pull_result(Ok(()))
mock_podman.set_inspect_result(Ok("sha256:abc123"))
mock_podman.set_run_result(Ok(()))
grpc_service.install_adapter(image_ref, "sha256:abc123")
tokio::time::sleep(100ms).await
ASSERT mock_podman.run_calls() == [("parkhaus-munich-v1.0.0", image_ref)]
```

### TS-07-5: State Transitions to RUNNING on Success

**Requirement:** 07-REQ-1.5
**Type:** unit
**Description:** After successful container start, the adapter state transitions to RUNNING.

**Preconditions:**
- Mock podman: all operations succeed.

**Input:**
- `InstallAdapter` with valid inputs.

**Expected:**
- Adapter final state is RUNNING.

**Assertion pseudocode:**
```
// ... setup mock podman with all successes ...
grpc_service.install_adapter(image_ref, checksum)
tokio::time::sleep(200ms).await
adapter = state_mgr.get_adapter("parkhaus-munich-v1.0.0")
ASSERT adapter.state == Running
```

### TS-07-6: Adapter ID Derivation

**Requirement:** 07-REQ-1.6
**Type:** unit
**Description:** The adapter_id is derived from the image_ref by extracting the last path segment and replacing the colon with a hyphen.

**Preconditions:** None.

**Input:**
- `"us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0"`
- `"registry.example.com/my-adapter:latest"`
- `"simple-image:v2"`

**Expected:**
- `"parkhaus-munich-v1.0.0"`
- `"my-adapter-latest"`
- `"simple-image-v2"`

**Assertion pseudocode:**
```
ASSERT derive_adapter_id("us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0") == "parkhaus-munich-v1.0.0"
ASSERT derive_adapter_id("registry.example.com/my-adapter:latest") == "my-adapter-latest"
ASSERT derive_adapter_id("simple-image:v2") == "simple-image-v2"
```

### TS-07-7: Single Adapter Constraint Stops Running Adapter

**Requirement:** 07-REQ-2.1, 07-REQ-2.2
**Type:** unit
**Description:** When InstallAdapter is called while another adapter is RUNNING, the running adapter is stopped first.

**Preconditions:**
- Adapter A is in state RUNNING.
- Mock podman: stop succeeds for adapter A, all operations succeed for adapter B.

**Input:**
- `InstallAdapter` for adapter B.

**Expected:**
- Mock podman received a `stop` call for adapter A's adapter_id.
- Adapter A transitions to STOPPED.
- Adapter B transitions to RUNNING.

**Assertion pseudocode:**
```
// Install adapter A first
grpc_service.install_adapter(image_ref_a, checksum_a)
tokio::time::sleep(200ms).await
ASSERT state_mgr.get_adapter(adapter_id_a).state == Running

// Install adapter B
grpc_service.install_adapter(image_ref_b, checksum_b)
tokio::time::sleep(200ms).await
ASSERT mock_podman.stop_calls().contains(adapter_id_a)
ASSERT state_mgr.get_adapter(adapter_id_a).state == Stopped
ASSERT state_mgr.get_adapter(adapter_id_b).state == Running
```

### TS-07-8: WatchAdapterStates Streams Events

**Requirement:** 07-REQ-3.1, 07-REQ-3.2, 07-REQ-3.3
**Type:** unit
**Description:** WatchAdapterStates returns a stream that delivers AdapterStateEvent messages for all state transitions.

**Preconditions:**
- A subscriber is connected via WatchAdapterStates.

**Input:**
- `InstallAdapter` triggers state transitions.

**Expected:**
- Subscriber receives events with adapter_id, old_state, new_state, and timestamp for each transition.

**Assertion pseudocode:**
```
stream = grpc_service.watch_adapter_states()
grpc_service.install_adapter(image_ref, checksum)
events = collect_events(stream, timeout=500ms)
ASSERT events.len() >= 3  // UNKNOWN->DOWNLOADING, DOWNLOADING->INSTALLING, INSTALLING->RUNNING
ASSERT events[0].old_state == Unknown
ASSERT events[0].new_state == Downloading
ASSERT events[0].adapter_id == "parkhaus-munich-v1.0.0"
ASSERT events[0].timestamp > 0
ASSERT events[1].old_state == Downloading
ASSERT events[1].new_state == Installing
ASSERT events[2].old_state == Installing
ASSERT events[2].new_state == Running
```

### TS-07-9: WatchAdapterStates No Historical Replay

**Requirement:** 07-REQ-3.4
**Type:** unit
**Description:** A new subscriber does not receive events from before the subscription started.

**Preconditions:**
- An adapter was installed and is RUNNING before the subscriber connects.

**Input:**
- Subscribe to WatchAdapterStates after installation completes.
- Then trigger a new state transition (e.g., stop the adapter).

**Expected:**
- Subscriber receives only the RUNNING->STOPPED event, not the earlier transitions.

**Assertion pseudocode:**
```
grpc_service.install_adapter(image_ref, checksum)
tokio::time::sleep(200ms).await
stream = grpc_service.watch_adapter_states()
grpc_service.remove_adapter(adapter_id)
events = collect_events(stream, timeout=500ms)
// Should NOT contain UNKNOWN->DOWNLOADING or DOWNLOADING->INSTALLING
FOR event IN events:
    ASSERT event.old_state != Unknown
    ASSERT event.new_state != Downloading
```

### TS-07-10: ListAdapters Returns All Known Adapters

**Requirement:** 07-REQ-4.1
**Type:** unit
**Description:** ListAdapters returns all known adapters with their current states.

**Preconditions:**
- Two adapters have been installed (one RUNNING, one STOPPED).

**Input:**
- `ListAdapters()`

**Expected:**
- Response contains exactly 2 adapters with correct adapter_ids and states.

**Assertion pseudocode:**
```
grpc_service.install_adapter(image_ref_a, checksum_a)
tokio::time::sleep(200ms).await
grpc_service.install_adapter(image_ref_b, checksum_b)  // stops A
tokio::time::sleep(200ms).await
resp = grpc_service.list_adapters()
ASSERT resp.adapters.len() == 2
ids = resp.adapters.map(|a| a.adapter_id).sort()
ASSERT ids == [adapter_id_a, adapter_id_b].sort()
```

### TS-07-11: GetAdapterStatus Returns Current State

**Requirement:** 07-REQ-4.2
**Type:** unit
**Description:** GetAdapterStatus returns the current state of a specific adapter.

**Preconditions:**
- An adapter is installed and RUNNING.

**Input:**
- `GetAdapterStatus(adapter_id)`

**Expected:**
- Response contains the adapter_id and state RUNNING.

**Assertion pseudocode:**
```
grpc_service.install_adapter(image_ref, checksum)
tokio::time::sleep(200ms).await
resp = grpc_service.get_adapter_status("parkhaus-munich-v1.0.0")
ASSERT resp.adapter_id == "parkhaus-munich-v1.0.0"
ASSERT resp.state == Running
```

### TS-07-12: RemoveAdapter Cleans Up Container and Image

**Requirement:** 07-REQ-5.1, 07-REQ-5.2
**Type:** unit
**Description:** RemoveAdapter stops the container, removes the container and image, and removes the adapter from state.

**Preconditions:**
- An adapter is installed and RUNNING.
- Mock podman: stop, rm, rmi all succeed.

**Input:**
- `RemoveAdapter(adapter_id)`

**Expected:**
- Mock podman received stop, rm, and rmi calls.
- Adapter is no longer in state manager.

**Assertion pseudocode:**
```
grpc_service.install_adapter(image_ref, checksum)
tokio::time::sleep(200ms).await
grpc_service.remove_adapter("parkhaus-munich-v1.0.0")
ASSERT mock_podman.stop_calls().contains("parkhaus-munich-v1.0.0")
ASSERT mock_podman.rm_calls().contains("parkhaus-munich-v1.0.0")
ASSERT mock_podman.rmi_calls().contains(image_ref)
ASSERT state_mgr.get_adapter("parkhaus-munich-v1.0.0").is_none()
```

### TS-07-13: Offload Timer Triggers After Inactivity

**Requirement:** 07-REQ-6.1, 07-REQ-6.2, 07-REQ-6.3, 07-REQ-6.4
**Type:** unit
**Description:** A STOPPED adapter is offloaded after the inactivity timeout expires.

**Preconditions:**
- Config with inactivity_timeout_secs = 1 (short for testing).
- An adapter is in STOPPED state.
- Mock podman: rm, rmi succeed.

**Input:**
- Wait for offload timer to fire.

**Expected:**
- Adapter transitions to OFFLOADING, then is removed from state.
- Mock podman received rm and rmi calls.
- Event subscribers received STOPPED->OFFLOADING event.

**Assertion pseudocode:**
```
config.inactivity_timeout_secs = 1
// ... install and stop adapter ...
ASSERT state_mgr.get_adapter(adapter_id).state == Stopped
tokio::time::sleep(2s).await  // wait for offload timer
ASSERT state_mgr.get_adapter(adapter_id).is_none()
ASSERT mock_podman.rm_calls().contains(adapter_id)
ASSERT mock_podman.rmi_calls().contains(image_ref)
ASSERT events_received.contains(AdapterStateEvent { old_state: Stopped, new_state: Offloading })
```

### TS-07-14: Config Loading From File

**Requirement:** 07-REQ-7.1, 07-REQ-7.2
**Type:** unit
**Description:** The service loads configuration from a JSON file specified by CONFIG_PATH.

**Preconditions:**
- A temporary JSON config file with custom values.

**Input:**
- `load_config("/tmp/test-config.json")`

**Expected:**
- Config struct populated with values from the file.

**Assertion pseudocode:**
```
write_file("/tmp/test-config.json", r#"{"grpc_port":50099,"registry_url":"example.com","inactivity_timeout_secs":3600,"container_storage_path":"/tmp/adapters/"}"#)
cfg = config::load_config("/tmp/test-config.json")
ASSERT cfg.grpc_port == 50099
ASSERT cfg.registry_url == "example.com"
ASSERT cfg.inactivity_timeout_secs == 3600
ASSERT cfg.container_storage_path == "/tmp/adapters/"
```

### TS-07-15: Container Exit Non-Zero Transitions to ERROR

**Requirement:** 07-REQ-9.1
**Type:** unit
**Description:** When a running container exits with a non-zero exit code, the adapter transitions to ERROR.

**Preconditions:**
- An adapter is RUNNING.
- Mock podman: wait returns exit code 1.

**Input:**
- Container exits (mock podman wait completes with code 1).

**Expected:**
- Adapter state transitions to ERROR.
- Error state event emitted.

**Assertion pseudocode:**
```
mock_podman.set_wait_result(Ok(1))  // non-zero exit
grpc_service.install_adapter(image_ref, checksum)
tokio::time::sleep(200ms).await  // container starts and then "exits"
adapter = state_mgr.get_adapter(adapter_id)
ASSERT adapter.state == Error
```

### TS-07-16: Container Exit Code Zero Transitions to STOPPED

**Requirement:** 07-REQ-9.2
**Type:** unit
**Description:** When a running container exits with exit code 0, the adapter transitions to STOPPED.

**Preconditions:**
- An adapter is RUNNING.
- Mock podman: wait returns exit code 0.

**Input:**
- Container exits (mock podman wait completes with code 0).

**Expected:**
- Adapter state transitions to STOPPED.
- STOPPED state event emitted.

**Assertion pseudocode:**
```
mock_podman.set_wait_result(Ok(0))  // clean exit
grpc_service.install_adapter(image_ref, checksum)
tokio::time::sleep(200ms).await
adapter = state_mgr.get_adapter(adapter_id)
ASSERT adapter.state == Stopped
```

### TS-07-17: Startup Logging

**Requirement:** 07-REQ-10.1
**Type:** integration
**Description:** On startup, the service logs its configuration and a ready message.

**Preconditions:**
- Service starts with default config.

**Input:**
- Capture stdout/stderr during startup.

**Expected:**
- Log output contains port number (50052), inactivity timeout, and ready indicator.

**Assertion pseudocode:**
```
output = captureStartupLogs()
ASSERT "50052" IN output
ASSERT "ready" IN output.to_lowercase()
```

### TS-07-18: Graceful Shutdown

**Requirement:** 07-REQ-10.2
**Type:** integration
**Description:** On SIGTERM, the service stops accepting RPCs and exits with code 0.

**Preconditions:**
- Service is running.

**Input:**
- Send SIGTERM to the service process.

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

### TS-07-E1: Empty image_ref Returns INVALID_ARGUMENT

**Requirement:** 07-REQ-1.E1
**Type:** unit
**Description:** InstallAdapter with an empty image_ref returns INVALID_ARGUMENT.

**Preconditions:** None.

**Input:**
- `InstallAdapter(image_ref: "", checksum_sha256: "sha256:abc123")`

**Expected:**
- gRPC status INVALID_ARGUMENT with message containing `"image_ref is required"`.

**Assertion pseudocode:**
```
result = grpc_service.install_adapter("", "sha256:abc123")
ASSERT result.is_err()
ASSERT result.err().code() == INVALID_ARGUMENT
ASSERT result.err().message().contains("image_ref is required")
```

### TS-07-E2: Empty checksum_sha256 Returns INVALID_ARGUMENT

**Requirement:** 07-REQ-1.E2
**Type:** unit
**Description:** InstallAdapter with an empty checksum returns INVALID_ARGUMENT.

**Preconditions:** None.

**Input:**
- `InstallAdapter(image_ref: "example.com/img:v1", checksum_sha256: "")`

**Expected:**
- gRPC status INVALID_ARGUMENT with message containing `"checksum_sha256 is required"`.

**Assertion pseudocode:**
```
result = grpc_service.install_adapter("example.com/img:v1", "")
ASSERT result.is_err()
ASSERT result.err().code() == INVALID_ARGUMENT
ASSERT result.err().message().contains("checksum_sha256 is required")
```

### TS-07-E3: Podman Pull Failure Transitions to ERROR

**Requirement:** 07-REQ-1.E3
**Type:** unit
**Description:** When podman pull fails, the adapter transitions to ERROR.

**Preconditions:**
- Mock podman: pull returns error with stderr "connection refused".

**Input:**
- `InstallAdapter(image_ref: "bad-registry.com/img:v1", checksum_sha256: "sha256:abc")`

**Expected:**
- Adapter state is ERROR.
- Error event emitted with podman stderr.

**Assertion pseudocode:**
```
mock_podman.set_pull_result(Err(PodmanError::new("connection refused")))
grpc_service.install_adapter("bad-registry.com/img:v1", "sha256:abc")
tokio::time::sleep(200ms).await
adapter = state_mgr.get_adapter(adapter_id)
ASSERT adapter.state == Error
ASSERT adapter.error_message.contains("connection refused")
```

### TS-07-E4: Checksum Mismatch Transitions to ERROR and Removes Image

**Requirement:** 07-REQ-1.E4
**Type:** unit
**Description:** When the pulled image digest does not match the provided checksum, the adapter transitions to ERROR and the image is removed.

**Preconditions:**
- Mock podman: pull succeeds, inspect returns `"sha256:different"`.

**Input:**
- `InstallAdapter(image_ref: "example.com/img:v1", checksum_sha256: "sha256:expected")`

**Expected:**
- Adapter state is ERROR with error_message containing `"checksum_mismatch"`.
- Mock podman received an `rmi` call for the image.

**Assertion pseudocode:**
```
mock_podman.set_pull_result(Ok(()))
mock_podman.set_inspect_result(Ok("sha256:different"))
grpc_service.install_adapter("example.com/img:v1", "sha256:expected")
tokio::time::sleep(200ms).await
adapter = state_mgr.get_adapter(adapter_id)
ASSERT adapter.state == Error
ASSERT adapter.error_message.contains("checksum_mismatch")
ASSERT mock_podman.rmi_calls().contains("example.com/img:v1")
```

### TS-07-E5: Podman Run Failure Transitions to ERROR

**Requirement:** 07-REQ-1.E5
**Type:** unit
**Description:** When podman run fails, the adapter transitions to ERROR.

**Preconditions:**
- Mock podman: pull and inspect succeed, run returns error.

**Input:**
- `InstallAdapter(image_ref: "example.com/img:v1", checksum_sha256: "sha256:abc")`

**Expected:**
- Adapter state is ERROR.

**Assertion pseudocode:**
```
mock_podman.set_pull_result(Ok(()))
mock_podman.set_inspect_result(Ok("sha256:abc"))
mock_podman.set_run_result(Err(PodmanError::new("container create failed")))
grpc_service.install_adapter("example.com/img:v1", "sha256:abc")
tokio::time::sleep(200ms).await
adapter = state_mgr.get_adapter(adapter_id)
ASSERT adapter.state == Error
```

### TS-07-E6: Stop Running Adapter Fails But Install Proceeds

**Requirement:** 07-REQ-2.E1
**Type:** unit
**Description:** If stopping the running adapter fails, the old adapter transitions to ERROR but the new install still proceeds.

**Preconditions:**
- Adapter A is RUNNING.
- Mock podman: stop returns error for adapter A, all operations succeed for adapter B.

**Input:**
- `InstallAdapter` for adapter B.

**Expected:**
- Adapter A state is ERROR.
- Adapter B state is RUNNING (install proceeded).

**Assertion pseudocode:**
```
// adapter A is RUNNING
mock_podman.set_stop_result_for(adapter_id_a, Err(PodmanError::new("timeout")))
grpc_service.install_adapter(image_ref_b, checksum_b)
tokio::time::sleep(200ms).await
ASSERT state_mgr.get_adapter(adapter_id_a).state == Error
ASSERT state_mgr.get_adapter(adapter_id_b).state == Running
```

### TS-07-E7: Subscriber Disconnect Does Not Affect Others

**Requirement:** 07-REQ-3.E1
**Type:** unit
**Description:** When one subscriber disconnects, other subscribers continue receiving events.

**Preconditions:**
- Two subscribers are connected.

**Input:**
- Subscriber 1 disconnects.
- Trigger a state transition.

**Expected:**
- Subscriber 2 receives the event.
- No panics or errors in the service.

**Assertion pseudocode:**
```
stream1 = grpc_service.watch_adapter_states()
stream2 = grpc_service.watch_adapter_states()
drop(stream1)  // disconnect subscriber 1
grpc_service.install_adapter(image_ref, checksum)
events = collect_events(stream2, timeout=500ms)
ASSERT events.len() >= 1
```

### TS-07-E8: GetAdapterStatus Unknown ID Returns NOT_FOUND

**Requirement:** 07-REQ-4.E1
**Type:** unit
**Description:** GetAdapterStatus with an unknown adapter_id returns NOT_FOUND.

**Preconditions:**
- No adapters installed.

**Input:**
- `GetAdapterStatus("nonexistent-adapter")`

**Expected:**
- gRPC status NOT_FOUND with message containing `"adapter not found"`.

**Assertion pseudocode:**
```
result = grpc_service.get_adapter_status("nonexistent-adapter")
ASSERT result.is_err()
ASSERT result.err().code() == NOT_FOUND
ASSERT result.err().message().contains("adapter not found")
```

### TS-07-E9: ListAdapters Returns Empty When None Installed

**Requirement:** 07-REQ-4.E2
**Type:** unit
**Description:** ListAdapters returns an empty list when no adapters have been installed.

**Preconditions:**
- No adapters installed.

**Input:**
- `ListAdapters()`

**Expected:**
- Response contains an empty adapters list.

**Assertion pseudocode:**
```
resp = grpc_service.list_adapters()
ASSERT resp.adapters.len() == 0
```

### TS-07-E10: RemoveAdapter Unknown ID Returns NOT_FOUND

**Requirement:** 07-REQ-5.E1
**Type:** unit
**Description:** RemoveAdapter with an unknown adapter_id returns NOT_FOUND.

**Preconditions:**
- No adapters installed.

**Input:**
- `RemoveAdapter("nonexistent-adapter")`

**Expected:**
- gRPC status NOT_FOUND with message containing `"adapter not found"`.

**Assertion pseudocode:**
```
result = grpc_service.remove_adapter("nonexistent-adapter")
ASSERT result.is_err()
ASSERT result.err().code() == NOT_FOUND
ASSERT result.err().message().contains("adapter not found")
```

### TS-07-E11: Podman Removal Failure Returns INTERNAL

**Requirement:** 07-REQ-5.E2
**Type:** unit
**Description:** When podman rm or rmi fails during RemoveAdapter, the service returns INTERNAL.

**Preconditions:**
- An adapter is STOPPED.
- Mock podman: rm returns error.

**Input:**
- `RemoveAdapter(adapter_id)`

**Expected:**
- gRPC status INTERNAL.
- Adapter state is ERROR.

**Assertion pseudocode:**
```
mock_podman.set_rm_result(Err(PodmanError::new("container in use")))
result = grpc_service.remove_adapter(adapter_id)
ASSERT result.is_err()
ASSERT result.err().code() == INTERNAL
adapter = state_mgr.get_adapter(adapter_id)
ASSERT adapter.state == Error
```

### TS-07-E12: Offload Cleanup Failure Transitions to ERROR

**Requirement:** 07-REQ-6.E1
**Type:** unit
**Description:** If offloading cleanup fails, the adapter transitions to ERROR.

**Preconditions:**
- Config with short inactivity timeout (1s).
- Adapter is STOPPED.
- Mock podman: rmi returns error.

**Input:**
- Wait for offload timer to fire.

**Expected:**
- Adapter transitions to ERROR (not removed from state).

**Assertion pseudocode:**
```
mock_podman.set_rm_result(Ok(()))
mock_podman.set_rmi_result(Err(PodmanError::new("image in use")))
// ... adapter is in STOPPED state ...
tokio::time::sleep(2s).await
adapter = state_mgr.get_adapter(adapter_id)
ASSERT adapter.state == Error
```

### TS-07-E13: Config File Missing Uses Defaults

**Requirement:** 07-REQ-7.E1
**Type:** unit
**Description:** When the config file does not exist, the service starts with built-in defaults.

**Preconditions:**
- No config file at the specified path.

**Input:**
- `load_config("/nonexistent/path/config.json")`

**Expected:**
- Returns a valid Config with default values.

**Assertion pseudocode:**
```
cfg = config::load_config("/nonexistent/path/config.json")
ASSERT cfg.is_ok()
cfg = cfg.unwrap()
ASSERT cfg.grpc_port == 50052
ASSERT cfg.inactivity_timeout_secs == 86400
ASSERT cfg.container_storage_path == "/var/lib/containers/adapters/"
```

### TS-07-E14: Invalid JSON Config Exits With Error

**Requirement:** 07-REQ-7.E2
**Type:** unit
**Description:** When the config file contains invalid JSON, load_config returns an error.

**Preconditions:**
- A temporary file containing `{invalid json`.

**Input:**
- `load_config("/tmp/invalid-config.json")`

**Expected:**
- Returns an error.

**Assertion pseudocode:**
```
write_file("/tmp/invalid-config.json", "{invalid json")
result = config::load_config("/tmp/invalid-config.json")
ASSERT result.is_err()
```

### TS-07-E15: No Subscribers Active During Transition

**Requirement:** 07-REQ-8.E1
**Type:** unit
**Description:** When no subscribers are active, state transitions still update in-memory state without errors.

**Preconditions:**
- No WatchAdapterStates subscribers.

**Input:**
- `InstallAdapter` (triggers state transitions).

**Expected:**
- Adapter reaches RUNNING state.
- No panics or errors.

**Assertion pseudocode:**
```
// No subscribers registered
grpc_service.install_adapter(image_ref, checksum)
tokio::time::sleep(200ms).await
adapter = state_mgr.get_adapter(adapter_id)
ASSERT adapter.state == Running
```

### TS-07-E16: Podman Wait Failure Transitions to ERROR

**Requirement:** 07-REQ-9.E1
**Type:** unit
**Description:** If podman wait fails, the adapter transitions to ERROR.

**Preconditions:**
- An adapter is RUNNING.
- Mock podman: wait returns error.

**Input:**
- Container monitor detects podman wait failure.

**Expected:**
- Adapter state transitions to ERROR.

**Assertion pseudocode:**
```
mock_podman.set_wait_result(Err(PodmanError::new("connection lost")))
grpc_service.install_adapter(image_ref, checksum)
tokio::time::sleep(200ms).await
adapter = state_mgr.get_adapter(adapter_id)
ASSERT adapter.state == Error
```

### TS-07-E17: Shutdown Force-Terminates After Timeout

**Requirement:** 07-REQ-10.E1
**Type:** integration
**Description:** If in-flight RPCs do not complete within 10 seconds of receiving a shutdown signal, the service force-terminates and exits with code 0.

**Preconditions:**
- Service is running with a long-running RPC in flight.

**Input:**
- Send SIGTERM while an RPC is blocked for >10 seconds.

**Expected:**
- Service exits with code 0 after 10 seconds.

**Assertion pseudocode:**
```
proc = startService()
startLongRunningRPC()  // blocks for 15+ seconds
proc.Signal(SIGTERM)
start_time = now()
exitCode = proc.Wait()
elapsed = now() - start_time
ASSERT exitCode == 0
ASSERT elapsed >= 10s
ASSERT elapsed < 12s  // force-terminated, not waiting full 15s
```

## Property Test Cases

### TS-07-P1: Adapter ID Determinism

**Property:** Property 1 from design.md
**Validates:** 07-REQ-1.6
**Type:** property
**Description:** For any valid OCI image reference, derive_adapter_id returns the same result on every call, and different last-segment:tag combinations produce different IDs.

**For any:** Random strings in the format `registry/path/name:tag` where name and tag are alphanumeric with hyphens.
**Invariant:** `derive_adapter_id(ref) == derive_adapter_id(ref)` and `name1:tag1 != name2:tag2 => derive_adapter_id(ref1) != derive_adapter_id(ref2)`.

**Assertion pseudocode:**
```
FOR ANY image_ref IN random_image_refs:
    id1 = derive_adapter_id(image_ref)
    id2 = derive_adapter_id(image_ref)
    ASSERT id1 == id2

FOR ANY ref_a, ref_b IN random_image_refs WHERE last_segment(ref_a) != last_segment(ref_b):
    ASSERT derive_adapter_id(ref_a) != derive_adapter_id(ref_b)
```

### TS-07-P2: Single Adapter Invariant

**Property:** Property 2 from design.md
**Validates:** 07-REQ-2.1, 07-REQ-2.2
**Type:** property
**Description:** At most one adapter is in RUNNING state at any time, regardless of the sequence of InstallAdapter calls.

**For any:** Sequence of 1..5 InstallAdapter calls with distinct image_refs.
**Invariant:** After each call settles, at most one adapter in the state manager has state RUNNING.

**Assertion pseudocode:**
```
FOR ANY install_sequence IN random_sequences(1..5):
    state_mgr.reset()
    FOR image_ref IN install_sequence:
        grpc_service.install_adapter(image_ref, checksum)
        tokio::time::sleep(300ms).await
        running_count = state_mgr.list_adapters().filter(|a| a.state == Running).count()
        ASSERT running_count <= 1
```

### TS-07-P3: State Transition Validity

**Property:** Property 3 from design.md
**Validates:** 07-REQ-8.1
**Type:** property
**Description:** Every observed state transition follows the valid state machine edges.

**For any:** Any sequence of operations (install, remove, stop).
**Invariant:** Each AdapterStateEvent has a valid (old_state, new_state) pair from the state machine.

**Assertion pseudocode:**
```
VALID_TRANSITIONS = {
    (Unknown, Downloading), (Downloading, Installing), (Downloading, Error),
    (Installing, Running), (Installing, Error),
    (Running, Stopped), (Running, Error),
    (Stopped, Running), (Stopped, Offloading),
    (Offloading, Error),
}
FOR ANY events IN observed_events:
    FOR event IN events:
        ASSERT (event.old_state, event.new_state) IN VALID_TRANSITIONS
```

### TS-07-P4: Event Delivery Completeness

**Property:** Property 4 from design.md
**Validates:** 07-REQ-3.3, 07-REQ-8.3
**Type:** property
**Description:** All active subscribers receive the same state events for any transition.

**For any:** N subscribers (N in 1..3) and any operation triggering state transitions.
**Invariant:** All N subscribers receive identical event sequences.

**Assertion pseudocode:**
```
FOR ANY n IN 1..3:
    streams = [grpc_service.watch_adapter_states() FOR _ IN 1..n]
    grpc_service.install_adapter(image_ref, checksum)
    all_events = [collect_events(s, timeout=500ms) FOR s IN streams]
    FOR i IN 1..n:
        ASSERT all_events[i] == all_events[0]
```

### TS-07-P5: Checksum Verification Soundness

**Property:** Property 5 from design.md
**Validates:** 07-REQ-1.3, 07-REQ-1.E4
**Type:** property
**Description:** When the pulled image digest does not match the provided checksum, the adapter transitions to ERROR and the image is removed.

**For any:** Random digest and checksum where digest != checksum.
**Invariant:** Adapter state is ERROR and rmi was called.

**Assertion pseudocode:**
```
FOR ANY digest, checksum IN random_sha256_pairs WHERE digest != checksum:
    mock_podman.set_inspect_result(Ok(digest))
    mock_podman.set_pull_result(Ok(()))
    grpc_service.install_adapter(image_ref, checksum)
    tokio::time::sleep(200ms).await
    ASSERT state_mgr.get_adapter(adapter_id).state == Error
    ASSERT mock_podman.rmi_calls().contains(image_ref)
```

### TS-07-P6: Offload Timing Correctness

**Property:** Property 6 from design.md
**Validates:** 07-REQ-6.1
**Type:** property
**Description:** Offloading does not occur before the inactivity timeout has elapsed since entering STOPPED state.

**For any:** Inactivity timeout T in 1..5 seconds, check time S < T.
**Invariant:** At time S, the adapter is still in STOPPED state (not offloaded).

**Assertion pseudocode:**
```
FOR ANY timeout_secs IN 2..5:
    config.inactivity_timeout_secs = timeout_secs
    // ... install adapter, then stop it ...
    tokio::time::sleep((timeout_secs - 1) seconds).await
    ASSERT state_mgr.get_adapter(adapter_id).is_some()
    ASSERT state_mgr.get_adapter(adapter_id).state == Stopped
```

## Integration Smoke Tests

### TS-07-SMOKE-1: End-to-End Install and Query

**Type:** integration
**Description:** Start the UPDATE_SERVICE binary, call InstallAdapter via gRPC, then verify ListAdapters and GetAdapterStatus return the installed adapter.

**Preconditions:**
- UPDATE_SERVICE binary is built.
- A mock OCI registry or pre-pulled image is available.

**Steps:**
1. Start UPDATE_SERVICE process.
2. Call `InstallAdapter(image_ref, checksum)` via grpcurl or Go client.
3. Call `ListAdapters()` and verify the adapter appears.
4. Call `GetAdapterStatus(adapter_id)` and verify state is RUNNING.
5. Call `RemoveAdapter(adapter_id)` and verify success.
6. Call `ListAdapters()` and verify the adapter is gone.
7. Send SIGTERM and verify clean exit.

### TS-07-SMOKE-2: WatchAdapterStates Stream

**Type:** integration
**Description:** Subscribe to WatchAdapterStates, install an adapter, and verify the stream delivers state transition events.

**Preconditions:**
- UPDATE_SERVICE binary is running.

**Steps:**
1. Open `WatchAdapterStates()` stream via grpcurl or Go client.
2. Call `InstallAdapter(image_ref, checksum)`.
3. Collect events from the stream.
4. Verify events include DOWNLOADING, INSTALLING, RUNNING transitions.

## Coverage Matrix

| Requirement | Test Spec Entry | Type |
|-------------|-----------------|------|
| 07-REQ-1.1 | TS-07-1 | unit |
| 07-REQ-1.2 | TS-07-2 | unit |
| 07-REQ-1.3 | TS-07-3 | unit |
| 07-REQ-1.4 | TS-07-4 | unit |
| 07-REQ-1.5 | TS-07-5 | unit |
| 07-REQ-1.6 | TS-07-6 | unit |
| 07-REQ-1.E1 | TS-07-E1 | unit |
| 07-REQ-1.E2 | TS-07-E2 | unit |
| 07-REQ-1.E3 | TS-07-E3 | unit |
| 07-REQ-1.E4 | TS-07-E4 | unit |
| 07-REQ-1.E5 | TS-07-E5 | unit |
| 07-REQ-2.1 | TS-07-7 | unit |
| 07-REQ-2.2 | TS-07-7 | unit |
| 07-REQ-2.E1 | TS-07-E6 | unit |
| 07-REQ-3.1 | TS-07-8 | unit |
| 07-REQ-3.2 | TS-07-8 | unit |
| 07-REQ-3.3 | TS-07-8 | unit |
| 07-REQ-3.4 | TS-07-9 | unit |
| 07-REQ-3.E1 | TS-07-E7 | unit |
| 07-REQ-4.1 | TS-07-10 | unit |
| 07-REQ-4.2 | TS-07-11 | unit |
| 07-REQ-4.E1 | TS-07-E8 | unit |
| 07-REQ-4.E2 | TS-07-E9 | unit |
| 07-REQ-5.1 | TS-07-12 | unit |
| 07-REQ-5.2 | TS-07-12 | unit |
| 07-REQ-5.E1 | TS-07-E10 | unit |
| 07-REQ-5.E2 | TS-07-E11 | unit |
| 07-REQ-6.1 | TS-07-13 | unit |
| 07-REQ-6.2 | TS-07-13 | unit |
| 07-REQ-6.3 | TS-07-13 | unit |
| 07-REQ-6.4 | TS-07-13 | unit |
| 07-REQ-6.E1 | TS-07-E12 | unit |
| 07-REQ-7.1 | TS-07-14 | unit |
| 07-REQ-7.2 | TS-07-14 | unit |
| 07-REQ-7.3 | TS-07-17 | integration |
| 07-REQ-7.E1 | TS-07-E13 | unit |
| 07-REQ-7.E2 | TS-07-E14 | unit |
| 07-REQ-8.1 | TS-07-8 | unit |
| 07-REQ-8.2 | TS-07-8 | unit |
| 07-REQ-8.3 | TS-07-8 | unit |
| 07-REQ-8.E1 | TS-07-E15 | unit |
| 07-REQ-9.1 | TS-07-15 | unit |
| 07-REQ-9.2 | TS-07-16 | unit |
| 07-REQ-9.E1 | TS-07-E16 | unit |
| 07-REQ-10.1 | TS-07-17 | integration |
| 07-REQ-10.2 | TS-07-18 | integration |
| 07-REQ-10.E1 | TS-07-E17 | integration |
| Property 1 | TS-07-P1 | property |
| Property 2 | TS-07-P2 | property |
| Property 3 | TS-07-P3 | property |
| Property 4 | TS-07-P4 | property |
| Property 5 | TS-07-P5 | property |
| Property 6 | TS-07-P6 | property |
