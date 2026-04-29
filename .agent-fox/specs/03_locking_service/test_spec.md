# Test Specification: LOCKING_SERVICE

## Overview

Tests for the LOCKING_SERVICE cover command parsing and validation, safety constraint enforcement, lock state management, response formatting, configurable endpoint, and service lifecycle. Unit tests use `MockBrokerClient` and live in co-located `#[cfg(test)]` modules within the Rust crate. Property tests use proptest and are marked `#[ignore]`. Integration tests live in `tests/locking-service/` as a standalone Go module and require a running DATA_BROKER container (Podman).

## Test Cases

### TS-03-1: Command Subscription on Startup

**Requirement:** 03-REQ-1.1
**Type:** integration
**Description:** Verify the service subscribes to `Vehicle.Command.Door.Lock` on startup and receives published commands.

**Preconditions:**
- DATA_BROKER container is running.
- LOCKING_SERVICE binary is started with correct `DATABROKER_ADDR`.

**Input:**
- Publish a lock command JSON to `Vehicle.Command.Door.Lock` via gRPC.

**Expected:**
- A response appears on `Vehicle.Command.Door.Response` within 5 seconds.

**Assertion pseudocode:**
```
start_databroker()
start_locking_service()
wait_for_log("locking-service ready")
grpc_set("Vehicle.Command.Door.Lock", valid_lock_json)
response = grpc_get("Vehicle.Command.Door.Response", timeout=5s)
ASSERT response != nil
```

### TS-03-2: Command Deserialization

**Requirement:** 03-REQ-2.1
**Type:** unit
**Description:** Verify a valid lock command JSON is deserialized into a `LockCommand` struct with all fields correctly populated.

**Preconditions:**
- None (pure function test).

**Input:**
- JSON string: `{"command_id":"abc-123","action":"lock","doors":["driver"],"source":"companion_app","vin":"WDB123","timestamp":1700000000}`

**Expected:**
- `command_id` == "abc-123", `action` == Lock, `doors` == ["driver"], optional fields populated.

**Assertion pseudocode:**
```
cmd = parse_command(json)
ASSERT cmd.is_ok()
ASSERT cmd.command_id == "abc-123"
ASSERT cmd.action == Action::Lock
ASSERT cmd.doors == ["driver"]
ASSERT cmd.source == Some("companion_app")
```

### TS-03-3: Configurable Databroker Address

**Requirement:** 03-REQ-7.1, 03-REQ-7.2
**Type:** unit
**Description:** Verify the service reads `DATABROKER_ADDR` from environment with default fallback.

**Preconditions:**
- None.

**Input:**
- Case 1: `DATABROKER_ADDR` not set.
- Case 2: `DATABROKER_ADDR` set to `http://10.0.0.5:55556`.

**Expected:**
- Case 1: returns `http://localhost:55556`.
- Case 2: returns `http://10.0.0.5:55556`.

**Assertion pseudocode:**
```
unset_env("DATABROKER_ADDR")
ASSERT get_databroker_addr() == "http://localhost:55556"
set_env("DATABROKER_ADDR", "http://10.0.0.5:55556")
ASSERT get_databroker_addr() == "http://10.0.0.5:55556"
```

### TS-03-4: Validate command_id Required

**Requirement:** 03-REQ-2.3
**Type:** unit
**Description:** Verify that an empty `command_id` is rejected with reason "invalid_command".

**Preconditions:**
- None.

**Input:**
- JSON: `{"command_id":"","action":"lock","doors":["driver"]}`

**Expected:**
- Validation fails with reason "invalid_command".

**Assertion pseudocode:**
```
cmd = parse_command(json).unwrap()
result = validate_command(cmd)
ASSERT result.is_err()
ASSERT result.unwrap_err().reason() == "invalid_command"
```

### TS-03-5: Validate Action Field

**Requirement:** 03-REQ-2.1
**Type:** unit
**Description:** Verify that an invalid action value (e.g. "toggle") is rejected.

**Preconditions:**
- None.

**Input:**
- JSON: `{"command_id":"x","action":"toggle","doors":["driver"]}`

**Expected:**
- `parse_command` returns error (serde cannot match "toggle" to Action enum).

**Assertion pseudocode:**
```
result = parse_command(json)
ASSERT result.is_err()
```

### TS-03-6: Validate Doors Field

**Requirement:** 03-REQ-2.2
**Type:** unit
**Description:** Verify that a non-"driver" door value is rejected with reason "unsupported_door".

**Preconditions:**
- None.

**Input:**
- JSON: `{"command_id":"x","action":"lock","doors":["passenger"]}`

**Expected:**
- Validation fails with reason "unsupported_door".

**Assertion pseudocode:**
```
cmd = parse_command(json).unwrap()
result = validate_command(cmd)
ASSERT result.is_err()
ASSERT result.unwrap_err().reason() == "unsupported_door"
```

### TS-03-7: Lock Rejected When Vehicle Moving

**Requirement:** 03-REQ-3.1
**Type:** unit
**Description:** Verify lock command is rejected with "vehicle_moving" when speed >= 1.0.

**Preconditions:**
- MockBrokerClient with speed = 50.0, door_open = false.

**Input:**
- Call `check_safety(&mock)`.

**Expected:**
- Returns `SafetyResult::VehicleMoving`.

**Assertion pseudocode:**
```
mock = MockBrokerClient::new().with_speed(50.0).with_door_open(false)
result = check_safety(&mock).await
ASSERT result == SafetyResult::VehicleMoving
```

### TS-03-8: Lock Rejected When Door Open

**Requirement:** 03-REQ-3.2
**Type:** unit
**Description:** Verify lock command is rejected with "door_open" when door is ajar and vehicle is stationary.

**Preconditions:**
- MockBrokerClient with speed = 0.0, door_open = true.

**Input:**
- Call `check_safety(&mock)`.

**Expected:**
- Returns `SafetyResult::DoorOpen`.

**Assertion pseudocode:**
```
mock = MockBrokerClient::new().with_speed(0.0).with_door_open(true)
result = check_safety(&mock).await
ASSERT result == SafetyResult::DoorOpen
```

### TS-03-9: Lock Allowed When Safe

**Requirement:** 03-REQ-3.3
**Type:** unit
**Description:** Verify lock command is allowed when speed < 1.0 and door is closed.

**Preconditions:**
- MockBrokerClient with speed = 0.0, door_open = false.

**Input:**
- Call `check_safety(&mock)`.

**Expected:**
- Returns `SafetyResult::Safe`.

**Assertion pseudocode:**
```
mock = MockBrokerClient::new().with_speed(0.0).with_door_open(false)
result = check_safety(&mock).await
ASSERT result == SafetyResult::Safe
```

### TS-03-10: Unlock Bypasses Safety

**Requirement:** 03-REQ-3.4
**Type:** unit
**Description:** Verify unlock succeeds regardless of speed and door state.

**Preconditions:**
- MockBrokerClient with speed = 100.0, door_open = true, locked = true.

**Input:**
- Process an unlock command.

**Expected:**
- Response status is "success".

**Assertion pseudocode:**
```
mock = MockBrokerClient::new().with_speed(100.0).with_door_open(true).with_locked(true)
lock_state = true
response = process_command(&mock, unlock_cmd, &mut lock_state).await
ASSERT parse_json(response)["status"] == "success"
```

### TS-03-11: Lock Sets IsLocked True

**Requirement:** 03-REQ-4.1
**Type:** unit
**Description:** Verify that a successful lock command sets IsLocked to true on the broker.

**Preconditions:**
- MockBrokerClient with speed = 0.0, door_open = false. lock_state = false.

**Input:**
- Process a lock command.

**Expected:**
- `set_bool` called with (SIGNAL_IS_LOCKED, true). lock_state is true. Response status is "success".

**Assertion pseudocode:**
```
mock = MockBrokerClient::new().with_speed(0.0).with_door_open(false)
lock_state = false
response = process_command(&mock, lock_cmd, &mut lock_state).await
ASSERT lock_state == true
ASSERT mock.set_bool_calls().contains(("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", true))
ASSERT parse_json(response)["status"] == "success"
```

### TS-03-12: Unlock Sets IsLocked False

**Requirement:** 03-REQ-4.2
**Type:** unit
**Description:** Verify that an unlock command sets IsLocked to false on the broker.

**Preconditions:**
- MockBrokerClient with locked = true. lock_state = true.

**Input:**
- Process an unlock command.

**Expected:**
- `set_bool` called with (SIGNAL_IS_LOCKED, false). lock_state is false. Response status is "success".

**Assertion pseudocode:**
```
mock = MockBrokerClient::new().with_speed(0.0).with_door_open(false).with_locked(true)
lock_state = true
response = process_command(&mock, unlock_cmd, &mut lock_state).await
ASSERT lock_state == false
ASSERT mock.set_bool_calls().contains(("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", false))
ASSERT parse_json(response)["status"] == "success"
```

### TS-03-13: Initial State Published on Startup

**Requirement:** 03-REQ-4.3
**Type:** integration
**Description:** Verify the service publishes IsLocked = false on startup.

**Preconditions:**
- DATA_BROKER container is running.

**Input:**
- Start LOCKING_SERVICE.

**Expected:**
- `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` is false in DATA_BROKER.

**Assertion pseudocode:**
```
start_databroker()
start_locking_service()
wait_for_log("locking-service ready")
result = grpc_get("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked")
ASSERT result.value == false
```

### TS-03-14: Success Response Format

**Requirement:** 03-REQ-5.1, 03-REQ-5.3
**Type:** unit
**Description:** Verify the success response JSON contains command_id, status "success", timestamp, and no reason field.

**Preconditions:**
- None.

**Input:**
- Call `success_response("abc-123")`.

**Expected:**
- Valid JSON with command_id="abc-123", status="success", timestamp > 0, no reason field.

**Assertion pseudocode:**
```
json = success_response("abc-123")
parsed = parse_json(json)
ASSERT parsed["command_id"] == "abc-123"
ASSERT parsed["status"] == "success"
ASSERT parsed["timestamp"] > 0
ASSERT parsed["reason"] is null or absent
```

### TS-03-15: Failure Response Format

**Requirement:** 03-REQ-5.2
**Type:** unit
**Description:** Verify the failure response JSON contains command_id, status "failed", reason, and timestamp.

**Preconditions:**
- None.

**Input:**
- Call `failure_response("abc-123", "vehicle_moving")`.

**Expected:**
- Valid JSON with command_id="abc-123", status="failed", reason="vehicle_moving", timestamp > 0.

**Assertion pseudocode:**
```
json = failure_response("abc-123", "vehicle_moving")
parsed = parse_json(json)
ASSERT parsed["command_id"] == "abc-123"
ASSERT parsed["status"] == "failed"
ASSERT parsed["reason"] == "vehicle_moving"
ASSERT parsed["timestamp"] > 0
```

### TS-03-16: Response Timestamp

**Requirement:** 03-REQ-5.1
**Type:** unit
**Description:** Verify the response timestamp is a valid Unix timestamp close to current time.

**Preconditions:**
- None.

**Input:**
- Record time before, call `success_response("x")`, record time after.

**Expected:**
- Timestamp is between before and after.

**Assertion pseudocode:**
```
before = unix_timestamp_now()
json = success_response("x")
after = unix_timestamp_now()
ts = parse_json(json)["timestamp"]
ASSERT ts >= before
ASSERT ts <= after
```

## Edge Case Tests

### TS-03-E1: DATA_BROKER Connection Retry

**Requirement:** 03-REQ-1.E1
**Type:** integration
**Description:** Verify the service retries connection to DATA_BROKER with exponential backoff and exits on failure.

**Preconditions:**
- No DATA_BROKER running.

**Input:**
- Start LOCKING_SERVICE with `DATABROKER_ADDR` pointing to a non-existent endpoint.

**Expected:**
- Service exits with non-zero code after retries.

**Assertion pseudocode:**
```
exit_code = start_locking_service(env={"DATABROKER_ADDR": "http://localhost:99999"})
ASSERT exit_code != 0
```

### TS-03-E2: Subscription Stream Interrupted

**Requirement:** 03-REQ-1.E2
**Type:** integration
**Description:** Verify the service attempts to resubscribe when the subscription stream is interrupted.

**Preconditions:**
- DATA_BROKER is running. LOCKING_SERVICE is connected and subscribed.

**Input:**
- Restart the DATA_BROKER while LOCKING_SERVICE is running.

**Expected:**
- Service logs a resubscribe warning.

**Assertion pseudocode:**
```
start_locking_service()
restart_databroker()
ASSERT logs_contain("resubscribing")
```

### TS-03-E3: Invalid JSON Payload

**Requirement:** 03-REQ-2.E1
**Type:** unit
**Description:** Verify that non-JSON payloads are discarded without publishing a response.

**Preconditions:**
- None.

**Input:**
- `parse_command("not valid json {{{")`.

**Expected:**
- Returns `Err(CommandError::InvalidJson(_))`.

**Assertion pseudocode:**
```
result = parse_command("not valid json {{{")
ASSERT result.is_err()
ASSERT matches!(result, Err(CommandError::InvalidJson(_)))
```

### TS-03-E4: Missing Required Field

**Requirement:** 03-REQ-2.E2
**Type:** unit
**Description:** Verify that a payload missing the `action` field is rejected.

**Preconditions:**
- None.

**Input:**
- JSON: `{"command_id":"x","doors":["driver"]}`

**Expected:**
- `parse_command` returns error.

**Assertion pseudocode:**
```
result = parse_command(json)
ASSERT result.is_err()
```

### TS-03-E5: Unsupported Door Value

**Requirement:** 03-REQ-2.2
**Type:** unit
**Description:** Verify that a non-"driver" door value results in "unsupported_door" rejection.

**Preconditions:**
- None.

**Input:**
- JSON: `{"command_id":"x","action":"lock","doors":["rear_left"]}`

**Expected:**
- Validation returns error with reason "unsupported_door".

**Assertion pseudocode:**
```
cmd = parse_command(json).unwrap()
result = validate_command(cmd)
ASSERT result.unwrap_err().reason() == "unsupported_door"
```

### TS-03-E6: Speed Signal Unset

**Requirement:** 03-REQ-3.E1
**Type:** unit
**Description:** Verify that an unset speed signal is treated as 0.0 (safe default).

**Preconditions:**
- MockBrokerClient with speed = None, door_open = false.

**Input:**
- Call `check_safety(&mock)`.

**Expected:**
- Returns `SafetyResult::Safe`.

**Assertion pseudocode:**
```
mock = MockBrokerClient::new().with_speed(None).with_door_open(Some(false))
result = check_safety(&mock).await
ASSERT result == SafetyResult::Safe
```

### TS-03-E7: Door Signal Unset

**Requirement:** 03-REQ-3.E2
**Type:** unit
**Description:** Verify that an unset door signal is treated as closed (safe default).

**Preconditions:**
- MockBrokerClient with speed = 0.0, door_open = None.

**Input:**
- Call `check_safety(&mock)`.

**Expected:**
- Returns `SafetyResult::Safe`.

**Assertion pseudocode:**
```
mock = MockBrokerClient::new().with_speed(Some(0.0)).with_door_open(None)
result = check_safety(&mock).await
ASSERT result == SafetyResult::Safe
```

### TS-03-E8: Lock When Already Locked (Idempotent)

**Requirement:** 03-REQ-4.E1
**Type:** unit
**Description:** Verify locking an already-locked door returns success without changing state.

**Preconditions:**
- MockBrokerClient with speed = 0.0, door_open = false, locked = true. lock_state = true.

**Input:**
- Process a lock command.

**Expected:**
- Response status "success". No `set_bool` calls.

**Assertion pseudocode:**
```
mock = MockBrokerClient::new().with_speed(0.0).with_door_open(false).with_locked(true)
lock_state = true
response = process_command(&mock, lock_cmd, &mut lock_state).await
ASSERT parse_json(response)["status"] == "success"
ASSERT mock.set_bool_calls().len() == 0
```

### TS-03-E9: Unlock When Already Unlocked (Idempotent)

**Requirement:** 03-REQ-4.E2
**Type:** unit
**Description:** Verify unlocking an already-unlocked door returns success without changing state.

**Preconditions:**
- MockBrokerClient with speed = 0.0, door_open = false. lock_state = false.

**Input:**
- Process an unlock command.

**Expected:**
- Response status "success". No `set_bool` calls.

**Assertion pseudocode:**
```
mock = MockBrokerClient::new().with_speed(0.0).with_door_open(false)
lock_state = false
response = process_command(&mock, unlock_cmd, &mut lock_state).await
ASSERT parse_json(response)["status"] == "success"
ASSERT mock.set_bool_calls().len() == 0
```

### TS-03-E10: Response Publish Failure

**Requirement:** 03-REQ-5.E1
**Type:** unit
**Description:** Verify the service continues processing after a response publish failure.

**Preconditions:**
- MockBrokerClient configured to fail next `set_string` call.

**Input:**
- Process a lock command (response publish fails).
- Process a second command.

**Expected:**
- Second command returns success.

**Assertion pseudocode:**
```
mock = MockBrokerClient::new().with_speed(0.0).with_door_open(false)
mock.fail_next_set_string()
lock_state = false
process_command(&mock, lock_cmd, &mut lock_state).await
response2 = process_command(&mock, unlock_cmd, &mut lock_state).await
ASSERT parse_json(response2)["status"] == "success"
```

### TS-03-E11: Empty Command ID

**Requirement:** 03-REQ-2.E3
**Type:** unit
**Description:** Verify that an empty command_id field is rejected with reason "invalid_command".

**Preconditions:**
- None.

**Input:**
- JSON: `{"command_id":"","action":"lock","doors":["driver"]}`

**Expected:**
- Validation fails with reason "invalid_command".

**Assertion pseudocode:**
```
cmd = parse_command(json).unwrap()
result = validate_command(cmd)
ASSERT result.is_err()
ASSERT result.unwrap_err().reason() == "invalid_command"
```

### TS-03-E12: SIGTERM During Command Processing

**Requirement:** 03-REQ-6.E1
**Type:** integration
**Description:** Verify that SIGTERM received during command processing allows the current command to complete before exiting.

**Preconditions:**
- DATA_BROKER and LOCKING_SERVICE running.

**Input:**
- Start processing a lock command (inject delay if needed for test timing).
- Send SIGTERM to the LOCKING_SERVICE process.

**Expected:**
- Response for the in-flight command is published to Vehicle.Command.Door.Response.
- Service exits with code 0.

**Assertion pseudocode:**
```
start_locking_service()
grpc_set("Vehicle.Command.Door.Lock", lock_json)
send_signal(locking_service_pid, SIGTERM)
response = grpc_get("Vehicle.Command.Door.Response", timeout=5s)
ASSERT response != nil
ASSERT exit_code == 0
```

## Property Test Cases

### TS-03-P1: Command Validation Completeness

**Property:** Property 1 from design.md
**Validates:** 03-REQ-2.1, 03-REQ-2.2, 03-REQ-2.3, 03-REQ-2.E1, 03-REQ-2.E2, 03-REQ-2.E3
**Type:** property
**Description:** Any string input either parses to a valid LockCommand or is rejected.

**For any:** arbitrary string input
**Invariant:** `parse_command` returns `Ok(cmd)` where `validate_command(cmd)` is Ok only if command_id is non-empty, action is Lock or Unlock, and doors contains "driver"; otherwise it returns `Err`.

**Assertion pseudocode:**
```
FOR ANY input IN arbitrary_strings:
    match parse_command(input):
        Ok(cmd) => match validate_command(cmd):
            Ok(()) => ASSERT !cmd.command_id.is_empty()
                      ASSERT cmd.action IN {Lock, Unlock}
                      ASSERT "driver" IN cmd.doors
            Err(_) => pass  // rejected is fine
        Err(_) => pass  // rejected is fine
```

### TS-03-P2: Safety Gate for Lock

**Property:** Property 2 from design.md
**Validates:** 03-REQ-3.1, 03-REQ-3.2, 03-REQ-3.3
**Type:** property
**Description:** Lock is allowed iff speed < 1.0 and door closed. Speed check takes priority.

**For any:** speed in [0.0, 200.0], door_open in {true, false}
**Invariant:** If speed < 1.0 and !door_open then Safe; if speed >= 1.0 then VehicleMoving; else DoorOpen.

**Assertion pseudocode:**
```
FOR ANY speed IN 0.0..200.0, door_open IN {true, false}:
    mock = MockBrokerClient::new().with_speed(speed).with_door_open(door_open)
    result = check_safety(&mock).await
    IF speed < 1.0 AND !door_open: ASSERT result == Safe
    ELSE IF speed >= 1.0: ASSERT result == VehicleMoving
    ELSE: ASSERT result == DoorOpen
```

### TS-03-P3: Unlock Always Succeeds

**Property:** Property 3 from design.md
**Validates:** 03-REQ-3.4
**Type:** property
**Description:** Unlock succeeds regardless of speed and door state.

**For any:** speed in [0.0, 200.0], door_open in {true, false}
**Invariant:** process_command with unlock action returns status "success".

**Assertion pseudocode:**
```
FOR ANY speed IN 0.0..200.0, door_open IN {true, false}:
    mock = MockBrokerClient::new().with_speed(speed).with_door_open(door_open).with_locked(true)
    lock_state = true
    response = process_command(&mock, unlock_cmd, &mut lock_state).await
    ASSERT parse_json(response)["status"] == "success"
```

### TS-03-P4: State-Response Consistency

**Property:** Property 4 from design.md
**Validates:** 03-REQ-4.1, 03-REQ-4.2, 03-REQ-5.1
**Type:** property
**Description:** After a successful command, lock_state matches the requested action.

**For any:** action in {Lock, Unlock} with safety conditions met
**Invariant:** If lock then lock_state == true; if unlock then lock_state == false. Response status is "success".

**Assertion pseudocode:**
```
FOR ANY action_is_lock IN {true, false}:
    mock = MockBrokerClient::new().with_speed(0.0).with_door_open(false)
    lock_state = false
    cmd = make_cmd(if action_is_lock then Lock else Unlock)
    response = process_command(&mock, cmd, &mut lock_state).await
    ASSERT parse_json(response)["status"] == "success"
    IF action_is_lock: ASSERT lock_state == true
    ELSE: ASSERT lock_state == false
```

### TS-03-P5: Idempotent Operations

**Property:** Property 5 from design.md
**Validates:** 03-REQ-4.E1, 03-REQ-4.E2
**Type:** property
**Description:** Repeating the same command N times results in at most one state write.

**For any:** action in {Lock, Unlock}, N in [1, 5)
**Invariant:** All N responses are "success". `set_bool` called at most once.

**Assertion pseudocode:**
```
FOR ANY action_is_lock IN {true, false}, n IN 1..5:
    mock = MockBrokerClient::new().with_speed(0.0).with_door_open(false)
    lock_state = false
    FOR i IN 0..n:
        response = process_command(&mock, cmd, &mut lock_state).await
        ASSERT parse_json(response)["status"] == "success"
    ASSERT mock.set_bool_calls().len() <= 1
```

### TS-03-P6: Response Completeness

**Property:** Property 6 from design.md
**Validates:** 03-REQ-5.1, 03-REQ-5.2, 03-REQ-5.3
**Type:** property
**Description:** Every processed command produces exactly one response with required fields.

**For any:** action in {Lock, Unlock}, speed in {0.0, 50.0}, door_open in {true, false}
**Invariant:** Response contains command_id, status in {"success", "failed"}, timestamp > 0. Exactly one `set_string` call to SIGNAL_RESPONSE.

**Assertion pseudocode:**
```
FOR ANY action, speed, door_open:
    mock = MockBrokerClient::new().with_speed(speed).with_door_open(door_open)
    lock_state = false
    response = process_command(&mock, cmd, &mut lock_state).await
    parsed = parse_json(response)
    ASSERT parsed["command_id"] == cmd.command_id
    ASSERT parsed["status"] IN {"success", "failed"}
    ASSERT parsed["timestamp"] > 0
    ASSERT mock.set_string_calls().len() == 1
```

## Integration Smoke Tests

### TS-03-SMOKE-1: Lock Happy Path

**Type:** integration
**Description:** End-to-end lock: set speed=0 and door=closed, publish lock command, verify IsLocked=true and success response.

**Preconditions:**
- DATA_BROKER and LOCKING_SERVICE running.

**Input:**
- Set Vehicle.Speed = 0.0, Vehicle.Cabin.Door.Row1.DriverSide.IsOpen = false.
- Publish lock command to Vehicle.Command.Door.Lock.

**Expected:**
- Vehicle.Cabin.Door.Row1.DriverSide.IsLocked == true.
- Vehicle.Command.Door.Response contains status "success".

**Assertion pseudocode:**
```
grpc_set("Vehicle.Speed", 0.0)
grpc_set("Vehicle.Cabin.Door.Row1.DriverSide.IsOpen", false)
grpc_set("Vehicle.Command.Door.Lock", lock_json)
sleep(1s)
ASSERT grpc_get("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked").value == true
resp = parse_json(grpc_get("Vehicle.Command.Door.Response").value)
ASSERT resp["status"] == "success"
```

### TS-03-SMOKE-2: Unlock Happy Path

**Type:** integration
**Description:** End-to-end unlock after lock: verify IsLocked=false and success response.

**Preconditions:**
- DATA_BROKER and LOCKING_SERVICE running. Door is locked (from TS-03-SMOKE-1 or setup).

**Input:**
- Publish unlock command to Vehicle.Command.Door.Lock.

**Expected:**
- Vehicle.Cabin.Door.Row1.DriverSide.IsLocked == false.
- Vehicle.Command.Door.Response contains status "success".

**Assertion pseudocode:**
```
grpc_set("Vehicle.Command.Door.Lock", unlock_json)
sleep(1s)
ASSERT grpc_get("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked").value == false
resp = parse_json(grpc_get("Vehicle.Command.Door.Response").value)
ASSERT resp["status"] == "success"
```

### TS-03-SMOKE-3: Lock Rejected (Vehicle Moving)

**Type:** integration
**Description:** End-to-end lock rejection due to vehicle speed.

**Preconditions:**
- DATA_BROKER and LOCKING_SERVICE running.

**Input:**
- Set Vehicle.Speed = 50.0.
- Publish lock command.

**Expected:**
- Vehicle.Command.Door.Response contains status "failed", reason "vehicle_moving".
- Vehicle.Cabin.Door.Row1.DriverSide.IsLocked remains false.

**Assertion pseudocode:**
```
grpc_set("Vehicle.Speed", 50.0)
grpc_set("Vehicle.Command.Door.Lock", lock_json)
sleep(1s)
resp = parse_json(grpc_get("Vehicle.Command.Door.Response").value)
ASSERT resp["status"] == "failed"
ASSERT resp["reason"] == "vehicle_moving"
ASSERT grpc_get("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked").value == false
```

## Coverage Matrix

| Requirement | Test Spec Entry | Type |
|-------------|-----------------|------|
| 03-REQ-1.1 | TS-03-1 | integration |
| 03-REQ-1.2 | TS-03-1 | integration |
| 03-REQ-1.3 | (sequential processing verified via TS-03-SMOKE-1, TS-03-SMOKE-2) | integration |
| 03-REQ-1.E1 | TS-03-E1 | integration |
| 03-REQ-1.E2 | TS-03-E2 | integration |
| 03-REQ-2.1 | TS-03-2, TS-03-5 | unit |
| 03-REQ-2.2 | TS-03-6, TS-03-E5 | unit |
| 03-REQ-2.3 | TS-03-4 | unit |
| 03-REQ-2.4 | TS-03-2 | unit |
| 03-REQ-2.E1 | TS-03-E3 | unit |
| 03-REQ-2.E2 | TS-03-E4 | unit |
| 03-REQ-2.E3 | TS-03-E11 | unit |
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
| 03-REQ-5.1 | TS-03-14, TS-03-16 | unit |
| 03-REQ-5.2 | TS-03-15 | unit |
| 03-REQ-5.3 | TS-03-14 | unit |
| 03-REQ-5.E1 | TS-03-E10 | unit |
| 03-REQ-6.1 | (verified via integration test process exit code) | integration |
| 03-REQ-6.2 | TS-03-1 (startup log) | integration |
| 03-REQ-6.E1 | TS-03-E12 | integration |
| 03-REQ-7.1 | TS-03-3 | unit |
| 03-REQ-7.2 | TS-03-3 | unit |
| Property 1 | TS-03-P1 | property |
| Property 2 | TS-03-P2 | property |
| Property 3 | TS-03-P3 | property |
| Property 4 | TS-03-P4 | property |
| Property 5 | TS-03-P5 | property |
| Property 6 | TS-03-P6 | property |
