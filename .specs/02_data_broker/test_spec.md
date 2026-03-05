# Test Specification: DATA_BROKER (Spec 02)

## Overview

This test specification defines concrete test contracts for the DATA_BROKER deployment and configuration. Tests validate that Eclipse Kuksa Databroker is correctly deployed with proper signal registration, dual-interface availability, and signal read/write/subscribe operations. Tests connect to the DATA_BROKER via gRPC and verify behavior against the requirements and correctness properties.

Test IDs follow three categories:
- **TS-02-N** -- Normal / happy path tests
- **TS-02-PN** -- Property-based / parameterized tests
- **TS-02-EN** -- Error / edge case tests

## Test Cases

### TS-02-1: Databroker Starts and Accepts Connections

**Requirement:** 02-REQ-1.1, 02-REQ-1.2, 02-REQ-8.1
**Type:** integration
**Description:** Verify that the Kuksa Databroker starts and accepts gRPC connections within the expected timeframe.

**Preconditions:**
- Local infrastructure started via `make infra-up`

**Input:**
- gRPC connection attempt to `localhost:55556`

**Expected:**
- gRPC connection succeeds
- A metadata or health check query returns a successful response
- Connection is established within 30 seconds of infrastructure start

**Assertion pseudocode:**
```
conn = grpc.connect("localhost:55556", timeout=30s)
ASSERT conn.is_connected()
response = conn.list_metadata("Vehicle.**")
ASSERT response.status == OK
```

### TS-02-2: Standard VSS Signals Are Registered

**Requirement:** 02-REQ-3.1
**Type:** integration
**Description:** Verify that all 5 standard VSS v5.1 signals are registered with correct data types.

**Preconditions:**
- DATA_BROKER is healthy (TS-02-1 passes)

**Input:**
- Metadata queries for each standard signal path

**Expected:**
- Each signal path exists in the DATA_BROKER signal tree
- Data types match: IsLocked=bool, IsOpen=bool, Latitude=double, Longitude=double, Speed=float

**Assertion pseudocode:**
```
signals = {
  "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked": "bool",
  "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen": "bool",
  "Vehicle.CurrentLocation.Latitude": "double",
  "Vehicle.CurrentLocation.Longitude": "double",
  "Vehicle.Speed": "float"
}
FOR EACH (path, expected_type) IN signals:
  metadata = conn.get_metadata(path)
  ASSERT metadata.exists == true
  ASSERT metadata.data_type == expected_type
```

### TS-02-3: Custom VSS Signals Are Registered

**Requirement:** 02-REQ-2.1, 02-REQ-2.2
**Type:** integration
**Description:** Verify that all 3 custom overlay signals are registered with correct data types alongside standard signals.

**Preconditions:**
- DATA_BROKER is healthy (TS-02-1 passes)

**Input:**
- Metadata queries for each custom signal path

**Expected:**
- Each custom signal path exists in the DATA_BROKER signal tree
- Data types match: SessionActive=boolean, Lock=string, Response=string
- Standard signals remain accessible (no conflicts from overlay)

**Assertion pseudocode:**
```
custom_signals = {
  "Vehicle.Parking.SessionActive": "boolean",
  "Vehicle.Command.Door.Lock": "string",
  "Vehicle.Command.Door.Response": "string"
}
FOR EACH (path, expected_type) IN custom_signals:
  metadata = conn.get_metadata(path)
  ASSERT metadata.exists == true
  ASSERT metadata.data_type == expected_type

// Verify standard signals still accessible after overlay
std_metadata = conn.get_metadata("Vehicle.Speed")
ASSERT std_metadata.exists == true
```

### TS-02-4: Cross-Partition Network Access on Port 55556

**Requirement:** 02-REQ-5.1, 02-REQ-5.2
**Type:** integration
**Description:** Verify that the DATA_BROKER is accessible via network TCP on port 55556 for cross-partition consumers.

**Preconditions:**
- DATA_BROKER is healthy (TS-02-1 passes)

**Input:**
- gRPC connection from test host to `localhost:55556`
- Signal operations (metadata query, write, read)

**Expected:**
- Connection succeeds from outside the container
- Signal operations work correctly over the network connection

**Assertion pseudocode:**
```
conn = grpc.connect("localhost:55556")
ASSERT conn.is_connected()

// Write and read a signal
conn.set("Vehicle.Speed", 42.0)
result = conn.get("Vehicle.Speed")
ASSERT result.value == 42.0
```

### TS-02-5: UDS Listener Accepts Connections

**Requirement:** 02-REQ-4.1, 02-REQ-4.2
**Type:** integration
**Description:** Verify that the DATA_BROKER exposes a UDS endpoint and accepts same-partition gRPC connections.

**Preconditions:**
- DATA_BROKER is healthy (TS-02-1 passes)
- UDS socket path is accessible from the test environment

**Input:**
- gRPC connection to UDS path `/tmp/kuksa/databroker.sock`

**Expected:**
- Connection succeeds
- Signal operations (get, set, subscribe) work via UDS

**Assertion pseudocode:**
```
conn = grpc.connect("unix:///tmp/kuksa/databroker.sock")
ASSERT conn.is_connected()

conn.set("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", true)
result = conn.get("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked")
ASSERT result.value == true
```

## Property Test Cases

### TS-02-P1: Signal Write/Read Round-Trip for All Types

**Property:** Property 2 from design.md
**Validates:** 02-REQ-6.1, 02-REQ-6.2
**Type:** property
**Description:** Verify that values written to signals can be read back correctly for each data type.

**For any:** signal in the registered tree and a value of the correct type
**Invariant:** A get after a set returns the most recently written value.

**Test Parameters:**

| Signal Path | Write Value | Data Type |
|-------------|------------|-----------|
| `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` | `true` | bool |
| `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` | `false` | bool |
| `Vehicle.CurrentLocation.Latitude` | `48.1351` | double |
| `Vehicle.CurrentLocation.Longitude` | `11.5820` | double |
| `Vehicle.Speed` | `0.0` | float |
| `Vehicle.Parking.SessionActive` | `true` | boolean |
| `Vehicle.Command.Door.Lock` | `{"command_id":"test-uuid","action":"lock"}` | string |
| `Vehicle.Command.Door.Response` | `{"command_id":"test-uuid","status":"success"}` | string |

**Assertion pseudocode:**
```
FOR EACH (path, value, type) IN test_parameters:
  conn.set(path, value)
  result = conn.get(path)
  ASSERT result.value == value
  ASSERT result.data_type == type
```

### TS-02-P2: Subscription Delivers Updates to Subscribers

**Property:** Property 3 from design.md
**Validates:** 02-REQ-7.1, 02-REQ-7.2
**Type:** property
**Description:** Verify that subscribers receive value updates when a signal is written to.

**For any:** active subscription on a signal and a new value written to that signal
**Invariant:** The subscriber receives the updated value on the subscription stream.

**Test Parameters:**

| Signal Path | Value 1 | Value 2 |
|-------------|---------|---------|
| `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` | `false` | `true` |
| `Vehicle.Parking.SessionActive` | `false` | `true` |

**Assertion pseudocode:**
```
FOR EACH (path, val1, val2) IN test_parameters:
  subscriber = conn_a.subscribe(path)
  conn_b.set(path, val1)
  update1 = subscriber.next(timeout=5s)
  ASSERT update1.value == val1

  conn_b.set(path, val2)
  update2 = subscriber.next(timeout=5s)
  ASSERT update2.value == val2
```

### TS-02-P3: Overlay Merge Preserves Standard Signals

**Property:** Property 6 from design.md
**Validates:** 02-REQ-2.2, 02-REQ-3.1
**Type:** property
**Description:** Verify that custom overlay signals coexist with standard VSS signals without conflicts.

**For any:** standard signal path and custom signal path after overlay merge
**Invariant:** Both standard and custom signals are independently accessible and writable.

**Assertion pseudocode:**
```
// Write to a custom signal
conn.set("Vehicle.Parking.SessionActive", true)
custom = conn.get("Vehicle.Parking.SessionActive")
ASSERT custom.value == true

// Verify standard signal is still independently accessible
conn.set("Vehicle.Speed", 50.0)
standard = conn.get("Vehicle.Speed")
ASSERT standard.value == 50.0

// Both exist in metadata
all_metadata = conn.list_metadata("Vehicle.**")
ASSERT "Vehicle.Parking.SessionActive" IN all_metadata
ASSERT "Vehicle.Speed" IN all_metadata
```

## Edge Case Tests

### TS-02-E1: Non-Existent Signal Path Returns Error

**Requirement:** 02-REQ-6.E1
**Type:** integration
**Description:** Verify that accessing a non-existent signal path returns an appropriate gRPC error.

**Preconditions:**
- DATA_BROKER is healthy (TS-02-1 passes)

**Input:**
- Get and set attempts on `Vehicle.NonExistent.Signal`

**Expected:**
- gRPC NOT_FOUND error (or equivalent Kuksa error indicating signal does not exist)

**Assertion pseudocode:**
```
err_get = conn.get("Vehicle.NonExistent.Signal")
ASSERT err_get.status == NOT_FOUND

err_set = conn.set("Vehicle.NonExistent.Signal", "value")
ASSERT err_set.status == NOT_FOUND
```

### TS-02-E2: Unset Signal Returns No Current Value

**Requirement:** 02-REQ-3.2
**Type:** integration
**Description:** Verify that reading a signal that has never been written returns metadata without a current value.

**Preconditions:**
- DATA_BROKER freshly started (no prior writes to target signal)

**Input:**
- Get request for `Vehicle.Parking.SessionActive` before any writes

**Expected:**
- Response indicates signal exists (metadata present)
- Response indicates no current value (value field is empty/null/not-yet-set)

**Assertion pseudocode:**
```
result = conn.get("Vehicle.Parking.SessionActive")
ASSERT result.metadata.exists == true
ASSERT result.value == NOT_SET
```

### TS-02-E3: Type Mismatch on Write Returns Error

**Requirement:** 02-REQ-6.E2
**Type:** integration
**Description:** Verify that writing a value with an incompatible type is rejected.

**Preconditions:**
- DATA_BROKER is healthy (TS-02-1 passes)

**Input:**
- Write string value `"not_a_boolean"` to `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` (bool signal)

**Expected:**
- gRPC INVALID_ARGUMENT error (or equivalent Kuksa type mismatch error)

**Assertion pseudocode:**
```
err = conn.set("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", "not_a_boolean")
ASSERT err.status == INVALID_ARGUMENT
```

> **Note:** Some Kuksa Databroker versions may coerce types silently. The test should document observed behavior and pass if either strict rejection or documented coercion occurs.

### TS-02-E4: Health Check Reports Not Ready During Startup

**Requirement:** 02-REQ-8.2
**Type:** integration
**Description:** Verify that the health check mechanism reports not-ready before the DATA_BROKER is fully initialized.

**Preconditions:**
- Infrastructure is being started (DATA_BROKER not yet healthy)

**Input:**
- Health check query immediately after start

**Expected:**
- Health check reports unhealthy/not-ready until initialization completes

**Assertion pseudocode:**
```
// Start infrastructure
start_infra()

// Immediate check may fail
initial = health_check("localhost:55556")
// Either UNAVAILABLE or NOT_SERVING is acceptable before ready

// Eventually becomes healthy
ASSERT wait_for_healthy("localhost:55556", timeout=30s) == true
```

## Coverage Matrix

| Requirement | Test Spec Entry | Type |
|-------------|-----------------|------|
| 02-REQ-1.1 | TS-02-1 | integration |
| 02-REQ-1.2 | TS-02-1 | integration |
| 02-REQ-1.E1 | TS-02-E4 | integration |
| 02-REQ-2.1 | TS-02-3 | integration |
| 02-REQ-2.2 | TS-02-3, TS-02-P3 | integration, property |
| 02-REQ-2.E1 | (validated by startup failure; not separately testable in integration) | -- |
| 02-REQ-3.1 | TS-02-2 | integration |
| 02-REQ-3.2 | TS-02-E2 | integration |
| 02-REQ-4.1 | TS-02-5 | integration |
| 02-REQ-4.2 | TS-02-5 | integration |
| 02-REQ-4.E1 | (UDS error path; validated by connection failure to bad path) | -- |
| 02-REQ-5.1 | TS-02-4 | integration |
| 02-REQ-5.2 | TS-02-4 | integration |
| 02-REQ-5.E1 | TS-02-E4 | integration |
| 02-REQ-6.1 | TS-02-P1 | property |
| 02-REQ-6.2 | TS-02-P1 | property |
| 02-REQ-6.E1 | TS-02-E1 | integration |
| 02-REQ-6.E2 | TS-02-E3 | integration |
| 02-REQ-7.1 | TS-02-P2 | property |
| 02-REQ-7.2 | TS-02-P2 | property |
| 02-REQ-7.E1 | (stream termination; validated implicitly by subscription tests) | -- |
| 02-REQ-8.1 | TS-02-1 | integration |
| 02-REQ-8.2 | TS-02-E4 | integration |
| Property 1 | TS-02-2, TS-02-3 | integration |
| Property 2 | TS-02-P1 | property |
| Property 3 | TS-02-P2 | property |
| Property 4 | TS-02-E3 | integration |
| Property 5 | TS-02-4, TS-02-5 | integration |
| Property 6 | TS-02-P3 | property |
