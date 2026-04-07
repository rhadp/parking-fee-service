# Test Specification: DATA_BROKER (Spec 02)

## Overview

This test specification defines the integration, property, edge case, and smoke tests for the DATA_BROKER component. Since the DATA_BROKER is a pre-built container (Eclipse Kuksa Databroker 0.5.1) with no custom application code, all verification is performed through integration tests that exercise the gRPC API over TCP and UDS transports. Tests are implemented in Go under `tests/databroker/`.

## Test Cases

### TS-02-1: TCP connectivity

- **Requirement:** 02-REQ-2
- **Type:** Integration
- **Description:** Verify that a gRPC client can establish a connection to the DATA_BROKER via TCP on host port 55556.
- **Preconditions:** DATA_BROKER container is running via `podman compose up kuksa-databroker`.
- **Input:** gRPC channel open to `localhost:55556`.
- **Expected:** Channel connects successfully; a simple metadata or get request returns a valid gRPC response (not UNAVAILABLE).
- **Assertion pseudocode:**
  ```
  conn = grpc.Dial("localhost:55556")
  resp = conn.GetMetadata("Vehicle.Speed")
  assert resp.status == OK
  ```

### TS-02-2: UDS connectivity

- **Requirement:** 02-REQ-3
- **Type:** Integration
- **Description:** Verify that a gRPC client can establish a connection to the DATA_BROKER via UDS.
- **Preconditions:** DATA_BROKER container is running; UDS socket volume is mounted.
- **Input:** gRPC channel open to `unix:///tmp/kuksa-databroker.sock`.
- **Expected:** Channel connects successfully; a simple metadata or get request returns a valid gRPC response.
- **Assertion pseudocode:**
  ```
  conn = grpc.Dial("unix:///tmp/kuksa-databroker.sock")
  resp = conn.GetMetadata("Vehicle.Speed")
  assert resp.status == OK
  ```

### TS-02-3: Pinned image version

- **Requirement:** 02-REQ-1
- **Type:** Integration
- **Description:** Verify that the running DATA_BROKER container uses the pinned image version 0.5.1.
- **Preconditions:** DATA_BROKER container is running.
- **Input:** Inspect the running container image reference.
- **Expected:** Image reference matches `ghcr.io/eclipse-kuksa/kuksa-databroker:0.5.1`.
- **Assertion pseudocode:**
  ```
  image = podman_inspect("kuksa-databroker").image
  assert image contains "kuksa-databroker:0.5.1"
  ```

### TS-02-4: Standard VSS signal metadata

- **Requirement:** 02-REQ-5
- **Type:** Integration
- **Description:** Verify that all 5 standard VSS v5.1 signals are present in the DATA_BROKER metadata with correct types.
- **Preconditions:** DATA_BROKER container is running.
- **Input:** GetMetadata requests for each standard signal.
- **Expected:** Each signal returns valid metadata with the correct data type.
- **Assertion pseudocode:**
  ```
  expected = {
    "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked": BOOL,
    "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen": BOOL,
    "Vehicle.CurrentLocation.Latitude": DOUBLE,
    "Vehicle.CurrentLocation.Longitude": DOUBLE,
    "Vehicle.Speed": FLOAT
  }
  for signal, type in expected:
    resp = conn.GetMetadata(signal)
    assert resp.status == OK
    assert resp.dataType == type
  ```

### TS-02-5: Custom VSS signal metadata

- **Requirement:** 02-REQ-6
- **Type:** Integration
- **Description:** Verify that all 3 custom VSS signals from the overlay are present with correct types.
- **Preconditions:** DATA_BROKER container is running with overlay loaded.
- **Input:** GetMetadata requests for each custom signal.
- **Expected:** Each custom signal returns valid metadata with the correct data type.
- **Assertion pseudocode:**
  ```
  expected = {
    "Vehicle.Parking.SessionActive": BOOL,
    "Vehicle.Command.Door.Lock": STRING,
    "Vehicle.Command.Door.Response": STRING
  }
  for signal, type in expected:
    resp = conn.GetMetadata(signal)
    assert resp.status == OK
    assert resp.dataType == type
  ```

### TS-02-6: Signal set/get via TCP

- **Requirement:** 02-REQ-8
- **Type:** Integration
- **Description:** Verify that signals can be set and retrieved via the TCP gRPC interface.
- **Preconditions:** DATA_BROKER container is running; TCP connection established.
- **Input:** Set multiple signals of different types via TCP, then get each.
- **Expected:** Each get returns the value that was set.
- **Assertion pseudocode:**
  ```
  tcp = grpc.Dial("localhost:55556")
  tcp.Set("Vehicle.Speed", 50.0)
  assert tcp.Get("Vehicle.Speed").value == 50.0

  tcp.Set("Vehicle.Parking.SessionActive", true)
  assert tcp.Get("Vehicle.Parking.SessionActive").value == true

  json = '{"command_id":"abc","action":"lock"}'
  tcp.Set("Vehicle.Command.Door.Lock", json)
  assert tcp.Get("Vehicle.Command.Door.Lock").value == json
  ```

### TS-02-7: Signal set/get via UDS

- **Requirement:** 02-REQ-9
- **Type:** Integration
- **Description:** Verify that signals can be set and retrieved via the UDS gRPC interface.
- **Preconditions:** DATA_BROKER container is running; UDS connection established.
- **Input:** Set signals via UDS, then get each.
- **Expected:** Each get returns the value that was set.
- **Assertion pseudocode:**
  ```
  uds = grpc.Dial("unix:///tmp/kuksa-databroker.sock")
  uds.Set("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", true)
  assert uds.Get("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked").value == true

  uds.Set("Vehicle.CurrentLocation.Latitude", 48.1351)
  assert uds.Get("Vehicle.CurrentLocation.Latitude").value == 48.1351
  ```

### TS-02-8: Cross-transport consistency (TCP write, UDS read)

- **Requirement:** 02-REQ-4, 02-REQ-9
- **Type:** Integration
- **Description:** Verify that a signal written via TCP is readable via UDS.
- **Preconditions:** DATA_BROKER running; both TCP and UDS connections established.
- **Input:** Set a signal via TCP, read via UDS.
- **Expected:** UDS read returns the value written via TCP.
- **Assertion pseudocode:**
  ```
  tcp = grpc.Dial("localhost:55556")
  uds = grpc.Dial("unix:///tmp/kuksa-databroker.sock")
  tcp.Set("Vehicle.Speed", 75.5)
  assert uds.Get("Vehicle.Speed").value == 75.5
  ```

### TS-02-9: Cross-transport consistency (UDS write, TCP read)

- **Requirement:** 02-REQ-4, 02-REQ-9
- **Type:** Integration
- **Description:** Verify that a signal written via UDS is readable via TCP.
- **Preconditions:** DATA_BROKER running; both TCP and UDS connections established.
- **Input:** Set a signal via UDS, read via TCP.
- **Expected:** TCP read returns the value written via UDS.
- **Assertion pseudocode:**
  ```
  tcp = grpc.Dial("localhost:55556")
  uds = grpc.Dial("unix:///tmp/kuksa-databroker.sock")
  uds.Set("Vehicle.Parking.SessionActive", true)
  assert tcp.Get("Vehicle.Parking.SessionActive").value == true
  ```

### TS-02-10: Signal subscription via TCP

- **Requirement:** 02-REQ-10
- **Type:** Integration
- **Description:** Verify that a TCP subscriber receives notifications when a signal value changes.
- **Preconditions:** DATA_BROKER running; TCP connection established.
- **Input:** Subscribe to a signal via TCP, then set the signal from another client.
- **Expected:** Subscriber receives the updated value in the subscription stream.
- **Assertion pseudocode:**
  ```
  tcp1 = grpc.Dial("localhost:55556")
  tcp2 = grpc.Dial("localhost:55556")
  stream = tcp1.Subscribe("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked")
  tcp2.Set("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", true)
  update = stream.Recv(timeout=5s)
  assert update.value == true
  ```

### TS-02-11: Signal subscription cross-transport

- **Requirement:** 02-REQ-10, 02-REQ-4
- **Type:** Integration
- **Description:** Verify that a UDS subscriber receives notifications when a signal is set via TCP.
- **Preconditions:** DATA_BROKER running; both connections established.
- **Input:** Subscribe via UDS, set via TCP.
- **Expected:** UDS subscriber receives the update.
- **Assertion pseudocode:**
  ```
  uds = grpc.Dial("unix:///tmp/kuksa-databroker.sock")
  tcp = grpc.Dial("localhost:55556")
  stream = uds.Subscribe("Vehicle.Parking.SessionActive")
  tcp.Set("Vehicle.Parking.SessionActive", true)
  update = stream.Recv(timeout=5s)
  assert update.value == true
  ```

### TS-02-12: Permissive mode (no auth required)

- **Requirement:** 02-REQ-7
- **Type:** Integration
- **Description:** Verify that the DATA_BROKER accepts requests without authorization tokens.
- **Preconditions:** DATA_BROKER running in permissive mode.
- **Input:** gRPC request with no authorization metadata.
- **Expected:** Request succeeds (no PERMISSION_DENIED error).
- **Assertion pseudocode:**
  ```
  conn = grpc.Dial("localhost:55556", no_credentials)
  resp = conn.Set("Vehicle.Speed", 10.0)
  assert resp.status == OK
  ```

## Property Test Cases

### TS-02-P1: Signal completeness property

- **Requirement:** 02-REQ-5, 02-REQ-6
- **Type:** Property
- **Description:** For every signal in the expected set of 8, metadata is present and type-correct.
- **Preconditions:** DATA_BROKER running with overlay loaded.
- **Input:** Complete list of 8 expected signals with their types.
- **Expected:** All 8 signals found, zero missing.
- **Assertion pseudocode:**
  ```
  all_signals = [5 standard + 3 custom]
  for signal in all_signals:
    resp = conn.GetMetadata(signal.path)
    assert resp.status == OK
    assert resp.dataType == signal.expected_type
  assert found_count == 8
  ```

### TS-02-P2: Write-read roundtrip property

- **Requirement:** 02-REQ-8, 02-REQ-9
- **Type:** Property
- **Description:** For any signal, setting a value and immediately getting it returns the same value.
- **Preconditions:** DATA_BROKER running.
- **Input:** Set/get pairs for each signal type (bool, float, double, string).
- **Expected:** Get value equals set value for every signal.
- **Assertion pseudocode:**
  ```
  test_values = {
    BOOL: [true, false],
    FLOAT: [0.0, 50.0, 999.9],
    DOUBLE: [48.1351, -122.4194],
    STRING: ['{"command_id":"x"}', '{}']
  }
  for signal in all_signals:
    for val in test_values[signal.type]:
      conn.Set(signal.path, val)
      assert conn.Get(signal.path).value == val
  ```

### TS-02-P3: Cross-transport equivalence property

- **Requirement:** 02-REQ-4, 02-REQ-9
- **Type:** Property
- **Description:** For any signal, the value read via TCP equals the value read via UDS after a write on either transport.
- **Preconditions:** DATA_BROKER running; both connections available.
- **Input:** Write via TCP, read via both; write via UDS, read via both.
- **Expected:** All reads return identical values.
- **Assertion pseudocode:**
  ```
  for signal in all_signals:
    tcp.Set(signal.path, test_value)
    assert tcp.Get(signal.path).value == uds.Get(signal.path).value
    uds.Set(signal.path, other_value)
    assert tcp.Get(signal.path).value == uds.Get(signal.path).value
  ```

### TS-02-P4: Subscription delivery

- **Requirement:** 02-REQ-10
- **Type:** Property
- **Description:** For any active subscription on a signal, a value change is delivered exactly once.
- **Preconditions:** DATA_BROKER running; subscription established on target signal.
- **Input:** Subscribe to a signal, then set the signal to a new value.
- **Expected:** Subscriber receives exactly one update matching the set value.
- **Assertion pseudocode:**
  ```
  for signal in all_signals:
    stream = conn.Subscribe(signal.path)
    conn.Set(signal.path, test_value)
    update = stream.Recv(timeout=5s)
    assert update.value == test_value
    no_more = stream.Recv(timeout=1s)
    assert no_more == TIMEOUT
  ```

## Edge Case Tests

### TS-02-E1: Set non-existent signal

- **Requirement:** 02-REQ-8.E1
- **Type:** Edge case
- **Description:** Verify that setting a non-existent signal returns an appropriate error.
- **Preconditions:** DATA_BROKER running.
- **Input:** Set request for `Vehicle.NonExistent.Signal`.
- **Expected:** gRPC NOT_FOUND or equivalent error status.
- **Assertion pseudocode:**
  ```
  resp = conn.Set("Vehicle.NonExistent.Signal", 42)
  assert resp.status == NOT_FOUND
  ```

### TS-02-E2: Overlay syntax error

- **Requirement:** 02-REQ-6.E1
- **Type:** Edge case
- **Description:** Verify that the DATA_BROKER fails to start when the overlay file has a syntax error.
- **Preconditions:** Overlay file replaced with malformed content.
- **Input:** Start DATA_BROKER container with invalid overlay.
- **Expected:** Container exits with non-zero status; logs contain parse error.
- **Assertion pseudocode:**
  ```
  replace_overlay_with_invalid_content()
  exit_code = podman_compose_up("kuksa-databroker")
  assert exit_code != 0
  logs = podman_logs("kuksa-databroker")
  assert "error" in logs.lower() or "parse" in logs.lower()
  restore_valid_overlay()
  ```

### TS-02-E3: Missing overlay file

- **Requirement:** 02-REQ-6.E2
- **Type:** Edge case
- **Description:** Verify that the DATA_BROKER fails to start when the overlay file is missing.
- **Preconditions:** Overlay file removed or path misconfigured.
- **Input:** Start DATA_BROKER container with missing overlay path.
- **Expected:** Container exits with non-zero status.
- **Assertion pseudocode:**
  ```
  rename_overlay_to_backup()
  exit_code = podman_compose_up("kuksa-databroker")
  assert exit_code != 0
  restore_overlay_from_backup()
  ```

### TS-02-E4: Permissive mode with arbitrary token

- **Requirement:** 02-REQ-7.E1
- **Type:** Edge case
- **Description:** Verify that the DATA_BROKER accepts requests even when an invalid token is provided.
- **Preconditions:** DATA_BROKER running in permissive mode.
- **Input:** gRPC request with `Authorization: Bearer invalid-token-12345` metadata.
- **Expected:** Request succeeds (not rejected).
- **Assertion pseudocode:**
  ```
  conn = grpc.Dial("localhost:55556", metadata={"authorization": "Bearer invalid-token"})
  resp = conn.Get("Vehicle.Speed")
  assert resp.status == OK
  ```

## Integration Smoke Tests

### TS-02-SMOKE-1: Databroker health check

- **Requirement:** 02-REQ-1, 02-REQ-2
- **Type:** Smoke
- **Description:** Quick verification that the DATA_BROKER container starts and accepts TCP connections.
- **Preconditions:** None (starts container itself).
- **Input:** `podman compose up -d kuksa-databroker`, then gRPC connect to `localhost:55556`.
- **Expected:** Connection succeeds within 10 seconds of container start.
- **Assertion pseudocode:**
  ```
  podman_compose_up("kuksa-databroker", detached=true)
  wait_for_port(55556, timeout=10s)
  conn = grpc.Dial("localhost:55556")
  resp = conn.GetMetadata("Vehicle.Speed")
  assert resp.status == OK
  podman_compose_down()
  ```

### TS-02-SMOKE-2: Full signal inventory check

- **Requirement:** 02-REQ-5, 02-REQ-6
- **Type:** Smoke
- **Description:** Quick verification that all 8 signals are present after startup.
- **Preconditions:** DATA_BROKER container running.
- **Input:** Metadata queries for all 8 signals.
- **Expected:** All 8 return valid metadata.
- **Assertion pseudocode:**
  ```
  signals = [
    "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked",
    "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen",
    "Vehicle.CurrentLocation.Latitude",
    "Vehicle.CurrentLocation.Longitude",
    "Vehicle.Speed",
    "Vehicle.Parking.SessionActive",
    "Vehicle.Command.Door.Lock",
    "Vehicle.Command.Door.Response"
  ]
  for s in signals:
    assert conn.GetMetadata(s).status == OK
  ```

## Coverage Matrix

| Requirement | Acceptance Tests | Property Tests | Edge Case Tests | Smoke Tests |
|------------|-----------------|----------------|-----------------|-------------|
| 02-REQ-1.1 | TS-02-3 | | | TS-02-SMOKE-1 |
| 02-REQ-1.2 | TS-02-3 | | | TS-02-SMOKE-1 |
| 02-REQ-1.E1 | | | | |
| 02-REQ-2.1 | TS-02-1 | | | TS-02-SMOKE-1 |
| 02-REQ-2.2 | TS-02-1 | | | TS-02-SMOKE-1 |
| 02-REQ-2.E1 | | | | |
| 02-REQ-3.1 | TS-02-2 | | | |
| 02-REQ-3.2 | TS-02-2 | | | |
| 02-REQ-3.E1 | | | | |
| 02-REQ-3.E2 | | | | |
| 02-REQ-4.1 | TS-02-8, TS-02-9, TS-02-11 | TS-02-P3 | | |
| 02-REQ-4.E1 | | | | |
| 02-REQ-5.1 | TS-02-4 | TS-02-P1 | | TS-02-SMOKE-2 |
| 02-REQ-5.2 | TS-02-4 | TS-02-P1 | | TS-02-SMOKE-2 |
| 02-REQ-5.E1 | | | | |
| 02-REQ-6.1 | TS-02-5 | TS-02-P1 | | TS-02-SMOKE-2 |
| 02-REQ-6.2 | TS-02-5 | TS-02-P1 | | TS-02-SMOKE-2 |
| 02-REQ-6.3 | TS-02-5 | TS-02-P1 | | TS-02-SMOKE-2 |
| 02-REQ-6.4 | TS-02-5 | TS-02-P1 | | TS-02-SMOKE-2 |
| 02-REQ-6.E1 | | | TS-02-E2 | |
| 02-REQ-6.E2 | | | TS-02-E3 | |
| 02-REQ-7.1 | TS-02-12 | | | |
| 02-REQ-7.E1 | | | TS-02-E4 | |
| 02-REQ-8.1 | TS-02-6 | TS-02-P2 | | |
| 02-REQ-8.2 | TS-02-6 | TS-02-P2 | | |
| 02-REQ-8.E1 | | | TS-02-E1 | |
| 02-REQ-9.1 | TS-02-7 | TS-02-P2, TS-02-P3 | | |
| 02-REQ-9.2 | TS-02-8, TS-02-9 | TS-02-P3 | | |
| 02-REQ-9.E1 | | | | |
| 02-REQ-10.1 | TS-02-10, TS-02-11 | TS-02-P4 | | |
| 02-REQ-10.E1 | | | | |
