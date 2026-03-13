# Test Specification: LOCKING_SERVICE

## Overview

Tests cover command parsing/validation (unit), safety constraint logic (unit), lock state management (unit + integration), response publishing (unit), and end-to-end command processing (integration). Unit tests use mock DATA_BROKER clients. Integration tests require a running Kuksa Databroker container. Property tests use the `proptest` crate for Rust.

## Test Cases

### TS-03-1: Command Subscription on Startup

**Requirement:** 03-REQ-1.1
**Type:** integration
**Description:** Verify the service subscribes to Vehicle.Command.Door.Lock on startup.

**Preconditions:**
- DATA_BROKER container is running.

**Input:**
- Start locking-service with DATABROKER_ADDR=http://localhost:55556.

**Expected:**
- Service connects and subscribes successfully (logs "ready" message).
- Setting a value on Vehicle.Command.Door.Lock triggers processing.

**Assertion pseudocode:**
```
start_locking_service()
ASSERT log_contains("ready")
set_signal("Vehicle.Command.Door.Lock", valid_lock_command)
response = get_signal("Vehicle.Command.Door.Response")
ASSERT response != nil
```

### TS-03-2: Command Deserialization

**Requirement:** 03-REQ-1.2
**Type:** unit
**Description:** Verify a valid JSON command payload is deserialized into a LockCommand struct.

**Preconditions:**
- None.

**Input:**
- `{"command_id":"abc-123","action":"lock","doors":["driver"],"source":"companion_app","vin":"WDB123","timestamp":1700000000}`

**Expected:**
- Parsed LockCommand with command_id="abc-123", action=Lock, doors=["driver"].

**Assertion pseudocode:**
```
cmd = parse_command(json_input)
ASSERT cmd.is_ok()
ASSERT cmd.command_id == "abc-123"
ASSERT cmd.action == Lock
ASSERT cmd.doors == ["driver"]
```

### TS-03-3: Configurable Databroker Address

**Requirement:** 03-REQ-1.3
**Type:** unit
**Description:** Verify the service reads DATABROKER_ADDR env var with correct default.

**Preconditions:**
- None.

**Input:**
- Case 1: DATABROKER_ADDR not set.
- Case 2: DATABROKER_ADDR="http://10.0.0.5:55556".

**Expected:**
- Case 1: Address is "http://localhost:55556".
- Case 2: Address is "http://10.0.0.5:55556".

**Assertion pseudocode:**
```
// Case 1
unset_env("DATABROKER_ADDR")
addr = get_databroker_addr()
ASSERT addr == "http://localhost:55556"

// Case 2
set_env("DATABROKER_ADDR", "http://10.0.0.5:55556")
addr = get_databroker_addr()
ASSERT addr == "http://10.0.0.5:55556"
```

### TS-03-4: Validate command_id Required

**Requirement:** 03-REQ-2.1
**Type:** unit
**Description:** Verify command_id must be a non-empty string.

**Preconditions:**
- None.

**Input:**
- `{"command_id":"","action":"lock","doors":["driver"]}`

**Expected:**
- Validation fails.

**Assertion pseudocode:**
```
cmd = parse_command(input)
result = validate_command(cmd)
ASSERT result.is_err()
ASSERT result.error == "invalid_command"
```

### TS-03-5: Validate Action Field

**Requirement:** 03-REQ-2.2
**Type:** unit
**Description:** Verify action must be "lock" or "unlock".

**Preconditions:**
- None.

**Input:**
- `{"command_id":"x","action":"toggle","doors":["driver"]}`

**Expected:**
- Validation fails.

**Assertion pseudocode:**
```
cmd = parse_command(input)
ASSERT cmd.is_err() // "toggle" is not a valid action
```

### TS-03-6: Validate Doors Field

**Requirement:** 03-REQ-2.3
**Type:** unit
**Description:** Verify doors array must contain "driver".

**Preconditions:**
- None.

**Input:**
- `{"command_id":"x","action":"lock","doors":["passenger"]}`

**Expected:**
- Validation fails with reason "unsupported_door".

**Assertion pseudocode:**
```
cmd = parse_command(input)
result = validate_command(cmd)
ASSERT result.is_err()
ASSERT result.error == "unsupported_door"
```

### TS-03-7: Lock Rejected When Vehicle Moving

**Requirement:** 03-REQ-3.1
**Type:** unit
**Description:** Verify lock is rejected when Vehicle.Speed >= 1.0.

**Preconditions:**
- Mock broker returns Vehicle.Speed = 50.0, IsOpen = false.

**Input:**
- Valid lock command.

**Expected:**
- SafetyResult::VehicleMoving.

**Assertion pseudocode:**
```
mock_broker.set_speed(50.0)
mock_broker.set_door_open(false)
result = check_safety(&mock_broker)
ASSERT result == SafetyResult::VehicleMoving
```

### TS-03-8: Lock Rejected When Door Open

**Requirement:** 03-REQ-3.2
**Type:** unit
**Description:** Verify lock is rejected when door is ajar.

**Preconditions:**
- Mock broker returns Vehicle.Speed = 0.0, IsOpen = true.

**Input:**
- Valid lock command.

**Expected:**
- SafetyResult::DoorOpen.

**Assertion pseudocode:**
```
mock_broker.set_speed(0.0)
mock_broker.set_door_open(true)
result = check_safety(&mock_broker)
ASSERT result == SafetyResult::DoorOpen
```

### TS-03-9: Lock Allowed When Safe

**Requirement:** 03-REQ-3.3
**Type:** unit
**Description:** Verify lock is allowed when stationary and door closed.

**Preconditions:**
- Mock broker returns Vehicle.Speed = 0.0, IsOpen = false.

**Input:**
- Valid lock command.

**Expected:**
- SafetyResult::Safe.

**Assertion pseudocode:**
```
mock_broker.set_speed(0.0)
mock_broker.set_door_open(false)
result = check_safety(&mock_broker)
ASSERT result == SafetyResult::Safe
```

### TS-03-10: Unlock Bypasses Safety

**Requirement:** 03-REQ-3.4
**Type:** unit
**Description:** Verify unlock proceeds regardless of speed or door state.

**Preconditions:**
- Mock broker returns Vehicle.Speed = 100.0, IsOpen = true.

**Input:**
- Valid unlock command.

**Expected:**
- Command executes successfully (no safety check called for unlock).

**Assertion pseudocode:**
```
mock_broker.set_speed(100.0)
mock_broker.set_door_open(true)
// process_command should succeed for unlock regardless
result = process_unlock_command(&mock_broker, unlock_cmd)
ASSERT result.status == "success"
```

### TS-03-11: Lock Sets IsLocked True

**Requirement:** 03-REQ-4.1
**Type:** unit
**Description:** Verify successful lock sets IsLocked to true.

**Preconditions:**
- Mock broker with safe conditions, IsLocked initially false.

**Input:**
- Valid lock command.

**Expected:**
- mock_broker.set_bool was called with ("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", true).

**Assertion pseudocode:**
```
process_command(&mock_broker, lock_cmd)
ASSERT mock_broker.last_set_bool == ("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", true)
```

### TS-03-12: Unlock Sets IsLocked False

**Requirement:** 03-REQ-4.2
**Type:** unit
**Description:** Verify successful unlock sets IsLocked to false.

**Preconditions:**
- Mock broker, IsLocked initially true.

**Input:**
- Valid unlock command.

**Expected:**
- mock_broker.set_bool was called with ("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", false).

**Assertion pseudocode:**
```
process_command(&mock_broker, unlock_cmd)
ASSERT mock_broker.last_set_bool == ("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", false)
```

### TS-03-13: Initial State Published

**Requirement:** 03-REQ-4.3
**Type:** integration
**Description:** Verify the service publishes IsLocked=false on startup.

**Preconditions:**
- DATA_BROKER container is running, no prior IsLocked value.

**Input:**
- Start locking-service.

**Expected:**
- Vehicle.Cabin.Door.Row1.DriverSide.IsLocked is false in DATA_BROKER.

**Assertion pseudocode:**
```
start_locking_service()
wait_for_ready()
value = get_signal("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked")
ASSERT value == false
```

### TS-03-14: Success Response Format

**Requirement:** 03-REQ-5.1
**Type:** unit
**Description:** Verify success response JSON contains command_id and status "success".

**Preconditions:**
- None.

**Input:**
- command_id = "abc-123".

**Expected:**
- JSON with command_id="abc-123", status="success", timestamp present.

**Assertion pseudocode:**
```
json = success_response("abc-123")
parsed = parse_json(json)
ASSERT parsed.command_id == "abc-123"
ASSERT parsed.status == "success"
ASSERT parsed.timestamp > 0
ASSERT parsed.reason == nil
```

### TS-03-15: Failure Response Format

**Requirement:** 03-REQ-5.2
**Type:** unit
**Description:** Verify failure response JSON contains command_id, status "failed", and reason.

**Preconditions:**
- None.

**Input:**
- command_id = "abc-123", reason = "vehicle_moving".

**Expected:**
- JSON with command_id="abc-123", status="failed", reason="vehicle_moving".

**Assertion pseudocode:**
```
json = failure_response("abc-123", "vehicle_moving")
parsed = parse_json(json)
ASSERT parsed.command_id == "abc-123"
ASSERT parsed.status == "failed"
ASSERT parsed.reason == "vehicle_moving"
ASSERT parsed.timestamp > 0
```

### TS-03-16: Response Timestamp

**Requirement:** 03-REQ-5.3
**Type:** unit
**Description:** Verify response includes a current Unix timestamp.

**Preconditions:**
- None.

**Input:**
- Generate a success response.

**Expected:**
- Timestamp is a reasonable Unix timestamp (within 5 seconds of current time).

**Assertion pseudocode:**
```
before = current_unix_timestamp()
json = success_response("x")
after = current_unix_timestamp()
parsed = parse_json(json)
ASSERT parsed.timestamp >= before
ASSERT parsed.timestamp <= after
```

### TS-03-17: Graceful Shutdown

**Requirement:** 03-REQ-6.1
**Type:** integration
**Description:** Verify the service exits cleanly on SIGTERM.

**Preconditions:**
- Locking-service is running with active DATA_BROKER subscription.

**Input:**
- Send SIGTERM to locking-service process.

**Expected:**
- Process exits with code 0.
- Log contains shutdown message.

**Assertion pseudocode:**
```
proc = start_locking_service()
wait_for_ready()
send_signal(proc, SIGTERM)
exit_code = wait_for_exit(proc, timeout=5s)
ASSERT exit_code == 0
```

### TS-03-18: Startup Logging

**Requirement:** 03-REQ-6.2
**Type:** integration
**Description:** Verify the service logs version and address on startup.

**Preconditions:**
- DATA_BROKER container is running.

**Input:**
- Start locking-service.

**Expected:**
- Log output contains version string and DATA_BROKER address.

**Assertion pseudocode:**
```
output = start_and_capture_logs()
ASSERT output contains "locking-service"
ASSERT output contains "localhost:55556" OR DATABROKER_ADDR value
ASSERT output contains "ready"
```

## Edge Case Tests

### TS-03-E1: Databroker Unreachable on Startup

**Requirement:** 03-REQ-1.E1
**Type:** integration
**Description:** Verify retry behavior when DATA_BROKER is unreachable.

**Preconditions:**
- No DATA_BROKER running.

**Input:**
- Start locking-service with DATABROKER_ADDR pointing to a non-listening port.

**Expected:**
- Service retries connection (logs indicate retries).
- Service exits with non-zero code after max retries.

**Assertion pseudocode:**
```
proc = start_locking_service(addr="http://localhost:19999")
exit_code = wait_for_exit(proc, timeout=30s)
ASSERT exit_code != 0
ASSERT log_contains("retry")
```

### TS-03-E2: Subscription Stream Interrupted

**Requirement:** 03-REQ-1.E2
**Type:** integration
**Description:** Verify resubscribe behavior when subscription stream breaks.

**Preconditions:**
- Locking-service is running with active subscription.

**Input:**
- Restart the DATA_BROKER container while locking-service is running.

**Expected:**
- Service attempts to resubscribe.

**Assertion pseudocode:**
```
start_locking_service()
wait_for_ready()
restart_databroker()
// If resubscribe succeeds, service remains running
// If 3 resubscribe attempts fail, service exits non-zero
```

### TS-03-E3: Invalid JSON Payload

**Requirement:** 03-REQ-2.E1
**Type:** unit
**Description:** Verify invalid JSON is discarded without response.

**Preconditions:**
- None.

**Input:**
- `"not valid json {{{"`

**Expected:**
- parse_command returns error.
- No response is published (caller should log and skip).

**Assertion pseudocode:**
```
result = parse_command("not valid json {{{")
ASSERT result.is_err()
```

### TS-03-E4: Missing Required Field

**Requirement:** 03-REQ-2.E2
**Type:** unit
**Description:** Verify missing action field produces "invalid_command" error.

**Preconditions:**
- None.

**Input:**
- `{"command_id":"x","doors":["driver"]}`

**Expected:**
- Parse or validation fails.

**Assertion pseudocode:**
```
result = parse_command(input)
ASSERT result.is_err()
```

### TS-03-E5: Unsupported Door Value

**Requirement:** 03-REQ-2.E3
**Type:** unit
**Description:** Verify non-"driver" door value produces "unsupported_door".

**Preconditions:**
- None.

**Input:**
- `{"command_id":"x","action":"lock","doors":["rear_left"]}`

**Expected:**
- Validation error with reason "unsupported_door".

**Assertion pseudocode:**
```
cmd = parse_command(input)
result = validate_command(cmd)
ASSERT result.is_err()
ASSERT result.error == "unsupported_door"
```

### TS-03-E6: Speed Signal Unset

**Requirement:** 03-REQ-3.E1
**Type:** unit
**Description:** Verify unset speed is treated as 0.0 (stationary).

**Preconditions:**
- Mock broker returns None for Vehicle.Speed, IsOpen = false.

**Input:**
- Valid lock command.

**Expected:**
- SafetyResult::Safe.

**Assertion pseudocode:**
```
mock_broker.set_speed(None)
mock_broker.set_door_open(false)
result = check_safety(&mock_broker)
ASSERT result == SafetyResult::Safe
```

### TS-03-E7: Door Signal Unset

**Requirement:** 03-REQ-3.E2
**Type:** unit
**Description:** Verify unset door state is treated as closed (false).

**Preconditions:**
- Mock broker returns Vehicle.Speed = 0.0, None for IsOpen.

**Input:**
- Valid lock command.

**Expected:**
- SafetyResult::Safe.

**Assertion pseudocode:**
```
mock_broker.set_speed(0.0)
mock_broker.set_door_open(None)
result = check_safety(&mock_broker)
ASSERT result == SafetyResult::Safe
```

### TS-03-E8: Lock When Already Locked

**Requirement:** 03-REQ-4.E1
**Type:** unit
**Description:** Verify locking an already-locked door succeeds idempotently.

**Preconditions:**
- Internal lock state is already true. Safe conditions.

**Input:**
- Valid lock command.

**Expected:**
- Success response. No state change published.

**Assertion pseudocode:**
```
service.lock_state = true
result = process_command(&mock_broker, lock_cmd)
ASSERT result.status == "success"
ASSERT mock_broker.set_bool_calls == 0  // no state change
```

### TS-03-E9: Unlock When Already Unlocked

**Requirement:** 03-REQ-4.E2
**Type:** unit
**Description:** Verify unlocking an already-unlocked door succeeds idempotently.

**Preconditions:**
- Internal lock state is already false.

**Input:**
- Valid unlock command.

**Expected:**
- Success response. No state change published.

**Assertion pseudocode:**
```
service.lock_state = false
result = process_command(&mock_broker, unlock_cmd)
ASSERT result.status == "success"
ASSERT mock_broker.set_bool_calls == 0
```

### TS-03-E10: Response Publish Failure

**Requirement:** 03-REQ-5.E1
**Type:** unit
**Description:** Verify the service continues processing after a response publish failure.

**Preconditions:**
- Mock broker configured to fail on set_string for response signal.

**Input:**
- Valid lock command, then another valid lock command.

**Expected:**
- First command: response publish fails, logged.
- Second command: processed normally.

**Assertion pseudocode:**
```
mock_broker.fail_next_set_string()
process_command(&mock_broker, lock_cmd_1)  // logs error
process_command(&mock_broker, lock_cmd_2)  // succeeds
ASSERT mock_broker.commands_processed == 2
```

### TS-03-E11: SIGTERM During Command

**Requirement:** 03-REQ-6.E1
**Type:** integration
**Description:** Verify in-flight command completes before shutdown.

**Preconditions:**
- Locking-service is running.

**Input:**
- Send a lock command and SIGTERM nearly simultaneously.

**Expected:**
- Response is published for the command.
- Service exits with code 0.

**Assertion pseudocode:**
```
proc = start_locking_service()
send_lock_command()
send_signal(proc, SIGTERM)
exit_code = wait_for_exit(proc, timeout=5s)
ASSERT exit_code == 0
response = get_signal("Vehicle.Command.Door.Response")
ASSERT response.status == "success"
```

## Property Test Cases

### TS-03-P1: Command Validation Completeness

**Property:** Property 1 from design.md
**Validates:** 03-REQ-2.1, 03-REQ-2.2, 03-REQ-2.3, 03-REQ-2.E1, 03-REQ-2.E2
**Type:** property
**Description:** Any string input either parses to a valid command or is rejected.

**For any:** Arbitrary string (including valid JSON, invalid JSON, empty string, random bytes)
**Invariant:** parse_command(input) returns Ok(valid LockCommand) or Err(reason).

**Assertion pseudocode:**
```
FOR ANY input IN arbitrary_strings:
    result = parse_command(input)
    IF result.is_ok():
        cmd = result.unwrap()
        ASSERT cmd.command_id.len() > 0
        ASSERT cmd.action IN [Lock, Unlock]
        ASSERT "driver" IN cmd.doors
    ELSE:
        ASSERT result.is_err()
```

### TS-03-P2: Safety Gate for Lock

**Property:** Property 2 from design.md
**Validates:** 03-REQ-3.1, 03-REQ-3.2, 03-REQ-3.3
**Type:** property
**Description:** Lock is allowed iff speed < 1.0 and door closed.

**For any:** speed in [0.0, 200.0], door_open in {true, false}
**Invariant:** check_safety returns Safe iff speed < 1.0 AND !door_open.

**Assertion pseudocode:**
```
FOR ANY speed IN float_range(0.0, 200.0), door_open IN bool:
    mock_broker.set_speed(speed)
    mock_broker.set_door_open(door_open)
    result = check_safety(&mock_broker)
    IF speed < 1.0 AND !door_open:
        ASSERT result == SafetyResult::Safe
    ELSE IF speed >= 1.0:
        ASSERT result == SafetyResult::VehicleMoving
    ELSE:
        ASSERT result == SafetyResult::DoorOpen
```

### TS-03-P3: Unlock Always Succeeds

**Property:** Property 3 from design.md
**Validates:** 03-REQ-3.4
**Type:** property
**Description:** Unlock commands always succeed regardless of speed/door state.

**For any:** speed in [0.0, 200.0], door_open in {true, false}
**Invariant:** process_unlock always returns success.

**Assertion pseudocode:**
```
FOR ANY speed IN float_range(0.0, 200.0), door_open IN bool:
    mock_broker.set_speed(speed)
    mock_broker.set_door_open(door_open)
    result = process_command(&mock_broker, unlock_cmd)
    ASSERT result.status == "success"
```

### TS-03-P4: State-Response Consistency

**Property:** Property 4 from design.md
**Validates:** 03-REQ-4.1, 03-REQ-4.2, 03-REQ-5.1
**Type:** property
**Description:** After a successful command, lock state matches the action.

**For any:** action in {Lock, Unlock}, with safe conditions for lock
**Invariant:** IsLocked state matches action after success response.

**Assertion pseudocode:**
```
FOR ANY action IN [Lock, Unlock]:
    mock_broker.set_speed(0.0)
    mock_broker.set_door_open(false)
    cmd = make_command(action)
    result = process_command(&mock_broker, cmd)
    ASSERT result.status == "success"
    IF action == Lock:
        ASSERT mock_broker.is_locked == true
    ELSE:
        ASSERT mock_broker.is_locked == false
```

### TS-03-P5: Idempotent Operations

**Property:** Property 5 from design.md
**Validates:** 03-REQ-4.E1, 03-REQ-4.E2
**Type:** property
**Description:** Repeating the same command produces success without state changes.

**For any:** action in {Lock, Unlock}, repeated N times (N in 1..5)
**Invariant:** All N responses are "success", state only set once.

**Assertion pseudocode:**
```
FOR ANY action IN [Lock, Unlock], n IN 1..5:
    mock_broker.reset()
    mock_broker.set_speed(0.0)
    mock_broker.set_door_open(false)
    FOR i IN 1..n:
        result = process_command(&mock_broker, make_command(action))
        ASSERT result.status == "success"
    ASSERT mock_broker.set_bool_calls <= 1
```

### TS-03-P6: Response Completeness

**Property:** Property 6 from design.md
**Validates:** 03-REQ-5.1, 03-REQ-5.2, 03-REQ-5.3
**Type:** property
**Description:** Every processed command produces exactly one response with required fields.

**For any:** valid command (lock or unlock) with any safety state
**Invariant:** Exactly one response with command_id, status, and timestamp.

**Assertion pseudocode:**
```
FOR ANY action IN [Lock, Unlock], speed IN [0.0, 50.0], door_open IN bool:
    mock_broker.reset()
    mock_broker.set_speed(speed)
    mock_broker.set_door_open(door_open)
    cmd = make_command(action, command_id="test-id")
    result = process_command(&mock_broker, cmd)
    response = parse_json(mock_broker.last_response)
    ASSERT response.command_id == "test-id"
    ASSERT response.status IN ["success", "failed"]
    ASSERT response.timestamp > 0
    ASSERT mock_broker.response_publish_count == 1
```

## Coverage Matrix

| Requirement | Test Spec Entry | Type |
|-------------|-----------------|------|
| 03-REQ-1.1 | TS-03-1 | integration |
| 03-REQ-1.2 | TS-03-2 | unit |
| 03-REQ-1.3 | TS-03-3 | unit |
| 03-REQ-1.E1 | TS-03-E1 | integration |
| 03-REQ-1.E2 | TS-03-E2 | integration |
| 03-REQ-2.1 | TS-03-4 | unit |
| 03-REQ-2.2 | TS-03-5 | unit |
| 03-REQ-2.3 | TS-03-6 | unit |
| 03-REQ-2.E1 | TS-03-E3 | unit |
| 03-REQ-2.E2 | TS-03-E4 | unit |
| 03-REQ-2.E3 | TS-03-E5 | unit |
| 03-REQ-3.1 | TS-03-7 | unit |
| 03-REQ-3.2 | TS-03-8 | unit |
| 03-REQ-3.3 | TS-03-9 | unit |
| 03-REQ-3.4 | TS-03-10 | unit |
| 03-REQ-3.E1 | TS-03-E6 | unit |
| 03-REQ-3.E2 | TS-03-E7 | unit |
| 03-REQ-4.1 | TS-03-11 | unit |
| 03-REQ-4.2 | TS-03-12 | unit |
| 03-REQ-4.3 | TS-03-13 | integration |
| 03-REQ-4.E1 | TS-03-E8 | unit |
| 03-REQ-4.E2 | TS-03-E9 | unit |
| 03-REQ-5.1 | TS-03-14 | unit |
| 03-REQ-5.2 | TS-03-15 | unit |
| 03-REQ-5.3 | TS-03-16 | unit |
| 03-REQ-5.E1 | TS-03-E10 | unit |
| 03-REQ-6.1 | TS-03-17 | integration |
| 03-REQ-6.2 | TS-03-18 | integration |
| 03-REQ-6.E1 | TS-03-E11 | integration |
| Property 1 | TS-03-P1 | property |
| Property 2 | TS-03-P2 | property |
| Property 3 | TS-03-P3 | property |
| Property 4 | TS-03-P4 | property |
| Property 5 | TS-03-P5 | property |
| Property 6 | TS-03-P6 | property |
