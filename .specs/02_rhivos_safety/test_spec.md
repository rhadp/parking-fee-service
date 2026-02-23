# Test Specification: RHIVOS Safety Partition (Phase 2.1)

## Overview

This test specification translates every acceptance criterion, correctness
property, and edge case from the requirements and design documents into
concrete, executable test contracts. Tests are organized into three categories:

- **Acceptance criterion tests (TS-02-N):** One per acceptance criterion.
  Implemented as Rust integration tests in `rhivos/tests/` and as structural
  assertion tests in a Go test module (`tests/safety/`).
- **Property tests (TS-02-PN):** One per correctness property. Verify
  invariants across the safety partition.
- **Edge case tests (TS-02-EN):** One per edge case requirement. Verify
  error handling and boundary behavior.

Integration tests require running infrastructure (`make infra-up`). Unit
tests and structural tests do not require infrastructure.

## Test Cases

### TS-02-1: VSS overlay defines custom command signals

**Requirement:** 02-REQ-1.1
**Type:** unit
**Description:** Verify the VSS overlay file defines Vehicle.Command.Door.Lock
and Vehicle.Command.Door.Response as string-typed signals.

**Preconditions:**
- VSS overlay file exists at `infra/kuksa/vss-overlay.json`.

**Input:**
- File: `infra/kuksa/vss-overlay.json`

**Expected:**
- File contains definitions for Vehicle.Command.Door.Lock (string) and
  Vehicle.Command.Door.Response (string).

**Assertion pseudocode:**
```
content = read_file("infra/kuksa/vss-overlay.json")
parsed = json_parse(content)
ASSERT parsed["Vehicle"]["children"]["Command"]["children"]["Door"]["children"]["Lock"]["datatype"] == "string"
ASSERT parsed["Vehicle"]["children"]["Command"]["children"]["Door"]["children"]["Response"]["datatype"] == "string"
```

---

### TS-02-2: Standard VSS signals are accessible in DATA_BROKER

**Requirement:** 02-REQ-1.2
**Type:** integration
**Description:** Verify DATA_BROKER serves all required standard VSS signals
after startup.

**Preconditions:**
- DATA_BROKER running via `make infra-up`.

**Input:**
- gRPC GetMetadata requests for each standard signal.

**Expected:**
- Each signal exists and has the correct data type.

**Assertion pseudocode:**
```
signals = {
    "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked": "bool",
    "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen": "bool",
    "Vehicle.CurrentLocation.Latitude": "double",
    "Vehicle.CurrentLocation.Longitude": "double",
    "Vehicle.Speed": "float",
}
FOR path, expected_type IN signals:
    metadata = databroker_get_metadata(path)
    ASSERT metadata.exists == true
    ASSERT metadata.data_type == expected_type
```

---

### TS-02-3: DATA_BROKER UDS endpoint reachable

**Requirement:** 02-REQ-1.3
**Type:** integration
**Description:** Verify DATA_BROKER's gRPC interface is reachable via Unix
Domain Socket.

**Preconditions:**
- DATA_BROKER running via `make infra-up`.

**Input:**
- gRPC connection attempt to UDS path.

**Expected:**
- Connection succeeds. A simple GetServerInfo call returns without error.

**Assertion pseudocode:**
```
client = grpc_connect_uds("/tmp/kuksa-databroker.sock")
result = client.get_server_info()
ASSERT result.is_ok()
```

---

### TS-02-4: DATA_BROKER network TCP endpoint reachable

**Requirement:** 02-REQ-1.4
**Type:** integration
**Description:** Verify DATA_BROKER's gRPC interface is reachable via network
TCP.

**Preconditions:**
- DATA_BROKER running via `make infra-up`.

**Input:**
- gRPC connection attempt to TCP port 55555.

**Expected:**
- Connection succeeds. A simple GetServerInfo call returns without error.

**Assertion pseudocode:**
```
client = grpc_connect_tcp("http://localhost:55555")
result = client.get_server_info()
ASSERT result.is_ok()
```

---

### TS-02-5: DATA_BROKER bearer token access control

**Requirement:** 02-REQ-1.5
**Type:** integration
**Description:** Verify DATA_BROKER enforces bearer token authentication
for write operations.

**Preconditions:**
- DATA_BROKER running with token configuration.

**Input:**
- Write request with valid token, write request without token.

**Expected:**
- Valid token: write succeeds.
- Missing token: write rejected with permission error.

**Assertion pseudocode:**
```
client = grpc_connect("http://localhost:55555")
result_ok = client.set_value("Vehicle.Speed", 0.0, token="speed-sensor-token")
ASSERT result_ok.is_ok()
result_fail = client.set_value("Vehicle.Speed", 0.0, token=None)
ASSERT result_fail.is_err()
ASSERT result_fail.error_code == PERMISSION_DENIED
```

---

### TS-02-6: LOCKING_SERVICE subscribes to command signals

**Requirement:** 02-REQ-2.1
**Type:** integration
**Description:** Verify LOCKING_SERVICE subscribes to Vehicle.Command.Door.Lock
from DATA_BROKER via UDS.

**Preconditions:**
- DATA_BROKER running. LOCKING_SERVICE running.

**Input:**
- Write a lock command to Vehicle.Command.Door.Lock via DATA_BROKER.

**Expected:**
- LOCKING_SERVICE receives the command and writes a response to
  Vehicle.Command.Door.Response.

**Assertion pseudocode:**
```
command = {"command_id": "test-1", "action": "lock", "doors": ["driver"], "source": "test", "vin": "VIN12345", "timestamp": now()}
databroker_set("Vehicle.Command.Door.Lock", json_encode(command))
response = databroker_subscribe_wait("Vehicle.Command.Door.Response", timeout=5s)
ASSERT response.is_some()
parsed = json_parse(response.value)
ASSERT parsed["command_id"] == "test-1"
```

---

### TS-02-7: LOCKING_SERVICE parses command JSON

**Requirement:** 02-REQ-2.2
**Type:** unit
**Description:** Verify LOCKING_SERVICE correctly parses command JSON
payloads.

**Preconditions:**
- None (unit test, no infrastructure required).

**Input:**
- Valid JSON: `{"command_id":"abc","action":"lock","doors":["driver"],"source":"companion_app","vin":"VIN12345","timestamp":1700000000}`

**Expected:**
- Parsed struct has correct field values.

**Assertion pseudocode:**
```
result = parse_lock_command(json_input)
ASSERT result.is_ok()
cmd = result.unwrap()
ASSERT cmd.command_id == "abc"
ASSERT cmd.action == LockAction::Lock
ASSERT cmd.doors == ["driver"]
```

---

### TS-02-8: LOCKING_SERVICE executes lock action

**Requirement:** 02-REQ-2.3
**Type:** integration
**Description:** Verify LOCKING_SERVICE locks doors when action is "lock"
and safety constraints are met.

**Preconditions:**
- DATA_BROKER running. LOCKING_SERVICE running.
- Vehicle.Speed = 0. IsOpen = false.

**Input:**
- Lock command with action "lock".

**Expected:**
- Vehicle.Cabin.Door.Row1.DriverSide.IsLocked becomes true.

**Assertion pseudocode:**
```
set_speed(0.0)
set_door_open(false)
send_lock_command("lock")
wait_for_response(timeout=5s)
locked = databroker_get("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked")
ASSERT locked == true
```

---

### TS-02-9: LOCKING_SERVICE executes unlock action

**Requirement:** 02-REQ-2.4
**Type:** integration
**Description:** Verify LOCKING_SERVICE unlocks doors when action is "unlock"
and safety constraints are met.

**Preconditions:**
- DATA_BROKER running. LOCKING_SERVICE running.
- Vehicle.Speed = 0. Door currently locked.

**Input:**
- Lock command with action "unlock".

**Expected:**
- Vehicle.Cabin.Door.Row1.DriverSide.IsLocked becomes false.

**Assertion pseudocode:**
```
set_speed(0.0)
set_door_open(false)
set_locked(true)
send_lock_command("unlock")
wait_for_response(timeout=5s)
locked = databroker_get("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked")
ASSERT locked == false
```

---

### TS-02-10: LOCKING_SERVICE rejects lock when vehicle moving

**Requirement:** 02-REQ-3.1
**Type:** integration
**Description:** Verify LOCKING_SERVICE rejects a lock command when
Vehicle.Speed > 0.

**Preconditions:**
- DATA_BROKER running. LOCKING_SERVICE running.
- Vehicle.Speed > 0 (e.g., 30.0).

**Input:**
- Lock command with action "lock".

**Expected:**
- Response has status "failed", reason "vehicle_moving".
- IsLocked is NOT changed.

**Assertion pseudocode:**
```
initial_locked = databroker_get("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked")
set_speed(30.0)
send_lock_command("lock")
response = wait_for_response(timeout=5s)
ASSERT response["status"] == "failed"
ASSERT response["reason"] == "vehicle_moving"
current_locked = databroker_get("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked")
ASSERT current_locked == initial_locked
```

---

### TS-02-11: LOCKING_SERVICE rejects lock when door open

**Requirement:** 02-REQ-3.2
**Type:** integration
**Description:** Verify LOCKING_SERVICE rejects a lock command when
the door is open.

**Preconditions:**
- DATA_BROKER running. LOCKING_SERVICE running.
- Vehicle.Speed = 0. IsOpen = true.

**Input:**
- Lock command with action "lock".

**Expected:**
- Response has status "failed", reason "door_open".
- IsLocked is NOT changed.

**Assertion pseudocode:**
```
initial_locked = databroker_get("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked")
set_speed(0.0)
set_door_open(true)
send_lock_command("lock")
response = wait_for_response(timeout=5s)
ASSERT response["status"] == "failed"
ASSERT response["reason"] == "door_open"
current_locked = databroker_get("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked")
ASSERT current_locked == initial_locked
```

---

### TS-02-12: LOCKING_SERVICE rejects unlock when vehicle moving

**Requirement:** 02-REQ-3.3
**Type:** integration
**Description:** Verify LOCKING_SERVICE rejects an unlock command when
Vehicle.Speed > 0.

**Preconditions:**
- DATA_BROKER running. LOCKING_SERVICE running.
- Vehicle.Speed > 0. Door currently locked.

**Input:**
- Lock command with action "unlock".

**Expected:**
- Response has status "failed", reason "vehicle_moving".
- IsLocked remains true.

**Assertion pseudocode:**
```
set_locked(true)
set_speed(15.0)
send_lock_command("unlock")
response = wait_for_response(timeout=5s)
ASSERT response["status"] == "failed"
ASSERT response["reason"] == "vehicle_moving"
locked = databroker_get("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked")
ASSERT locked == true
```

---

### TS-02-13: LOCKING_SERVICE writes failure response with reason

**Requirement:** 02-REQ-3.4
**Type:** integration
**Description:** Verify the failure response includes the specific constraint
violation reason.

**Preconditions:**
- DATA_BROKER running. LOCKING_SERVICE running.

**Input:**
- Lock command while vehicle is moving (speed > 0).

**Expected:**
- Response JSON contains status "failed" and a non-empty reason field.

**Assertion pseudocode:**
```
set_speed(10.0)
send_lock_command("lock")
response = wait_for_response(timeout=5s)
parsed = json_parse(response)
ASSERT parsed["status"] == "failed"
ASSERT parsed["reason"] is not empty
ASSERT parsed["command_id"] is not empty
```

---

### TS-02-14: LOCKING_SERVICE writes success response and lock state

**Requirement:** 02-REQ-3.5
**Type:** integration
**Description:** Verify successful lock command writes both lock state and
success response.

**Preconditions:**
- DATA_BROKER running. LOCKING_SERVICE running.
- Vehicle.Speed = 0. IsOpen = false.

**Input:**
- Lock command with action "lock".

**Expected:**
- IsLocked = true. Response has status "success" and matching command_id.

**Assertion pseudocode:**
```
set_speed(0.0)
set_door_open(false)
command_id = uuid()
send_lock_command("lock", command_id=command_id)
response = wait_for_response(timeout=5s)
parsed = json_parse(response)
ASSERT parsed["status"] == "success"
ASSERT parsed["command_id"] == command_id
locked = databroker_get("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked")
ASSERT locked == true
```

---

### TS-02-15: CLOUD_GATEWAY_CLIENT connects to MQTT broker

**Requirement:** 02-REQ-4.1
**Type:** integration
**Description:** Verify CLOUD_GATEWAY_CLIENT connects to the configured
MQTT broker.

**Preconditions:**
- Mosquitto running on :1883.

**Input:**
- Start CLOUD_GATEWAY_CLIENT with default MQTT configuration.

**Expected:**
- CLOUD_GATEWAY_CLIENT establishes connection (no connection error in logs).

**Assertion pseudocode:**
```
process = start_cloud_gateway_client()
sleep(3s)
ASSERT process.is_running()
ASSERT process.stderr NOT contains "connection refused"
ASSERT process.stderr NOT contains "MQTT error"
stop(process)
```

---

### TS-02-16: CLOUD_GATEWAY_CLIENT subscribes to command topic

**Requirement:** 02-REQ-4.2
**Type:** integration
**Description:** Verify CLOUD_GATEWAY_CLIENT subscribes to
`vehicles/{vin}/commands`.

**Preconditions:**
- Mosquitto and DATA_BROKER running. CLOUD_GATEWAY_CLIENT running.

**Input:**
- Publish a command to `vehicles/VIN12345/commands` via mosquitto_pub.

**Expected:**
- The command is written to Vehicle.Command.Door.Lock in DATA_BROKER.

**Assertion pseudocode:**
```
command = {"command_id": "mqtt-test", "action": "lock", "doors": ["driver"], "source": "test", "vin": "VIN12345", "timestamp": now()}
mqtt_publish("vehicles/VIN12345/commands", json_encode(command))
sleep(2s)
value = databroker_get("Vehicle.Command.Door.Lock")
ASSERT value is not empty
parsed = json_parse(value)
ASSERT parsed["command_id"] == "mqtt-test"
```

---

### TS-02-17: CLOUD_GATEWAY_CLIENT writes validated command to DATA_BROKER

**Requirement:** 02-REQ-4.3
**Type:** integration
**Description:** Verify CLOUD_GATEWAY_CLIENT writes validated MQTT commands
to DATA_BROKER.

**Preconditions:**
- Full infrastructure running. CLOUD_GATEWAY_CLIENT running.

**Input:**
- Valid lock command via MQTT.

**Expected:**
- Exact command payload appears in Vehicle.Command.Door.Lock.

**Assertion pseudocode:**
```
command = {"command_id": "relay-test", "action": "unlock", "doors": ["driver"], "source": "companion_app", "vin": "VIN12345", "timestamp": 1700000000}
mqtt_publish("vehicles/VIN12345/commands", json_encode(command))
sleep(2s)
value = databroker_get("Vehicle.Command.Door.Lock")
parsed = json_parse(value)
ASSERT parsed["command_id"] == "relay-test"
ASSERT parsed["action"] == "unlock"
```

---

### TS-02-18: CLOUD_GATEWAY_CLIENT relays command response to MQTT

**Requirement:** 02-REQ-4.4
**Type:** integration
**Description:** Verify CLOUD_GATEWAY_CLIENT publishes command responses
from DATA_BROKER to MQTT.

**Preconditions:**
- Full infrastructure running. CLOUD_GATEWAY_CLIENT and LOCKING_SERVICE running.

**Input:**
- Send a lock command via MQTT. Wait for LOCKING_SERVICE to process.

**Expected:**
- Response appears on `vehicles/VIN12345/command_responses` MQTT topic.

**Assertion pseudocode:**
```
set_speed(0.0)
set_door_open(false)
mqtt_subscribe("vehicles/VIN12345/command_responses")
command = {"command_id": "e2e-test", "action": "lock", "doors": ["driver"], "source": "test", "vin": "VIN12345", "timestamp": now()}
mqtt_publish("vehicles/VIN12345/commands", json_encode(command))
response = mqtt_wait_for_message("vehicles/VIN12345/command_responses", timeout=10s)
ASSERT response is not empty
parsed = json_parse(response)
ASSERT parsed["command_id"] == "e2e-test"
ASSERT parsed["status"] == "success"
```

---

### TS-02-19: CLOUD_GATEWAY_CLIENT subscribes to vehicle state signals

**Requirement:** 02-REQ-5.1
**Type:** integration
**Description:** Verify CLOUD_GATEWAY_CLIENT subscribes to state signals
in DATA_BROKER.

**Preconditions:**
- DATA_BROKER and Mosquitto running. CLOUD_GATEWAY_CLIENT running.

**Input:**
- Write a signal value to DATA_BROKER (e.g., set speed via mock sensor).

**Expected:**
- Telemetry message appears on MQTT topic.

**Assertion pseudocode:**
```
mqtt_subscribe("vehicles/VIN12345/telemetry")
set_speed(42.0)
message = mqtt_wait_for_message("vehicles/VIN12345/telemetry", timeout=10s)
ASSERT message is not empty
parsed = json_parse(message)
ASSERT any signal in parsed["signals"] has path containing "Speed" and value == 42.0
```

---

### TS-02-20: CLOUD_GATEWAY_CLIENT publishes telemetry on signal change

**Requirement:** 02-REQ-5.2
**Type:** integration
**Description:** Verify CLOUD_GATEWAY_CLIENT publishes telemetry when
a subscribed signal changes.

**Preconditions:**
- Full infrastructure and CLOUD_GATEWAY_CLIENT running.

**Input:**
- Change multiple signals sequentially.

**Expected:**
- Each change produces a telemetry message on MQTT.

**Assertion pseudocode:**
```
mqtt_subscribe("vehicles/VIN12345/telemetry")
set_location(48.1351, 11.5820)
msg1 = mqtt_wait_for_message("vehicles/VIN12345/telemetry", timeout=10s)
ASSERT msg1 is not empty
set_speed(60.0)
msg2 = mqtt_wait_for_message("vehicles/VIN12345/telemetry", timeout=10s)
ASSERT msg2 is not empty
```

---

### TS-02-21: LOCATION_SENSOR CLI writes latitude and longitude

**Requirement:** 02-REQ-6.1
**Type:** integration
**Description:** Verify LOCATION_SENSOR CLI tool writes location signals
to DATA_BROKER.

**Preconditions:**
- DATA_BROKER running.

**Input:**
- Run: `location-sensor --lat 48.1351 --lon 11.5820`

**Expected:**
- Vehicle.CurrentLocation.Latitude = 48.1351
- Vehicle.CurrentLocation.Longitude = 11.5820

**Assertion pseudocode:**
```
result = exec("location-sensor --lat 48.1351 --lon 11.5820")
ASSERT result.exit_code == 0
lat = databroker_get("Vehicle.CurrentLocation.Latitude")
lon = databroker_get("Vehicle.CurrentLocation.Longitude")
ASSERT abs(lat - 48.1351) < 0.0001
ASSERT abs(lon - 11.5820) < 0.0001
```

---

### TS-02-22: SPEED_SENSOR CLI writes speed

**Requirement:** 02-REQ-6.2
**Type:** integration
**Description:** Verify SPEED_SENSOR CLI tool writes speed signal to
DATA_BROKER.

**Preconditions:**
- DATA_BROKER running.

**Input:**
- Run: `speed-sensor --speed 55.5`

**Expected:**
- Vehicle.Speed = 55.5

**Assertion pseudocode:**
```
result = exec("speed-sensor --speed 55.5")
ASSERT result.exit_code == 0
speed = databroker_get("Vehicle.Speed")
ASSERT abs(speed - 55.5) < 0.1
```

---

### TS-02-23: DOOR_SENSOR CLI writes door state

**Requirement:** 02-REQ-6.3
**Type:** integration
**Description:** Verify DOOR_SENSOR CLI tool writes door open/closed state
to DATA_BROKER.

**Preconditions:**
- DATA_BROKER running.

**Input:**
- Run: `door-sensor --open true`

**Expected:**
- Vehicle.Cabin.Door.Row1.DriverSide.IsOpen = true

**Assertion pseudocode:**
```
result = exec("door-sensor --open true")
ASSERT result.exit_code == 0
is_open = databroker_get("Vehicle.Cabin.Door.Row1.DriverSide.IsOpen")
ASSERT is_open == true

result2 = exec("door-sensor --open false")
ASSERT result2.exit_code == 0
is_open2 = databroker_get("Vehicle.Cabin.Door.Row1.DriverSide.IsOpen")
ASSERT is_open2 == false
```

---

### TS-02-24: Mock sensor tools exit 0 on success

**Requirement:** 02-REQ-6.4
**Type:** integration
**Description:** Verify each mock sensor tool exits with code 0 after
successfully writing a value.

**Preconditions:**
- DATA_BROKER running.

**Input:**
- Run each sensor with valid arguments.

**Expected:**
- Exit code 0 for each.

**Assertion pseudocode:**
```
ASSERT exec("speed-sensor --speed 0").exit_code == 0
ASSERT exec("door-sensor --open false").exit_code == 0
ASSERT exec("location-sensor --lat 0 --lon 0").exit_code == 0
```

---

### TS-02-25: Mock sensor tools show usage without arguments

**Requirement:** 02-REQ-6.5
**Type:** unit
**Description:** Verify each mock sensor tool displays usage when run
without required arguments.

**Preconditions:**
- Mock sensor binaries built.

**Input:**
- Run each sensor tool without arguments.

**Expected:**
- Non-zero exit code. Output contains usage information.

**Assertion pseudocode:**
```
FOR EACH tool IN ["speed-sensor", "door-sensor", "location-sensor"]:
    result = exec(tool)
    ASSERT result.exit_code != 0
    ASSERT result.output contains "Usage" OR result.output contains "usage" OR result.output contains "--"
```

---

### TS-02-26: LOCKING_SERVICE uses UDS for DATA_BROKER

**Requirement:** 02-REQ-7.1
**Type:** unit
**Description:** Verify LOCKING_SERVICE configuration uses UDS endpoint
for DATA_BROKER connection.

**Preconditions:**
- LOCKING_SERVICE source exists.

**Input:**
- Inspect configuration/source for DATA_BROKER endpoint.

**Expected:**
- Default endpoint is a UDS path, not a TCP address.

**Assertion pseudocode:**
```
content = read_file("rhivos/locking-service/src/main.rs")
ASSERT contains(content, "unix") OR contains(content, "uds") OR contains(content, ".sock")
```

---

### TS-02-27: CLOUD_GATEWAY_CLIENT uses UDS for DATA_BROKER

**Requirement:** 02-REQ-7.2
**Type:** unit
**Description:** Verify CLOUD_GATEWAY_CLIENT configuration uses UDS endpoint
for DATA_BROKER connection.

**Preconditions:**
- CLOUD_GATEWAY_CLIENT source exists.

**Input:**
- Inspect configuration/source for DATA_BROKER endpoint.

**Expected:**
- Default endpoint is a UDS path, not a TCP address.

**Assertion pseudocode:**
```
content = read_file("rhivos/cloud-gateway-client/src/main.rs")
ASSERT contains(content, "unix") OR contains(content, "uds") OR contains(content, ".sock")
```

---

### TS-02-28: Mock sensors support configurable endpoint

**Requirement:** 02-REQ-7.3
**Type:** unit
**Description:** Verify mock sensor CLIs accept a configurable DATA_BROKER
endpoint.

**Preconditions:**
- Mock sensor source exists.

**Input:**
- Inspect source or help output for endpoint flag.

**Expected:**
- CLI supports an `--endpoint` or `--databroker-addr` flag.

**Assertion pseudocode:**
```
result = exec("speed-sensor --help")
ASSERT contains(result.output, "endpoint") OR contains(result.output, "databroker") OR contains(result.output, "addr")
```

---

### TS-02-29: UDS socket path configurable via environment

**Requirement:** 02-REQ-7.4
**Type:** unit
**Description:** Verify the UDS socket path can be configured via
environment variable.

**Preconditions:**
- Service source exists.

**Input:**
- Inspect source for DATABROKER_UDS_PATH environment variable usage.

**Expected:**
- Source references DATABROKER_UDS_PATH or equivalent env var.

**Assertion pseudocode:**
```
FOR EACH svc IN ["locking-service", "cloud-gateway-client"]:
    content = read_all_rust_files("rhivos/" + svc + "/src/")
    ASSERT contains(content, "DATABROKER_UDS_PATH") OR contains(content, "databroker") AND contains(content, "env")
```

---

### TS-02-30: Integration test for lock command flow

**Requirement:** 02-REQ-8.1
**Type:** integration
**Description:** Verify end-to-end lock command flow through DATA_BROKER.

**Preconditions:**
- DATA_BROKER, Mosquitto, LOCKING_SERVICE, and CLOUD_GATEWAY_CLIENT running.

**Input:**
- Publish lock command to MQTT. Set speed = 0, door closed.

**Expected:**
- Lock state changes. Response appears on MQTT.

**Assertion pseudocode:**
```
set_speed(0.0)
set_door_open(false)
mqtt_subscribe("vehicles/VIN12345/command_responses")
mqtt_publish("vehicles/VIN12345/commands", lock_command_json)
response = mqtt_wait_for_message(timeout=10s)
ASSERT json_parse(response)["status"] == "success"
locked = databroker_get("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked")
ASSERT locked == true
```

---

### TS-02-31: Integration tests runnable with single command

**Requirement:** 02-REQ-8.2
**Type:** unit
**Description:** Verify integration tests can be run with a single cargo
test command.

**Preconditions:**
- Rust workspace exists with integration test files.

**Input:**
- Check for integration test files.

**Expected:**
- Integration test file exists at expected path.

**Assertion pseudocode:**
```
ASSERT file_exists("rhivos/tests/integration.rs") OR
       file_exists("rhivos/locking-service/tests/integration.rs") OR
       glob("rhivos/**/tests/integration*.rs").len() >= 1
```

---

### TS-02-32: Integration tests require infrastructure

**Requirement:** 02-REQ-8.3
**Type:** integration
**Description:** Verify integration tests expect running infrastructure.

**Preconditions:**
- Infrastructure stopped.

**Input:**
- Run integration tests without infrastructure.

**Expected:**
- Tests fail or skip with a message about missing infrastructure.

**Assertion pseudocode:**
```
exec("make infra-down")
result = exec("cargo test --test integration", cwd="rhivos/")
ASSERT result.exit_code != 0 OR result.output contains "skip" OR result.output contains "unavailable"
```

---

## Property Test Cases

### TS-02-P1: Command-Response Pairing

**Property:** Property 1 from design.md
**Validates:** 02-REQ-2.2, 02-REQ-3.4, 02-REQ-3.5
**Type:** property
**Description:** For any lock/unlock command, LOCKING_SERVICE writes exactly
one response with the matching command_id.

**For any:** Command C with unique command_id
**Invariant:** Exactly one response R exists where R.command_id == C.command_id.

**Assertion pseudocode:**
```
set_speed(0.0)
set_door_open(false)
subscribe_responses = databroker_subscribe("Vehicle.Command.Door.Response")
FOR i IN 1..5:
    cmd_id = uuid()
    send_lock_command("lock", command_id=cmd_id)
    response = subscribe_responses.next(timeout=5s)
    ASSERT response is not None
    ASSERT json_parse(response)["command_id"] == cmd_id
```

---

### TS-02-P2: Safety Constraint Enforcement (Speed)

**Property:** Property 2 from design.md
**Validates:** 02-REQ-3.1, 02-REQ-3.4
**Type:** property
**Description:** For any lock command when speed > 0, IsLocked is not changed
and response is "failed".

**For any:** Speed S where S > 0
**Invariant:** IsLocked unchanged AND response.status == "failed".

**Assertion pseudocode:**
```
FOR speed IN [1.0, 10.0, 100.0, 0.1]:
    initial = databroker_get("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked")
    set_speed(speed)
    send_lock_command("lock")
    response = wait_for_response(timeout=5s)
    ASSERT response["status"] == "failed"
    ASSERT response["reason"] == "vehicle_moving"
    current = databroker_get("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked")
    ASSERT current == initial
```

---

### TS-02-P3: Door Ajar Protection

**Property:** Property 3 from design.md
**Validates:** 02-REQ-3.2, 02-REQ-3.4
**Type:** property
**Description:** For any lock command when door is open, IsLocked is not
changed and response is "failed" with reason "door_open".

**For any:** Door state where IsOpen == true
**Invariant:** IsLocked unchanged AND response.reason == "door_open".

**Assertion pseudocode:**
```
set_speed(0.0)
set_door_open(true)
initial = databroker_get("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked")
send_lock_command("lock")
response = wait_for_response(timeout=5s)
ASSERT response["status"] == "failed"
ASSERT response["reason"] == "door_open"
current = databroker_get("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked")
ASSERT current == initial
```

---

### TS-02-P4: Lock State Consistency

**Property:** Property 4 from design.md
**Validates:** 02-REQ-3.5
**Type:** property
**Description:** For any successful command, IsLocked matches the action.

**For any:** Successful lock/unlock sequence
**Invariant:** After "lock" -> IsLocked == true; after "unlock" -> IsLocked == false.

**Assertion pseudocode:**
```
set_speed(0.0)
set_door_open(false)
send_lock_command("lock")
wait_for_response(timeout=5s)
ASSERT databroker_get("IsLocked") == true

send_lock_command("unlock")
wait_for_response(timeout=5s)
ASSERT databroker_get("IsLocked") == false

send_lock_command("lock")
wait_for_response(timeout=5s)
ASSERT databroker_get("IsLocked") == true
```

---

### TS-02-P5: MQTT Command Relay Integrity

**Property:** Property 5 from design.md
**Validates:** 02-REQ-4.3
**Type:** property
**Description:** For any valid MQTT command, the DATA_BROKER receives the
identical command payload.

**For any:** Valid command JSON C
**Invariant:** Vehicle.Command.Door.Lock value == C.

**Assertion pseudocode:**
```
commands = [
    {"command_id": "p5-1", "action": "lock", "doors": ["driver"], "source": "test", "vin": "VIN12345", "timestamp": 1},
    {"command_id": "p5-2", "action": "unlock", "doors": ["driver"], "source": "test", "vin": "VIN12345", "timestamp": 2},
]
FOR cmd IN commands:
    mqtt_publish("vehicles/VIN12345/commands", json_encode(cmd))
    sleep(2s)
    value = databroker_get("Vehicle.Command.Door.Lock")
    parsed = json_parse(value)
    ASSERT parsed["command_id"] == cmd["command_id"]
    ASSERT parsed["action"] == cmd["action"]
```

---

### TS-02-P6: Telemetry Signal Coverage

**Property:** Property 6 from design.md
**Validates:** 02-REQ-5.1, 02-REQ-5.2
**Type:** property
**Description:** For any vehicle state signal change, a telemetry message
is published to MQTT.

**For any:** Signal change in {Speed, IsLocked, IsOpen, Latitude, Longitude}
**Invariant:** Telemetry message appears on MQTT with signal path and value.

**Assertion pseudocode:**
```
mqtt_subscribe("vehicles/VIN12345/telemetry")
set_speed(99.0)
msg = mqtt_wait_for_message(timeout=10s)
ASSERT msg is not empty
ASSERT json_parse(msg) contains signal "Speed" with value 99.0
```

---

### TS-02-P7: UDS Exclusivity

**Property:** Property 7 from design.md
**Validates:** 02-REQ-7.1, 02-REQ-7.2
**Type:** unit
**Description:** For LOCKING_SERVICE and CLOUD_GATEWAY_CLIENT, the
DATA_BROKER connection transport is UDS.

**For any:** Service S in {LOCKING_SERVICE, CLOUD_GATEWAY_CLIENT}
**Invariant:** Default DATA_BROKER endpoint is a UDS path.

**Assertion pseudocode:**
```
FOR svc IN ["locking-service", "cloud-gateway-client"]:
    source = read_all_files("rhivos/" + svc + "/src/")
    ASSERT source contains "unix:" OR source contains ".sock" OR source contains "UDS"
    ASSERT default config does NOT contain "localhost:55555" as DATA_BROKER endpoint
```

---

### TS-02-P8: Sensor Idempotency

**Property:** Property 8 from design.md
**Validates:** 02-REQ-6.1 through 02-REQ-6.4
**Type:** integration
**Description:** For any sensor CLI invocation, repeated calls with the same
value produce the same DATA_BROKER state.

**For any:** Sensor tool T with value V
**Invariant:** After T(V) and T(V), signal value == V.

**Assertion pseudocode:**
```
exec("speed-sensor --speed 77.7")
val1 = databroker_get("Vehicle.Speed")
exec("speed-sensor --speed 77.7")
val2 = databroker_get("Vehicle.Speed")
ASSERT val1 == val2
ASSERT abs(val1 - 77.7) < 0.1
```

---

## Edge Case Tests

### TS-02-E1: Unknown VSS signal write

**Requirement:** 02-REQ-1.E1
**Type:** integration
**Description:** Verify DATA_BROKER returns error for unknown signal write.

**Preconditions:**
- DATA_BROKER running.

**Input:**
- Attempt to write to `Vehicle.Nonexistent.Signal`.

**Expected:**
- Error response indicating unknown signal.

**Assertion pseudocode:**
```
result = databroker_set("Vehicle.Nonexistent.Signal", "test")
ASSERT result.is_err()
ASSERT result.error contains "not found" OR result.error contains "unknown"
```

---

### TS-02-E2: Missing bearer token on write

**Requirement:** 02-REQ-1.E2
**Type:** integration
**Description:** Verify DATA_BROKER rejects writes without valid bearer token.

**Preconditions:**
- DATA_BROKER running with token enforcement.

**Input:**
- Write request without bearer token.

**Expected:**
- Permission denied error.

**Assertion pseudocode:**
```
result = databroker_set("Vehicle.Speed", 0.0, token=None)
ASSERT result.is_err()
ASSERT result.error_code == PERMISSION_DENIED
```

---

### TS-02-E3: Invalid JSON in lock command signal

**Requirement:** 02-REQ-2.E1
**Type:** integration
**Description:** Verify LOCKING_SERVICE handles invalid JSON in command signal.

**Preconditions:**
- DATA_BROKER running. LOCKING_SERVICE running.

**Input:**
- Write invalid JSON string to Vehicle.Command.Door.Lock.

**Expected:**
- Response with status "failed", reason "invalid_payload".

**Assertion pseudocode:**
```
databroker_set("Vehicle.Command.Door.Lock", "not valid json {{{")
response = wait_for_response(timeout=5s)
parsed = json_parse(response)
ASSERT parsed["status"] == "failed"
ASSERT parsed["reason"] == "invalid_payload"
```

---

### TS-02-E4: Unknown action in lock command

**Requirement:** 02-REQ-2.E2
**Type:** integration
**Description:** Verify LOCKING_SERVICE rejects unknown action values.

**Preconditions:**
- DATA_BROKER running. LOCKING_SERVICE running.

**Input:**
- Command with action "toggle" (not "lock" or "unlock").

**Expected:**
- Response with status "failed", reason "unknown_action".

**Assertion pseudocode:**
```
command = {"command_id": "edge-4", "action": "toggle", "doors": ["driver"], "source": "test", "vin": "VIN12345", "timestamp": now()}
databroker_set("Vehicle.Command.Door.Lock", json_encode(command))
response = wait_for_response(timeout=5s)
parsed = json_parse(response)
ASSERT parsed["status"] == "failed"
ASSERT parsed["reason"] == "unknown_action"
```

---

### TS-02-E5: Missing fields in lock command

**Requirement:** 02-REQ-2.E3
**Type:** integration
**Description:** Verify LOCKING_SERVICE rejects commands with missing
required fields.

**Preconditions:**
- DATA_BROKER running. LOCKING_SERVICE running.

**Input:**
- Command JSON missing "command_id" field.

**Expected:**
- Response with status "failed", reason "missing_fields".

**Assertion pseudocode:**
```
command = {"action": "lock", "doors": ["driver"], "source": "test", "vin": "VIN12345", "timestamp": now()}
databroker_set("Vehicle.Command.Door.Lock", json_encode(command))
response = wait_for_response(timeout=5s)
parsed = json_parse(response)
ASSERT parsed["status"] == "failed"
ASSERT parsed["reason"] == "missing_fields"
```

---

### TS-02-E6: Speed signal not set (default safe)

**Requirement:** 02-REQ-3.E1
**Type:** integration
**Description:** Verify LOCKING_SERVICE treats unset speed as zero (safe).

**Preconditions:**
- DATA_BROKER running with no prior speed value. LOCKING_SERVICE running.
- Fresh DATA_BROKER (restarted, no speed written).

**Input:**
- Lock command without setting speed first.

**Expected:**
- Command succeeds (speed treated as 0).

**Assertion pseudocode:**
```
# Restart DATA_BROKER to clear state, or use fresh instance
set_door_open(false)
send_lock_command("lock")
response = wait_for_response(timeout=5s)
ASSERT response["status"] == "success"
```

---

### TS-02-E7: Door signal not set (default safe)

**Requirement:** 02-REQ-3.E2
**Type:** integration
**Description:** Verify LOCKING_SERVICE treats unset door state as closed
(safe).

**Preconditions:**
- DATA_BROKER running with no prior door value. LOCKING_SERVICE running.

**Input:**
- Lock command without setting door state first.

**Expected:**
- Command succeeds (door treated as closed).

**Assertion pseudocode:**
```
# Fresh DATA_BROKER, no door state written
set_speed(0.0)
send_lock_command("lock")
response = wait_for_response(timeout=5s)
ASSERT response["status"] == "success"
```

---

### TS-02-E8: MQTT broker unreachable at startup

**Requirement:** 02-REQ-4.E1
**Type:** integration
**Description:** Verify CLOUD_GATEWAY_CLIENT retries MQTT connection.

**Preconditions:**
- Mosquitto NOT running.

**Input:**
- Start CLOUD_GATEWAY_CLIENT. Observe logs.

**Expected:**
- Logs show retry attempts. Service does not crash immediately.

**Assertion pseudocode:**
```
exec("make infra-down")  # ensure MQTT is stopped
process = start_cloud_gateway_client()
sleep(5s)
ASSERT process.is_running()
ASSERT process.stderr contains "retry" OR process.stderr contains "reconnect"
stop(process)
```

---

### TS-02-E9: MQTT connection lost during operation

**Requirement:** 02-REQ-4.E2
**Type:** integration
**Description:** Verify CLOUD_GATEWAY_CLIENT reconnects after MQTT
connection loss.

**Preconditions:**
- Full infrastructure running. CLOUD_GATEWAY_CLIENT running.

**Input:**
- Restart Mosquitto while CLOUD_GATEWAY_CLIENT is connected.

**Expected:**
- CLOUD_GATEWAY_CLIENT reconnects and resumes operation.

**Assertion pseudocode:**
```
process = start_cloud_gateway_client()
sleep(3s)
exec("make infra-down")  # stop MQTT
sleep(5s)
exec("make infra-up")    # restart MQTT
sleep(10s)
ASSERT process.is_running()
# Verify functionality restored
mqtt_publish("vehicles/VIN12345/commands", valid_lock_command)
sleep(3s)
value = databroker_get("Vehicle.Command.Door.Lock")
ASSERT value is not empty
stop(process)
```

---

### TS-02-E10: Invalid JSON in MQTT command message

**Requirement:** 02-REQ-4.E3
**Type:** integration
**Description:** Verify CLOUD_GATEWAY_CLIENT discards invalid MQTT messages.

**Preconditions:**
- Full infrastructure running. CLOUD_GATEWAY_CLIENT running.

**Input:**
- Publish invalid JSON to commands topic.

**Expected:**
- Message is discarded. No crash. No write to DATA_BROKER.

**Assertion pseudocode:**
```
initial = databroker_get("Vehicle.Command.Door.Lock")
mqtt_publish("vehicles/VIN12345/commands", "not valid json")
sleep(3s)
current = databroker_get("Vehicle.Command.Door.Lock")
ASSERT current == initial  # no change
ASSERT process.is_running()  # no crash
```

---

### TS-02-E11: DATA_BROKER unreachable for telemetry subscription

**Requirement:** 02-REQ-5.E1
**Type:** integration
**Description:** Verify CLOUD_GATEWAY_CLIENT handles DATA_BROKER being
unreachable.

**Preconditions:**
- DATA_BROKER NOT running. Mosquitto running.

**Input:**
- Start CLOUD_GATEWAY_CLIENT without DATA_BROKER.

**Expected:**
- Logs show retry attempts for DATA_BROKER connection.
- Service does not crash.

**Assertion pseudocode:**
```
# Stop DATA_BROKER but keep Mosquitto
process = start_cloud_gateway_client()
sleep(5s)
ASSERT process.is_running()
ASSERT process.stderr contains "retry" OR process.stderr contains "reconnect" OR process.stderr contains "databroker"
stop(process)
```

---

### TS-02-E12: Mock sensor DATA_BROKER unreachable

**Requirement:** 02-REQ-6.E1
**Type:** integration
**Description:** Verify mock sensor tools report connection failure.

**Preconditions:**
- DATA_BROKER NOT running.

**Input:**
- Run: `speed-sensor --speed 10`

**Expected:**
- Non-zero exit code. Error message mentions connection.

**Assertion pseudocode:**
```
exec("make infra-down")
result = exec("speed-sensor --speed 10")
ASSERT result.exit_code != 0
ASSERT result.stderr contains "connect" OR result.stderr contains "unreachable" OR result.stderr contains "error"
```

---

### TS-02-E13: Mock sensor invalid value argument

**Requirement:** 02-REQ-6.E2
**Type:** unit
**Description:** Verify mock sensor tools reject invalid argument values.

**Preconditions:**
- Mock sensor binaries built.

**Input:**
- Run: `speed-sensor --speed not_a_number`

**Expected:**
- Non-zero exit code. Error message about invalid value.

**Assertion pseudocode:**
```
result = exec("speed-sensor --speed not_a_number")
ASSERT result.exit_code != 0
ASSERT result.stderr contains "invalid" OR result.stderr contains "error" OR result.stderr contains "parse"
```

---

### TS-02-E14: UDS socket file does not exist

**Requirement:** 02-REQ-7.E1
**Type:** integration
**Description:** Verify services log error when UDS socket is missing.

**Preconditions:**
- No DATA_BROKER running (no socket file).

**Input:**
- Start LOCKING_SERVICE with UDS path pointing to non-existent socket.

**Expected:**
- Error log mentions socket path.

**Assertion pseudocode:**
```
process = start_locking_service(env={"DATABROKER_UDS_PATH": "/tmp/nonexistent.sock"})
sleep(3s)
ASSERT process.stderr contains "/tmp/nonexistent.sock" OR process.stderr contains "socket" OR process.stderr contains "connect"
stop(process)
```

---

### TS-02-E15: Integration tests fail without infrastructure

**Requirement:** 02-REQ-8.E1
**Type:** integration
**Description:** Verify integration tests report missing infrastructure.

**Preconditions:**
- Infrastructure stopped.

**Input:**
- Run integration tests.

**Expected:**
- Tests fail or skip with informative message.

**Assertion pseudocode:**
```
exec("make infra-down")
result = exec("cargo test --test integration", cwd="rhivos/")
ASSERT result.exit_code != 0 OR result.output contains "skip" OR result.output contains "infrastructure"
```

---

## Coverage Matrix

| Requirement    | Test Spec Entry | Type        |
|----------------|-----------------|-------------|
| 02-REQ-1.1     | TS-02-1         | unit        |
| 02-REQ-1.2     | TS-02-2         | integration |
| 02-REQ-1.3     | TS-02-3         | integration |
| 02-REQ-1.4     | TS-02-4         | integration |
| 02-REQ-1.5     | TS-02-5         | integration |
| 02-REQ-1.E1    | TS-02-E1        | integration |
| 02-REQ-1.E2    | TS-02-E2        | integration |
| 02-REQ-2.1     | TS-02-6         | integration |
| 02-REQ-2.2     | TS-02-7         | unit        |
| 02-REQ-2.3     | TS-02-8         | integration |
| 02-REQ-2.4     | TS-02-9         | integration |
| 02-REQ-2.E1    | TS-02-E3        | integration |
| 02-REQ-2.E2    | TS-02-E4        | integration |
| 02-REQ-2.E3    | TS-02-E5        | integration |
| 02-REQ-3.1     | TS-02-10        | integration |
| 02-REQ-3.2     | TS-02-11        | integration |
| 02-REQ-3.3     | TS-02-12        | integration |
| 02-REQ-3.4     | TS-02-13        | integration |
| 02-REQ-3.5     | TS-02-14        | integration |
| 02-REQ-3.E1    | TS-02-E6        | integration |
| 02-REQ-3.E2    | TS-02-E7        | integration |
| 02-REQ-4.1     | TS-02-15        | integration |
| 02-REQ-4.2     | TS-02-16        | integration |
| 02-REQ-4.3     | TS-02-17        | integration |
| 02-REQ-4.4     | TS-02-18        | integration |
| 02-REQ-4.E1    | TS-02-E8        | integration |
| 02-REQ-4.E2    | TS-02-E9        | integration |
| 02-REQ-4.E3    | TS-02-E10       | integration |
| 02-REQ-5.1     | TS-02-19        | integration |
| 02-REQ-5.2     | TS-02-20        | integration |
| 02-REQ-5.E1    | TS-02-E11       | integration |
| 02-REQ-6.1     | TS-02-21        | integration |
| 02-REQ-6.2     | TS-02-22        | integration |
| 02-REQ-6.3     | TS-02-23        | integration |
| 02-REQ-6.4     | TS-02-24        | integration |
| 02-REQ-6.5     | TS-02-25        | unit        |
| 02-REQ-6.E1    | TS-02-E12       | integration |
| 02-REQ-6.E2    | TS-02-E13       | unit        |
| 02-REQ-7.1     | TS-02-26        | unit        |
| 02-REQ-7.2     | TS-02-27        | unit        |
| 02-REQ-7.3     | TS-02-28        | unit        |
| 02-REQ-7.4     | TS-02-29        | unit        |
| 02-REQ-7.E1    | TS-02-E14       | integration |
| 02-REQ-8.1     | TS-02-30        | integration |
| 02-REQ-8.2     | TS-02-31        | unit        |
| 02-REQ-8.3     | TS-02-32        | integration |
| 02-REQ-8.E1    | TS-02-E15       | integration |
| Property 1     | TS-02-P1        | property    |
| Property 2     | TS-02-P2        | property    |
| Property 3     | TS-02-P3        | property    |
| Property 4     | TS-02-P4        | property    |
| Property 5     | TS-02-P5        | property    |
| Property 6     | TS-02-P6        | property    |
| Property 7     | TS-02-P7        | unit        |
| Property 8     | TS-02-P8        | integration |
