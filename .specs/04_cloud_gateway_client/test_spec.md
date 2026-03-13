# Test Specification: CLOUD_GATEWAY_CLIENT

## Overview

Tests cover NATS connection/subscription (integration), command validation with bearer tokens (unit), DATA_BROKER forwarding (unit + integration), response relay (unit + integration), telemetry aggregation (unit), and lifecycle (integration). Unit tests use mock NATS messages and mock BrokerClient. Integration tests require running NATS and Kuksa Databroker containers.

## Test Cases

### TS-04-1: NATS Connection Config

**Requirement:** 04-REQ-1.1
**Type:** unit
**Description:** Verify the service reads NATS_URL with correct default.

**Preconditions:**
- None.

**Input:**
- Case 1: NATS_URL not set. Case 2: NATS_URL="nats://10.0.0.5:4222".

**Expected:**
- Case 1: URL is "nats://localhost:4222". Case 2: URL is "nats://10.0.0.5:4222".

**Assertion pseudocode:**
```
unset_env("NATS_URL")
config = load_config_partial()
ASSERT config.nats_url == "nats://localhost:4222"
set_env("NATS_URL", "nats://10.0.0.5:4222")
config = load_config_partial()
ASSERT config.nats_url == "nats://10.0.0.5:4222"
```

### TS-04-2: NATS Command Subscription

**Requirement:** 04-REQ-1.2
**Type:** integration
**Description:** Verify the service subscribes to vehicles.{VIN}.commands.

**Preconditions:**
- NATS and DATA_BROKER containers running. VIN=TEST_VIN_001.

**Input:**
- Start service, publish a valid command to vehicles.TEST_VIN_001.commands.

**Expected:**
- Service processes the command (visible via DATA_BROKER signal change or logs).

**Assertion pseudocode:**
```
start_service(vin="TEST_VIN_001")
nats_publish("vehicles.TEST_VIN_001.commands", valid_command, auth_header)
response = wait_for_signal("Vehicle.Command.Door.Lock", timeout=5s)
ASSERT response != nil
```

### TS-04-3: Bearer Token Extraction

**Requirement:** 04-REQ-1.3
**Type:** unit
**Description:** Verify the service extracts and validates the Authorization header.

**Preconditions:**
- None.

**Input:**
- NATS message with header `Authorization: Bearer demo-token`.

**Expected:**
- validate_bearer_token returns true.

**Assertion pseudocode:**
```
result = validate_bearer_token("Bearer demo-token", "demo-token")
ASSERT result == true
```

### TS-04-4: Bearer Token Validation

**Requirement:** 04-REQ-2.1
**Type:** unit
**Description:** Verify the service accepts matching tokens and rejects mismatches.

**Preconditions:**
- BEARER_TOKEN="demo-token".

**Input:**
- Case 1: header "Bearer demo-token". Case 2: header "Bearer wrong-token". Case 3: no header.

**Expected:**
- Case 1: valid. Case 2: invalid. Case 3: invalid.

**Assertion pseudocode:**
```
ASSERT validate_bearer_token("Bearer demo-token", "demo-token") == true
ASSERT validate_bearer_token("Bearer wrong-token", "demo-token") == false
ASSERT validate_bearer_token(None, "demo-token") == false
```

### TS-04-5: Command JSON Validation

**Requirement:** 04-REQ-2.2
**Type:** unit
**Description:** Verify command JSON is parsed and validated for required fields.

**Preconditions:**
- None.

**Input:**
- `{"command_id":"abc-123","action":"lock","doors":["driver"],"source":"companion_app","vin":"WDB123","timestamp":1700000000}`

**Expected:**
- parse_and_validate_command returns Ok with command_id="abc-123", action="lock".

**Assertion pseudocode:**
```
cmd = parse_and_validate_command(input)
ASSERT cmd.is_ok()
ASSERT cmd.command_id == "abc-123"
ASSERT cmd.action == "lock"
```

### TS-04-6: Command Forwarding to DATA_BROKER

**Requirement:** 04-REQ-2.3
**Type:** unit
**Description:** Verify validated commands are written to Vehicle.Command.Door.Lock.

**Preconditions:**
- Mock broker available.

**Input:**
- Valid command after passing token and JSON validation.

**Expected:**
- mock_broker.set_string called with ("Vehicle.Command.Door.Lock", command_json).

**Assertion pseudocode:**
```
process_command(&mock_broker, valid_nats_message)
ASSERT mock_broker.last_set_string.signal == "Vehicle.Command.Door.Lock"
ASSERT mock_broker.last_set_string.value contains "abc-123"
```

### TS-04-7: Response Subscription

**Requirement:** 04-REQ-3.1
**Type:** integration
**Description:** Verify the service subscribes to Vehicle.Command.Door.Response.

**Preconditions:**
- Service running with NATS and DATA_BROKER.

**Input:**
- Set Vehicle.Command.Door.Response in DATA_BROKER.

**Expected:**
- Response appears on vehicles.{VIN}.command_responses NATS subject.

**Assertion pseudocode:**
```
start_service()
nats_sub = nats_subscribe("vehicles.{VIN}.command_responses")
set_signal("Vehicle.Command.Door.Response", response_json)
msg = nats_sub.receive(timeout=5s)
ASSERT msg != nil
```

### TS-04-8: Response Relay Verbatim

**Requirement:** 04-REQ-3.2
**Type:** unit
**Description:** Verify response JSON is relayed verbatim to NATS.

**Preconditions:**
- None.

**Input:**
- Response JSON: `{"command_id":"abc-123","status":"success","timestamp":1700000001}`

**Expected:**
- Same JSON string published to NATS.

**Assertion pseudocode:**
```
response_json = '{"command_id":"abc-123","status":"success","timestamp":1700000001}'
relay_response(&mock_nats, "WDB123", response_json)
ASSERT mock_nats.last_publish.subject == "vehicles.WDB123.command_responses"
ASSERT mock_nats.last_publish.payload == response_json
```

### TS-04-9: Telemetry Signal Subscription

**Requirement:** 04-REQ-4.1
**Type:** integration
**Description:** Verify the service subscribes to all telemetry signals in DATA_BROKER.

**Preconditions:**
- Service running with DATA_BROKER.

**Input:**
- Set Vehicle.Cabin.Door.Row1.DriverSide.IsLocked to true in DATA_BROKER.

**Expected:**
- Telemetry message appears on vehicles.{VIN}.telemetry NATS subject.

**Assertion pseudocode:**
```
start_service()
nats_sub = nats_subscribe("vehicles.{VIN}.telemetry")
set_signal("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", true)
msg = nats_sub.receive(timeout=5s)
ASSERT msg != nil
parsed = parse_json(msg.payload)
ASSERT parsed.is_locked == true
```

### TS-04-10: Telemetry On-Change Publishing

**Requirement:** 04-REQ-4.2
**Type:** unit
**Description:** Verify telemetry is published when any signal changes.

**Preconditions:**
- TelemetryState with is_locked=false.

**Input:**
- Update is_locked to true.

**Expected:**
- build_telemetry produces a message with is_locked=true.

**Assertion pseudocode:**
```
state = TelemetryState { is_locked: Some(true), latitude: Some(48.85), longitude: Some(2.35), parking_active: None }
msg = build_telemetry("WDB123", &state)
ASSERT msg.vin == "WDB123"
ASSERT msg.is_locked == Some(true)
ASSERT msg.latitude == Some(48.85)
ASSERT msg.parking_active == None
```

### TS-04-11: Telemetry Payload Format

**Requirement:** 04-REQ-4.3
**Type:** unit
**Description:** Verify telemetry message contains VIN, signal values, and timestamp.

**Preconditions:**
- None.

**Input:**
- TelemetryState with all values set.

**Expected:**
- JSON contains vin, is_locked, latitude, longitude, parking_active, timestamp.

**Assertion pseudocode:**
```
state = TelemetryState { is_locked: Some(true), latitude: Some(48.85), longitude: Some(2.35), parking_active: Some(true) }
msg = build_telemetry("WDB123", &state)
json = serialize(msg)
parsed = parse_json(json)
ASSERT parsed.vin == "WDB123"
ASSERT parsed.is_locked == true
ASSERT parsed.latitude == 48.85
ASSERT parsed.longitude == 2.35
ASSERT parsed.parking_active == true
ASSERT parsed.timestamp > 0
```

### TS-04-12: DATA_BROKER Address Config

**Requirement:** 04-REQ-5.1
**Type:** unit
**Description:** Verify DATABROKER_ADDR env var with default.

**Preconditions:**
- None.

**Input:**
- DATABROKER_ADDR not set, then DATABROKER_ADDR="http://10.0.0.5:55556".

**Expected:**
- Default is "http://localhost:55556", override works.

**Assertion pseudocode:**
```
unset_env("DATABROKER_ADDR")
config = load_config_partial()
ASSERT config.databroker_addr == "http://localhost:55556"
```

### TS-04-13: VIN Config

**Requirement:** 04-REQ-6.1
**Type:** unit
**Description:** Verify VIN is read from env var.

**Preconditions:**
- None.

**Input:**
- VIN="WDB123456789".

**Expected:**
- config.vin == "WDB123456789".

**Assertion pseudocode:**
```
set_env("VIN", "WDB123456789")
config = load_config()
ASSERT config.vin == "WDB123456789"
```

### TS-04-14: Self-Registration on Startup

**Requirement:** 04-REQ-6.2
**Type:** integration
**Description:** Verify the service publishes a registration message on startup.

**Preconditions:**
- NATS running. VIN=TEST_VIN_001.

**Input:**
- Start service.

**Expected:**
- Message on vehicles.TEST_VIN_001.status with status="online".

**Assertion pseudocode:**
```
nats_sub = nats_subscribe("vehicles.TEST_VIN_001.status")
start_service(vin="TEST_VIN_001")
msg = nats_sub.receive(timeout=5s)
parsed = parse_json(msg.payload)
ASSERT parsed.vin == "TEST_VIN_001"
ASSERT parsed.status == "online"
ASSERT parsed.timestamp > 0
```

### TS-04-15: Graceful Shutdown

**Requirement:** 04-REQ-7.1
**Type:** integration
**Description:** Verify clean shutdown on SIGTERM.

**Preconditions:**
- Service running.

**Input:**
- Send SIGTERM.

**Expected:**
- Exit code 0, log contains shutdown message.

**Assertion pseudocode:**
```
proc = start_service()
wait_for_ready()
send_signal(proc, SIGTERM)
exit_code = wait_for_exit(proc, timeout=5s)
ASSERT exit_code == 0
```

### TS-04-16: Startup Logging

**Requirement:** 04-REQ-7.2
**Type:** integration
**Description:** Verify startup log output.

**Preconditions:**
- NATS and DATA_BROKER running.

**Input:**
- Start service with VIN=TEST_VIN_001.

**Expected:**
- Log contains VIN, NATS URL, DATABROKER_ADDR, "ready".

**Assertion pseudocode:**
```
output = start_and_capture_logs(vin="TEST_VIN_001")
ASSERT output contains "TEST_VIN_001"
ASSERT output contains "nats://localhost:4222"
ASSERT output contains "ready"
```

### TS-04-17: DATA_BROKER gRPC Operations

**Requirement:** 04-REQ-5.2
**Type:** integration
**Description:** Verify the service can set and subscribe to DATA_BROKER signals.

**Preconditions:**
- DATA_BROKER running.

**Input:**
- Service sets Vehicle.Command.Door.Lock and subscribes to response.

**Expected:**
- Set operation succeeds, subscription receives updates.

**Assertion pseudocode:**
```
start_service()
nats_publish("vehicles.{VIN}.commands", valid_command, auth_header)
lock_signal = get_signal("Vehicle.Command.Door.Lock")
ASSERT lock_signal contains valid command JSON
```

## Edge Case Tests

### TS-04-E1: NATS Unreachable on Startup

**Requirement:** 04-REQ-1.E1
**Type:** integration
**Description:** Verify retry then exit when NATS is unreachable.

**Preconditions:**
- No NATS running.

**Input:**
- Start service with NATS_URL pointing to non-listening port.

**Expected:**
- Service retries, exits with non-zero code.

**Assertion pseudocode:**
```
proc = start_service(nats_url="nats://localhost:19999")
exit_code = wait_for_exit(proc, timeout=30s)
ASSERT exit_code != 0
```

### TS-04-E2: NATS Connection Lost

**Requirement:** 04-REQ-1.E2
**Type:** integration
**Description:** Verify reconnect behavior when NATS drops.

**Preconditions:**
- Service running with active NATS connection.

**Input:**
- Stop NATS container.

**Expected:**
- Service attempts to reconnect (logs indicate retries).

**Assertion pseudocode:**
```
start_service()
wait_for_ready()
stop_nats()
// Service should attempt reconnect
```

### TS-04-E3: Invalid Bearer Token

**Requirement:** 04-REQ-2.E1
**Type:** unit
**Description:** Verify commands with wrong tokens are discarded.

**Preconditions:**
- BEARER_TOKEN="demo-token".

**Input:**
- NATS message with header "Bearer wrong-token".

**Expected:**
- validate_bearer_token returns false. Command not forwarded.

**Assertion pseudocode:**
```
ASSERT validate_bearer_token("Bearer wrong-token", "demo-token") == false
```

### TS-04-E4: Invalid JSON Command

**Requirement:** 04-REQ-2.E2
**Type:** unit
**Description:** Verify non-JSON payloads are discarded.

**Preconditions:**
- None.

**Input:**
- `"not valid json {{{"`

**Expected:**
- parse_and_validate_command returns error.

**Assertion pseudocode:**
```
result = parse_and_validate_command(b"not valid json {{{")
ASSERT result.is_err()
```

### TS-04-E5: Missing Required Field

**Requirement:** 04-REQ-2.E3
**Type:** unit
**Description:** Verify missing command_id is rejected.

**Preconditions:**
- None.

**Input:**
- `{"action":"lock","doors":["driver"]}`

**Expected:**
- Validation fails.

**Assertion pseudocode:**
```
result = parse_and_validate_command(input)
ASSERT result.is_err()
```

### TS-04-E6: Response NATS Publish Failure

**Requirement:** 04-REQ-3.E1
**Type:** unit
**Description:** Verify service continues after response publish failure.

**Preconditions:**
- Mock NATS configured to fail on publish.

**Input:**
- Response signal changes in DATA_BROKER.

**Expected:**
- Error logged, service continues.

**Assertion pseudocode:**
```
mock_nats.fail_next_publish()
relay_response(&mock_nats, "VIN", response_json)
// Should not panic, logs error
ASSERT mock_nats.error_logged == true
```

### TS-04-E7: Telemetry Signal Never Set

**Requirement:** 04-REQ-4.E1
**Type:** unit
**Description:** Verify unset signals are omitted from telemetry payload.

**Preconditions:**
- None.

**Input:**
- TelemetryState with only is_locked set, others None.

**Expected:**
- JSON omits latitude, longitude, parking_active fields.

**Assertion pseudocode:**
```
state = TelemetryState { is_locked: Some(true), latitude: None, longitude: None, parking_active: None }
msg = build_telemetry("VIN", &state)
json = serialize(msg)
ASSERT json NOT contains "latitude"
ASSERT json NOT contains "longitude"
ASSERT json NOT contains "parking_active"
ASSERT json contains "is_locked"
```

### TS-04-E8: Telemetry NATS Publish Failure

**Requirement:** 04-REQ-4.E2
**Type:** unit
**Description:** Verify service continues after telemetry publish failure.

**Preconditions:**
- Mock NATS configured to fail.

**Input:**
- Telemetry signal changes.

**Expected:**
- Error logged, service continues.

**Assertion pseudocode:**
```
mock_nats.fail_next_publish()
publish_telemetry(&mock_nats, "VIN", &state)
ASSERT mock_nats.error_logged == true
```

### TS-04-E9: DATA_BROKER Unreachable

**Requirement:** 04-REQ-5.E1
**Type:** integration
**Description:** Verify retry then exit when DATA_BROKER is unreachable.

**Preconditions:**
- No DATA_BROKER running.

**Input:**
- Start service with DATABROKER_ADDR pointing to non-listening port.

**Expected:**
- Service retries, exits non-zero.

**Assertion pseudocode:**
```
proc = start_service(databroker_addr="http://localhost:19999")
exit_code = wait_for_exit(proc, timeout=30s)
ASSERT exit_code != 0
```

### TS-04-E10: VIN Not Set

**Requirement:** 04-REQ-6.E1
**Type:** unit
**Description:** Verify service exits with error when VIN is not set.

**Preconditions:**
- VIN env var not set.

**Input:**
- Call load_config() without VIN.

**Expected:**
- Error returned.

**Assertion pseudocode:**
```
unset_env("VIN")
result = load_config()
ASSERT result.is_err()
```

### TS-04-E11: SIGTERM During Command

**Requirement:** 04-REQ-7.E1
**Type:** integration
**Description:** Verify in-flight command completes before shutdown.

**Preconditions:**
- Service running.

**Input:**
- Send command and SIGTERM nearly simultaneously.

**Expected:**
- Command forwarded to DATA_BROKER, then exit 0.

**Assertion pseudocode:**
```
proc = start_service()
nats_publish("vehicles.{VIN}.commands", valid_command, auth_header)
send_signal(proc, SIGTERM)
exit_code = wait_for_exit(proc, timeout=5s)
ASSERT exit_code == 0
```

## Property Test Cases

### TS-04-P1: Command Authentication Gate

**Property:** Property 1 from design.md
**Validates:** 04-REQ-2.1, 04-REQ-2.E1
**Type:** property
**Description:** Only commands with matching bearer tokens are forwarded.

**For any:** arbitrary token strings (valid, invalid, empty, missing)
**Invariant:** validate_bearer_token returns true iff token matches expected value.

**Assertion pseudocode:**
```
FOR ANY token IN arbitrary_strings, expected IN arbitrary_strings:
    result = validate_bearer_token(format("Bearer {}", token), expected)
    ASSERT result == (token == expected)
```

### TS-04-P2: Command Validation Completeness

**Property:** Property 2 from design.md
**Validates:** 04-REQ-2.2, 04-REQ-2.3, 04-REQ-2.E2, 04-REQ-2.E3
**Type:** property
**Description:** Any payload is either parsed to a valid command or rejected.

**For any:** arbitrary byte strings
**Invariant:** parse_and_validate_command returns Ok with valid fields or Err.

**Assertion pseudocode:**
```
FOR ANY payload IN arbitrary_bytes:
    result = parse_and_validate_command(payload)
    IF result.is_ok():
        cmd = result.unwrap()
        ASSERT cmd.command_id.len() > 0
        ASSERT cmd.action IN ["lock", "unlock"]
    ELSE:
        ASSERT result.is_err()
```

### TS-04-P3: Response Relay Fidelity

**Property:** Property 3 from design.md
**Validates:** 04-REQ-3.1, 04-REQ-3.2
**Type:** property
**Description:** Response JSON is relayed verbatim.

**For any:** valid response JSON strings
**Invariant:** published NATS payload equals input DATA_BROKER value.

**Assertion pseudocode:**
```
FOR ANY response_json IN valid_response_jsons:
    relay_response(&mock_nats, "VIN", response_json)
    ASSERT mock_nats.last_publish.payload == response_json
```

### TS-04-P4: Telemetry Aggregation

**Property:** Property 4 from design.md
**Validates:** 04-REQ-4.1, 04-REQ-4.2, 04-REQ-4.3
**Type:** property
**Description:** Telemetry messages contain VIN and all known signal values.

**For any:** TelemetryState with arbitrary Option<bool>/Option<f64> fields
**Invariant:** build_telemetry includes vin, timestamp, and all Some() fields.

**Assertion pseudocode:**
```
FOR ANY is_locked IN option_bool, lat IN option_f64, lon IN option_f64, parking IN option_bool:
    state = TelemetryState { is_locked, latitude: lat, longitude: lon, parking_active: parking }
    msg = build_telemetry("VIN", &state)
    ASSERT msg.vin == "VIN"
    ASSERT msg.timestamp > 0
    ASSERT msg.is_locked == is_locked
    ASSERT msg.latitude == lat
```

### TS-04-P5: VIN Subject Consistency

**Property:** Property 5 from design.md
**Validates:** 04-REQ-1.2, 04-REQ-6.1
**Type:** property
**Description:** All NATS subjects contain the configured VIN.

**For any:** VIN string (alphanumeric, 5-20 chars)
**Invariant:** generated subjects contain the VIN.

**Assertion pseudocode:**
```
FOR ANY vin IN alphanumeric_strings(5, 20):
    ASSERT format("vehicles.{}.commands", vin) contains vin
    ASSERT format("vehicles.{}.command_responses", vin) contains vin
    ASSERT format("vehicles.{}.telemetry", vin) contains vin
    ASSERT format("vehicles.{}.status", vin) contains vin
```

### TS-04-P6: Graceful Shutdown

**Property:** Property 6 from design.md
**Validates:** 04-REQ-7.1
**Type:** property
**Description:** Service always exits with code 0 on SIGTERM.

**For any:** service state (idle, processing command, publishing telemetry)
**Invariant:** exit code is 0.

**Assertion pseudocode:**
```
// This is an integration-level property, tested via TS-04-15 and TS-04-E11
```

## Coverage Matrix

| Requirement | Test Spec Entry | Type |
|-------------|-----------------|------|
| 04-REQ-1.1 | TS-04-1 | unit |
| 04-REQ-1.2 | TS-04-2 | integration |
| 04-REQ-1.3 | TS-04-3 | unit |
| 04-REQ-1.E1 | TS-04-E1 | integration |
| 04-REQ-1.E2 | TS-04-E2 | integration |
| 04-REQ-2.1 | TS-04-4 | unit |
| 04-REQ-2.2 | TS-04-5 | unit |
| 04-REQ-2.3 | TS-04-6 | unit |
| 04-REQ-2.E1 | TS-04-E3 | unit |
| 04-REQ-2.E2 | TS-04-E4 | unit |
| 04-REQ-2.E3 | TS-04-E5 | unit |
| 04-REQ-3.1 | TS-04-7 | integration |
| 04-REQ-3.2 | TS-04-8 | unit |
| 04-REQ-3.E1 | TS-04-E6 | unit |
| 04-REQ-4.1 | TS-04-9 | integration |
| 04-REQ-4.2 | TS-04-10 | unit |
| 04-REQ-4.3 | TS-04-11 | unit |
| 04-REQ-4.E1 | TS-04-E7 | unit |
| 04-REQ-4.E2 | TS-04-E8 | unit |
| 04-REQ-5.1 | TS-04-12 | unit |
| 04-REQ-5.2 | TS-04-17 | integration |
| 04-REQ-5.E1 | TS-04-E9 | integration |
| 04-REQ-6.1 | TS-04-13 | unit |
| 04-REQ-6.2 | TS-04-14 | integration |
| 04-REQ-6.E1 | TS-04-E10 | unit |
| 04-REQ-7.1 | TS-04-15 | integration |
| 04-REQ-7.2 | TS-04-16 | integration |
| 04-REQ-7.E1 | TS-04-E11 | integration |
| Property 1 | TS-04-P1 | property |
| Property 2 | TS-04-P2 | property |
| Property 3 | TS-04-P3 | property |
| Property 4 | TS-04-P4 | property |
| Property 5 | TS-04-P5 | property |
| Property 6 | TS-04-P6 | property |
