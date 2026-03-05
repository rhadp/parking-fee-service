# Test Specification: PARKING_OPERATOR_ADAPTOR (Spec 08)

> Test specifications for the PARKING_OPERATOR_ADAPTOR and mock PARKING_OPERATOR.
> Derived from `.specs/08_parking_operator_adaptor/requirements.md` and `.specs/08_parking_operator_adaptor/design.md`.

## Test ID Convention

- **TS-08-N**: Functional tests (happy path)
- **TS-08-PN**: Property / state machine tests
- **TS-08-EN**: Error and edge case tests

## Test Infrastructure

All integration tests require:
- DATA_BROKER (Kuksa Databroker) running on port 55556
- Mock PARKING_OPERATOR running on port 8080
- PARKING_OPERATOR_ADAPTOR running on port 50052

Start infrastructure: `make infra-up`

## Functional Tests

### TS-08-1: Lock Event Triggers Session Start with Operator

**Traces to:** 08-REQ-1.1

**Preconditions:**
- DATA_BROKER is running with `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` signal registered.
- Mock PARKING_OPERATOR is running on port 8080.
- PARKING_OPERATOR_ADAPTOR is running and subscribed to DATA_BROKER.
- No active parking session.

**Steps:**
1. Write `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = true` to DATA_BROKER via gRPC.
2. Wait up to 2 seconds for the adaptor to process the event.
3. Query the mock PARKING_OPERATOR for active sessions (or inspect adaptor state via `GetStatus` RPC).

**Expected result:**
- The mock PARKING_OPERATOR received a `POST /parking/start` request with `vehicle_id`, `zone_id`, and `timestamp` fields.
- `GetStatus` RPC returns `state = "active"` and a non-empty `session_id`.

---

### TS-08-2: Unlock Event Triggers Session Stop with Operator

**Traces to:** 08-REQ-1.2

**Preconditions:**
- An active parking session exists (TS-08-1 completed successfully).

**Steps:**
1. Write `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = false` to DATA_BROKER via gRPC.
2. Wait up to 2 seconds for the adaptor to process the event.
3. Query adaptor state via `GetStatus` RPC.

**Expected result:**
- The mock PARKING_OPERATOR received a `POST /parking/stop` request with the active `session_id` and a `timestamp`.
- `GetStatus` RPC returns `state = "idle"` and an empty `session_id`.
- The stop response contains a non-negative `duration` and `fee`.

---

### TS-08-3: SessionActive Signal Published to DATA_BROKER on Start

**Traces to:** 08-REQ-2.1

**Preconditions:**
- DATA_BROKER is running.
- Mock PARKING_OPERATOR is running.
- PARKING_OPERATOR_ADAPTOR is running.
- No active parking session.

**Steps:**
1. Subscribe to `Vehicle.Parking.SessionActive` on DATA_BROKER via gRPC.
2. Write `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = true` to DATA_BROKER.
3. Wait up to 2 seconds for the subscription to deliver an update.

**Expected result:**
- The subscription stream delivers `Vehicle.Parking.SessionActive = true`.

---

### TS-08-4: SessionActive Signal Published to DATA_BROKER on Stop

**Traces to:** 08-REQ-2.1

**Preconditions:**
- An active parking session exists (TS-08-3 completed successfully).

**Steps:**
1. Continue listening on the `Vehicle.Parking.SessionActive` subscription.
2. Write `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = false` to DATA_BROKER.
3. Wait up to 2 seconds for the subscription to deliver an update.

**Expected result:**
- The subscription stream delivers `Vehicle.Parking.SessionActive = false`.

---

### TS-08-5: GetStatus Returns Current Session State

**Traces to:** 08-REQ-3.1

**Preconditions:**
- PARKING_OPERATOR_ADAPTOR is running on port 50052.

**Steps:**
1. Call `GetStatus` RPC while no session is active.
2. Trigger a session start (lock event or manual StartSession).
3. Call `GetStatus` RPC while session is active.

**Expected result:**
- Step 1: Response has `state = "idle"`, empty `session_id`.
- Step 3: Response has `state = "active"`, non-empty `session_id`, non-empty `zone_id`.

---

### TS-08-6: GetRate Returns Parking Rate

**Traces to:** 08-REQ-3.1

**Preconditions:**
- PARKING_OPERATOR_ADAPTOR is running on port 50052 with a configured zone.

**Steps:**
1. Call `GetRate` RPC.

**Expected result:**
- Response contains `rate_per_minute > 0`, non-empty `currency` (e.g., "EUR"), and non-empty `zone_id`.

---

### TS-08-7: Manual StartSession Overrides Autonomous Behavior

**Traces to:** 08-REQ-3.1

**Preconditions:**
- PARKING_OPERATOR_ADAPTOR is running. No active session.
- Mock PARKING_OPERATOR is running.

**Steps:**
1. Call `StartSession(zone_id = "zone-demo-1")` via gRPC on port 50052.
2. Call `GetStatus` RPC.
3. Read `Vehicle.Parking.SessionActive` from DATA_BROKER.

**Expected result:**
- `StartSession` returns a `session_id` and `status = "active"`.
- `GetStatus` returns `state = "active"`.
- `Vehicle.Parking.SessionActive = true` on DATA_BROKER.

---

### TS-08-8: Manual StopSession Stops Active Session

**Traces to:** 08-REQ-3.1

**Preconditions:**
- An active session exists (from TS-08-7 or autonomous start).

**Steps:**
1. Call `StopSession(session_id)` via gRPC on port 50052 with the active session ID.
2. Call `GetStatus` RPC.
3. Read `Vehicle.Parking.SessionActive` from DATA_BROKER.

**Expected result:**
- `StopSession` returns the `session_id`, a `duration_seconds >= 0`, a `fee >= 0.0`, and `status = "completed"`.
- `GetStatus` returns `state = "idle"`.
- `Vehicle.Parking.SessionActive = false` on DATA_BROKER.

---

### TS-08-9: Mock Operator Returns Valid Session IDs and Fees

**Traces to:** 08-REQ-5.1

**Preconditions:**
- Mock PARKING_OPERATOR is running on port 8080.

**Steps:**
1. Send `POST /parking/start` with `{"vehicle_id": "VIN-001", "zone_id": "zone-1", "timestamp": <now>}` to the mock operator.
2. Record the returned `session_id`.
3. Wait 2 seconds.
4. Send `POST /parking/stop` with `{"session_id": "<from step 2>", "timestamp": <now>}` to the mock operator.

**Expected result:**
- Start response: HTTP 200, `session_id` is a valid UUID, `status = "active"`.
- Stop response: HTTP 200, `session_id` matches, `duration > 0`, `fee > 0`, `status = "completed"`.

## Property Tests

### TS-08-P1: Double Lock Does Not Create Duplicate Session

**Traces to:** 08-REQ-1.1, CP-2

**Preconditions:**
- PARKING_OPERATOR_ADAPTOR is running. No active session.
- Mock PARKING_OPERATOR is running.

**Steps:**
1. Write `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = true` to DATA_BROKER.
2. Wait 1 second.
3. Write `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = true` to DATA_BROKER again.
4. Wait 1 second.
5. Call `GetStatus` RPC.

**Expected result:**
- The mock PARKING_OPERATOR received exactly one `POST /parking/start` call (not two).
- `GetStatus` returns `state = "active"` with a single `session_id`.

---

### TS-08-P2: Double Unlock Does Not Call Operator Twice

**Traces to:** 08-REQ-1.2, CP-3

**Preconditions:**
- An active parking session exists.

**Steps:**
1. Write `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = false` to DATA_BROKER.
2. Wait 1 second.
3. Write `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = false` to DATA_BROKER again.
4. Wait 1 second.
5. Call `GetStatus` RPC.

**Expected result:**
- The mock PARKING_OPERATOR received exactly one `POST /parking/stop` call (not two).
- `GetStatus` returns `state = "idle"`.

---

### TS-08-P3: Manual Start Followed by Autonomous Unlock

**Traces to:** 08-REQ-3.1, CP-4

**Preconditions:**
- PARKING_OPERATOR_ADAPTOR is running. No active session.

**Steps:**
1. Call `StartSession(zone_id = "zone-demo-1")` via gRPC.
2. Verify `GetStatus` returns `state = "active"`.
3. Write `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = false` to DATA_BROKER.
4. Wait 2 seconds.
5. Call `GetStatus` RPC.

**Expected result:**
- After step 3, the adaptor calls `POST /parking/stop` on the operator.
- `GetStatus` returns `state = "idle"`.
- `Vehicle.Parking.SessionActive = false` on DATA_BROKER.

---

### TS-08-P4: State-Signal Consistency After Full Cycle

**Traces to:** 08-REQ-2.1, CP-5

**Preconditions:**
- PARKING_OPERATOR_ADAPTOR is running. No active session.

**Steps:**
1. Read `Vehicle.Parking.SessionActive` from DATA_BROKER. Expect `false` (or not set).
2. Write `IsLocked = true`. Wait 2 seconds.
3. Read `Vehicle.Parking.SessionActive`. Expect `true`.
4. Call `GetStatus`. Expect `state = "active"`.
5. Write `IsLocked = false`. Wait 2 seconds.
6. Read `Vehicle.Parking.SessionActive`. Expect `false`.
7. Call `GetStatus`. Expect `state = "idle"`.

**Expected result:**
- At every step, `Vehicle.Parking.SessionActive` on DATA_BROKER matches the adaptor's internal session state.

## Error Tests

### TS-08-E1: Operator Unreachable on Session Start

**Traces to:** 08-REQ-6.1, CP-6

**Preconditions:**
- PARKING_OPERATOR_ADAPTOR is running.
- Mock PARKING_OPERATOR is NOT running (stopped).

**Steps:**
1. Write `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = true` to DATA_BROKER.
2. Wait 2 seconds.
3. Call `GetStatus` RPC.
4. Read `Vehicle.Parking.SessionActive` from DATA_BROKER.

**Expected result:**
- `GetStatus` returns `state = "idle"` (session did not start).
- `Vehicle.Parking.SessionActive` is `false` (or not set).

---

### TS-08-E2: Operator Unreachable on Session Stop

**Traces to:** 08-REQ-6.1

**Preconditions:**
- An active parking session exists.
- Stop the mock PARKING_OPERATOR after session start.

**Steps:**
1. Stop the mock PARKING_OPERATOR.
2. Write `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = false` to DATA_BROKER.
3. Wait 2 seconds.
4. Call `GetStatus` RPC.

**Expected result:**
- `GetStatus` returns `state = "idle"` (adaptor transitions to idle to avoid stuck state).
- `Vehicle.Parking.SessionActive` remains `true` (operator did not confirm stop, signal may be stale).

---

### TS-08-E3: StartSession When Session Already Active

**Traces to:** 08-REQ-3.1

**Preconditions:**
- An active parking session exists.

**Steps:**
1. Call `StartSession(zone_id = "zone-demo-1")` via gRPC.

**Expected result:**
- The RPC returns a gRPC `ALREADY_EXISTS` error with message containing "session already active".

---

### TS-08-E4: StopSession When No Session Active

**Traces to:** 08-REQ-3.1

**Preconditions:**
- No active parking session.

**Steps:**
1. Call `StopSession(session_id = "nonexistent-id")` via gRPC.

**Expected result:**
- The RPC returns a gRPC `NOT_FOUND` error with message containing "no active session".

---

### TS-08-E5: Mock Operator Stop with Unknown Session ID

**Traces to:** 08-REQ-5.1

**Preconditions:**
- Mock PARKING_OPERATOR is running.

**Steps:**
1. Send `POST /parking/stop` with `{"session_id": "unknown-id-12345", "timestamp": <now>}` to the mock operator.

**Expected result:**
- HTTP 404 response with body `{"error": "session not found"}`.

---

### TS-08-E6: Mock Operator with Malformed Request

**Traces to:** 08-REQ-5.1

**Preconditions:**
- Mock PARKING_OPERATOR is running.

**Steps:**
1. Send `POST /parking/start` with body `{"invalid": "data"}` (missing required fields) to the mock operator.
2. Send `POST /parking/start` with body `not-json` to the mock operator.

**Expected result:**
- Both requests return HTTP 400 with body `{"error": "bad request"}`.

## Test Commands Summary

| Test Scope | Command |
|-----------|---------|
| Rust unit tests | `cd rhivos && cargo test -p parking-operator-adaptor` |
| Rust lint | `cd rhivos && cargo clippy -p parking-operator-adaptor` |
| Mock operator unit tests | `cd mock/parking-operator && go test ./... -v` |
| Integration tests | Requires infra; see individual test preconditions |
