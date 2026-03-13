# Test Specification: DATA_BROKER

## Overview

All tests for the DATA_BROKER spec are integration tests that require a running Kuksa Databroker container. Tests verify compose.yml configuration (dual listeners, version pinning), VSS signal availability (custom and standard), and pub/sub functionality via gRPC. Tests live in `tests/databroker/` as a standalone Go module and use the `kuksa.val.v1` gRPC API or `grpcurl` CLI.

Tests that require Podman skip gracefully when Podman is not available.

## Test Cases

### TS-02-1: TCP Listener Configuration

**Requirement:** 02-REQ-1.1
**Type:** integration
**Description:** Verify the databroker compose service configures a TCP listener on 0.0.0.0:55555 mapped to host port 55556.

**Preconditions:**
- `deployments/compose.yml` exists.

**Input:**
- Parse compose.yml and inspect the databroker service definition.

**Expected:**
- Port mapping `55556:55555` is present.
- Command args include `--address` and `0.0.0.0:55555`.

**Assertion pseudocode:**
```
config = parse_yaml("deployments/compose.yml")
svc = config.services.databroker
ASSERT "55556:55555" IN svc.ports
ASSERT "--address" IN svc.command
ASSERT "0.0.0.0:55555" IN svc.command
```

### TS-02-2: UDS Listener Configuration

**Requirement:** 02-REQ-1.2
**Type:** integration
**Description:** Verify the databroker compose service configures a UDS listener at /tmp/kuksa-databroker.sock.

**Preconditions:**
- `deployments/compose.yml` exists.

**Input:**
- Parse compose.yml and inspect the databroker service command.

**Expected:**
- Command args include `--uds-path` and `/tmp/kuksa-databroker.sock`.

**Assertion pseudocode:**
```
config = parse_yaml("deployments/compose.yml")
svc = config.services.databroker
ASSERT "--uds-path" IN svc.command
ASSERT "/tmp/kuksa-databroker.sock" IN svc.command
```

### TS-02-3: UDS Volume Mount

**Requirement:** 02-REQ-1.3
**Type:** integration
**Description:** Verify the databroker service exposes the UDS socket directory to the host via a volume mount.

**Preconditions:**
- `deployments/compose.yml` exists.

**Input:**
- Parse compose.yml and inspect volumes.

**Expected:**
- A volume mount maps a host path to `/tmp` in the container for UDS socket access.

**Assertion pseudocode:**
```
config = parse_yaml("deployments/compose.yml")
svc = config.services.databroker
ASSERT any volume in svc.volumes maps to "/tmp" in container
```

### TS-02-4: Dual Listener Connectivity

**Requirement:** 02-REQ-1.4
**Type:** integration
**Description:** Verify the running databroker accepts gRPC connections on both TCP and UDS.

**Preconditions:**
- Databroker container is running via `podman compose up -d`.
- Podman is available (skip if not).

**Input:**
- Connect to `localhost:55556` via TCP gRPC.
- Connect to `/tmp/kuksa/kuksa-databroker.sock` via UDS gRPC.

**Expected:**
- Both connections succeed and respond to a gRPC health or metadata call.

**Assertion pseudocode:**
```
start_databroker()
tcp_conn = grpc_connect("localhost:55556")
ASSERT tcp_conn.get_metadata("Vehicle.Speed") succeeds
uds_conn = grpc_connect("unix:///tmp/kuksa/kuksa-databroker.sock")
ASSERT uds_conn.get_metadata("Vehicle.Speed") succeeds
stop_databroker()
```

### TS-02-5: Image Version Pinning

**Requirement:** 02-REQ-2.1
**Type:** unit
**Description:** Verify the compose.yml uses a pinned image version, not latest.

**Preconditions:**
- `deployments/compose.yml` exists.

**Input:**
- Parse compose.yml and read the databroker image field.

**Expected:**
- Image is `ghcr.io/eclipse-kuksa/kuksa-databroker:0.5.1`.

**Assertion pseudocode:**
```
config = parse_yaml("deployments/compose.yml")
ASSERT config.services.databroker.image == "ghcr.io/eclipse-kuksa/kuksa-databroker:0.5.1"
```

### TS-02-6: Custom Signal SessionActive

**Requirement:** 02-REQ-3.1
**Type:** integration
**Description:** Verify Vehicle.Parking.SessionActive is exposed with boolean datatype.

**Preconditions:**
- Databroker container is running.

**Input:**
- gRPC GetMetadata for `Vehicle.Parking.SessionActive`.

**Expected:**
- Signal exists with datatype boolean.

**Assertion pseudocode:**
```
metadata = grpc_get_metadata("Vehicle.Parking.SessionActive")
ASSERT metadata.exists == true
ASSERT metadata.datatype == BOOLEAN
```

### TS-02-7: Custom Signal Door Lock Command

**Requirement:** 02-REQ-3.2
**Type:** integration
**Description:** Verify Vehicle.Command.Door.Lock is exposed with string datatype.

**Preconditions:**
- Databroker container is running.

**Input:**
- gRPC GetMetadata for `Vehicle.Command.Door.Lock`.

**Expected:**
- Signal exists with datatype string.

**Assertion pseudocode:**
```
metadata = grpc_get_metadata("Vehicle.Command.Door.Lock")
ASSERT metadata.exists == true
ASSERT metadata.datatype == STRING
```

### TS-02-8: Custom Signal Door Response

**Requirement:** 02-REQ-3.3
**Type:** integration
**Description:** Verify Vehicle.Command.Door.Response is exposed with string datatype.

**Preconditions:**
- Databroker container is running.

**Input:**
- gRPC GetMetadata for `Vehicle.Command.Door.Response`.

**Expected:**
- Signal exists with datatype string.

**Assertion pseudocode:**
```
metadata = grpc_get_metadata("Vehicle.Command.Door.Response")
ASSERT metadata.exists == true
ASSERT metadata.datatype == STRING
```

### TS-02-9: Custom Signal Set/Get Roundtrip

**Requirement:** 02-REQ-3.4
**Type:** integration
**Description:** Verify a custom signal value can be set and read back.

**Preconditions:**
- Databroker container is running.

**Input:**
- Set `Vehicle.Parking.SessionActive` to `true` via gRPC.
- Get `Vehicle.Parking.SessionActive` via gRPC.

**Expected:**
- Returned value is `true`.

**Assertion pseudocode:**
```
grpc_set("Vehicle.Parking.SessionActive", true)
result = grpc_get("Vehicle.Parking.SessionActive")
ASSERT result.value == true
```

### TS-02-10: Standard Signal IsLocked

**Requirement:** 02-REQ-4.1
**Type:** integration
**Description:** Verify Vehicle.Cabin.Door.Row1.DriverSide.IsLocked is available.

**Preconditions:**
- Databroker container is running.

**Input:**
- gRPC GetMetadata for `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked`.

**Expected:**
- Signal exists with datatype boolean.

**Assertion pseudocode:**
```
metadata = grpc_get_metadata("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked")
ASSERT metadata.exists == true
ASSERT metadata.datatype == BOOLEAN
```

### TS-02-11: Standard Signal IsOpen

**Requirement:** 02-REQ-4.2
**Type:** integration
**Description:** Verify Vehicle.Cabin.Door.Row1.DriverSide.IsOpen is available.

**Preconditions:**
- Databroker container is running.

**Input:**
- gRPC GetMetadata for `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen`.

**Expected:**
- Signal exists with datatype boolean.

**Assertion pseudocode:**
```
metadata = grpc_get_metadata("Vehicle.Cabin.Door.Row1.DriverSide.IsOpen")
ASSERT metadata.exists == true
ASSERT metadata.datatype == BOOLEAN
```

### TS-02-12: Standard Signal Latitude

**Requirement:** 02-REQ-4.3
**Type:** integration
**Description:** Verify Vehicle.CurrentLocation.Latitude is available.

**Preconditions:**
- Databroker container is running.

**Input:**
- gRPC GetMetadata for `Vehicle.CurrentLocation.Latitude`.

**Expected:**
- Signal exists with datatype double.

**Assertion pseudocode:**
```
metadata = grpc_get_metadata("Vehicle.CurrentLocation.Latitude")
ASSERT metadata.exists == true
ASSERT metadata.datatype == DOUBLE
```

### TS-02-13: Standard Signal Longitude

**Requirement:** 02-REQ-4.4
**Type:** integration
**Description:** Verify Vehicle.CurrentLocation.Longitude is available.

**Preconditions:**
- Databroker container is running.

**Input:**
- gRPC GetMetadata for `Vehicle.CurrentLocation.Longitude`.

**Expected:**
- Signal exists with datatype double.

**Assertion pseudocode:**
```
metadata = grpc_get_metadata("Vehicle.CurrentLocation.Longitude")
ASSERT metadata.exists == true
ASSERT metadata.datatype == DOUBLE
```

### TS-02-14: Standard Signal Speed

**Requirement:** 02-REQ-4.5
**Type:** integration
**Description:** Verify Vehicle.Speed is available.

**Preconditions:**
- Databroker container is running.

**Input:**
- gRPC GetMetadata for `Vehicle.Speed`.

**Expected:**
- Signal exists with datatype float.

**Assertion pseudocode:**
```
metadata = grpc_get_metadata("Vehicle.Speed")
ASSERT metadata.exists == true
ASSERT metadata.datatype == FLOAT
```

### TS-02-15: Pub/Sub Notification

**Requirement:** 02-REQ-5.1
**Type:** integration
**Description:** Verify that a subscriber receives a notification when a signal value is set.

**Preconditions:**
- Databroker container is running.

**Input:**
- Client A subscribes to `Vehicle.Parking.SessionActive`.
- Client B sets `Vehicle.Parking.SessionActive` to `true`.

**Expected:**
- Client A receives a notification with value `true`.

**Assertion pseudocode:**
```
subscription = grpc_subscribe("Vehicle.Parking.SessionActive")
grpc_set("Vehicle.Parking.SessionActive", true)
notification = subscription.receive(timeout=5s)
ASSERT notification.value == true
```

### TS-02-16: Boolean Set/Get Roundtrip

**Requirement:** 02-REQ-5.2
**Type:** integration
**Description:** Verify boolean signal roundtrip integrity.

**Preconditions:**
- Databroker container is running.

**Input:**
- Set `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` to `true`.
- Get `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked`.

**Expected:**
- Returned value is `true`.

**Assertion pseudocode:**
```
grpc_set("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", true)
result = grpc_get("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked")
ASSERT result.value == true
```

### TS-02-17: String/JSON Set/Get Roundtrip

**Requirement:** 02-REQ-5.3
**Type:** integration
**Description:** Verify string signal roundtrip with JSON payload.

**Preconditions:**
- Databroker container is running.

**Input:**
- Set `Vehicle.Command.Door.Lock` to `{"command_id":"abc-123","action":"lock","doors":["driver"],"source":"companion_app","vin":"WDB123","timestamp":1700000000}`.
- Get `Vehicle.Command.Door.Lock`.

**Expected:**
- Returned value is the exact same JSON string.

**Assertion pseudocode:**
```
payload = '{"command_id":"abc-123","action":"lock","doors":["driver"],"source":"companion_app","vin":"WDB123","timestamp":1700000000}'
grpc_set("Vehicle.Command.Door.Lock", payload)
result = grpc_get("Vehicle.Command.Door.Lock")
ASSERT result.value == payload
```

## Edge Case Tests

### TS-02-E1: UDS Socket Overwrite on Restart

**Requirement:** 02-REQ-1.E1
**Type:** integration
**Description:** Verify the databroker starts successfully even if the UDS socket file exists from a previous run.

**Preconditions:**
- Databroker container was previously running and stopped (socket file may persist).

**Input:**
- Start the databroker container again.

**Expected:**
- Container starts without error.

**Assertion pseudocode:**
```
start_databroker()
stop_databroker()
start_databroker()
ASSERT databroker_is_healthy()
stop_databroker()
```

### TS-02-E2: Concurrent TCP and UDS Clients

**Requirement:** 02-REQ-1.E2
**Type:** integration
**Description:** Verify TCP and UDS clients can operate simultaneously.

**Preconditions:**
- Databroker container is running.

**Input:**
- TCP client sets `Vehicle.Speed` to 50.0.
- UDS client reads `Vehicle.Speed`.

**Expected:**
- UDS client reads 50.0.

**Assertion pseudocode:**
```
tcp_conn = grpc_connect("localhost:55556")
uds_conn = grpc_connect("unix:///tmp/kuksa/kuksa-databroker.sock")
tcp_conn.set("Vehicle.Speed", 50.0)
result = uds_conn.get("Vehicle.Speed")
ASSERT result.value == 50.0
```

### TS-02-E3: Malformed VSS Overlay

**Requirement:** 02-REQ-3.E1
**Type:** integration
**Description:** Verify the databroker fails to start with malformed overlay JSON.

**Preconditions:**
- A temporary malformed overlay file is created.

**Input:**
- Start databroker with a malformed JSON overlay file.

**Expected:**
- Container exits with non-zero code or fails to reach healthy state.

**Assertion pseudocode:**
```
write_file("deployments/vss-overlay-bad.json", "{invalid json")
start_databroker_with_overlay("vss-overlay-bad.json")
ASSERT container_exit_code != 0 OR NOT databroker_is_healthy()
cleanup("deployments/vss-overlay-bad.json")
```

### TS-02-E4: Get Unset Custom Signal

**Requirement:** 02-REQ-3.E2
**Type:** integration
**Description:** Verify getting a custom signal that has never been set returns no value (not an error).

**Preconditions:**
- Databroker container is freshly started (no values set).

**Input:**
- gRPC Get for `Vehicle.Parking.SessionActive` without prior Set.

**Expected:**
- Response succeeds (no error) but contains no value.

**Assertion pseudocode:**
```
result = grpc_get("Vehicle.Parking.SessionActive")
ASSERT result.error == nil
ASSERT result.has_value == false
```

### TS-02-E5: Query Non-Existent Signal

**Requirement:** 02-REQ-4.E1
**Type:** integration
**Description:** Verify querying a non-existent signal path returns NOT_FOUND.

**Preconditions:**
- Databroker container is running.

**Input:**
- gRPC Get for `Vehicle.NonExistent.Signal`.

**Expected:**
- Response contains a NOT_FOUND error.

**Assertion pseudocode:**
```
result = grpc_get("Vehicle.NonExistent.Signal")
ASSERT result.error == NOT_FOUND
```

### TS-02-E6: Subscriber Reconnect

**Requirement:** 02-REQ-5.E1
**Type:** integration
**Description:** Verify a subscriber can disconnect and reconnect without error.

**Preconditions:**
- Databroker container is running.

**Input:**
- Subscribe to `Vehicle.Parking.SessionActive`, then cancel subscription.
- Subscribe again to the same signal.
- Set the signal value.

**Expected:**
- Second subscription receives the notification.

**Assertion pseudocode:**
```
sub1 = grpc_subscribe("Vehicle.Parking.SessionActive")
sub1.cancel()
sub2 = grpc_subscribe("Vehicle.Parking.SessionActive")
grpc_set("Vehicle.Parking.SessionActive", true)
notification = sub2.receive(timeout=5s)
ASSERT notification.value == true
```

## Property Test Cases

### TS-02-P1: Dual Listener Availability

**Property:** Property 1 from design.md
**Validates:** 02-REQ-1.1, 02-REQ-1.2, 02-REQ-1.4
**Type:** property
**Description:** Both TCP and UDS listeners accept gRPC connections simultaneously.

**For any:** Running databroker instance (single instance per test run)
**Invariant:** A gRPC metadata query succeeds on both TCP (localhost:55556) and UDS (/tmp/kuksa/kuksa-databroker.sock).

**Assertion pseudocode:**
```
FOR ANY signal IN all_catalog_signals:
    tcp_result = tcp_conn.get_metadata(signal)
    uds_result = uds_conn.get_metadata(signal)
    ASSERT tcp_result.exists == uds_result.exists
    ASSERT tcp_result.datatype == uds_result.datatype
```

### TS-02-P2: Custom Signal Completeness

**Property:** Property 2 from design.md
**Validates:** 02-REQ-3.1, 02-REQ-3.2, 02-REQ-3.3
**Type:** property
**Description:** All custom signals from the overlay are exposed with correct datatypes.

**For any:** Custom signal in the set {Vehicle.Parking.SessionActive, Vehicle.Command.Door.Lock, Vehicle.Command.Door.Response}
**Invariant:** The signal exists in the databroker metadata with the expected datatype.

**Assertion pseudocode:**
```
expected = {
    "Vehicle.Parking.SessionActive": BOOLEAN,
    "Vehicle.Command.Door.Lock": STRING,
    "Vehicle.Command.Door.Response": STRING,
}
FOR ANY (signal, dtype) IN expected:
    metadata = grpc_get_metadata(signal)
    ASSERT metadata.exists == true
    ASSERT metadata.datatype == dtype
```

### TS-02-P3: Standard Signal Availability

**Property:** Property 3 from design.md
**Validates:** 02-REQ-4.1, 02-REQ-4.2, 02-REQ-4.3, 02-REQ-4.4, 02-REQ-4.5
**Type:** property
**Description:** All standard VSS signals from the catalog are present with correct datatypes.

**For any:** Standard signal in the catalog (5 signals)
**Invariant:** The signal exists in the databroker metadata with the expected datatype.

**Assertion pseudocode:**
```
expected = {
    "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked": BOOLEAN,
    "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen": BOOLEAN,
    "Vehicle.CurrentLocation.Latitude": DOUBLE,
    "Vehicle.CurrentLocation.Longitude": DOUBLE,
    "Vehicle.Speed": FLOAT,
}
FOR ANY (signal, dtype) IN expected:
    metadata = grpc_get_metadata(signal)
    ASSERT metadata.exists == true
    ASSERT metadata.datatype == dtype
```

### TS-02-P4: Set/Get Roundtrip Integrity

**Property:** Property 4 from design.md
**Validates:** 02-REQ-3.4, 02-REQ-5.2, 02-REQ-5.3
**Type:** property
**Description:** Setting a signal value and reading it back returns the identical value.

**For any:** Signal in the catalog and a valid value for that signal's datatype
**Invariant:** get(signal) == value after set(signal, value)

**Assertion pseudocode:**
```
test_values = {
    "Vehicle.Parking.SessionActive": [true, false],
    "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked": [true, false],
    "Vehicle.Speed": [0.0, 50.5, 130.0],
    "Vehicle.Command.Door.Lock": ['{"command_id":"x","action":"lock"}', '{"command_id":"y","action":"unlock"}'],
}
FOR ANY (signal, values) IN test_values:
    FOR ANY value IN values:
        grpc_set(signal, value)
        result = grpc_get(signal)
        ASSERT result.value == value
```

### TS-02-P5: Pub/Sub Notification Delivery

**Property:** Property 5 from design.md
**Validates:** 02-REQ-5.1
**Type:** property
**Description:** Active subscribers receive notifications when signal values change.

**For any:** Signal with an active subscription and a new value set
**Invariant:** The subscriber receives a notification containing the new value.

**Assertion pseudocode:**
```
FOR ANY signal IN ["Vehicle.Parking.SessionActive", "Vehicle.Speed"]:
    sub = grpc_subscribe(signal)
    FOR ANY value IN test_values[signal]:
        grpc_set(signal, value)
        notification = sub.receive(timeout=5s)
        ASSERT notification.value == value
    sub.cancel()
```

## Coverage Matrix

| Requirement | Test Spec Entry | Type |
|-------------|-----------------|------|
| 02-REQ-1.1 | TS-02-1 | integration |
| 02-REQ-1.2 | TS-02-2 | integration |
| 02-REQ-1.3 | TS-02-3 | integration |
| 02-REQ-1.4 | TS-02-4 | integration |
| 02-REQ-1.E1 | TS-02-E1 | integration |
| 02-REQ-1.E2 | TS-02-E2 | integration |
| 02-REQ-2.1 | TS-02-5 | unit |
| 02-REQ-2.E1 | (runtime — covered by Podman pull behavior) | — |
| 02-REQ-3.1 | TS-02-6 | integration |
| 02-REQ-3.2 | TS-02-7 | integration |
| 02-REQ-3.3 | TS-02-8 | integration |
| 02-REQ-3.4 | TS-02-9 | integration |
| 02-REQ-3.E1 | TS-02-E3 | integration |
| 02-REQ-3.E2 | TS-02-E4 | integration |
| 02-REQ-4.1 | TS-02-10 | integration |
| 02-REQ-4.2 | TS-02-11 | integration |
| 02-REQ-4.3 | TS-02-12 | integration |
| 02-REQ-4.4 | TS-02-13 | integration |
| 02-REQ-4.5 | TS-02-14 | integration |
| 02-REQ-4.E1 | TS-02-E5 | integration |
| 02-REQ-5.1 | TS-02-15 | integration |
| 02-REQ-5.2 | TS-02-16 | integration |
| 02-REQ-5.3 | TS-02-17 | integration |
| 02-REQ-5.E1 | TS-02-E6 | integration |
| Property 1 | TS-02-P1 | property |
| Property 2 | TS-02-P2 | property |
| Property 3 | TS-02-P3 | property |
| Property 4 | TS-02-P4 | property |
| Property 5 | TS-02-P5 | property |
