# Test Specification: RHIVOS QM Partition (Phase 2.3)

## Overview

This test specification translates every acceptance criterion and correctness
property from the requirements and design documents into concrete, executable
test contracts. Tests are organized into three categories:

- **Acceptance criterion tests (TS-04-N):** One per acceptance criterion.
  Verify gRPC service behavior, autonomous session management, DATA_BROKER
  integration, adapter lifecycle management, OCI operations, mock operator
  REST API, and CLI commands.
- **Property tests (TS-04-PN):** One per correctness property. Verify
  invariants that must hold across sequences of operations.
- **Edge case tests (TS-04-EN):** One per edge case requirement. Verify
  error handling and boundary behavior.

PARKING_OPERATOR_ADAPTOR and UPDATE_SERVICE are Rust services. Mock
PARKING_OPERATOR is a Go service. Tests are Rust integration tests (using
`#[tokio::test]`) and Go tests (using standard `testing` package with
`httptest` for HTTP handler tests).

## Test Cases

### TS-04-1: PARKING_OPERATOR_ADAPTOR exposes gRPC service

**Requirement:** 04-REQ-1.1
**Type:** integration
**Description:** Verify the PARKING_OPERATOR_ADAPTOR exposes a gRPC service on
a configurable network address implementing the `ParkingAdaptor` service.

**Preconditions:**
- PARKING_OPERATOR_ADAPTOR binary built.
- Mock PARKING_OPERATOR running on a known port.
- DATA_BROKER running or mocked.

**Input:**
- Start PARKING_OPERATOR_ADAPTOR with `ADAPTOR_GRPC_ADDR=127.0.0.1:50052`.
- Attempt a gRPC connection to `127.0.0.1:50052`.

**Expected:**
- gRPC connection succeeds.
- The server responds to a known RPC method.

**Assertion pseudocode:**
```
adaptor = start_adaptor(grpc_addr="127.0.0.1:50052")
client = grpc_connect("127.0.0.1:50052")
response = client.get_rate(GetRateRequest{zone_id: "zone-munich-central"})
ASSERT response IS Ok
stop(adaptor)
```

---

### TS-04-2: StartSession returns session_id and status

**Requirement:** 04-REQ-1.2
**Type:** integration
**Description:** Verify that calling `StartSession` with a valid `vehicle_id`
and `zone_id` starts a parking session and returns `session_id` and `status`.

**Preconditions:**
- PARKING_OPERATOR_ADAPTOR running.
- Mock PARKING_OPERATOR running and reachable.
- No active session.

**Input:**
- gRPC call: `StartSession{vehicle_id: "VIN12345", zone_id: "zone-munich-central"}`.

**Expected:**
- Response contains non-empty `session_id`.
- Response contains `status` of "active".

**Assertion pseudocode:**
```
response = client.start_session(StartSessionRequest{
    vehicle_id: "VIN12345",
    zone_id: "zone-munich-central"
})
ASSERT response.status_code == OK
ASSERT response.session_id != ""
ASSERT response.status == "active"
```

---

### TS-04-3: StopSession returns fee, duration, and currency

**Requirement:** 04-REQ-1.3
**Type:** integration
**Description:** Verify that calling `StopSession` with a valid `session_id`
stops the session and returns fee, duration, and currency.

**Preconditions:**
- PARKING_OPERATOR_ADAPTOR running.
- Mock PARKING_OPERATOR running.
- An active session exists (started via `StartSession`).

**Input:**
- Start a session to obtain `session_id`.
- gRPC call: `StopSession{session_id: <obtained_session_id>}`.

**Expected:**
- Response contains `session_id` matching the stopped session.
- Response contains `total_fee` >= 0.
- Response contains `duration_seconds` >= 0.
- Response contains non-empty `currency`.

**Assertion pseudocode:**
```
start_resp = client.start_session(StartSessionRequest{
    vehicle_id: "VIN12345", zone_id: "zone-munich-central"
})
session_id = start_resp.session_id
sleep(1s)
stop_resp = client.stop_session(StopSessionRequest{session_id: session_id})
ASSERT stop_resp.status_code == OK
ASSERT stop_resp.session_id == session_id
ASSERT stop_resp.total_fee >= 0.0
ASSERT stop_resp.duration_seconds >= 0
ASSERT stop_resp.currency != ""
```

---

### TS-04-4: GetStatus returns current session state

**Requirement:** 04-REQ-1.4
**Type:** integration
**Description:** Verify that calling `GetStatus` with a valid `session_id`
returns the current session state including active status, start time, and fee.

**Preconditions:**
- PARKING_OPERATOR_ADAPTOR running.
- Mock PARKING_OPERATOR running.
- An active session exists.

**Input:**
- Start a session to obtain `session_id`.
- gRPC call: `GetStatus{session_id: <obtained_session_id>}`.

**Expected:**
- Response contains `session_id` matching the queried session.
- Response contains `active == true`.
- Response contains `start_time` > 0.
- Response contains `current_fee` >= 0.
- Response contains non-empty `currency`.

**Assertion pseudocode:**
```
start_resp = client.start_session(StartSessionRequest{
    vehicle_id: "VIN12345", zone_id: "zone-munich-central"
})
session_id = start_resp.session_id
status_resp = client.get_status(GetStatusRequest{session_id: session_id})
ASSERT status_resp.status_code == OK
ASSERT status_resp.session_id == session_id
ASSERT status_resp.active == true
ASSERT status_resp.start_time > 0
ASSERT status_resp.current_fee >= 0.0
ASSERT status_resp.currency != ""
```

---

### TS-04-5: GetRate returns rate information

**Requirement:** 04-REQ-1.5
**Type:** integration
**Description:** Verify that calling `GetRate` with a `zone_id` returns the
parking rate, currency, and zone name.

**Preconditions:**
- PARKING_OPERATOR_ADAPTOR running.
- Mock PARKING_OPERATOR running with pre-configured zones.

**Input:**
- gRPC call: `GetRate{zone_id: "zone-munich-central"}`.

**Expected:**
- Response contains `rate_per_hour` of 2.50.
- Response contains `currency` of "EUR".
- Response contains `zone_name` of "Munich Central".

**Assertion pseudocode:**
```
response = client.get_rate(GetRateRequest{zone_id: "zone-munich-central"})
ASSERT response.status_code == OK
ASSERT response.rate_per_hour == 2.50
ASSERT response.currency == "EUR"
ASSERT response.zone_name == "Munich Central"
```

---

### TS-04-6: Lock event triggers autonomous session start

**Requirement:** 04-REQ-2.1
**Type:** integration
**Description:** Verify that receiving a lock event from DATA_BROKER triggers
the adaptor to autonomously start a parking session via the PARKING_OPERATOR.

**Preconditions:**
- PARKING_OPERATOR_ADAPTOR running and subscribed to DATA_BROKER.
- Mock PARKING_OPERATOR running.
- No active session.

**Input:**
- Publish `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = true` to DATA_BROKER.

**Expected:**
- Mock PARKING_OPERATOR receives a `POST /parking/start` request.
- A new session is created.

**Assertion pseudocode:**
```
publish_to_databroker("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", true)
sleep(2s)
mock_calls = mock_operator.get_recorded_calls()
ASSERT mock_calls CONTAINS POST("/parking/start")
ASSERT mock_calls.last().body.vehicle_id != ""
```

---

### TS-04-7: Unlock event triggers autonomous session stop

**Requirement:** 04-REQ-2.2
**Type:** integration
**Description:** Verify that receiving an unlock event from DATA_BROKER
triggers the adaptor to autonomously stop the active parking session.

**Preconditions:**
- PARKING_OPERATOR_ADAPTOR running.
- Mock PARKING_OPERATOR running.
- An active session exists (started via lock event).

**Input:**
- Publish `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = true` (start session).
- Wait for session to start.
- Publish `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = false` (unlock).

**Expected:**
- Mock PARKING_OPERATOR receives a `POST /parking/stop` request.

**Assertion pseudocode:**
```
publish_to_databroker("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", true)
sleep(2s)
publish_to_databroker("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", false)
sleep(2s)
mock_calls = mock_operator.get_recorded_calls()
ASSERT mock_calls CONTAINS POST("/parking/stop")
```

---

### TS-04-8: Autonomous start writes SessionActive true

**Requirement:** 04-REQ-2.3
**Type:** integration
**Description:** Verify that after autonomously starting a session, the adaptor
writes `Vehicle.Parking.SessionActive = true` to DATA_BROKER.

**Preconditions:**
- PARKING_OPERATOR_ADAPTOR running and connected to DATA_BROKER.
- Mock PARKING_OPERATOR running and responding successfully.

**Input:**
- Publish `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = true`.

**Expected:**
- `Vehicle.Parking.SessionActive` in DATA_BROKER is `true`.

**Assertion pseudocode:**
```
publish_to_databroker("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", true)
sleep(2s)
value = read_from_databroker("Vehicle.Parking.SessionActive")
ASSERT value == true
```

---

### TS-04-9: Autonomous stop writes SessionActive false

**Requirement:** 04-REQ-2.4
**Type:** integration
**Description:** Verify that after autonomously stopping a session, the adaptor
writes `Vehicle.Parking.SessionActive = false` to DATA_BROKER.

**Preconditions:**
- PARKING_OPERATOR_ADAPTOR running.
- An active session exists (started via lock event).

**Input:**
- Start session via lock event.
- Publish `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = false`.

**Expected:**
- `Vehicle.Parking.SessionActive` in DATA_BROKER is `false`.

**Assertion pseudocode:**
```
publish_to_databroker("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", true)
sleep(2s)
publish_to_databroker("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", false)
sleep(2s)
value = read_from_databroker("Vehicle.Parking.SessionActive")
ASSERT value == false
```

---

### TS-04-10: gRPC override updates SessionActive

**Requirement:** 04-REQ-2.5
**Type:** integration
**Description:** Verify that a manual `StartSession` or `StopSession` gRPC
call overrides autonomous behavior and updates `Vehicle.Parking.SessionActive`.

**Preconditions:**
- PARKING_OPERATOR_ADAPTOR running and connected to DATA_BROKER.
- Mock PARKING_OPERATOR running.

**Input:**
- gRPC call: `StartSession{vehicle_id: "VIN12345", zone_id: "zone-munich-central"}`.
- Read `Vehicle.Parking.SessionActive`.
- gRPC call: `StopSession{session_id: <obtained>}`.
- Read `Vehicle.Parking.SessionActive`.

**Expected:**
- After StartSession: `Vehicle.Parking.SessionActive == true`.
- After StopSession: `Vehicle.Parking.SessionActive == false`.

**Assertion pseudocode:**
```
start_resp = client.start_session(StartSessionRequest{
    vehicle_id: "VIN12345", zone_id: "zone-munich-central"
})
sleep(1s)
value = read_from_databroker("Vehicle.Parking.SessionActive")
ASSERT value == true
client.stop_session(StopSessionRequest{session_id: start_resp.session_id})
sleep(1s)
value = read_from_databroker("Vehicle.Parking.SessionActive")
ASSERT value == false
```

---

### TS-04-11: DATA_BROKER connection via network gRPC

**Requirement:** 04-REQ-3.1
**Type:** integration
**Description:** Verify the PARKING_OPERATOR_ADAPTOR connects to DATA_BROKER
using network gRPC (TCP) at a configurable address.

**Preconditions:**
- DATA_BROKER running on a known address.

**Input:**
- Start PARKING_OPERATOR_ADAPTOR with `DATABROKER_ADDR=localhost:55556`.

**Expected:**
- Adaptor successfully connects and subscribes (no connection errors in logs).

**Assertion pseudocode:**
```
adaptor = start_adaptor(databroker_addr="localhost:55556")
sleep(3s)
ASSERT adaptor.stderr NOT CONTAINS "connection refused"
ASSERT adaptor.stderr NOT CONTAINS "connection error"
stop(adaptor)
```

---

### TS-04-12: Subscribe to IsLocked events

**Requirement:** 04-REQ-3.2
**Type:** integration
**Description:** Verify the PARKING_OPERATOR_ADAPTOR subscribes to the
`Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` signal from DATA_BROKER.

**Preconditions:**
- DATA_BROKER running.
- PARKING_OPERATOR_ADAPTOR running and connected.

**Input:**
- Publish `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = true` to DATA_BROKER.

**Expected:**
- Adaptor reacts to the event (starts a session or logs the event).

**Assertion pseudocode:**
```
adaptor = start_adaptor()
publish_to_databroker("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", true)
sleep(2s)
mock_calls = mock_operator.get_recorded_calls()
ASSERT mock_calls CONTAINS POST("/parking/start")
stop(adaptor)
```

---

### TS-04-13: Read location from DATA_BROKER

**Requirement:** 04-REQ-3.3
**Type:** integration
**Description:** Verify the PARKING_OPERATOR_ADAPTOR reads
`Vehicle.CurrentLocation.Latitude` and `Vehicle.CurrentLocation.Longitude`
from DATA_BROKER for zone context.

**Preconditions:**
- DATA_BROKER running with location signals set.
- PARKING_OPERATOR_ADAPTOR running.

**Input:**
- Set `Vehicle.CurrentLocation.Latitude = 48.1351` and
  `Vehicle.CurrentLocation.Longitude = 11.5820` in DATA_BROKER.
- Trigger a lock event.

**Expected:**
- Adaptor reads location data from DATA_BROKER (visible in logs or in the
  request to the PARKING_OPERATOR).

**Assertion pseudocode:**
```
set_in_databroker("Vehicle.CurrentLocation.Latitude", 48.1351)
set_in_databroker("Vehicle.CurrentLocation.Longitude", 11.5820)
publish_to_databroker("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", true)
sleep(2s)
ASSERT adaptor.logs CONTAINS "latitude" OR adaptor.logs CONTAINS "location"
```

---

### TS-04-14: Write SessionActive to DATA_BROKER

**Requirement:** 04-REQ-3.4
**Type:** integration
**Description:** Verify the PARKING_OPERATOR_ADAPTOR writes
`Vehicle.Parking.SessionActive` to DATA_BROKER to publish session state.

**Preconditions:**
- DATA_BROKER running.
- PARKING_OPERATOR_ADAPTOR running and connected.
- Mock PARKING_OPERATOR running.

**Input:**
- Trigger a lock event to start a session.

**Expected:**
- `Vehicle.Parking.SessionActive` is readable from DATA_BROKER as `true`.

**Assertion pseudocode:**
```
publish_to_databroker("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", true)
sleep(2s)
value = read_from_databroker("Vehicle.Parking.SessionActive")
ASSERT value == true
```

---

### TS-04-15: UPDATE_SERVICE exposes gRPC service

**Requirement:** 04-REQ-4.1
**Type:** integration
**Description:** Verify the UPDATE_SERVICE exposes a gRPC service on a
configurable network address implementing the `UpdateService` service.

**Preconditions:**
- UPDATE_SERVICE binary built.

**Input:**
- Start UPDATE_SERVICE with `UPDATE_GRPC_ADDR=127.0.0.1:50051`.
- Attempt a gRPC connection.

**Expected:**
- gRPC connection succeeds.
- The server responds to a known RPC method.

**Assertion pseudocode:**
```
update_svc = start_update_service(grpc_addr="127.0.0.1:50051")
client = grpc_connect("127.0.0.1:50051")
response = client.list_adapters(ListAdaptersRequest{})
ASSERT response IS Ok
stop(update_svc)
```

---

### TS-04-16: InstallAdapter returns job_id, adapter_id, state

**Requirement:** 04-REQ-4.2
**Type:** integration
**Description:** Verify that calling `InstallAdapter` with an `image_ref` and
`checksum_sha256` returns `job_id`, `adapter_id`, and initial `state` of
`DOWNLOADING`.

**Preconditions:**
- UPDATE_SERVICE running.
- OCI registry available or mocked.

**Input:**
- gRPC call: `InstallAdapter{image_ref: "localhost:5000/adaptor:v1", checksum_sha256: "abc123..."}`.

**Expected:**
- Response contains non-empty `job_id`.
- Response contains non-empty `adapter_id`.
- Response contains `state` of `DOWNLOADING`.

**Assertion pseudocode:**
```
response = client.install_adapter(InstallAdapterRequest{
    image_ref: "localhost:5000/adaptor:v1",
    checksum_sha256: "abc123def456..."
})
ASSERT response.status_code == OK
ASSERT response.job_id != ""
ASSERT response.adapter_id != ""
ASSERT response.state == DOWNLOADING
```

---

### TS-04-17: WatchAdapterStates streams events

**Requirement:** 04-REQ-4.3
**Type:** integration
**Description:** Verify that calling `WatchAdapterStates` returns a
server-streaming response emitting `AdapterStateEvent` messages on adapter
state transitions.

**Preconditions:**
- UPDATE_SERVICE running.

**Input:**
- Open a `WatchAdapterStates` stream.
- Trigger an adapter installation.

**Expected:**
- Stream emits at least one `AdapterStateEvent` with the adapter's ID and
  new state.

**Assertion pseudocode:**
```
stream = client.watch_adapter_states(WatchAdapterStatesRequest{})
client.install_adapter(InstallAdapterRequest{...})
event = stream.next(timeout=10s)
ASSERT event IS Some
ASSERT event.adapter_id != ""
ASSERT event.state IN [DOWNLOADING, INSTALLING, RUNNING, ERROR]
```

---

### TS-04-18: ListAdapters returns all known adapters

**Requirement:** 04-REQ-4.4
**Type:** integration
**Description:** Verify that calling `ListAdapters` returns all known adapters
with their current states.

**Preconditions:**
- UPDATE_SERVICE running.
- At least one adapter installed.

**Input:**
- Install an adapter.
- gRPC call: `ListAdapters{}`.

**Expected:**
- Response contains a list with at least one adapter entry.
- Each entry has `adapter_id` and `state`.

**Assertion pseudocode:**
```
client.install_adapter(InstallAdapterRequest{...})
sleep(1s)
response = client.list_adapters(ListAdaptersRequest{})
ASSERT response.status_code == OK
ASSERT len(response.adapters) >= 1
ASSERT response.adapters[0].adapter_id != ""
ASSERT response.adapters[0].state != UNKNOWN
```

---

### TS-04-19: RemoveAdapter stops and removes adapter

**Requirement:** 04-REQ-4.5
**Type:** integration
**Description:** Verify that calling `RemoveAdapter` with a valid `adapter_id`
stops and removes the adapter container.

**Preconditions:**
- UPDATE_SERVICE running.
- An adapter is installed and known.

**Input:**
- Install an adapter.
- gRPC call: `RemoveAdapter{adapter_id: <obtained>}`.
- gRPC call: `ListAdapters{}`.

**Expected:**
- `RemoveAdapter` returns success.
- The adapter no longer appears in `ListAdapters` response.

**Assertion pseudocode:**
```
install_resp = client.install_adapter(InstallAdapterRequest{...})
adapter_id = install_resp.adapter_id
remove_resp = client.remove_adapter(RemoveAdapterRequest{adapter_id: adapter_id})
ASSERT remove_resp.status_code == OK
list_resp = client.list_adapters(ListAdaptersRequest{})
ASSERT adapter_id NOT IN [a.adapter_id FOR a IN list_resp.adapters]
```

---

### TS-04-20: GetAdapterStatus returns adapter info

**Requirement:** 04-REQ-4.6
**Type:** integration
**Description:** Verify that calling `GetAdapterStatus` with a valid
`adapter_id` returns the adapter's current `AdapterInfo`.

**Preconditions:**
- UPDATE_SERVICE running.
- An adapter is installed.

**Input:**
- Install an adapter.
- gRPC call: `GetAdapterStatus{adapter_id: <obtained>}`.

**Expected:**
- Response contains `adapter` with matching `adapter_id`.
- Response contains current `state`.

**Assertion pseudocode:**
```
install_resp = client.install_adapter(InstallAdapterRequest{...})
adapter_id = install_resp.adapter_id
status_resp = client.get_adapter_status(GetAdapterStatusRequest{adapter_id: adapter_id})
ASSERT status_resp.status_code == OK
ASSERT status_resp.adapter.adapter_id == adapter_id
ASSERT status_resp.adapter.state != UNKNOWN
```

---

### TS-04-21: OCI image pull on InstallAdapter

**Requirement:** 04-REQ-5.1
**Type:** integration
**Description:** Verify that when `InstallAdapter` is invoked, the
UPDATE_SERVICE pulls the OCI container image from the configured registry.

**Preconditions:**
- UPDATE_SERVICE running.
- Mock OCI registry running with a valid image.

**Input:**
- gRPC call: `InstallAdapter{image_ref: "localhost:5000/adaptor:v1", checksum_sha256: <valid>}`.

**Expected:**
- UPDATE_SERVICE makes HTTP requests to the registry to pull the manifest
  and layers.

**Assertion pseudocode:**
```
response = client.install_adapter(InstallAdapterRequest{
    image_ref: "localhost:5000/adaptor:v1",
    checksum_sha256: VALID_CHECKSUM
})
sleep(5s)
registry_calls = mock_registry.get_recorded_calls()
ASSERT registry_calls CONTAINS GET("/v2/adaptor/manifests/v1")
```

---

### TS-04-22: SHA-256 checksum verification

**Requirement:** 04-REQ-5.2
**Type:** unit
**Description:** Verify that the UPDATE_SERVICE computes the SHA-256 digest of
the OCI manifest and compares it against the provided `checksum_sha256`.

**Preconditions:**
- UPDATE_SERVICE checksum module available.

**Input:**
- A known OCI manifest blob and its precomputed SHA-256 digest.

**Expected:**
- When the correct checksum is provided, verification passes.
- When an incorrect checksum is provided, verification fails.

**Assertion pseudocode:**
```
manifest = b"test manifest content"
correct_checksum = sha256(manifest)
ASSERT verify_checksum(manifest, correct_checksum) == true
ASSERT verify_checksum(manifest, "wrong_checksum") == false
```

---

### TS-04-23: Checksum match transitions to INSTALLING

**Requirement:** 04-REQ-5.3
**Type:** integration
**Description:** Verify that if the checksum matches, the adapter state
transitions from `DOWNLOADING` to `INSTALLING`.

**Preconditions:**
- UPDATE_SERVICE running.
- Mock OCI registry with a valid image.
- Correct checksum provided.

**Input:**
- gRPC call: `InstallAdapter` with matching checksum.
- Watch adapter state events.

**Expected:**
- Adapter transitions through `DOWNLOADING` -> `INSTALLING`.

**Assertion pseudocode:**
```
stream = client.watch_adapter_states(WatchAdapterStatesRequest{})
client.install_adapter(InstallAdapterRequest{
    image_ref: "localhost:5000/adaptor:v1",
    checksum_sha256: VALID_CHECKSUM
})
states = collect_states(stream, timeout=10s)
ASSERT DOWNLOADING IN states
ASSERT INSTALLING IN states
ASSERT index_of(DOWNLOADING, states) < index_of(INSTALLING, states)
```

---

### TS-04-24: Configurable inactivity timeout

**Requirement:** 04-REQ-6.1
**Type:** unit
**Description:** Verify the UPDATE_SERVICE supports a configurable inactivity
timeout for automatic adapter offloading (default: 24 hours).

**Preconditions:**
- UPDATE_SERVICE configuration module available.

**Input:**
- Start UPDATE_SERVICE with `OFFLOAD_TIMEOUT_HOURS=1`.
- Start UPDATE_SERVICE with default configuration.

**Expected:**
- Custom timeout: offload timeout is 1 hour.
- Default: offload timeout is 24 hours.

**Assertion pseudocode:**
```
config_custom = load_config(env={"OFFLOAD_TIMEOUT_HOURS": "1"})
ASSERT config_custom.offload_timeout == Duration::hours(1)
config_default = load_config(env={})
ASSERT config_default.offload_timeout == Duration::hours(24)
```

---

### TS-04-25: Stopped adapter offloaded after timeout

**Requirement:** 04-REQ-6.2
**Type:** integration
**Description:** Verify that a stopped adapter is offloaded after the
inactivity timeout expires and removed from the known adapters list.

**Preconditions:**
- UPDATE_SERVICE running with a short inactivity timeout (e.g. 2 seconds).
- An adapter installed and then stopped.

**Input:**
- Install and run an adapter, then stop it.
- Wait for the inactivity timeout to expire.

**Expected:**
- Adapter transitions to `OFFLOADING`, then is removed from the adapters list.

**Assertion pseudocode:**
```
update_svc = start_update_service(offload_timeout=2s)
install_resp = client.install_adapter(InstallAdapterRequest{...})
adapter_id = install_resp.adapter_id
wait_until_state(adapter_id, RUNNING)
client.remove_adapter(RemoveAdapterRequest{adapter_id: adapter_id})  // stops it
sleep(3s)  // wait past timeout
list_resp = client.list_adapters(ListAdaptersRequest{})
ASSERT adapter_id NOT IN [a.adapter_id FOR a IN list_resp.adapters]
```

---

### TS-04-26: Offloading emits state events

**Requirement:** 04-REQ-6.3
**Type:** integration
**Description:** Verify that the UPDATE_SERVICE emits `AdapterStateEvent`
messages during offloading transitions so watchers are notified.

**Preconditions:**
- UPDATE_SERVICE running with a short inactivity timeout.
- A watcher stream is open.
- An adapter is stopped.

**Input:**
- Open watcher stream.
- Stop an adapter and wait for offloading.

**Expected:**
- Stream receives an event with state `OFFLOADING`.

**Assertion pseudocode:**
```
stream = client.watch_adapter_states(WatchAdapterStatesRequest{})
// ... install, run, stop adapter ...
sleep(timeout + 1s)
events = collect_events(stream, timeout=5s)
ASSERT events CONTAINS AdapterStateEvent{state: OFFLOADING}
```

---

### TS-04-27: Valid state transitions enforced

**Requirement:** 04-REQ-7.1
**Type:** unit
**Description:** Verify that the UPDATE_SERVICE enforces the allowed adapter
lifecycle state transitions as defined in the requirements.

**Preconditions:**
- Adapter state machine module available.

**Input:**
- For each valid transition pair, attempt the transition.

**Expected:**
- All valid transitions succeed: UNKNOWN->DOWNLOADING, DOWNLOADING->INSTALLING,
  DOWNLOADING->ERROR, INSTALLING->RUNNING, INSTALLING->ERROR, RUNNING->STOPPED,
  STOPPED->OFFLOADING, STOPPED->DOWNLOADING, OFFLOADING->UNKNOWN,
  ERROR->DOWNLOADING.

**Assertion pseudocode:**
```
valid_transitions = [
    (UNKNOWN, DOWNLOADING), (DOWNLOADING, INSTALLING),
    (DOWNLOADING, ERROR), (INSTALLING, RUNNING),
    (INSTALLING, ERROR), (RUNNING, STOPPED),
    (STOPPED, OFFLOADING), (STOPPED, DOWNLOADING),
    (OFFLOADING, UNKNOWN), (ERROR, DOWNLOADING)
]
FOR (from, to) IN valid_transitions:
    adapter = create_adapter_in_state(from)
    result = adapter.transition(to)
    ASSERT result == Ok
```

---

### TS-04-28: Invalid state transitions rejected

**Requirement:** 04-REQ-7.2
**Type:** unit
**Description:** Verify that the UPDATE_SERVICE rejects any state transition
not in the allowed set and logs a warning.

**Preconditions:**
- Adapter state machine module available.

**Input:**
- Attempt invalid transitions such as UNKNOWN->RUNNING, DOWNLOADING->STOPPED,
  RUNNING->DOWNLOADING.

**Expected:**
- Each invalid transition is rejected (returns error).
- A warning is logged.

**Assertion pseudocode:**
```
invalid_transitions = [
    (UNKNOWN, RUNNING), (UNKNOWN, INSTALLING),
    (DOWNLOADING, STOPPED), (DOWNLOADING, RUNNING),
    (RUNNING, DOWNLOADING), (RUNNING, INSTALLING),
    (OFFLOADING, RUNNING)
]
FOR (from, to) IN invalid_transitions:
    adapter = create_adapter_in_state(from)
    result = adapter.transition(to)
    ASSERT result == Err(InvalidTransition)
```

---

### TS-04-29: Mock PARKING_OPERATOR HTTP server configurable port

**Requirement:** 04-REQ-8.1
**Type:** unit
**Description:** Verify the mock PARKING_OPERATOR exposes an HTTP server on a
configurable port (default: 8090).

**Preconditions:**
- Mock PARKING_OPERATOR binary built.

**Input:**
- Start mock with `PORT=9090`.
- Start mock with default config.

**Expected:**
- Custom port: HTTP server reachable on 9090.
- Default: HTTP server reachable on 8090.

**Assertion pseudocode:**
```
mock = start_mock_operator(port=9090)
response = http_get("http://localhost:9090/health")
ASSERT response.status_code == 200
stop(mock)

mock = start_mock_operator()
response = http_get("http://localhost:8090/health")
ASSERT response.status_code == 200
stop(mock)
```

---

### TS-04-30: POST /parking/start creates session

**Requirement:** 04-REQ-8.2
**Type:** unit
**Description:** Verify that `POST /parking/start` creates a session and
returns `session_id` and `status` of "active".

**Preconditions:**
- Mock PARKING_OPERATOR running.

**Input:**
- HTTP POST to `/parking/start` with body:
  `{"vehicle_id": "VIN12345", "zone_id": "zone-munich-central", "timestamp": 1708700000}`.

**Expected:**
- HTTP 200.
- Response body contains non-empty `session_id`.
- Response body contains `status` of "active".

**Assertion pseudocode:**
```
response = http_post("/parking/start", {
    vehicle_id: "VIN12345",
    zone_id: "zone-munich-central",
    timestamp: 1708700000
})
ASSERT response.status_code == 200
body = parse_json(response.body)
ASSERT body.session_id != ""
ASSERT body.status == "active"
```

---

### TS-04-31: POST /parking/stop calculates fee

**Requirement:** 04-REQ-8.3
**Type:** unit
**Description:** Verify that `POST /parking/stop` calculates the fee, marks
the session as stopped, and returns session details.

**Preconditions:**
- Mock PARKING_OPERATOR running.
- A session has been started.

**Input:**
- Start a session via `POST /parking/start`.
- Wait a known duration.
- `POST /parking/stop` with the obtained `session_id`.

**Expected:**
- HTTP 200.
- Response contains `session_id`, `fee` >= 0, `duration_seconds` >= 0,
  and `currency`.

**Assertion pseudocode:**
```
start_resp = http_post("/parking/start", {
    vehicle_id: "VIN12345", zone_id: "zone-munich-central", timestamp: now()
})
session_id = parse_json(start_resp.body).session_id
sleep(1s)
stop_resp = http_post("/parking/stop", {session_id: session_id})
ASSERT stop_resp.status_code == 200
body = parse_json(stop_resp.body)
ASSERT body.session_id == session_id
ASSERT body.fee >= 0.0
ASSERT body.duration_seconds >= 0
ASSERT body.currency == "EUR"
```

---

### TS-04-32: GET /parking/{session_id}/status returns session status

**Requirement:** 04-REQ-8.4
**Type:** unit
**Description:** Verify that `GET /parking/{session_id}/status` returns the
session's current status details.

**Preconditions:**
- Mock PARKING_OPERATOR running.
- A session has been started.

**Input:**
- Start a session.
- `GET /parking/{session_id}/status`.

**Expected:**
- HTTP 200.
- Response contains `session_id`, `active == true`, `start_time` > 0,
  `current_fee` >= 0, and `currency`.

**Assertion pseudocode:**
```
start_resp = http_post("/parking/start", {
    vehicle_id: "VIN12345", zone_id: "zone-munich-central", timestamp: now()
})
session_id = parse_json(start_resp.body).session_id
status_resp = http_get("/parking/" + session_id + "/status")
ASSERT status_resp.status_code == 200
body = parse_json(status_resp.body)
ASSERT body.session_id == session_id
ASSERT body.active == true
ASSERT body.start_time > 0
ASSERT body.current_fee >= 0.0
ASSERT body.currency == "EUR"
```

---

### TS-04-33: GET /rate/{zone_id} returns zone rate

**Requirement:** 04-REQ-8.5
**Type:** unit
**Description:** Verify that `GET /rate/{zone_id}` returns the parking rate
for the zone.

**Preconditions:**
- Mock PARKING_OPERATOR running with pre-configured zones.

**Input:**
- `GET /rate/zone-munich-central`.

**Expected:**
- HTTP 200.
- Response contains `rate_per_hour` of 2.50, `currency` of "EUR", and
  `zone_name` of "Munich Central".

**Assertion pseudocode:**
```
response = http_get("/rate/zone-munich-central")
ASSERT response.status_code == 200
body = parse_json(response.body)
ASSERT body.rate_per_hour == 2.50
ASSERT body.currency == "EUR"
ASSERT body.zone_name == "Munich Central"
```

---

### TS-04-34: CLI install command calls InstallAdapter

**Requirement:** 04-REQ-9.1
**Type:** integration
**Description:** Verify the `install` CLI command calls UPDATE_SERVICE
`InstallAdapter` and prints the response.

**Preconditions:**
- UPDATE_SERVICE running.
- Mock PARKING_APP CLI built.

**Input:**
- `parking-app-cli install --image-ref localhost:5000/adaptor:v1 --checksum abc123`.

**Expected:**
- CLI prints `job_id`, `adapter_id`, and `state`.
- Exit code 0.

**Assertion pseudocode:**
```
result = exec("parking-app-cli install --image-ref localhost:5000/adaptor:v1 --checksum abc123")
ASSERT result.exit_code == 0
ASSERT result.stdout CONTAINS "job_id"
ASSERT result.stdout CONTAINS "adapter_id"
ASSERT result.stdout CONTAINS "state"
```

---

### TS-04-35: CLI watch command streams events

**Requirement:** 04-REQ-9.2
**Type:** integration
**Description:** Verify the `watch` CLI command calls UPDATE_SERVICE
`WatchAdapterStates` and prints each event.

**Preconditions:**
- UPDATE_SERVICE running.
- Mock PARKING_APP CLI built.

**Input:**
- Start `parking-app-cli watch` in background.
- Trigger an adapter state change.
- Interrupt after receiving output.

**Expected:**
- CLI prints at least one `AdapterStateEvent`.

**Assertion pseudocode:**
```
watch_proc = start_background("parking-app-cli watch")
client.install_adapter(InstallAdapterRequest{...})
sleep(3s)
output = watch_proc.stdout_so_far()
ASSERT output CONTAINS "adapter_id" OR output CONTAINS "state"
stop(watch_proc)
```

---

### TS-04-36: CLI list command prints adapters table

**Requirement:** 04-REQ-9.3
**Type:** integration
**Description:** Verify the `list` CLI command calls UPDATE_SERVICE
`ListAdapters` and prints a table of adapters.

**Preconditions:**
- UPDATE_SERVICE running with at least one adapter installed.
- Mock PARKING_APP CLI built.

**Input:**
- `parking-app-cli list`.

**Expected:**
- CLI prints a table with adapter IDs and states.
- Exit code 0.

**Assertion pseudocode:**
```
client.install_adapter(InstallAdapterRequest{...})
result = exec("parking-app-cli list")
ASSERT result.exit_code == 0
ASSERT result.stdout CONTAINS "adapter_id" OR result.stdout CONTAINS "ID"
```

---

### TS-04-37: CLI start-session command calls StartSession

**Requirement:** 04-REQ-9.4
**Type:** integration
**Description:** Verify the `start-session` CLI command calls
PARKING_OPERATOR_ADAPTOR `StartSession` and prints the response.

**Preconditions:**
- PARKING_OPERATOR_ADAPTOR running.
- Mock PARKING_OPERATOR running.
- Mock PARKING_APP CLI built.

**Input:**
- `parking-app-cli start-session --vehicle-id VIN12345 --zone-id zone-munich-central`.

**Expected:**
- CLI prints `session_id` and `status`.
- Exit code 0.

**Assertion pseudocode:**
```
result = exec("parking-app-cli start-session --vehicle-id VIN12345 --zone-id zone-munich-central")
ASSERT result.exit_code == 0
ASSERT result.stdout CONTAINS "session_id"
ASSERT result.stdout CONTAINS "status"
```

---

### TS-04-38: CLI stop-session command calls StopSession

**Requirement:** 04-REQ-9.5
**Type:** integration
**Description:** Verify the `stop-session` CLI command calls
PARKING_OPERATOR_ADAPTOR `StopSession` and prints the response.

**Preconditions:**
- PARKING_OPERATOR_ADAPTOR running.
- Mock PARKING_OPERATOR running.
- An active session exists.
- Mock PARKING_APP CLI built.

**Input:**
- Start a session via CLI.
- `parking-app-cli stop-session --session-id <obtained>`.

**Expected:**
- CLI prints `session_id`, `fee`, `duration`, and `currency`.
- Exit code 0.

**Assertion pseudocode:**
```
start_result = exec("parking-app-cli start-session --vehicle-id VIN12345 --zone-id zone-munich-central")
session_id = extract_session_id(start_result.stdout)
result = exec("parking-app-cli stop-session --session-id " + session_id)
ASSERT result.exit_code == 0
ASSERT result.stdout CONTAINS "fee"
ASSERT result.stdout CONTAINS "duration"
ASSERT result.stdout CONTAINS "currency"
```

---

### TS-04-39: Integration test lock-to-session

**Requirement:** 04-REQ-10.1
**Type:** integration
**Description:** Verify end-to-end that a lock event published to DATA_BROKER
triggers the PARKING_OPERATOR_ADAPTOR to autonomously start a session with the
mock PARKING_OPERATOR.

**Preconditions:**
- DATA_BROKER running.
- PARKING_OPERATOR_ADAPTOR running and connected.
- Mock PARKING_OPERATOR running.

**Input:**
- Publish `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = true` to DATA_BROKER.

**Expected:**
- Mock PARKING_OPERATOR receives a `POST /parking/start` call.
- `Vehicle.Parking.SessionActive` is `true` in DATA_BROKER.

**Assertion pseudocode:**
```
publish_to_databroker("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", true)
sleep(3s)
mock_calls = mock_operator.get_recorded_calls()
ASSERT mock_calls CONTAINS POST("/parking/start")
value = read_from_databroker("Vehicle.Parking.SessionActive")
ASSERT value == true
```

---

### TS-04-40: Integration test CLI-to-UpdateService lifecycle

**Requirement:** 04-REQ-10.2
**Type:** integration
**Description:** Verify end-to-end that the mock PARKING_APP CLI can trigger
UPDATE_SERVICE adapter lifecycle operations (install, list, get-status).

**Preconditions:**
- UPDATE_SERVICE running.
- Mock PARKING_APP CLI built.

**Input:**
- Run `install` via CLI.
- Run `list` via CLI.

**Expected:**
- `install` returns success with adapter info.
- `list` shows the installed adapter.

**Assertion pseudocode:**
```
install_result = exec("parking-app-cli install --image-ref localhost:5000/adaptor:v1 --checksum abc123")
ASSERT install_result.exit_code == 0
list_result = exec("parking-app-cli list")
ASSERT list_result.exit_code == 0
ASSERT list_result.stdout CONTAINS "adaptor"
```

---

### TS-04-41: Integration test adaptor-to-operator communication

**Requirement:** 04-REQ-10.3
**Type:** integration
**Description:** Verify end-to-end that the PARKING_OPERATOR_ADAPTOR
communicates correctly with the mock PARKING_OPERATOR REST API.

**Preconditions:**
- PARKING_OPERATOR_ADAPTOR running.
- Mock PARKING_OPERATOR running.

**Input:**
- Call `StartSession` via gRPC.
- Call `StopSession` via gRPC.
- Call `GetRate` via gRPC.

**Expected:**
- Each gRPC call results in the corresponding REST call to the mock operator.
- Responses contain valid data.

**Assertion pseudocode:**
```
start_resp = client.start_session(StartSessionRequest{
    vehicle_id: "VIN12345", zone_id: "zone-munich-central"
})
ASSERT start_resp.session_id != ""
stop_resp = client.stop_session(StopSessionRequest{session_id: start_resp.session_id})
ASSERT stop_resp.total_fee >= 0.0
rate_resp = client.get_rate(GetRateRequest{zone_id: "zone-munich-central"})
ASSERT rate_resp.rate_per_hour == 2.50
```

---

## Property Test Cases

### TS-04-P1: Session State Consistency

**Property:** Property 1 from design.md
**Validates:** 04-REQ-2.3, 04-REQ-2.4
**Type:** property
**Description:** For any sequence of lock/unlock events processed by
PARKING_OPERATOR_ADAPTOR, the value of `Vehicle.Parking.SessionActive` in
DATA_BROKER matches whether the adaptor has an active session with the
PARKING_OPERATOR.

**For any:** Sequence S of lock/unlock events (length 1-10)
**Invariant:** After each event in S, `Vehicle.Parking.SessionActive` equals
`adaptor.has_active_session()`.

**Assertion pseudocode:**
```
FOR ANY sequence IN random_lock_unlock_sequences(count=20, max_len=10):
    reset_state()
    FOR event IN sequence:
        publish_to_databroker("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", event.locked)
        sleep(1s)
        session_active = read_from_databroker("Vehicle.Parking.SessionActive")
        has_session = mock_operator.has_active_session()
        ASSERT session_active == has_session
```

---

### TS-04-P2: Autonomous Idempotency

**Property:** Property 2 from design.md
**Validates:** 04-REQ-2.E1, 04-REQ-2.E3
**Type:** property
**Description:** For any repeated lock event when a session is already active,
the adaptor does not create duplicate sessions. Similarly, repeated unlock
events when no session is active have no effect.

**For any:** Sequence of N identical lock events (N >= 2) followed by M
identical unlock events (M >= 2)
**Invariant:** Exactly one `POST /parking/start` and one `POST /parking/stop`
are made to the PARKING_OPERATOR.

**Assertion pseudocode:**
```
FOR ANY n IN 2..5, m IN 2..5:
    reset_state()
    FOR i IN 1..n:
        publish_to_databroker("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", true)
        sleep(500ms)
    start_calls = mock_operator.count_calls(POST, "/parking/start")
    ASSERT start_calls == 1
    FOR i IN 1..m:
        publish_to_databroker("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", false)
        sleep(500ms)
    stop_calls = mock_operator.count_calls(POST, "/parking/stop")
    ASSERT stop_calls == 1
```

---

### TS-04-P3: Override Precedence

**Property:** Property 3 from design.md
**Validates:** 04-REQ-2.5
**Type:** property
**Description:** For any manual `StartSession` or `StopSession` gRPC call, the
adaptor executes the override regardless of the current lock state, and
`Vehicle.Parking.SessionActive` reflects the override result.

**For any:** Lock state L in {locked, unlocked}
**Invariant:** A manual StartSession succeeds regardless of L, and
SessionActive reflects the override.

**Assertion pseudocode:**
```
FOR ANY lock_state IN [true, false]:
    reset_state()
    publish_to_databroker("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", lock_state)
    sleep(1s)
    // Manual override: start
    start_resp = client.start_session(StartSessionRequest{
        vehicle_id: "VIN12345", zone_id: "zone-munich-central"
    })
    sleep(500ms)
    ASSERT read_from_databroker("Vehicle.Parking.SessionActive") == true
    // Manual override: stop
    client.stop_session(StopSessionRequest{session_id: start_resp.session_id})
    sleep(500ms)
    ASSERT read_from_databroker("Vehicle.Parking.SessionActive") == false
```

---

### TS-04-P4: State Machine Integrity

**Property:** Property 4 from design.md
**Validates:** 04-REQ-7.1, 04-REQ-7.2
**Type:** property
**Description:** For any adapter managed by UPDATE_SERVICE, the adapter's
lifecycle state only transitions via the allowed transitions. No invalid
transition occurs.

**For any:** State S and target state T where (S, T) is NOT in the valid
transitions set
**Invariant:** `transition(S, T)` returns an error.

**Assertion pseudocode:**
```
all_states = [UNKNOWN, DOWNLOADING, INSTALLING, RUNNING, STOPPED, OFFLOADING, ERROR]
valid = set([(UNKNOWN, DOWNLOADING), (DOWNLOADING, INSTALLING),
             (DOWNLOADING, ERROR), (INSTALLING, RUNNING),
             (INSTALLING, ERROR), (RUNNING, STOPPED),
             (STOPPED, OFFLOADING), (STOPPED, DOWNLOADING),
             (OFFLOADING, UNKNOWN), (ERROR, DOWNLOADING)])
FOR ANY (from_state, to_state) IN cartesian_product(all_states, all_states):
    IF (from_state, to_state) NOT IN valid:
        adapter = create_adapter_in_state(from_state)
        result = adapter.transition(to_state)
        ASSERT result == Err(InvalidTransition)
```

---

### TS-04-P5: Checksum Gate

**Property:** Property 5 from design.md
**Validates:** 04-REQ-5.2, 04-REQ-5.E1
**Type:** property
**Description:** For any adapter installation, the adapter does not transition
from `DOWNLOADING` to `INSTALLING` unless the SHA-256 checksum matches. A
mismatch always results in an `ERROR` state.

**For any:** Checksum C that does not match the actual manifest digest
**Invariant:** Adapter transitions to `ERROR`, never to `INSTALLING`.

**Assertion pseudocode:**
```
FOR ANY bad_checksum IN random_checksums(count=10):
    stream = client.watch_adapter_states(WatchAdapterStatesRequest{})
    client.install_adapter(InstallAdapterRequest{
        image_ref: "localhost:5000/adaptor:v1",
        checksum_sha256: bad_checksum
    })
    states = collect_states(stream, timeout=10s)
    ASSERT INSTALLING NOT IN states
    ASSERT ERROR IN states
```

---

### TS-04-P6: Offloading Correctness

**Property:** Property 6 from design.md
**Validates:** 04-REQ-6.1, 04-REQ-6.2
**Type:** property
**Description:** For any adapter in `STOPPED` state, if
`last_active.elapsed() > offload_timeout` and no re-install has been
requested, the adapter is offloaded. An adapter in `RUNNING` state is never
offloaded.

**For any:** Adapter A in STOPPED state with elapsed time > timeout
**Invariant:** A is offloaded. An adapter in RUNNING state is never offloaded.

**Assertion pseudocode:**
```
// Stopped adapters are offloaded
update_svc = start_update_service(offload_timeout=2s)
FOR ANY adapter IN install_adapters(count=3):
    stop_adapter(adapter)
sleep(3s)
list_resp = client.list_adapters(ListAdaptersRequest{})
ASSERT len(list_resp.adapters) == 0  // all offloaded

// Running adapters are NOT offloaded
running_adapter = install_and_run_adapter()
sleep(3s)
status = client.get_adapter_status(GetAdapterStatusRequest{adapter_id: running_adapter})
ASSERT status.adapter.state == RUNNING  // still running, not offloaded
```

---

### TS-04-P7: Mock Operator Fee Accuracy

**Property:** Property 7 from design.md
**Validates:** 04-REQ-8.3
**Type:** property
**Description:** For any parking session managed by the mock PARKING_OPERATOR,
the fee returned on stop equals `rate_per_hour * (duration_seconds / 3600.0)`,
using the rate for the session's zone.

**For any:** Zone Z with known rate, duration D > 0
**Invariant:** `fee == rate_per_hour(Z) * (D / 3600.0)`.

**Assertion pseudocode:**
```
FOR ANY zone IN ["zone-munich-central", "zone-munich-west"]:
    rate_resp = http_get("/rate/" + zone)
    rate = parse_json(rate_resp.body).rate_per_hour
    start_time = now()
    start_resp = http_post("/parking/start", {
        vehicle_id: "VIN12345", zone_id: zone, timestamp: start_time
    })
    session_id = parse_json(start_resp.body).session_id
    sleep(2s)
    stop_resp = http_post("/parking/stop", {session_id: session_id})
    body = parse_json(stop_resp.body)
    expected_fee = rate * (body.duration_seconds / 3600.0)
    ASSERT abs(body.fee - expected_fee) < 0.01
```

---

### TS-04-P8: Event Stream Completeness

**Property:** Property 8 from design.md
**Validates:** 04-REQ-4.3, 04-REQ-6.3
**Type:** property
**Description:** For any adapter state transition that occurs, a
`WatchAdapterStates` stream that was active at the time of the transition
receives the corresponding `AdapterStateEvent`.

**For any:** Adapter A undergoing state transition T while watcher W is active
**Invariant:** W receives an event for transition T.

**Assertion pseudocode:**
```
stream = client.watch_adapter_states(WatchAdapterStatesRequest{})
install_resp = client.install_adapter(InstallAdapterRequest{...})
adapter_id = install_resp.adapter_id
// Collect all events for this adapter
events = collect_events_for_adapter(stream, adapter_id, timeout=30s)
// Verify we see the full lifecycle
observed_states = [e.state FOR e IN events]
ASSERT DOWNLOADING IN observed_states
// If installation succeeds:
IF no_errors:
    ASSERT INSTALLING IN observed_states
    ASSERT RUNNING IN observed_states
```

---

## Edge Case Tests

### TS-04-E1: StartSession while session already active

**Requirement:** 04-REQ-1.E1
**Type:** integration
**Description:** Verify that calling `StartSession` while a session is already
active returns gRPC status `ALREADY_EXISTS`.

**Preconditions:**
- PARKING_OPERATOR_ADAPTOR running.
- Mock PARKING_OPERATOR running.
- An active session exists.

**Input:**
- Start a session via `StartSession`.
- Call `StartSession` again.

**Expected:**
- Second call returns gRPC status `ALREADY_EXISTS`.

**Assertion pseudocode:**
```
client.start_session(StartSessionRequest{
    vehicle_id: "VIN12345", zone_id: "zone-munich-central"
})
result = client.start_session(StartSessionRequest{
    vehicle_id: "VIN12345", zone_id: "zone-munich-central"
})
ASSERT result.status_code == ALREADY_EXISTS
```

---

### TS-04-E2: StopSession with unknown session_id

**Requirement:** 04-REQ-1.E2
**Type:** integration
**Description:** Verify that calling `StopSession` with a `session_id` that
does not correspond to an active session returns gRPC status `NOT_FOUND`.

**Preconditions:**
- PARKING_OPERATOR_ADAPTOR running.
- No active session with the given ID.

**Input:**
- gRPC call: `StopSession{session_id: "nonexistent-session-id"}`.

**Expected:**
- Returns gRPC status `NOT_FOUND`.

**Assertion pseudocode:**
```
result = client.stop_session(StopSessionRequest{session_id: "nonexistent-session-id"})
ASSERT result.status_code == NOT_FOUND
```

---

### TS-04-E3: StartSession when PARKING_OPERATOR unreachable

**Requirement:** 04-REQ-1.E3
**Type:** integration
**Description:** Verify that calling `StartSession` when the PARKING_OPERATOR
REST endpoint is unreachable returns gRPC status `UNAVAILABLE`.

**Preconditions:**
- PARKING_OPERATOR_ADAPTOR running.
- Mock PARKING_OPERATOR is NOT running (unreachable).

**Input:**
- gRPC call: `StartSession{vehicle_id: "VIN12345", zone_id: "zone-munich-central"}`.

**Expected:**
- Returns gRPC status `UNAVAILABLE` with details indicating operator unreachable.

**Assertion pseudocode:**
```
adaptor = start_adaptor(operator_url="http://localhost:19999")  // unreachable port
result = client.start_session(StartSessionRequest{
    vehicle_id: "VIN12345", zone_id: "zone-munich-central"
})
ASSERT result.status_code == UNAVAILABLE
ASSERT result.message CONTAINS "unreachable" OR result.message CONTAINS "connection"
```

---

### TS-04-E4: Unlock event with no active session

**Requirement:** 04-REQ-2.E1
**Type:** integration
**Description:** Verify that an unlock event is ignored when no session is
currently active.

**Preconditions:**
- PARKING_OPERATOR_ADAPTOR running.
- Mock PARKING_OPERATOR running.
- No active session.

**Input:**
- Publish `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = false` to DATA_BROKER.

**Expected:**
- No `POST /parking/stop` call is made to the mock PARKING_OPERATOR.

**Assertion pseudocode:**
```
initial_calls = mock_operator.count_calls(POST, "/parking/stop")
publish_to_databroker("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", false)
sleep(2s)
final_calls = mock_operator.count_calls(POST, "/parking/stop")
ASSERT final_calls == initial_calls
```

---

### TS-04-E5: Autonomous start fails when operator unreachable

**Requirement:** 04-REQ-2.E2
**Type:** integration
**Description:** Verify that when the PARKING_OPERATOR is unreachable during
autonomous session start, the adaptor logs the error and does NOT write
`Vehicle.Parking.SessionActive = true` to DATA_BROKER.

**Preconditions:**
- PARKING_OPERATOR_ADAPTOR running with an unreachable operator URL.
- DATA_BROKER running.

**Input:**
- Publish `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = true`.

**Expected:**
- Error is logged.
- `Vehicle.Parking.SessionActive` remains unset or `false`.

**Assertion pseudocode:**
```
adaptor = start_adaptor(operator_url="http://localhost:19999")
publish_to_databroker("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", true)
sleep(2s)
value = read_from_databroker("Vehicle.Parking.SessionActive")
ASSERT value == false OR value IS unset
ASSERT adaptor.logs CONTAINS "error" OR adaptor.logs CONTAINS "unreachable"
```

---

### TS-04-E6: Lock event while session already active

**Requirement:** 04-REQ-2.E3
**Type:** integration
**Description:** Verify that a lock event is ignored when a session is already
active (no duplicate session started).

**Preconditions:**
- PARKING_OPERATOR_ADAPTOR running.
- Mock PARKING_OPERATOR running.
- A session is already active (started via previous lock event).

**Input:**
- Publish lock event to start session.
- Publish another lock event.

**Expected:**
- Only one `POST /parking/start` call is made to mock PARKING_OPERATOR.

**Assertion pseudocode:**
```
publish_to_databroker("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", true)
sleep(2s)
publish_to_databroker("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", true)
sleep(2s)
start_calls = mock_operator.count_calls(POST, "/parking/start")
ASSERT start_calls == 1
```

---

### TS-04-E7: DATA_BROKER unreachable at startup with retry

**Requirement:** 04-REQ-3.E1
**Type:** integration
**Description:** Verify that when DATA_BROKER is unreachable at startup, the
adaptor retries the connection with exponential backoff and logs each retry.

**Preconditions:**
- DATA_BROKER NOT running.

**Input:**
- Start PARKING_OPERATOR_ADAPTOR pointing to an unreachable DATA_BROKER address.
- Observe logs for retry attempts.

**Expected:**
- Adaptor does not crash.
- Logs contain retry attempt messages.

**Assertion pseudocode:**
```
adaptor = start_adaptor(databroker_addr="localhost:19999")
sleep(5s)
ASSERT adaptor.is_running() == true
ASSERT adaptor.logs CONTAINS "retry" OR adaptor.logs CONTAINS "reconnect"
ASSERT count_occurrences(adaptor.logs, "retry") >= 2
stop(adaptor)
```

---

### TS-04-E8: InstallAdapter for already-installed adapter

**Requirement:** 04-REQ-4.E1
**Type:** integration
**Description:** Verify that calling `InstallAdapter` for an already installed
and running adapter returns gRPC status `ALREADY_EXISTS`.

**Preconditions:**
- UPDATE_SERVICE running.
- An adapter with the same `image_ref` is already installed and running.

**Input:**
- Install an adapter.
- Call `InstallAdapter` again with the same `image_ref`.

**Expected:**
- Second call returns gRPC status `ALREADY_EXISTS`.

**Assertion pseudocode:**
```
client.install_adapter(InstallAdapterRequest{
    image_ref: "localhost:5000/adaptor:v1", checksum_sha256: "abc123"
})
sleep(1s)
result = client.install_adapter(InstallAdapterRequest{
    image_ref: "localhost:5000/adaptor:v1", checksum_sha256: "abc123"
})
ASSERT result.status_code == ALREADY_EXISTS
```

---

### TS-04-E9: RemoveAdapter/GetAdapterStatus with unknown adapter_id

**Requirement:** 04-REQ-4.E2
**Type:** integration
**Description:** Verify that calling `RemoveAdapter` or `GetAdapterStatus`
with an unknown `adapter_id` returns gRPC status `NOT_FOUND`.

**Preconditions:**
- UPDATE_SERVICE running.
- No adapter with the given ID exists.

**Input:**
- gRPC call: `RemoveAdapter{adapter_id: "nonexistent-adapter"}`.
- gRPC call: `GetAdapterStatus{adapter_id: "nonexistent-adapter"}`.

**Expected:**
- Both calls return gRPC status `NOT_FOUND`.

**Assertion pseudocode:**
```
remove_result = client.remove_adapter(RemoveAdapterRequest{adapter_id: "nonexistent-adapter"})
ASSERT remove_result.status_code == NOT_FOUND
status_result = client.get_adapter_status(GetAdapterStatusRequest{adapter_id: "nonexistent-adapter"})
ASSERT status_result.status_code == NOT_FOUND
```

---

### TS-04-E10: Container start failure transitions to ERROR

**Requirement:** 04-REQ-4.E3
**Type:** integration
**Description:** Verify that when the container fails to start during
installation, the adapter transitions to `ERROR` state with the failure reason
included in the `AdapterStateEvent`.

**Preconditions:**
- UPDATE_SERVICE running.
- A deliberately broken image that fails to start.

**Input:**
- Install an adapter with a broken image.
- Watch adapter state events.

**Expected:**
- Adapter transitions to `ERROR`.
- The error event includes a failure reason.

**Assertion pseudocode:**
```
stream = client.watch_adapter_states(WatchAdapterStatesRequest{})
client.install_adapter(InstallAdapterRequest{
    image_ref: "localhost:5000/broken-image:v1",
    checksum_sha256: VALID_CHECKSUM_FOR_BROKEN_IMAGE
})
events = collect_events(stream, timeout=10s)
error_events = [e FOR e IN events IF e.state == ERROR]
ASSERT len(error_events) >= 1
ASSERT error_events[0].reason != ""
```

---

### TS-04-E11: Checksum mismatch transitions to ERROR

**Requirement:** 04-REQ-5.E1
**Type:** integration
**Description:** Verify that when the SHA-256 digest does not match, the
adapter transitions to `ERROR`, the image is discarded, and the event
includes "checksum mismatch" detail.

**Preconditions:**
- UPDATE_SERVICE running.
- Mock OCI registry serving a valid image.

**Input:**
- `InstallAdapter` with an incorrect `checksum_sha256`.

**Expected:**
- Adapter transitions to `ERROR`.
- Error event contains "checksum mismatch".

**Assertion pseudocode:**
```
stream = client.watch_adapter_states(WatchAdapterStatesRequest{})
client.install_adapter(InstallAdapterRequest{
    image_ref: "localhost:5000/adaptor:v1",
    checksum_sha256: "0000000000000000000000000000000000000000000000000000000000000000"
})
events = collect_events(stream, timeout=10s)
error_events = [e FOR e IN events IF e.state == ERROR]
ASSERT len(error_events) >= 1
ASSERT error_events[0].reason CONTAINS "checksum mismatch"
```

---

### TS-04-E12: Registry unreachable during pull

**Requirement:** 04-REQ-5.E2
**Type:** integration
**Description:** Verify that when the registry is unreachable during image
pull, the adapter transitions to `ERROR` with the failure reason.

**Preconditions:**
- UPDATE_SERVICE running.
- Registry URL points to an unreachable address.

**Input:**
- `InstallAdapter` with `image_ref` pointing to unreachable registry.

**Expected:**
- Adapter transitions to `ERROR`.
- Error event includes failure reason about registry.

**Assertion pseudocode:**
```
update_svc = start_update_service(registry_url="localhost:19999")
stream = client.watch_adapter_states(WatchAdapterStatesRequest{})
client.install_adapter(InstallAdapterRequest{
    image_ref: "localhost:19999/adaptor:v1",
    checksum_sha256: "abc123"
})
events = collect_events(stream, timeout=10s)
error_events = [e FOR e IN events IF e.state == ERROR]
ASSERT len(error_events) >= 1
ASSERT error_events[0].reason CONTAINS "registry" OR error_events[0].reason CONTAINS "unreachable"
```

---

### TS-04-E13: Re-install during OFFLOADING cancels offload

**Requirement:** 04-REQ-6.E1
**Type:** integration
**Description:** Verify that if an adapter is re-installed while in the
`OFFLOADING` state, the offload is cancelled and the adapter is re-downloaded.

**Preconditions:**
- UPDATE_SERVICE running with a short offload timeout.
- An adapter in `OFFLOADING` state.

**Input:**
- Stop an adapter and wait for offloading to begin.
- Call `InstallAdapter` with the same image during offloading.

**Expected:**
- Offload is cancelled.
- Adapter transitions to `DOWNLOADING` (re-download begins).

**Assertion pseudocode:**
```
update_svc = start_update_service(offload_timeout=3s)
install_resp = client.install_adapter(InstallAdapterRequest{...})
wait_until_state(install_resp.adapter_id, RUNNING)
stop_adapter(install_resp.adapter_id)
sleep(2s)  // wait for OFFLOADING to begin but not complete
// Re-install
stream = client.watch_adapter_states(WatchAdapterStatesRequest{})
client.install_adapter(InstallAdapterRequest{...})
events = collect_events(stream, timeout=10s)
ASSERT DOWNLOADING IN [e.state FOR e IN events]
```

---

### TS-04-E14: Mock operator stop unknown session returns 404

**Requirement:** 04-REQ-8.E1
**Type:** unit
**Description:** Verify that `POST /parking/stop` with a nonexistent
`session_id` returns HTTP 404.

**Preconditions:**
- Mock PARKING_OPERATOR running.

**Input:**
- `POST /parking/stop` with `{"session_id": "nonexistent-id"}`.

**Expected:**
- HTTP 404 with descriptive error message.

**Assertion pseudocode:**
```
response = http_post("/parking/stop", {session_id: "nonexistent-id"})
ASSERT response.status_code == 404
ASSERT response.body CONTAINS "not found" OR response.body CONTAINS "error"
```

---

### TS-04-E15: Mock operator status unknown session returns 404

**Requirement:** 04-REQ-8.E2
**Type:** unit
**Description:** Verify that `GET /parking/{session_id}/status` with a
nonexistent `session_id` returns HTTP 404.

**Preconditions:**
- Mock PARKING_OPERATOR running.

**Input:**
- `GET /parking/nonexistent-id/status`.

**Expected:**
- HTTP 404.

**Assertion pseudocode:**
```
response = http_get("/parking/nonexistent-id/status")
ASSERT response.status_code == 404
```

---

### TS-04-E16: Mock operator rate unknown zone returns 404

**Requirement:** 04-REQ-8.E3
**Type:** unit
**Description:** Verify that `GET /rate/{zone_id}` with an unknown `zone_id`
returns HTTP 404.

**Preconditions:**
- Mock PARKING_OPERATOR running.

**Input:**
- `GET /rate/unknown-zone-id`.

**Expected:**
- HTTP 404 with descriptive error message.

**Assertion pseudocode:**
```
response = http_get("/rate/unknown-zone-id")
ASSERT response.status_code == 404
ASSERT response.body CONTAINS "not found" OR response.body CONTAINS "error"
```

---

### TS-04-E17: CLI command when service unreachable

**Requirement:** 04-REQ-9.E1
**Type:** integration
**Description:** Verify that when the target gRPC service is unreachable, the
CLI command prints an error message including the target address and exits
with a non-zero exit code.

**Preconditions:**
- UPDATE_SERVICE and PARKING_OPERATOR_ADAPTOR are NOT running.
- Mock PARKING_APP CLI built.

**Input:**
- `parking-app-cli install --image-ref test:v1 --checksum abc123`.
- `parking-app-cli start-session --vehicle-id VIN12345 --zone-id zone1`.

**Expected:**
- Both commands exit with non-zero code.
- Error output includes the target address.

**Assertion pseudocode:**
```
result1 = exec("parking-app-cli install --image-ref test:v1 --checksum abc123",
    env={"UPDATE_SERVICE_ADDR": "localhost:19999"})
ASSERT result1.exit_code != 0
ASSERT result1.stderr CONTAINS "localhost:19999" OR result1.stderr CONTAINS "unreachable"

result2 = exec("parking-app-cli start-session --vehicle-id VIN12345 --zone-id zone1",
    env={"ADAPTOR_ADDR": "localhost:19998"})
ASSERT result2.exit_code != 0
ASSERT result2.stderr CONTAINS "localhost:19998" OR result2.stderr CONTAINS "unreachable"
```

---

## Coverage Matrix

| Requirement    | Test Spec Entry | Type        |
|----------------|-----------------|-------------|
| 04-REQ-1.1     | TS-04-1         | integration |
| 04-REQ-1.2     | TS-04-2         | integration |
| 04-REQ-1.3     | TS-04-3         | integration |
| 04-REQ-1.4     | TS-04-4         | integration |
| 04-REQ-1.5     | TS-04-5         | integration |
| 04-REQ-1.E1    | TS-04-E1        | integration |
| 04-REQ-1.E2    | TS-04-E2        | integration |
| 04-REQ-1.E3    | TS-04-E3        | integration |
| 04-REQ-2.1     | TS-04-6         | integration |
| 04-REQ-2.2     | TS-04-7         | integration |
| 04-REQ-2.3     | TS-04-8         | integration |
| 04-REQ-2.4     | TS-04-9         | integration |
| 04-REQ-2.5     | TS-04-10        | integration |
| 04-REQ-2.E1    | TS-04-E4        | integration |
| 04-REQ-2.E2    | TS-04-E5        | integration |
| 04-REQ-2.E3    | TS-04-E6        | integration |
| 04-REQ-3.1     | TS-04-11        | integration |
| 04-REQ-3.2     | TS-04-12        | integration |
| 04-REQ-3.3     | TS-04-13        | integration |
| 04-REQ-3.4     | TS-04-14        | integration |
| 04-REQ-3.E1    | TS-04-E7        | integration |
| 04-REQ-4.1     | TS-04-15        | integration |
| 04-REQ-4.2     | TS-04-16        | integration |
| 04-REQ-4.3     | TS-04-17        | integration |
| 04-REQ-4.4     | TS-04-18        | integration |
| 04-REQ-4.5     | TS-04-19        | integration |
| 04-REQ-4.6     | TS-04-20        | integration |
| 04-REQ-4.E1    | TS-04-E8        | integration |
| 04-REQ-4.E2    | TS-04-E9        | integration |
| 04-REQ-4.E3    | TS-04-E10       | integration |
| 04-REQ-5.1     | TS-04-21        | integration |
| 04-REQ-5.2     | TS-04-22        | unit        |
| 04-REQ-5.3     | TS-04-23        | integration |
| 04-REQ-5.E1    | TS-04-E11       | integration |
| 04-REQ-5.E2    | TS-04-E12       | integration |
| 04-REQ-6.1     | TS-04-24        | unit        |
| 04-REQ-6.2     | TS-04-25        | integration |
| 04-REQ-6.3     | TS-04-26        | integration |
| 04-REQ-6.E1    | TS-04-E13       | integration |
| 04-REQ-7.1     | TS-04-27        | unit        |
| 04-REQ-7.2     | TS-04-28        | unit        |
| 04-REQ-8.1     | TS-04-29        | unit        |
| 04-REQ-8.2     | TS-04-30        | unit        |
| 04-REQ-8.3     | TS-04-31        | unit        |
| 04-REQ-8.4     | TS-04-32        | unit        |
| 04-REQ-8.5     | TS-04-33        | unit        |
| 04-REQ-8.E1    | TS-04-E14       | unit        |
| 04-REQ-8.E2    | TS-04-E15       | unit        |
| 04-REQ-8.E3    | TS-04-E16       | unit        |
| 04-REQ-9.1     | TS-04-34        | integration |
| 04-REQ-9.2     | TS-04-35        | integration |
| 04-REQ-9.3     | TS-04-36        | integration |
| 04-REQ-9.4     | TS-04-37        | integration |
| 04-REQ-9.5     | TS-04-38        | integration |
| 04-REQ-9.E1    | TS-04-E17       | integration |
| 04-REQ-10.1    | TS-04-39        | integration |
| 04-REQ-10.2    | TS-04-40        | integration |
| 04-REQ-10.3    | TS-04-41        | integration |
| Property 1     | TS-04-P1        | property    |
| Property 2     | TS-04-P2        | property    |
| Property 3     | TS-04-P3        | property    |
| Property 4     | TS-04-P4        | property    |
| Property 5     | TS-04-P5        | property    |
| Property 6     | TS-04-P6        | property    |
| Property 7     | TS-04-P7        | property    |
| Property 8     | TS-04-P8        | property    |
