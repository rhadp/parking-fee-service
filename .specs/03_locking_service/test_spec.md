# Test Specification: LOCKING_SERVICE (Spec 03)

> Test specifications for the LOCKING_SERVICE and mock sensor CLI tools.
> Validates the requirements defined in `.specs/03_locking_service/requirements.md`.

## Test ID Convention

- **TS-03-N:** Positive / happy-path tests
- **TS-03-PN:** Precondition and state-dependent tests
- **TS-03-EN:** Error and edge-case tests

## Test Environment

- DATA_BROKER (Eclipse Kuksa Databroker) running locally on port 55556 or via UDS at `/tmp/kuksa/databroker.sock`.
- All VSS signals configured per `02_data_broker` spec.
- Mock sensor CLIs available on PATH.

---

## TS-03-1: Lock Command with Valid Preconditions Succeeds

**Requirement:** 03-REQ-2.1, 03-REQ-2.2, 03-REQ-3.1, 03-REQ-4.1

**Type:** Integration

**Preconditions:**
- DATA_BROKER is running with all signals registered.
- `Vehicle.Speed` is set to `0.0` (via SPEED_SENSOR or direct write).
- `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` is set to `false` (via DOOR_SENSOR or direct write).
- `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` is set to `false` (initial unlocked state).

**Steps:**
1. Write a valid lock command to `Vehicle.Command.Door.Lock`:
   ```json
   {
     "command_id": "cmd-001",
     "action": "lock",
     "doors": ["driver"],
     "source": "companion_app",
     "vin": "TEST_VIN_001",
     "timestamp": 1700000000
   }
   ```
2. Wait for the LOCKING_SERVICE to process the command (max 2 seconds).
3. Read `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` from DATA_BROKER.
4. Read `Vehicle.Command.Door.Response` from DATA_BROKER.

**Expected Results:**
- `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` equals `true`.
- `Vehicle.Command.Door.Response` contains:
  ```json
  {
    "command_id": "cmd-001",
    "status": "success",
    "timestamp": <any_unix_ts>
  }
  ```

---

## TS-03-2: Unlock Command Succeeds Regardless of Door State

**Requirement:** 03-REQ-2.1, 03-REQ-2.2, 03-REQ-3.1, 03-REQ-4.1

**Type:** Integration

**Preconditions:**
- DATA_BROKER is running.
- `Vehicle.Speed` is set to `0.0`.
- `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` is set to `true` (door is ajar).
- `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` is set to `true` (currently locked).

**Steps:**
1. Write a valid unlock command to `Vehicle.Command.Door.Lock`:
   ```json
   {
     "command_id": "cmd-002",
     "action": "unlock",
     "doors": ["driver"],
     "source": "companion_app",
     "vin": "TEST_VIN_001",
     "timestamp": 1700000001
   }
   ```
2. Wait for the LOCKING_SERVICE to process the command (max 2 seconds).
3. Read `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` from DATA_BROKER.
4. Read `Vehicle.Command.Door.Response` from DATA_BROKER.

**Expected Results:**
- `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` equals `false`.
- `Vehicle.Command.Door.Response` contains:
  ```json
  {
    "command_id": "cmd-002",
    "status": "success",
    "timestamp": <any_unix_ts>
  }
  ```

---

## TS-03-P1: Lock Command Rejected When Vehicle Is Moving

**Requirement:** 03-REQ-2.1, 03-REQ-4.1

**Type:** Integration

**Preconditions:**
- DATA_BROKER is running.
- `Vehicle.Speed` is set to `5.0` (vehicle is moving).
- `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` is set to `false`.
- `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` is set to `false`.

**Steps:**
1. Write a valid lock command to `Vehicle.Command.Door.Lock`:
   ```json
   {
     "command_id": "cmd-003",
     "action": "lock",
     "doors": ["driver"],
     "source": "companion_app",
     "vin": "TEST_VIN_001",
     "timestamp": 1700000002
   }
   ```
2. Wait for the LOCKING_SERVICE to process the command (max 2 seconds).
3. Read `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` from DATA_BROKER.
4. Read `Vehicle.Command.Door.Response` from DATA_BROKER.

**Expected Results:**
- `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` remains `false` (unchanged).
- `Vehicle.Command.Door.Response` contains:
  ```json
  {
    "command_id": "cmd-003",
    "status": "failed",
    "reason": "vehicle_moving",
    "timestamp": <any_unix_ts>
  }
  ```

---

## TS-03-P2: Lock Command Rejected When Door Is Ajar

**Requirement:** 03-REQ-2.2, 03-REQ-4.1

**Type:** Integration

**Preconditions:**
- DATA_BROKER is running.
- `Vehicle.Speed` is set to `0.0` (vehicle is stationary).
- `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` is set to `true` (door is ajar).
- `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` is set to `false`.

**Steps:**
1. Write a valid lock command to `Vehicle.Command.Door.Lock`:
   ```json
   {
     "command_id": "cmd-004",
     "action": "lock",
     "doors": ["driver"],
     "source": "companion_app",
     "vin": "TEST_VIN_001",
     "timestamp": 1700000003
   }
   ```
2. Wait for the LOCKING_SERVICE to process the command (max 2 seconds).
3. Read `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` from DATA_BROKER.
4. Read `Vehicle.Command.Door.Response` from DATA_BROKER.

**Expected Results:**
- `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` remains `false` (unchanged).
- `Vehicle.Command.Door.Response` contains:
  ```json
  {
    "command_id": "cmd-004",
    "status": "failed",
    "reason": "door_ajar",
    "timestamp": <any_unix_ts>
  }
  ```

---

## TS-03-P3: Unlock Command Rejected When Vehicle Is Moving

**Requirement:** 03-REQ-2.1, 03-REQ-4.1

**Type:** Integration

**Preconditions:**
- DATA_BROKER is running.
- `Vehicle.Speed` is set to `10.0` (vehicle is moving).
- `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` is set to `true`.

**Steps:**
1. Write a valid unlock command to `Vehicle.Command.Door.Lock`:
   ```json
   {
     "command_id": "cmd-005",
     "action": "unlock",
     "doors": ["driver"],
     "source": "companion_app",
     "vin": "TEST_VIN_001",
     "timestamp": 1700000004
   }
   ```
2. Wait for the LOCKING_SERVICE to process the command (max 2 seconds).
3. Read `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` from DATA_BROKER.
4. Read `Vehicle.Command.Door.Response` from DATA_BROKER.

**Expected Results:**
- `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` remains `true` (unchanged).
- `Vehicle.Command.Door.Response` contains:
  ```json
  {
    "command_id": "cmd-005",
    "status": "failed",
    "reason": "vehicle_moving",
    "timestamp": <any_unix_ts>
  }
  ```

---

## TS-03-E1: Invalid Command JSON Is Rejected Gracefully

**Requirement:** 03-REQ-4.2

**Type:** Unit / Integration

**Preconditions:**
- DATA_BROKER is running.
- LOCKING_SERVICE is running and subscribed.

**Steps:**
1. Write an invalid JSON string to `Vehicle.Command.Door.Lock`:
   ```
   {"command_id": "cmd-006", "action": "lock", INVALID
   ```
2. Wait 2 seconds.
3. Verify the LOCKING_SERVICE is still running and responsive.
4. Write a second invalid payload with a parseable `command_id` but missing `action`:
   ```json
   {"command_id": "cmd-007"}
   ```
5. Read `Vehicle.Command.Door.Response` from DATA_BROKER.

**Expected Results:**
- The LOCKING_SERVICE does not crash or exit.
- For step 1 (unparseable JSON): no response is published (command_id cannot be extracted from garbled JSON). The error is logged.
- For step 4 (missing `action` field): a failure response is published:
  ```json
  {
    "command_id": "cmd-007",
    "status": "failed",
    "reason": "invalid_command",
    "timestamp": <any_unix_ts>
  }
  ```

---

## TS-03-E2: Command Response Published to DATA_BROKER

**Requirement:** 03-REQ-4.1

**Type:** Integration

**Preconditions:**
- DATA_BROKER is running.
- A subscriber is listening on `Vehicle.Command.Door.Response` (e.g., a test client simulating CLOUD_GATEWAY_CLIENT).

**Steps:**
1. Set valid preconditions (speed = 0.0, door closed).
2. Write a lock command to `Vehicle.Command.Door.Lock` with `command_id = "cmd-008"`.
3. Observe the response signal via the test subscriber.

**Expected Results:**
- The test subscriber receives a notification for `Vehicle.Command.Door.Response`.
- The response JSON contains `"command_id": "cmd-008"` and `"status": "success"`.
- The `command_id` in the response exactly matches the one in the command.

---

## TS-03-E3: Mock Sensors Write Correct Signal Values

**Requirement:** 03-REQ-5.1, 03-REQ-5.2, 03-REQ-5.3

**Type:** Integration

**Preconditions:**
- DATA_BROKER is running with all signals registered.

**Steps:**
1. Run: `location-sensor --latitude 48.1351 --longitude 11.5820`
2. Read `Vehicle.CurrentLocation.Latitude` and `Vehicle.CurrentLocation.Longitude` from DATA_BROKER.
3. Run: `speed-sensor --speed 42.5`
4. Read `Vehicle.Speed` from DATA_BROKER.
5. Run: `door-sensor --open true`
6. Read `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` from DATA_BROKER.

**Expected Results:**
- `Vehicle.CurrentLocation.Latitude` equals `48.1351` (within floating-point tolerance).
- `Vehicle.CurrentLocation.Longitude` equals `11.5820` (within floating-point tolerance).
- `Vehicle.Speed` equals `42.5` (within floating-point tolerance).
- `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` equals `true`.
- All CLI invocations exit with code 0.

---

## TS-03-E4: Concurrent Command Handling

**Requirement:** 03-REQ-4.3

**Type:** Integration

**Preconditions:**
- DATA_BROKER is running.
- `Vehicle.Speed` is set to `0.0`.
- `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` is set to `false`.
- `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` is set to `false`.

**Steps:**
1. Write three commands to `Vehicle.Command.Door.Lock` in rapid succession (< 100ms apart):
   - Command A: `{"command_id": "cmd-A", "action": "lock", ...}`
   - Command B: `{"command_id": "cmd-B", "action": "unlock", ...}`
   - Command C: `{"command_id": "cmd-C", "action": "lock", ...}`
2. Wait for all commands to be processed (max 5 seconds).
3. Collect all responses from `Vehicle.Command.Door.Response`.

**Expected Results:**
- Three responses are published, one for each command.
- Each response contains the correct `command_id` (cmd-A, cmd-B, cmd-C).
- All three responses have `"status": "success"` (all preconditions are valid for each).
- Commands are processed in order: after all three, `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` is `true` (lock -> unlock -> lock).
- No commands are dropped.

---

## Traceability Matrix

| Test ID | Requirements Covered |
|---------|---------------------|
| TS-03-1 | 03-REQ-2.1, 03-REQ-2.2, 03-REQ-3.1, 03-REQ-4.1 |
| TS-03-2 | 03-REQ-2.1, 03-REQ-2.2, 03-REQ-3.1, 03-REQ-4.1 |
| TS-03-P1 | 03-REQ-2.1, 03-REQ-4.1 |
| TS-03-P2 | 03-REQ-2.2, 03-REQ-4.1 |
| TS-03-P3 | 03-REQ-2.1, 03-REQ-4.1 |
| TS-03-E1 | 03-REQ-4.2 |
| TS-03-E2 | 03-REQ-4.1 |
| TS-03-E3 | 03-REQ-5.1, 03-REQ-5.2, 03-REQ-5.3 |
| TS-03-E4 | 03-REQ-4.3 |
