# Test Specification: RHIVOS QM Partition (Phase 2.3)

## Overview

This test specification translates every acceptance criterion and correctness
property from the requirements and design documents into concrete, executable
test contracts. Tests are organized into three categories:

- **Acceptance criterion tests (TS-04-N):** One per acceptance criterion.
  Implemented as Rust unit/integration tests for RHIVOS services and Go tests
  for the mock PARKING_OPERATOR and CLI enhancements.
- **Property tests (TS-04-PN):** One per correctness property. Verify
  invariants that must hold across the system.
- **Edge case tests (TS-04-EN):** One per edge case requirement. Verify
  error handling and boundary behavior.

Tests for Rust components use `#[tokio::test]` with mocked dependencies.
Tests for Go components use the standard `testing` package. Integration tests
require local infrastructure (DATA_BROKER via `make infra-up`).

## Test Cases

### TS-04-1: PARKING_OPERATOR_ADAPTOR exposes gRPC service

**Requirement:** 04-REQ-1.1
**Type:** integration
**Description:** Verify the PARKING_OPERATOR_ADAPTOR starts a gRPC server on
the configured address and responds to service reflection or connection
requests.

**Preconditions:**
- PARKING_OPERATOR_ADAPTOR binary built.
- Mock PARKING_OPERATOR running on port 8090.

**Input:**
- Start PARKING_OPERATOR_ADAPTOR with `ADAPTOR_GRPC_ADDR=0.0.0.0:50052`.
- Connect a gRPC client to `localhost:50052`.

**Expected:**
- Connection succeeds. Service responds to gRPC health or method calls.

**Assertion pseudocode:**
```
process = start("parking-operator-adaptor", env={"ADAPTOR_GRPC_ADDR": "0.0.0.0:50052"})
wait_for_port(50052)
client = grpc_connect("localhost:50052")
response = client.GetRate(GetRateRequest{zone_id: "zone-munich-central"})
ASSERT response is not error OR response.status == UNAVAILABLE  // operator may not be running
stop(process)
```

---

### TS-04-2: StartSession returns session_id and status

**Requirement:** 04-REQ-1.2
**Type:** unit
**Description:** Verify that calling StartSession with valid input returns a
session_id and status.

**Preconditions:**
- Mock PARKING_OPERATOR running and reachable.
- No active session.

**Input:**
- `StartSessionRequest{vehicle_id: "VIN12345", zone_id: "zone-munich-central"}`

**Expected:**
- `StartSessionResponse` with non-empty `session_id` and `status` of "active".

**Assertion pseudocode:**
```
response = adaptor.StartSession({vehicle_id: "VIN12345", zone_id: "zone-munich-central"})
ASSERT response.session_id != ""
ASSERT response.status == "active"
```

---

### TS-04-3: StopSession returns fee and duration

**Requirement:** 04-REQ-1.3
**Type:** unit
**Description:** Verify that calling StopSession with a valid session_id
returns fee, duration, and currency.

**Preconditions:**
- An active session exists (started via StartSession).

**Input:**
- `StopSessionRequest{session_id: "<active_session_id>"}`

**Expected:**
- `StopSessionResponse` with matching `session_id`, `total_fee >= 0`,
  `duration_seconds >= 0`, and non-empty `currency`.

**Assertion pseudocode:**
```
start_resp = adaptor.StartSession({vehicle_id: "VIN12345", zone_id: "zone-munich-central"})
// wait briefly to accumulate some duration
sleep(1s)
stop_resp = adaptor.StopSession({session_id: start_resp.session_id})
ASSERT stop_resp.session_id == start_resp.session_id
ASSERT stop_resp.total_fee >= 0
ASSERT stop_resp.duration_seconds >= 1
ASSERT stop_resp.currency != ""
```

---

### TS-04-4: GetStatus returns session state

**Requirement:** 04-REQ-1.4
**Type:** unit
**Description:** Verify GetStatus returns current session state.

**Preconditions:**
- An active session exists.

**Input:**
- `GetStatusRequest{session_id: "<active_session_id>"}`

**Expected:**
- `GetStatusResponse` with `active == true`, valid `start_time`, and
  `current_fee >= 0`.

**Assertion pseudocode:**
```
start_resp = adaptor.StartSession({vehicle_id: "VIN12345", zone_id: "zone-munich-central"})
status_resp = adaptor.GetStatus({session_id: start_resp.session_id})
ASSERT status_resp.session_id == start_resp.session_id
ASSERT status_resp.active == true
ASSERT status_resp.start_time > 0
ASSERT status_resp.current_fee >= 0
```

---

### TS-04-5: GetRate returns zone rate

**Requirement:** 04-REQ-1.5
**Type:** unit
**Description:** Verify GetRate returns the parking rate for a zone.

**Preconditions:**
- Mock PARKING_OPERATOR running with preconfigured zones.

**Input:**
- `GetRateRequest{zone_id: "zone-munich-central"}`

**Expected:**
- `GetRateResponse` with `rate_per_hour == 2.50`, `currency == "EUR"`,
  `zone_name == "Munich Central"`.

**Assertion pseudocode:**
```
rate_resp = adaptor.GetRate({zone_id: "zone-munich-central"})
ASSERT rate_resp.rate_per_hour == 2.50
ASSERT rate_resp.currency == "EUR"
ASSERT rate_resp.zone_name == "Munich Central"
```

---

### TS-04-6: Lock event triggers autonomous session start

**Requirement:** 04-REQ-2.1
**Type:** integration
**Description:** Verify that a lock event from DATA_BROKER triggers the
adaptor to autonomously start a parking session.

**Preconditions:**
- DATA_BROKER running. PARKING_OPERATOR_ADAPTOR running and subscribed.
- Mock PARKING_OPERATOR running.
- Location set in DATA_BROKER.

**Input:**
- Write `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = true` to DATA_BROKER.

**Expected:**
- Mock PARKING_OPERATOR receives `POST /parking/start` within 5 seconds.
- `Vehicle.Parking.SessionActive = true` in DATA_BROKER.

**Assertion pseudocode:**
```
databroker.set("Vehicle.CurrentLocation.Latitude", 48.1351)
databroker.set("Vehicle.CurrentLocation.Longitude", 11.5820)
databroker.set("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", true)
wait(5s)
session_active = databroker.get("Vehicle.Parking.SessionActive")
ASSERT session_active == true
ASSERT mock_operator.received_start_request()
```

---

### TS-04-7: Unlock event triggers autonomous session stop

**Requirement:** 04-REQ-2.2
**Type:** integration
**Description:** Verify that an unlock event from DATA_BROKER triggers the
adaptor to autonomously stop the active parking session.

**Preconditions:**
- Active session running (started via lock event).
- Mock PARKING_OPERATOR running.

**Input:**
- Write `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = false` to DATA_BROKER.

**Expected:**
- Mock PARKING_OPERATOR receives `POST /parking/stop` within 5 seconds.
- `Vehicle.Parking.SessionActive = false` in DATA_BROKER.

**Assertion pseudocode:**
```
// Session already active from lock event
databroker.set("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", false)
wait(5s)
session_active = databroker.get("Vehicle.Parking.SessionActive")
ASSERT session_active == false
ASSERT mock_operator.received_stop_request()
```

---

### TS-04-8: SessionActive set to true after autonomous start

**Requirement:** 04-REQ-2.3
**Type:** integration
**Description:** Verify SessionActive is written to DATA_BROKER after
autonomous start.

**Preconditions:**
- DATA_BROKER running. Lock event processed.

**Input:**
- Lock event triggers autonomous start.

**Expected:**
- `Vehicle.Parking.SessionActive == true` in DATA_BROKER.

**Assertion pseudocode:**
```
databroker.set("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", true)
wait(5s)
ASSERT databroker.get("Vehicle.Parking.SessionActive") == true
```

---

### TS-04-9: SessionActive set to false after autonomous stop

**Requirement:** 04-REQ-2.4
**Type:** integration
**Description:** Verify SessionActive is written to DATA_BROKER after
autonomous stop.

**Preconditions:**
- Active session exists. Unlock event processed.

**Input:**
- Unlock event triggers autonomous stop.

**Expected:**
- `Vehicle.Parking.SessionActive == false` in DATA_BROKER.

**Assertion pseudocode:**
```
// Session active from previous lock
databroker.set("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", false)
wait(5s)
ASSERT databroker.get("Vehicle.Parking.SessionActive") == false
```

---

### TS-04-10: Manual override updates SessionActive

**Requirement:** 04-REQ-2.5
**Type:** unit
**Description:** Verify that a manual StartSession/StopSession call updates
SessionActive accordingly.

**Preconditions:**
- Adaptor running. No active session.

**Input:**
- Call `StartSession` via gRPC (override). Then call `StopSession`.

**Expected:**
- After StartSession: SessionActive = true.
- After StopSession: SessionActive = false.

**Assertion pseudocode:**
```
adaptor.StartSession({vehicle_id: "VIN12345", zone_id: "zone-munich-central"})
ASSERT databroker.get("Vehicle.Parking.SessionActive") == true
adaptor.StopSession({session_id: "..."})
ASSERT databroker.get("Vehicle.Parking.SessionActive") == false
```

---

### TS-04-11: Adaptor connects to DATA_BROKER via network gRPC

**Requirement:** 04-REQ-3.1
**Type:** integration
**Description:** Verify the adaptor connects to DATA_BROKER over TCP.

**Preconditions:**
- DATA_BROKER running on port 55555.

**Input:**
- Start adaptor with `DATABROKER_ADDR=localhost:55555`.

**Expected:**
- Adaptor establishes gRPC connection. No connection errors in logs.

**Assertion pseudocode:**
```
process = start("parking-operator-adaptor", env={"DATABROKER_ADDR": "localhost:55555"})
wait(3s)
ASSERT process.stderr does NOT contain "connection refused"
ASSERT process.stderr does NOT contain "connect error"
stop(process)
```

---

### TS-04-12: Adaptor subscribes to IsLocked signal

**Requirement:** 04-REQ-3.2
**Type:** integration
**Description:** Verify the adaptor subscribes to the IsLocked signal.

**Preconditions:**
- DATA_BROKER running. Adaptor running.

**Input:**
- Change IsLocked value in DATA_BROKER.

**Expected:**
- Adaptor reacts to the value change (starts/stops session).

**Assertion pseudocode:**
```
// This is verified implicitly by TS-04-6 and TS-04-7
// The subscription is confirmed by the adaptor's reaction to lock events
databroker.set("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", true)
wait(5s)
ASSERT mock_operator.received_start_request()
```

---

### TS-04-13: Adaptor reads location from DATA_BROKER

**Requirement:** 04-REQ-3.3
**Type:** integration
**Description:** Verify the adaptor reads location signals for zone context.

**Preconditions:**
- DATA_BROKER running with location values set.

**Input:**
- Set location in DATA_BROKER, then trigger lock event.

**Expected:**
- The start request to mock PARKING_OPERATOR includes zone context derived
  from the location.

**Assertion pseudocode:**
```
databroker.set("Vehicle.CurrentLocation.Latitude", 48.1351)
databroker.set("Vehicle.CurrentLocation.Longitude", 11.5820)
databroker.set("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", true)
wait(5s)
start_req = mock_operator.last_start_request()
ASSERT start_req.zone_id != ""
```

---

### TS-04-14: Adaptor writes SessionActive to DATA_BROKER

**Requirement:** 04-REQ-3.4
**Type:** integration
**Description:** Verify the adaptor writes the SessionActive signal.

**Preconditions:**
- DATA_BROKER running. Adaptor running.

**Input:**
- Trigger lock event to start session.

**Expected:**
- `Vehicle.Parking.SessionActive` is set in DATA_BROKER.

**Assertion pseudocode:**
```
databroker.set("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", true)
wait(5s)
value = databroker.get("Vehicle.Parking.SessionActive")
ASSERT value == true
```

---

### TS-04-15: UPDATE_SERVICE exposes gRPC service

**Requirement:** 04-REQ-4.1
**Type:** integration
**Description:** Verify UPDATE_SERVICE starts a gRPC server on the configured
address.

**Preconditions:**
- UPDATE_SERVICE binary built.

**Input:**
- Start UPDATE_SERVICE with `UPDATE_GRPC_ADDR=0.0.0.0:50051`.
- Connect a gRPC client.

**Expected:**
- Connection succeeds. Service responds to ListAdapters.

**Assertion pseudocode:**
```
process = start("update-service", env={"UPDATE_GRPC_ADDR": "0.0.0.0:50051"})
wait_for_port(50051)
client = grpc_connect("localhost:50051")
response = client.ListAdapters({})
ASSERT response is not error
ASSERT response.adapters is defined (may be empty list)
stop(process)
```

---

### TS-04-16: InstallAdapter returns DOWNLOADING state

**Requirement:** 04-REQ-4.2
**Type:** unit
**Description:** Verify InstallAdapter initiates download and returns
DOWNLOADING state.

**Preconditions:**
- UPDATE_SERVICE running.

**Input:**
- `InstallAdapterRequest{image_ref: "registry/adapter:v1", checksum_sha256: "abc123"}`

**Expected:**
- `InstallAdapterResponse` with non-empty `job_id`, non-empty `adapter_id`,
  `state == DOWNLOADING`.

**Assertion pseudocode:**
```
response = update_service.InstallAdapter({
    image_ref: "registry/adapter:v1",
    checksum_sha256: "abc123"
})
ASSERT response.job_id != ""
ASSERT response.adapter_id != ""
ASSERT response.state == ADAPTER_STATE_DOWNLOADING
```

---

### TS-04-17: WatchAdapterStates streams events

**Requirement:** 04-REQ-4.3
**Type:** unit
**Description:** Verify WatchAdapterStates returns a stream of state events.

**Preconditions:**
- UPDATE_SERVICE running.

**Input:**
- Call WatchAdapterStates, then trigger an InstallAdapter.

**Expected:**
- Stream receives at least one AdapterStateEvent with valid adapter_id and
  state transition.

**Assertion pseudocode:**
```
stream = update_service.WatchAdapterStates({})
update_service.InstallAdapter({image_ref: "registry/adapter:v1", checksum_sha256: "abc123"})
event = stream.next(timeout=10s)
ASSERT event.adapter_id != ""
ASSERT event.new_state != ADAPTER_STATE_UNKNOWN
ASSERT event.timestamp > 0
```

---

### TS-04-18: ListAdapters returns all known adapters

**Requirement:** 04-REQ-4.4
**Type:** unit
**Description:** Verify ListAdapters returns a list of all adapters.

**Preconditions:**
- UPDATE_SERVICE running. At least one adapter installed.

**Input:**
- Install an adapter, then call ListAdapters.

**Expected:**
- Response contains at least one adapter with matching adapter_id.

**Assertion pseudocode:**
```
install_resp = update_service.InstallAdapter({image_ref: "registry/adapter:v1", checksum_sha256: "abc123"})
list_resp = update_service.ListAdapters({})
ASSERT len(list_resp.adapters) >= 1
found = any(a.adapter_id == install_resp.adapter_id for a in list_resp.adapters)
ASSERT found == true
```

---

### TS-04-19: RemoveAdapter stops and removes adapter

**Requirement:** 04-REQ-4.5
**Type:** unit
**Description:** Verify RemoveAdapter stops and removes an adapter.

**Preconditions:**
- An adapter is installed.

**Input:**
- `RemoveAdapterRequest{adapter_id: "<installed_adapter_id>"}`

**Expected:**
- Returns success. Adapter no longer in ListAdapters response.

**Assertion pseudocode:**
```
install_resp = update_service.InstallAdapter({image_ref: "registry/adapter:v1", checksum_sha256: "abc123"})
update_service.RemoveAdapter({adapter_id: install_resp.adapter_id})
list_resp = update_service.ListAdapters({})
found = any(a.adapter_id == install_resp.adapter_id for a in list_resp.adapters)
ASSERT found == false
```

---

### TS-04-20: GetAdapterStatus returns adapter info

**Requirement:** 04-REQ-4.6
**Type:** unit
**Description:** Verify GetAdapterStatus returns the adapter's current info.

**Preconditions:**
- An adapter is installed.

**Input:**
- `GetAdapterStatusRequest{adapter_id: "<installed_adapter_id>"}`

**Expected:**
- Response contains AdapterInfo with matching adapter_id and valid state.

**Assertion pseudocode:**
```
install_resp = update_service.InstallAdapter({image_ref: "registry/adapter:v1", checksum_sha256: "abc123"})
status_resp = update_service.GetAdapterStatus({adapter_id: install_resp.adapter_id})
ASSERT status_resp.adapter.adapter_id == install_resp.adapter_id
ASSERT status_resp.adapter.image_ref == "registry/adapter:v1"
ASSERT status_resp.adapter.state != ADAPTER_STATE_UNKNOWN
```

---

### TS-04-21: OCI image pull from registry

**Requirement:** 04-REQ-5.1
**Type:** integration
**Description:** Verify UPDATE_SERVICE pulls an OCI image from the registry.

**Preconditions:**
- Local OCI registry running with a test image.

**Input:**
- Call InstallAdapter with a valid image_ref pointing to the local registry.

**Expected:**
- Adapter transitions through DOWNLOADING state. Image content is retrieved.

**Assertion pseudocode:**
```
// Push a test image to local registry first
push_test_image("localhost:5000/test-adapter:v1")
checksum = get_manifest_sha256("localhost:5000/test-adapter:v1")
response = update_service.InstallAdapter({
    image_ref: "localhost:5000/test-adapter:v1",
    checksum_sha256: checksum
})
ASSERT response.state == ADAPTER_STATE_DOWNLOADING
// Wait for download to complete
wait_for_state(response.adapter_id, INSTALLING, timeout=30s)
```

---

### TS-04-22: SHA-256 checksum verification passes

**Requirement:** 04-REQ-5.2
**Type:** unit
**Description:** Verify the checksum verification logic accepts matching
checksums.

**Preconditions:**
- A manifest with known SHA-256 digest.

**Input:**
- Manifest content and its correct SHA-256 checksum.

**Expected:**
- Verification succeeds; no error.

**Assertion pseudocode:**
```
manifest = b"test manifest content"
checksum = sha256(manifest)
result = verify_checksum(manifest, checksum)
ASSERT result == Ok
```

---

### TS-04-23: Successful checksum transitions DOWNLOADING to INSTALLING

**Requirement:** 04-REQ-5.3
**Type:** unit
**Description:** Verify that after successful checksum, adapter transitions
from DOWNLOADING to INSTALLING.

**Preconditions:**
- Adapter in DOWNLOADING state. Checksum matches.

**Input:**
- Provide correct checksum for downloaded manifest.

**Expected:**
- Adapter state transitions to INSTALLING.

**Assertion pseudocode:**
```
install_resp = update_service.InstallAdapter({image_ref: "...", checksum_sha256: correct_checksum})
wait_for_state(install_resp.adapter_id, INSTALLING, timeout=30s)
status = update_service.GetAdapterStatus({adapter_id: install_resp.adapter_id})
ASSERT status.adapter.state == ADAPTER_STATE_INSTALLING
```

---

### TS-04-24: Inactivity timeout triggers offloading

**Requirement:** 04-REQ-6.1
**Type:** unit
**Description:** Verify that a stopped adapter is offloaded after the
configured inactivity timeout.

**Preconditions:**
- UPDATE_SERVICE running with short offload timeout (e.g., 2 seconds for test).
- An adapter in STOPPED state.

**Input:**
- Wait for offload timeout to expire.

**Expected:**
- Adapter transitions to OFFLOADING, then is removed.

**Assertion pseudocode:**
```
// Start UPDATE_SERVICE with OFFLOAD_TIMEOUT_HOURS=0 (immediate, or use seconds-based test config)
install_resp = update_service.InstallAdapter({...})
// ... adapter reaches RUNNING, then STOPPED
wait_for_state(install_resp.adapter_id, STOPPED)
wait(offload_timeout + margin)
status = update_service.GetAdapterStatus({adapter_id: install_resp.adapter_id})
ASSERT status.error == NOT_FOUND  // adapter has been offloaded and removed
```

---

### TS-04-25: Offloading removes container resources

**Requirement:** 04-REQ-6.2
**Type:** unit
**Description:** Verify offloading removes the adapter container and frees
resources.

**Preconditions:**
- Adapter in STOPPED state, offload triggered.

**Input:**
- Offload timeout expires.

**Expected:**
- Container is removed. Adapter no longer in ListAdapters.

**Assertion pseudocode:**
```
// After offloading completes
list_resp = update_service.ListAdapters({})
found = any(a.adapter_id == offloaded_adapter_id for a in list_resp.adapters)
ASSERT found == false
```

---

### TS-04-26: Offloading emits AdapterStateEvent

**Requirement:** 04-REQ-6.3
**Type:** unit
**Description:** Verify WatchAdapterStates receives events during offloading.

**Preconditions:**
- WatchAdapterStates stream active. Adapter in STOPPED state.

**Input:**
- Offload timeout expires.

**Expected:**
- Stream receives event with `old_state=STOPPED, new_state=OFFLOADING`.

**Assertion pseudocode:**
```
stream = update_service.WatchAdapterStates({})
// ... trigger offloading
event = stream.next(timeout=offload_timeout + 10s)
ASSERT event.old_state == ADAPTER_STATE_STOPPED
ASSERT event.new_state == ADAPTER_STATE_OFFLOADING
```

---

### TS-04-27: Valid state transitions enforced

**Requirement:** 04-REQ-7.1
**Type:** unit
**Description:** Verify all valid state transitions are accepted by the
adapter manager.

**Preconditions:**
- Adapter manager initialized.

**Input:**
- Attempt each valid transition defined in 04-REQ-7.1.

**Expected:**
- All valid transitions succeed without error.

**Assertion pseudocode:**
```
valid_transitions = [
    (UNKNOWN, DOWNLOADING),
    (DOWNLOADING, INSTALLING),
    (DOWNLOADING, ERROR),
    (INSTALLING, RUNNING),
    (INSTALLING, ERROR),
    (RUNNING, STOPPED),
    (STOPPED, OFFLOADING),
    (STOPPED, DOWNLOADING),
    (OFFLOADING, UNKNOWN),
    (ERROR, DOWNLOADING),
]
FOR EACH (from, to) IN valid_transitions:
    result = adapter_manager.transition(adapter_id, from, to)
    ASSERT result == Ok
```

---

### TS-04-28: Invalid state transitions rejected

**Requirement:** 04-REQ-7.2
**Type:** unit
**Description:** Verify invalid state transitions are rejected.

**Preconditions:**
- Adapter manager initialized.

**Input:**
- Attempt invalid transitions (e.g., RUNNING -> DOWNLOADING,
  UNKNOWN -> RUNNING, ERROR -> RUNNING).

**Expected:**
- Each invalid transition returns an error. State does not change.

**Assertion pseudocode:**
```
invalid_transitions = [
    (RUNNING, DOWNLOADING),
    (UNKNOWN, RUNNING),
    (ERROR, RUNNING),
    (DOWNLOADING, STOPPED),
    (OFFLOADING, RUNNING),
]
FOR EACH (from, to) IN invalid_transitions:
    result = adapter_manager.transition(adapter_id, from, to)
    ASSERT result == Err
```

---

### TS-04-29: Mock operator POST /parking/start

**Requirement:** 04-REQ-8.1, 04-REQ-8.2
**Type:** unit
**Description:** Verify the mock PARKING_OPERATOR starts a session.

**Preconditions:**
- Mock PARKING_OPERATOR running.

**Input:**
- `POST /parking/start` with `{"vehicle_id": "VIN12345", "zone_id": "zone-munich-central", "timestamp": 1708700000}`.

**Expected:**
- HTTP 200. JSON response with `session_id` (non-empty UUID) and
  `status` "active".

**Assertion pseudocode:**
```
response = http_post("http://localhost:8090/parking/start", {
    "vehicle_id": "VIN12345",
    "zone_id": "zone-munich-central",
    "timestamp": 1708700000
})
ASSERT response.status_code == 200
body = json_decode(response.body)
ASSERT body.session_id != ""
ASSERT body.status == "active"
```

---

### TS-04-30: Mock operator POST /parking/stop

**Requirement:** 04-REQ-8.3
**Type:** unit
**Description:** Verify the mock PARKING_OPERATOR stops a session and returns
fee calculation.

**Preconditions:**
- Active session exists in mock operator.

**Input:**
- `POST /parking/stop` with `{"session_id": "<active_session_id>"}`.

**Expected:**
- HTTP 200. JSON response with `session_id`, `fee >= 0`,
  `duration_seconds >= 0`, `currency == "EUR"`.

**Assertion pseudocode:**
```
start_resp = http_post(".../parking/start", {vehicle_id: "VIN12345", zone_id: "zone-munich-central", timestamp: now()})
session_id = json_decode(start_resp.body).session_id
sleep(1s)
stop_resp = http_post(".../parking/stop", {session_id: session_id})
ASSERT stop_resp.status_code == 200
body = json_decode(stop_resp.body)
ASSERT body.session_id == session_id
ASSERT body.fee >= 0
ASSERT body.duration_seconds >= 1
ASSERT body.currency == "EUR"
```

---

### TS-04-31: Mock operator GET /parking/{session_id}/status

**Requirement:** 04-REQ-8.4
**Type:** unit
**Description:** Verify the mock PARKING_OPERATOR returns session status.

**Preconditions:**
- Active session exists.

**Input:**
- `GET /parking/{session_id}/status`

**Expected:**
- HTTP 200. JSON with `session_id`, `active == true`, `start_time > 0`,
  `current_fee >= 0`, `currency`.

**Assertion pseudocode:**
```
start_resp = http_post(".../parking/start", {vehicle_id: "VIN12345", zone_id: "zone-munich-central", timestamp: now()})
session_id = json_decode(start_resp.body).session_id
status_resp = http_get(".../parking/" + session_id + "/status")
ASSERT status_resp.status_code == 200
body = json_decode(status_resp.body)
ASSERT body.session_id == session_id
ASSERT body.active == true
ASSERT body.start_time > 0
```

---

### TS-04-32: Mock operator GET /rate/{zone_id}

**Requirement:** 04-REQ-8.5
**Type:** unit
**Description:** Verify the mock PARKING_OPERATOR returns zone rate.

**Preconditions:**
- Mock operator running with preconfigured zones.

**Input:**
- `GET /rate/zone-munich-central`

**Expected:**
- HTTP 200. JSON with `rate_per_hour == 2.50`, `currency == "EUR"`,
  `zone_name == "Munich Central"`.

**Assertion pseudocode:**
```
response = http_get("http://localhost:8090/rate/zone-munich-central")
ASSERT response.status_code == 200
body = json_decode(response.body)
ASSERT body.rate_per_hour == 2.50
ASSERT body.currency == "EUR"
ASSERT body.zone_name == "Munich Central"
```

---

### TS-04-33: CLI install command calls InstallAdapter

**Requirement:** 04-REQ-9.1
**Type:** integration
**Description:** Verify the `install` CLI command calls UPDATE_SERVICE.

**Preconditions:**
- UPDATE_SERVICE running.

**Input:**
- `parking-app-cli install --image-ref registry/adapter:v1 --checksum abc123`

**Expected:**
- Exit code 0. Output contains `job_id`, `adapter_id`, and `DOWNLOADING`.

**Assertion pseudocode:**
```
result = exec("parking-app-cli install --image-ref registry/adapter:v1 --checksum abc123 --update-addr localhost:50051")
ASSERT result.exit_code == 0
ASSERT contains(result.stdout, "job_id")
ASSERT contains(result.stdout, "adapter_id")
ASSERT contains(result.stdout, "DOWNLOADING") OR contains(result.stdout, "downloading")
```

---

### TS-04-34: CLI watch command streams events

**Requirement:** 04-REQ-9.2
**Type:** integration
**Description:** Verify the `watch` CLI command streams adapter state events.

**Preconditions:**
- UPDATE_SERVICE running.

**Input:**
- Start `parking-app-cli watch` in background, trigger InstallAdapter, wait
  for output.

**Expected:**
- Output contains at least one state event.

**Assertion pseudocode:**
```
watch_process = start_background("parking-app-cli watch --update-addr localhost:50051")
exec("parking-app-cli install --image-ref registry/adapter:v1 --checksum abc123 --update-addr localhost:50051")
wait(3s)
output = read(watch_process.stdout)
ASSERT len(output) > 0
ASSERT contains(output, "adapter_id") OR contains(output, "state")
stop(watch_process)
```

---

### TS-04-35: CLI list command shows adapters

**Requirement:** 04-REQ-9.3
**Type:** integration
**Description:** Verify the `list` CLI command shows installed adapters.

**Preconditions:**
- UPDATE_SERVICE running with at least one adapter.

**Input:**
- `parking-app-cli list --update-addr localhost:50051`

**Expected:**
- Exit code 0. Output contains adapter information.

**Assertion pseudocode:**
```
exec("parking-app-cli install --image-ref registry/adapter:v1 --checksum abc123 --update-addr localhost:50051")
result = exec("parking-app-cli list --update-addr localhost:50051")
ASSERT result.exit_code == 0
ASSERT contains(result.stdout, "adapter") OR contains(result.stdout, "registry/adapter")
```

---

### TS-04-36: CLI start-session command calls StartSession

**Requirement:** 04-REQ-9.4
**Type:** integration
**Description:** Verify the `start-session` CLI command calls adaptor.

**Preconditions:**
- PARKING_OPERATOR_ADAPTOR running. Mock PARKING_OPERATOR running.

**Input:**
- `parking-app-cli start-session --vehicle-id VIN12345 --zone-id zone-munich-central --adaptor-addr localhost:50052`

**Expected:**
- Exit code 0. Output contains `session_id` and `active` or `status`.

**Assertion pseudocode:**
```
result = exec("parking-app-cli start-session --vehicle-id VIN12345 --zone-id zone-munich-central --adaptor-addr localhost:50052")
ASSERT result.exit_code == 0
ASSERT contains(result.stdout, "session_id")
```

---

### TS-04-37: CLI stop-session command calls StopSession

**Requirement:** 04-REQ-9.5
**Type:** integration
**Description:** Verify the `stop-session` CLI command calls adaptor.

**Preconditions:**
- PARKING_OPERATOR_ADAPTOR running. Active session exists.

**Input:**
- `parking-app-cli stop-session --session-id <session_id> --adaptor-addr localhost:50052`

**Expected:**
- Exit code 0. Output contains `session_id`, `fee`, and `duration`.

**Assertion pseudocode:**
```
start_result = exec("parking-app-cli start-session --vehicle-id VIN12345 --zone-id zone-munich-central --adaptor-addr localhost:50052")
session_id = extract_session_id(start_result.stdout)
result = exec("parking-app-cli stop-session --session-id " + session_id + " --adaptor-addr localhost:50052")
ASSERT result.exit_code == 0
ASSERT contains(result.stdout, "fee")
ASSERT contains(result.stdout, "duration")
```

---

### TS-04-38: Integration — lock event to session start

**Requirement:** 04-REQ-10.1
**Type:** integration
**Description:** End-to-end test: lock event in DATA_BROKER causes
autonomous session start via mock PARKING_OPERATOR.

**Preconditions:**
- DATA_BROKER, PARKING_OPERATOR_ADAPTOR, and Mock PARKING_OPERATOR all running.

**Input:**
- Set location in DATA_BROKER. Set IsLocked = true in DATA_BROKER.

**Expected:**
- Session started on mock operator. SessionActive = true in DATA_BROKER.

**Assertion pseudocode:**
```
// Setup: set location
databroker.set("Vehicle.CurrentLocation.Latitude", 48.1351)
databroker.set("Vehicle.CurrentLocation.Longitude", 11.5820)
// Trigger
databroker.set("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", true)
// Verify
wait(5s)
ASSERT databroker.get("Vehicle.Parking.SessionActive") == true
ASSERT mock_operator.session_count() >= 1
```

---

### TS-04-39: Integration — CLI to UPDATE_SERVICE lifecycle

**Requirement:** 04-REQ-10.2
**Type:** integration
**Description:** End-to-end test: CLI install, list, and get-status against
UPDATE_SERVICE.

**Preconditions:**
- UPDATE_SERVICE running.

**Input:**
- Run `install`, then `list`, then `status` commands.

**Expected:**
- Install returns DOWNLOADING. List shows the adapter. Status shows adapter
  info.

**Assertion pseudocode:**
```
install_result = exec("parking-app-cli install --image-ref test:v1 --checksum abc --update-addr localhost:50051")
ASSERT install_result.exit_code == 0
list_result = exec("parking-app-cli list --update-addr localhost:50051")
ASSERT list_result.exit_code == 0
ASSERT contains(list_result.stdout, "test:v1") OR contains(list_result.stdout, "adapter")
```

---

### TS-04-40: Integration — adaptor to mock operator REST

**Requirement:** 04-REQ-10.3
**Type:** integration
**Description:** End-to-end test: adaptor communicates with mock operator via
REST for session management.

**Preconditions:**
- PARKING_OPERATOR_ADAPTOR and Mock PARKING_OPERATOR running.

**Input:**
- Call StartSession via gRPC on adaptor.

**Expected:**
- Mock operator receives POST /parking/start. Adaptor returns valid response.

**Assertion pseudocode:**
```
response = adaptor.StartSession({vehicle_id: "VIN12345", zone_id: "zone-munich-central"})
ASSERT response.session_id != ""
ASSERT response.status == "active"
ASSERT mock_operator.received_start_request()
```

---

## Property Test Cases

### TS-04-P1: Session State Consistency

**Property:** Property 1 from design.md
**Validates:** 04-REQ-2.3, 04-REQ-2.4
**Type:** property
**Description:** For any sequence of lock/unlock events, SessionActive in
DATA_BROKER matches the adaptor's active session state.

**For any:** Sequence S of lock/unlock events (length 1-6)
**Invariant:** After each event in S is processed, `Vehicle.Parking.SessionActive`
equals `adaptor.has_active_session()`.

**Assertion pseudocode:**
```
FOR ANY sequence IN [[lock], [lock, unlock], [lock, unlock, lock], [lock, lock]]:
    FOR event IN sequence:
        databroker.set("IsLocked", event == "lock")
        wait(3s)
        ASSERT databroker.get("SessionActive") == adaptor.has_active_session()
```

---

### TS-04-P2: Autonomous Idempotency

**Property:** Property 2 from design.md
**Validates:** 04-REQ-2.E1, 04-REQ-2.E3
**Type:** property
**Description:** Repeated lock events do not create duplicate sessions.
Repeated unlock events with no session have no effect.

**For any:** N repeated identical lock events (N >= 2)
**Invariant:** Exactly one session is created.

**Assertion pseudocode:**
```
FOR N IN [2, 3, 5]:
    FOR i IN 0..N:
        databroker.set("IsLocked", true)
        wait(1s)
    ASSERT mock_operator.start_request_count() == 1
    // Reset
    databroker.set("IsLocked", false)
    wait(3s)
    ASSERT mock_operator.stop_request_count() == 1
```

---

### TS-04-P3: Override Precedence

**Property:** Property 3 from design.md
**Validates:** 04-REQ-2.5
**Type:** property
**Description:** Manual gRPC calls override autonomous behavior and
SessionActive reflects the override.

**For any:** Manual operation M in {StartSession, StopSession}
**Invariant:** SessionActive reflects M's result regardless of lock state.

**Assertion pseudocode:**
```
// Override start while unlocked
adaptor.StartSession({vehicle_id: "VIN12345", zone_id: "zone-munich-central"})
ASSERT databroker.get("SessionActive") == true
// Override stop while still "unlocked" (no lock event)
adaptor.StopSession({session_id: "..."})
ASSERT databroker.get("SessionActive") == false
```

---

### TS-04-P4: State Machine Integrity

**Property:** Property 4 from design.md
**Validates:** 04-REQ-7.1, 04-REQ-7.2
**Type:** property
**Description:** Adapter state only transitions via allowed transitions.

**For any:** State S and target state T where (S, T) is not in the allowed
transition set
**Invariant:** `transition(S, T)` returns an error.

**Assertion pseudocode:**
```
all_states = [UNKNOWN, DOWNLOADING, INSTALLING, RUNNING, STOPPED, ERROR, OFFLOADING]
FOR ANY (from, to) IN cartesian_product(all_states, all_states):
    IF (from, to) NOT IN valid_transitions:
        result = adapter_manager.transition(from, to)
        ASSERT result == Err
```

---

### TS-04-P5: Checksum Gate

**Property:** Property 5 from design.md
**Validates:** 04-REQ-5.2, 04-REQ-5.E1
**Type:** property
**Description:** Adapter never transitions from DOWNLOADING to INSTALLING
without a checksum match.

**For any:** Manifest M with checksum C, and provided checksum P
**Invariant:** If C != P, adapter transitions to ERROR, not INSTALLING.

**Assertion pseudocode:**
```
correct_checksum = sha256(manifest)
wrong_checksum = "0000000000000000000000000000000000000000000000000000000000000000"
result_wrong = verify_and_transition(manifest, wrong_checksum)
ASSERT result_wrong.state == ERROR
result_correct = verify_and_transition(manifest, correct_checksum)
ASSERT result_correct.state == INSTALLING
```

---

### TS-04-P6: Offloading Correctness

**Property:** Property 6 from design.md
**Validates:** 04-REQ-6.1, 04-REQ-6.2
**Type:** property
**Description:** Only STOPPED adapters past the timeout are offloaded. RUNNING
adapters are never offloaded.

**For any:** Adapter A with state S and elapsed time E since last activity
**Invariant:** If S == RUNNING, A is not offloaded. If S == STOPPED and
E > timeout, A is offloaded.

**Assertion pseudocode:**
```
// RUNNING adapter should not be offloaded regardless of time
adapter_running.state = RUNNING
adapter_running.last_active = now() - 2 * timeout
ASSERT offloader.should_offload(adapter_running) == false

// STOPPED adapter past timeout should be offloaded
adapter_stopped.state = STOPPED
adapter_stopped.last_active = now() - 2 * timeout
ASSERT offloader.should_offload(adapter_stopped) == true

// STOPPED adapter within timeout should not be offloaded
adapter_recent.state = STOPPED
adapter_recent.last_active = now()
ASSERT offloader.should_offload(adapter_recent) == false
```

---

### TS-04-P7: Mock Operator Fee Accuracy

**Property:** Property 7 from design.md
**Validates:** 04-REQ-8.3
**Type:** property
**Description:** Fee calculation matches rate * duration.

**For any:** Zone Z with rate R and session duration D seconds
**Invariant:** `fee == R * (D / 3600.0)` within floating point tolerance.

**Assertion pseudocode:**
```
FOR ANY (zone, expected_rate) IN [("zone-munich-central", 2.50), ("zone-munich-west", 1.50)]:
    start = http_post("/parking/start", {vehicle_id: "V1", zone_id: zone, timestamp: now()})
    session_id = start.session_id
    sleep(2s)
    stop = http_post("/parking/stop", {session_id: session_id})
    expected_fee = expected_rate * (stop.duration_seconds / 3600.0)
    ASSERT abs(stop.fee - expected_fee) < 0.01
```

---

### TS-04-P8: Event Stream Completeness

**Property:** Property 8 from design.md
**Validates:** 04-REQ-4.3, 04-REQ-6.3
**Type:** property
**Description:** Every state transition produces an event on the watch stream.

**For any:** State transition T that occurs while a WatchAdapterStates stream
is active
**Invariant:** The stream receives an AdapterStateEvent for T.

**Assertion pseudocode:**
```
stream = update_service.WatchAdapterStates({})
events = []
// Trigger transitions: UNKNOWN -> DOWNLOADING -> INSTALLING -> RUNNING -> STOPPED
trigger_full_lifecycle()
wait(10s)
events = collect_all_events(stream)
ASSERT len(events) >= 4
states_seen = [(e.old_state, e.new_state) for e in events]
ASSERT (UNKNOWN, DOWNLOADING) IN states_seen
ASSERT (DOWNLOADING, INSTALLING) IN states_seen
ASSERT (INSTALLING, RUNNING) IN states_seen
ASSERT (RUNNING, STOPPED) IN states_seen
```

---

## Edge Case Tests

### TS-04-E1: StartSession while session already active

**Requirement:** 04-REQ-1.E1
**Type:** unit
**Description:** Verify ALREADY_EXISTS when starting a session while one is
active.

**Preconditions:**
- Active session exists.

**Input:**
- Call StartSession again.

**Expected:**
- gRPC status ALREADY_EXISTS.

**Assertion pseudocode:**
```
adaptor.StartSession({vehicle_id: "VIN12345", zone_id: "zone-munich-central"})
result = adaptor.StartSession({vehicle_id: "VIN12345", zone_id: "zone-munich-central"})
ASSERT result.status == ALREADY_EXISTS
```

---

### TS-04-E2: StopSession with unknown session_id

**Requirement:** 04-REQ-1.E2
**Type:** unit
**Description:** Verify NOT_FOUND when stopping a non-existent session.

**Preconditions:**
- No active session.

**Input:**
- `StopSessionRequest{session_id: "non-existent-id"}`

**Expected:**
- gRPC status NOT_FOUND.

**Assertion pseudocode:**
```
result = adaptor.StopSession({session_id: "non-existent-id"})
ASSERT result.status == NOT_FOUND
```

---

### TS-04-E3: PARKING_OPERATOR unreachable on StartSession

**Requirement:** 04-REQ-1.E3
**Type:** unit
**Description:** Verify UNAVAILABLE when operator is unreachable.

**Preconditions:**
- PARKING_OPERATOR_ADAPTOR running. No PARKING_OPERATOR at configured URL.

**Input:**
- Call StartSession.

**Expected:**
- gRPC status UNAVAILABLE.

**Assertion pseudocode:**
```
// Start adaptor with OPERATOR_URL pointing to non-existent service
adaptor = start_with(OPERATOR_URL="http://localhost:19999")
result = adaptor.StartSession({vehicle_id: "VIN12345", zone_id: "zone-munich-central"})
ASSERT result.status == UNAVAILABLE
```

---

### TS-04-E4: Unlock event with no active session

**Requirement:** 04-REQ-2.E1
**Type:** unit
**Description:** Verify unlock event is ignored when no session is active.

**Preconditions:**
- Adaptor running. No active session.

**Input:**
- Publish IsLocked = false to DATA_BROKER.

**Expected:**
- No call to PARKING_OPERATOR. No error.

**Assertion pseudocode:**
```
databroker.set("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", false)
wait(3s)
ASSERT mock_operator.stop_request_count() == 0
```

---

### TS-04-E5: Operator unreachable during autonomous start

**Requirement:** 04-REQ-2.E2
**Type:** integration
**Description:** Verify adaptor does not set SessionActive=true when operator
is unreachable.

**Preconditions:**
- DATA_BROKER running. PARKING_OPERATOR not running.
- Adaptor running with operator URL pointing to nothing.

**Input:**
- Set IsLocked = true in DATA_BROKER.

**Expected:**
- SessionActive remains unset (false or not present).
- Error logged.

**Assertion pseudocode:**
```
// Adaptor started with OPERATOR_URL pointing to nowhere
databroker.set("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", true)
wait(5s)
ASSERT databroker.get("Vehicle.Parking.SessionActive") != true
```

---

### TS-04-E6: Duplicate lock event ignored

**Requirement:** 04-REQ-2.E3
**Type:** unit
**Description:** Verify duplicate lock event does not create a second session.

**Preconditions:**
- Active session exists (started via first lock event).

**Input:**
- Publish IsLocked = true again.

**Expected:**
- No additional session started on PARKING_OPERATOR.

**Assertion pseudocode:**
```
databroker.set("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", true)
wait(3s)
count_before = mock_operator.start_request_count()
databroker.set("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", true)
wait(3s)
ASSERT mock_operator.start_request_count() == count_before
```

---

### TS-04-E7: DATA_BROKER unreachable at startup

**Requirement:** 04-REQ-3.E1
**Type:** integration
**Description:** Verify adaptor retries DATA_BROKER connection with
exponential backoff.

**Preconditions:**
- DATA_BROKER not running.

**Input:**
- Start adaptor with DATA_BROKER address pointing to unavailable port.

**Expected:**
- Adaptor logs retry attempts. Does not crash.

**Assertion pseudocode:**
```
process = start("parking-operator-adaptor", env={"DATABROKER_ADDR": "localhost:19999"})
wait(5s)
ASSERT process.is_running() == true
ASSERT contains(process.stderr, "retry") OR contains(process.stderr, "reconnect")
stop(process)
```

---

### TS-04-E8: InstallAdapter for already-installed adapter

**Requirement:** 04-REQ-4.E1
**Type:** unit
**Description:** Verify ALREADY_EXISTS when installing an already-running
adapter.

**Preconditions:**
- Adapter already installed and running.

**Input:**
- Call InstallAdapter with same image_ref.

**Expected:**
- gRPC status ALREADY_EXISTS.

**Assertion pseudocode:**
```
update_service.InstallAdapter({image_ref: "registry/adapter:v1", checksum_sha256: "abc123"})
result = update_service.InstallAdapter({image_ref: "registry/adapter:v1", checksum_sha256: "abc123"})
ASSERT result.status == ALREADY_EXISTS
```

---

### TS-04-E9: RemoveAdapter with unknown adapter_id

**Requirement:** 04-REQ-4.E2
**Type:** unit
**Description:** Verify NOT_FOUND when removing a non-existent adapter.

**Preconditions:**
- No such adapter installed.

**Input:**
- `RemoveAdapterRequest{adapter_id: "non-existent"}`

**Expected:**
- gRPC status NOT_FOUND.

**Assertion pseudocode:**
```
result = update_service.RemoveAdapter({adapter_id: "non-existent"})
ASSERT result.status == NOT_FOUND
```

---

### TS-04-E10: Container start failure transitions to ERROR

**Requirement:** 04-REQ-4.E3
**Type:** unit
**Description:** Verify adapter transitions to ERROR when container fails to
start.

**Preconditions:**
- UPDATE_SERVICE running. Container runtime configured to fail on start.

**Input:**
- Install adapter with an image that fails to start.

**Expected:**
- Adapter state transitions to ERROR.

**Assertion pseudocode:**
```
install_resp = update_service.InstallAdapter({image_ref: "bad-image:v1", checksum_sha256: "..."})
wait_for_state(install_resp.adapter_id, ERROR, timeout=30s)
status = update_service.GetAdapterStatus({adapter_id: install_resp.adapter_id})
ASSERT status.adapter.state == ADAPTER_STATE_ERROR
```

---

### TS-04-E11: Checksum mismatch transitions to ERROR

**Requirement:** 04-REQ-5.E1
**Type:** unit
**Description:** Verify checksum mismatch causes ERROR state and discards
image.

**Preconditions:**
- Registry with valid image.

**Input:**
- Install with wrong checksum.

**Expected:**
- Adapter transitions to ERROR. Image discarded.

**Assertion pseudocode:**
```
install_resp = update_service.InstallAdapter({
    image_ref: "registry/adapter:v1",
    checksum_sha256: "wrong_checksum_value"
})
wait_for_state(install_resp.adapter_id, ERROR, timeout=30s)
status = update_service.GetAdapterStatus({adapter_id: install_resp.adapter_id})
ASSERT status.adapter.state == ADAPTER_STATE_ERROR
```

---

### TS-04-E12: Registry unreachable during pull

**Requirement:** 04-REQ-5.E2
**Type:** unit
**Description:** Verify ERROR state when registry is unreachable.

**Preconditions:**
- No registry at configured URL.

**Input:**
- Install adapter with unreachable registry URL.

**Expected:**
- Adapter transitions to ERROR.

**Assertion pseudocode:**
```
install_resp = update_service.InstallAdapter({
    image_ref: "unreachable-registry.example.com/adapter:v1",
    checksum_sha256: "abc123"
})
wait_for_state(install_resp.adapter_id, ERROR, timeout=30s)
status = update_service.GetAdapterStatus({adapter_id: install_resp.adapter_id})
ASSERT status.adapter.state == ADAPTER_STATE_ERROR
```

---

### TS-04-E13: Re-install during OFFLOADING cancels offload

**Requirement:** 04-REQ-6.E1
**Type:** unit
**Description:** Verify re-install during offloading cancels the offload and
re-downloads.

**Preconditions:**
- Adapter in OFFLOADING state.

**Input:**
- Call InstallAdapter for the same image during offloading.

**Expected:**
- Offload cancelled. Adapter transitions back to DOWNLOADING.

**Assertion pseudocode:**
```
// Adapter is in OFFLOADING state
install_resp = update_service.InstallAdapter({
    image_ref: same_image_ref,
    checksum_sha256: correct_checksum
})
ASSERT install_resp.state == ADAPTER_STATE_DOWNLOADING
wait(3s)
status = update_service.GetAdapterStatus({adapter_id: install_resp.adapter_id})
ASSERT status.adapter.state != ADAPTER_STATE_OFFLOADING
```

---

### TS-04-E14: Mock operator stop unknown session

**Requirement:** 04-REQ-8.E1
**Type:** unit
**Description:** Verify HTTP 404 when stopping a non-existent session.

**Preconditions:**
- Mock operator running.

**Input:**
- `POST /parking/stop` with unknown session_id.

**Expected:**
- HTTP 404.

**Assertion pseudocode:**
```
response = http_post("http://localhost:8090/parking/stop", {session_id: "non-existent"})
ASSERT response.status_code == 404
```

---

### TS-04-E15: Mock operator status unknown session

**Requirement:** 04-REQ-8.E2
**Type:** unit
**Description:** Verify HTTP 404 when querying status of non-existent session.

**Preconditions:**
- Mock operator running.

**Input:**
- `GET /parking/non-existent/status`

**Expected:**
- HTTP 404.

**Assertion pseudocode:**
```
response = http_get("http://localhost:8090/parking/non-existent/status")
ASSERT response.status_code == 404
```

---

### TS-04-E16: Mock operator rate for unknown zone

**Requirement:** 04-REQ-8.E3
**Type:** unit
**Description:** Verify HTTP 404 when querying rate for unknown zone.

**Preconditions:**
- Mock operator running.

**Input:**
- `GET /rate/unknown-zone`

**Expected:**
- HTTP 404.

**Assertion pseudocode:**
```
response = http_get("http://localhost:8090/rate/unknown-zone")
ASSERT response.status_code == 404
```

---

### TS-04-E17: CLI error on unreachable service

**Requirement:** 04-REQ-9.E1
**Type:** integration
**Description:** Verify CLI prints error and exits non-zero when target service
is unreachable.

**Preconditions:**
- No services running.

**Input:**
- `parking-app-cli install --image-ref test:v1 --checksum abc --update-addr localhost:19999`

**Expected:**
- Non-zero exit code. Error message includes target address.

**Assertion pseudocode:**
```
result = exec("parking-app-cli install --image-ref test:v1 --checksum abc --update-addr localhost:19999")
ASSERT result.exit_code != 0
ASSERT contains(result.stderr, "localhost:19999") OR contains(result.stderr, "connection")
```

---

## Coverage Matrix

| Requirement    | Test Spec Entry | Type        |
|----------------|-----------------|-------------|
| 04-REQ-1.1     | TS-04-1         | integration |
| 04-REQ-1.2     | TS-04-2         | unit        |
| 04-REQ-1.3     | TS-04-3         | unit        |
| 04-REQ-1.4     | TS-04-4         | unit        |
| 04-REQ-1.5     | TS-04-5         | unit        |
| 04-REQ-1.E1    | TS-04-E1        | unit        |
| 04-REQ-1.E2    | TS-04-E2        | unit        |
| 04-REQ-1.E3    | TS-04-E3        | unit        |
| 04-REQ-2.1     | TS-04-6         | integration |
| 04-REQ-2.2     | TS-04-7         | integration |
| 04-REQ-2.3     | TS-04-8         | integration |
| 04-REQ-2.4     | TS-04-9         | integration |
| 04-REQ-2.5     | TS-04-10        | unit        |
| 04-REQ-2.E1    | TS-04-E4        | unit        |
| 04-REQ-2.E2    | TS-04-E5        | integration |
| 04-REQ-2.E3    | TS-04-E6        | unit        |
| 04-REQ-3.1     | TS-04-11        | integration |
| 04-REQ-3.2     | TS-04-12        | integration |
| 04-REQ-3.3     | TS-04-13        | integration |
| 04-REQ-3.4     | TS-04-14        | integration |
| 04-REQ-3.E1    | TS-04-E7        | integration |
| 04-REQ-4.1     | TS-04-15        | integration |
| 04-REQ-4.2     | TS-04-16        | unit        |
| 04-REQ-4.3     | TS-04-17        | unit        |
| 04-REQ-4.4     | TS-04-18        | unit        |
| 04-REQ-4.5     | TS-04-19        | unit        |
| 04-REQ-4.6     | TS-04-20        | unit        |
| 04-REQ-4.E1    | TS-04-E8        | unit        |
| 04-REQ-4.E2    | TS-04-E9        | unit        |
| 04-REQ-4.E3    | TS-04-E10       | unit        |
| 04-REQ-5.1     | TS-04-21        | integration |
| 04-REQ-5.2     | TS-04-22        | unit        |
| 04-REQ-5.3     | TS-04-23        | unit        |
| 04-REQ-5.E1    | TS-04-E11       | unit        |
| 04-REQ-5.E2    | TS-04-E12       | unit        |
| 04-REQ-6.1     | TS-04-24        | unit        |
| 04-REQ-6.2     | TS-04-25        | unit        |
| 04-REQ-6.3     | TS-04-26        | unit        |
| 04-REQ-6.E1    | TS-04-E13       | unit        |
| 04-REQ-7.1     | TS-04-27        | unit        |
| 04-REQ-7.2     | TS-04-28        | unit        |
| 04-REQ-8.1     | TS-04-29        | unit        |
| 04-REQ-8.2     | TS-04-29        | unit        |
| 04-REQ-8.3     | TS-04-30        | unit        |
| 04-REQ-8.4     | TS-04-31        | unit        |
| 04-REQ-8.5     | TS-04-32        | unit        |
| 04-REQ-8.E1    | TS-04-E14       | unit        |
| 04-REQ-8.E2    | TS-04-E15       | unit        |
| 04-REQ-8.E3    | TS-04-E16       | unit        |
| 04-REQ-9.1     | TS-04-33        | integration |
| 04-REQ-9.2     | TS-04-34        | integration |
| 04-REQ-9.3     | TS-04-35        | integration |
| 04-REQ-9.4     | TS-04-36        | integration |
| 04-REQ-9.5     | TS-04-37        | integration |
| 04-REQ-9.E1    | TS-04-E17       | integration |
| 04-REQ-10.1    | TS-04-38        | integration |
| 04-REQ-10.2    | TS-04-39        | integration |
| 04-REQ-10.3    | TS-04-40        | integration |
| Property 1     | TS-04-P1        | property    |
| Property 2     | TS-04-P2        | property    |
| Property 3     | TS-04-P3        | property    |
| Property 4     | TS-04-P4        | property    |
| Property 5     | TS-04-P5        | property    |
| Property 6     | TS-04-P6        | property    |
| Property 7     | TS-04-P7        | property    |
| Property 8     | TS-04-P8        | property    |
