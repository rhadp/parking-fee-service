# Test Specification: LOCKING_SERVICE (Spec 03)

> Test specifications for the LOCKING_SERVICE component.
> Verifies requirements from `.specs/03_locking_service/requirements.md`.

## References

- Requirements: `.specs/03_locking_service/requirements.md`
- Design: `.specs/03_locking_service/design.md`

## Test ID Convention

| Prefix | Category |
|--------|----------|
| TS-03-n | Positive / happy-path tests |
| TS-03-Pn | Property / invariant tests |
| TS-03-En | Error / edge-case tests |

## Test Infrastructure

- **Unit tests:** No external dependencies. Mock DATA_BROKER interactions for command parsing, safety checks, and response construction.
- **Integration tests:** Require running Kuksa Databroker (UDS at `/tmp/kuksa/databroker.sock` or configurable path). Start with `make infra-up`.

---

## TS-03-1: Lock Command Processing (Happy Path)

**Requirement:** 03-REQ-2.1, 03-REQ-3.1, 03-REQ-3.2, 03-REQ-4.1, 03-REQ-5.1

**Description:** Verify that a valid lock command is processed end-to-end: parsed, safety-checked, executed, and responded to.

**Preconditions:**
- DATA_BROKER is running.
- `Vehicle.Speed` is set to `0.0`.
- `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` is set to `false`.
- LOCKING_SERVICE is running and subscribed to `Vehicle.Command.Door.Lock`.

**Steps:**
1. Write a valid lock command JSON to `Vehicle.Command.Door.Lock` on DATA_BROKER:
   ```json
   {
     "command_id": "550e8400-e29b-41d4-a716-446655440000",
     "action": "lock",
     "doors": ["driver"],
     "source": "companion_app",
     "vin": "TEST_VIN_001",
     "timestamp": 1700000000
   }
   ```
2. Read `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` from DATA_BROKER.
3. Read `Vehicle.Command.Door.Response` from DATA_BROKER.

**Expected result:**
- `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` is `true`.
- `Vehicle.Command.Door.Response` contains a JSON with `command_id` matching `"550e8400-e29b-41d4-a716-446655440000"`, `status` equal to `"success"`, and a `timestamp`.

**Test type:** Integration

---

## TS-03-2: Unlock Command Processing

**Requirement:** 03-REQ-2.1, 03-REQ-3.1, 03-REQ-3.2, 03-REQ-4.2, 03-REQ-5.1

**Description:** Verify that a valid unlock command is processed end-to-end.

**Preconditions:**
- DATA_BROKER is running.
- `Vehicle.Speed` is set to `0.0`.
- `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` is set to `false`.
- `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` is set to `true` (vehicle is currently locked).
- LOCKING_SERVICE is running and subscribed to `Vehicle.Command.Door.Lock`.

**Steps:**
1. Write a valid unlock command JSON to `Vehicle.Command.Door.Lock` on DATA_BROKER:
   ```json
   {
     "command_id": "660e8400-e29b-41d4-a716-446655440001",
     "action": "unlock",
     "doors": ["driver"],
     "source": "companion_app",
     "vin": "TEST_VIN_001",
     "timestamp": 1700000010
   }
   ```
2. Read `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` from DATA_BROKER.
3. Read `Vehicle.Command.Door.Response` from DATA_BROKER.

**Expected result:**
- `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` is `false`.
- `Vehicle.Command.Door.Response` contains a JSON with `command_id` matching `"660e8400-e29b-41d4-a716-446655440001"`, `status` equal to `"success"`, and a `timestamp`.

**Test type:** Integration

---

## TS-03-3: Safety Constraint Rejection -- Vehicle Moving

**Requirement:** 03-REQ-3.1, 03-REQ-3.3, 03-REQ-5.2

**Description:** Verify that a lock command is rejected when the vehicle is moving.

**Preconditions:**
- DATA_BROKER is running.
- `Vehicle.Speed` is set to `30.0` (km/h).
- `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` is set to `false`.
- LOCKING_SERVICE is running and subscribed to `Vehicle.Command.Door.Lock`.

**Steps:**
1. Write a valid lock command JSON to `Vehicle.Command.Door.Lock` on DATA_BROKER:
   ```json
   {
     "command_id": "770e8400-e29b-41d4-a716-446655440002",
     "action": "lock",
     "doors": ["driver"],
     "source": "companion_app",
     "vin": "TEST_VIN_001",
     "timestamp": 1700000020
   }
   ```
2. Read `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` from DATA_BROKER.
3. Read `Vehicle.Command.Door.Response` from DATA_BROKER.

**Expected result:**
- `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` is NOT modified (unchanged from its previous value).
- `Vehicle.Command.Door.Response` contains a JSON with `command_id` matching `"770e8400-e29b-41d4-a716-446655440002"`, `status` equal to `"failed"`, and `reason` equal to `"vehicle_moving"`.

**Test type:** Integration

---

## TS-03-4: Safety Constraint Rejection -- Door Ajar

**Requirement:** 03-REQ-3.2, 03-REQ-3.3, 03-REQ-5.2

**Description:** Verify that a lock command is rejected when the door is ajar.

**Preconditions:**
- DATA_BROKER is running.
- `Vehicle.Speed` is set to `0.0`.
- `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` is set to `true`.
- LOCKING_SERVICE is running and subscribed to `Vehicle.Command.Door.Lock`.

**Steps:**
1. Write a valid lock command JSON to `Vehicle.Command.Door.Lock` on DATA_BROKER:
   ```json
   {
     "command_id": "880e8400-e29b-41d4-a716-446655440003",
     "action": "lock",
     "doors": ["driver"],
     "source": "companion_app",
     "vin": "TEST_VIN_001",
     "timestamp": 1700000030
   }
   ```
2. Read `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` from DATA_BROKER.
3. Read `Vehicle.Command.Door.Response` from DATA_BROKER.

**Expected result:**
- `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` is NOT modified (unchanged from its previous value).
- `Vehicle.Command.Door.Response` contains a JSON with `command_id` matching `"880e8400-e29b-41d4-a716-446655440003"`, `status` equal to `"failed"`, and `reason` equal to `"door_ajar"`.

**Test type:** Integration

---

## TS-03-E1: Invalid Command JSON Handling

**Requirement:** 03-REQ-2.2, 03-REQ-5.2

**Description:** Verify that malformed JSON on the command signal is handled gracefully with a failure response.

**Preconditions:**
- DATA_BROKER is running.
- LOCKING_SERVICE is running and subscribed to `Vehicle.Command.Door.Lock`.

**Steps:**
1. Write `not valid json {{{` to `Vehicle.Command.Door.Lock` on DATA_BROKER.
2. Read `Vehicle.Command.Door.Response` from DATA_BROKER.
3. Write a valid lock command JSON to `Vehicle.Command.Door.Lock`.
4. Read `Vehicle.Command.Door.Response` from DATA_BROKER.

**Expected result:**
- After step 1: a failure response is written to `Vehicle.Command.Door.Response` with `status` equal to `"failed"` and `reason` equal to `"invalid_command"`.
- After step 3: the service continues running and processes the subsequent valid command successfully.

**Test type:** Integration

---

## TS-03-E2: Command with Missing Required Fields

**Requirement:** 03-REQ-2.2, 03-REQ-5.2

**Description:** Verify that a command JSON missing required fields is rejected with a failure response.

**Preconditions:**
- DATA_BROKER is running.
- LOCKING_SERVICE is running and subscribed to `Vehicle.Command.Door.Lock`.

**Steps:**
1. Write a JSON payload missing the `action` field to `Vehicle.Command.Door.Lock`:
   ```json
   {
     "command_id": "990e8400-e29b-41d4-a716-446655440004",
     "doors": ["driver"],
     "source": "companion_app",
     "vin": "TEST_VIN_001",
     "timestamp": 1700000040
   }
   ```
2. Read `Vehicle.Command.Door.Response` from DATA_BROKER.

**Expected result:**
- `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` is NOT modified.
- `Vehicle.Command.Door.Response` contains a JSON with `status` equal to `"failed"` and `reason` equal to `"invalid_command"`.

**Test type:** Integration

---

## TS-03-E3: Command with Invalid Action Value

**Requirement:** 03-REQ-6.1, 03-REQ-5.2

**Description:** Verify that a command with an unrecognized action value is rejected.

**Preconditions:**
- DATA_BROKER is running.
- LOCKING_SERVICE is running and subscribed to `Vehicle.Command.Door.Lock`.

**Steps:**
1. Write a command JSON with `action` set to `"reboot"` to `Vehicle.Command.Door.Lock`:
   ```json
   {
     "command_id": "aa0e8400-e29b-41d4-a716-446655440005",
     "action": "reboot",
     "doors": ["driver"],
     "source": "companion_app",
     "vin": "TEST_VIN_001",
     "timestamp": 1700000050
   }
   ```
2. Read `Vehicle.Command.Door.Response` from DATA_BROKER.

**Expected result:**
- `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` is NOT modified.
- `Vehicle.Command.Door.Response` contains a JSON with `command_id` matching `"aa0e8400-e29b-41d4-a716-446655440005"`, `status` equal to `"failed"`, and `reason` equal to `"invalid_action"`.

**Test type:** Integration

---

## TS-03-E4: Command Response Format Validation

**Requirement:** 03-REQ-5.1, 03-REQ-5.2

**Description:** Verify that both success and failure responses conform to the expected JSON format.

**Preconditions:**
- DATA_BROKER is running.
- LOCKING_SERVICE is running.

**Steps:**
1. Trigger a successful lock command (vehicle stationary, door closed) and capture the response from `Vehicle.Command.Door.Response`.
2. Trigger a failed command (vehicle moving) and capture the response from `Vehicle.Command.Door.Response`.

**Expected result:**
- Success response contains exactly: `command_id` (string), `status` ("success"), `timestamp` (integer). No `reason` field.
- Failure response contains exactly: `command_id` (string), `status` ("failed"), `reason` (string), `timestamp` (integer).
- Both responses are valid JSON.
- `timestamp` values are valid Unix timestamps (non-negative integers).

**Test type:** Unit + Integration

---

## TS-03-P1: Safety Invariant -- No State Change on Constraint Violation

**Requirement:** 03-REQ-3.1, 03-REQ-3.2, 03-REQ-3.3, 03-REQ-4.1, 03-REQ-4.2

**Description:** Property test verifying that lock state is never modified when any safety constraint is violated.

**Property:**
For all combinations of:
- `speed`: any float >= 0.0
- `door_open`: true or false
- `action`: "lock" or "unlock"

If `speed >= 1.0` OR `door_open == true`, then `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` must remain unchanged after command processing.

**Test type:** Unit (property-based)

---

## TS-03-P2: Response Completeness -- Every Command Gets a Response

**Requirement:** 03-REQ-5.1, 03-REQ-5.2

**Description:** Property test verifying that every command input produces exactly one response output.

**Property:**
For all command inputs (valid, invalid, malformed), processing the command results in exactly one write to `Vehicle.Command.Door.Response` containing a `status` field of either `"success"` or `"failed"`.

**Test type:** Unit (property-based)

---

## TS-03-P3: Safety Invariant -- Successful Execution Implies Safe State

**Requirement:** 03-REQ-3.1, 03-REQ-3.2, 03-REQ-4.1, 03-REQ-4.2

**Description:** Property test verifying that a successful command response implies all safety constraints were satisfied at the time of execution.

**Property:**
If `Vehicle.Command.Door.Response` contains `status: "success"`, then at the time of execution:
- `Vehicle.Speed` was < 1.0 km/h
- `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` was `false`

**Test type:** Unit (property-based)
