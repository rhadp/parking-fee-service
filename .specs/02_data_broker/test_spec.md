# Test Specification: DATA_BROKER (Spec 02)

> Test specifications for Eclipse Kuksa Databroker deployment and configuration validation.

## References

- Requirements: `.specs/02_data_broker/requirements.md`
- Design: `.specs/02_data_broker/design.md`

## Test Environment

- **Infrastructure:** Local docker-compose environment started via `make infra-up`
- **Test runner:** Go test framework
- **Test location:** `tests/setup/`
- **Test command:** `cd tests/setup && go test -run TestDataBroker -v`
- **DATA_BROKER endpoint:** `localhost:55556` (gRPC over TCP)
- **Prerequisite:** `01_project_setup` infrastructure is operational

## Test Categories

- **TS-02-N** -- Normal / happy path tests
- **TS-02-P** -- Property-based / parameterized tests
- **TS-02-E** -- Error / edge case tests

---

## TS-02-1: Databroker Container Health

**Objective:** Verify that the Kuksa Databroker container starts and reaches a healthy state.

**Traces to:** 02-REQ-1.1, 02-REQ-1.2

**Preconditions:**
- docker-compose infrastructure is started via `make infra-up`

**Steps:**
1. Query the docker-compose service status for the `databroker` container.
2. Establish a gRPC connection to `localhost:55556`.
3. Perform a gRPC health check or list metadata call.

**Expected Results:**
- The `databroker` container is in a running and healthy state.
- The gRPC connection to `localhost:55556` succeeds.
- The health check or metadata query returns a successful response.

**Timeout:** 30 seconds for the container to become healthy.

---

## TS-02-2: Standard VSS Signals Are Registered

**Objective:** Verify that all 5 standard VSS v5.1 signals are registered and queryable.

**Traces to:** 02-REQ-2.1

**Preconditions:**
- DATA_BROKER is healthy (TS-02-1 passes)

**Steps:**
1. Connect to DATA_BROKER via gRPC at `localhost:55556`.
2. For each standard signal, query its metadata (e.g., via `GetMetadata` or equivalent Kuksa API):
   - `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked`
   - `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen`
   - `Vehicle.CurrentLocation.Latitude`
   - `Vehicle.CurrentLocation.Longitude`
   - `Vehicle.Speed`

**Expected Results:**
- Each signal path exists in the DATA_BROKER signal tree.
- Each signal reports the correct data type:
  - `IsLocked`: bool
  - `IsOpen`: bool
  - `Latitude`: double
  - `Longitude`: double
  - `Speed`: float

---

## TS-02-3: Custom Signals Are Registered with Correct Types

**Objective:** Verify that all 3 custom overlay signals are registered with the correct data types.

**Traces to:** 02-REQ-3.1

**Preconditions:**
- DATA_BROKER is healthy (TS-02-1 passes)

**Steps:**
1. Connect to DATA_BROKER via gRPC at `localhost:55556`.
2. For each custom signal, query its metadata:
   - `Vehicle.Parking.SessionActive`
   - `Vehicle.Command.Door.Lock`
   - `Vehicle.Command.Door.Response`

**Expected Results:**
- Each custom signal path exists in the DATA_BROKER signal tree.
- Each signal reports the correct data type:
  - `SessionActive`: bool
  - `Lock`: string
  - `Response`: string

---

## TS-02-P1: Signal Write and Read Round-Trip

**Objective:** Verify that values written to signals can be read back correctly for each data type.

**Traces to:** 02-REQ-4.2, 02-REQ-5.2, 02-REQ-6.2, CP-2

**Preconditions:**
- DATA_BROKER is healthy (TS-02-1 passes)

**Test Parameters:**

| Signal Path | Write Value | Data Type |
|-------------|------------|-----------|
| `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` | `true` | bool |
| `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` | `false` | bool |
| `Vehicle.CurrentLocation.Latitude` | `48.1351` | double |
| `Vehicle.CurrentLocation.Longitude` | `11.5820` | double |
| `Vehicle.Speed` | `0.0` | float |
| `Vehicle.Parking.SessionActive` | `true` | bool |
| `Vehicle.Command.Door.Lock` | `{"command_id":"test-uuid","action":"lock"}` | string |
| `Vehicle.Command.Door.Response` | `{"command_id":"test-uuid","status":"success"}` | string |

**Steps (for each parameter row):**
1. Connect to DATA_BROKER via gRPC at `localhost:55556`.
2. Write the specified value to the signal path using `SetRequest` (or equivalent Kuksa API).
3. Read the signal value back using `GetRequest` (or equivalent Kuksa API).

**Expected Results:**
- The write operation succeeds without error.
- The read operation returns the exact value that was written.
- The returned data type matches the signal's registered type.

---

## TS-02-P2: Pub/Sub Subscription Delivers Updates

**Objective:** Verify that subscribers receive value updates when a signal is written to.

**Traces to:** 02-REQ-6.3, CP-3

**Preconditions:**
- DATA_BROKER is healthy (TS-02-1 passes)

**Test Parameters:**

| Signal Path | Value 1 | Value 2 |
|-------------|---------|---------|
| `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` | `false` | `true` |
| `Vehicle.Parking.SessionActive` | `false` | `true` |

**Steps (for each parameter row):**
1. Connect to DATA_BROKER via gRPC at `localhost:55556` (two connections: subscriber and writer).
2. On the subscriber connection, create a subscription (via `Subscribe` or equivalent Kuksa API) for the signal path.
3. On the writer connection, write Value 1 to the signal path.
4. Verify the subscriber receives Value 1 on the subscription stream.
5. On the writer connection, write Value 2 to the signal path.
6. Verify the subscriber receives Value 2 on the subscription stream.

**Expected Results:**
- The subscriber receives both value updates in order.
- The received values match exactly what was written.

**Timeout:** 5 seconds for each subscription update to arrive.

---

## TS-02-4: Cross-Partition Network Access on Port 55556

**Objective:** Verify that the DATA_BROKER is accessible via network TCP on port 55556, simulating cross-partition access.

**Traces to:** 02-REQ-5.1, 02-REQ-5.2, CP-5

**Preconditions:**
- DATA_BROKER is healthy (TS-02-1 passes)

**Steps:**
1. From the test host (outside the docker container), establish a gRPC connection to `localhost:55556`.
2. List or query metadata for at least one standard signal and one custom signal.
3. Write a value to a signal.
4. Read the value back.

**Expected Results:**
- The gRPC connection to `localhost:55556` succeeds from the test host.
- Signal metadata queries return valid results.
- Write and read operations work correctly over the network connection.

---

## TS-02-E1: Non-Existent Signal Path Returns Error

**Objective:** Verify that accessing a non-existent signal path returns an appropriate error.

**Traces to:** 02-REQ-3.2, 02-EDGE-1

**Preconditions:**
- DATA_BROKER is healthy (TS-02-1 passes)

**Steps:**
1. Connect to DATA_BROKER via gRPC at `localhost:55556`.
2. Attempt to read (get) a non-existent signal path: `Vehicle.NonExistent.Signal`.
3. Attempt to write (set) a value to a non-existent signal path: `Vehicle.NonExistent.Signal`.

**Expected Results:**
- The get request returns a gRPC NOT_FOUND error (or the Kuksa equivalent indicating the signal does not exist).
- The set request returns a gRPC NOT_FOUND error (or the Kuksa equivalent).

---

## TS-02-E2: Unset Signal Returns No Current Value

**Objective:** Verify that reading a signal that has never been written returns metadata without a current value.

**Traces to:** 02-REQ-2.2, 02-EDGE-2

**Preconditions:**
- DATA_BROKER is freshly started (no prior writes to the target signal)

**Steps:**
1. Connect to DATA_BROKER via gRPC at `localhost:55556`.
2. Read the value of `Vehicle.Parking.SessionActive` (a signal that has not been written to since startup).

**Expected Results:**
- The response indicates the signal exists (metadata is present).
- The response indicates no current value is set (value field is empty, null, or equivalent not-yet-set indicator).

---

## TS-02-E3: Type Mismatch on Write Returns Error

**Objective:** Verify that writing a value with an incompatible type is rejected.

**Traces to:** 02-EDGE-5, CP-4

**Preconditions:**
- DATA_BROKER is healthy (TS-02-1 passes)

**Steps:**
1. Connect to DATA_BROKER via gRPC at `localhost:55556`.
2. Attempt to write a string value `"not_a_boolean"` to `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` (a bool signal).

**Expected Results:**
- The write operation fails with a gRPC INVALID_ARGUMENT error (or equivalent Kuksa type mismatch error).
- The signal's value remains unchanged (if previously set) or unset (if never written).

> **Note:** The exact error behavior depends on the Kuksa Databroker version. Some versions may coerce types silently. The test should document the observed behavior and pass if either strict rejection or documented coercion occurs.

---

## Test Summary Matrix

| Test ID | Description | Requirements Traced | Category |
|---------|-------------|-------------------|----------|
| TS-02-1 | Databroker container health | 02-REQ-1.1, 02-REQ-1.2 | Normal |
| TS-02-2 | Standard VSS signals registered | 02-REQ-2.1 | Normal |
| TS-02-3 | Custom signals registered with correct types | 02-REQ-3.1 | Normal |
| TS-02-P1 | Signal write/read round-trip | 02-REQ-4.2, 02-REQ-5.2, 02-REQ-6.2 | Property |
| TS-02-P2 | Pub/sub subscription delivers updates | 02-REQ-6.3 | Property |
| TS-02-4 | Cross-partition network access | 02-REQ-5.1, 02-REQ-5.2 | Normal |
| TS-02-E1 | Non-existent signal path error | 02-REQ-3.2 | Error |
| TS-02-E2 | Unset signal returns no value | 02-REQ-2.2 | Error |
| TS-02-E3 | Type mismatch on write | 02-EDGE-5 | Error |
