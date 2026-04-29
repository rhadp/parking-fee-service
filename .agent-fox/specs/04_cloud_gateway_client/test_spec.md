# Test Specification: CLOUD_GATEWAY_CLIENT

## Overview

This test specification defines the unit, edge case, property, and integration
tests for the CLOUD_GATEWAY_CLIENT component. Tests validate configuration
parsing, command authentication and validation, telemetry state aggregation, and
end-to-end data flows through NATS and DATA_BROKER. Each test case maps to
acceptance criteria in requirements.md and correctness properties in design.md.

## Test Cases

### TS-04-1: Config reads VIN from environment

**Requirement:** 04-REQ-1.1
**Type:** unit
**Description:** Verify that Config reads VIN from the environment and applies defaults for unset variables.

**Preconditions:**
- Environment variable `VIN` is set to `"TEST-VIN-001"`.
- `NATS_URL`, `DATABROKER_ADDR`, and `BEARER_TOKEN` are not set.

**Input:**
- Call `Config::from_env()`.

**Expected:**
- Returns `Ok(config)` with `config.vin == "TEST-VIN-001"`, `config.nats_url == "nats://localhost:4222"`, `config.databroker_addr == "http://localhost:55556"`, `config.bearer_token == "demo-token"`.

**Assertion pseudocode:**
```
set_env("VIN", "TEST-VIN-001")
unset_env("NATS_URL")
unset_env("DATABROKER_ADDR")
unset_env("BEARER_TOKEN")
config = Config::from_env()
ASSERT config is Ok
ASSERT config.vin == "TEST-VIN-001"
ASSERT config.nats_url == "nats://localhost:4222"
ASSERT config.databroker_addr == "http://localhost:55556"
ASSERT config.bearer_token == "demo-token"
```

### TS-04-2: Config reads all custom environment variables

**Requirement:** 04-REQ-1.2, 04-REQ-1.3, 04-REQ-1.4
**Type:** unit
**Description:** Verify that Config reads custom values for all environment variables.

**Preconditions:**
- All environment variables are set to custom values.

**Input:**
- `VIN="MY-VIN"`, `NATS_URL="nats://custom:9222"`, `DATABROKER_ADDR="http://custom:55557"`, `BEARER_TOKEN="secret-token"`.

**Expected:**
- Returns `Ok(config)` with all custom values populated.

**Assertion pseudocode:**
```
set_env("VIN", "MY-VIN")
set_env("NATS_URL", "nats://custom:9222")
set_env("DATABROKER_ADDR", "http://custom:55557")
set_env("BEARER_TOKEN", "secret-token")
config = Config::from_env()
ASSERT config is Ok
ASSERT config.nats_url == "nats://custom:9222"
ASSERT config.databroker_addr == "http://custom:55557"
ASSERT config.bearer_token == "secret-token"
```

### TS-04-3: Bearer token validation accepts valid token

**Requirement:** 04-REQ-5.1, 04-REQ-5.2
**Type:** unit
**Description:** Verify that a valid bearer token in the Authorization header is accepted.

**Preconditions:**
- NATS message headers contain `"Authorization" = "Bearer demo-token"`.

**Input:**
- `headers` with `Authorization: Bearer demo-token`, `expected_token = "demo-token"`.

**Expected:**
- Returns `Ok(())`.

**Assertion pseudocode:**
```
headers = {"Authorization": "Bearer demo-token"}
result = validate_bearer_token(headers, "demo-token")
ASSERT result is Ok(())
```

### TS-04-4: Command validation accepts valid lock payload

**Requirement:** 04-REQ-6.1, 04-REQ-6.2
**Type:** unit
**Description:** Verify that a well-formed lock command payload passes validation.

**Preconditions:**
- None.

**Input:**
- `'{"command_id":"abc-123","action":"lock","doors":["driver"],"source":"companion_app","vin":"VIN1","timestamp":1700000000}'`

**Expected:**
- Returns `Ok(cmd)` with `cmd.command_id == "abc-123"`, `cmd.action == "lock"`, `cmd.doors == ["driver"]`.

**Assertion pseudocode:**
```
payload = '{"command_id":"abc-123","action":"lock","doors":["driver"],"source":"companion_app","vin":"VIN1","timestamp":1700000000}'
result = validate_command_payload(payload)
ASSERT result is Ok(cmd)
ASSERT cmd.command_id == "abc-123"
ASSERT cmd.action == "lock"
ASSERT cmd.doors == ["driver"]
```

### TS-04-5: Command validation accepts unlock action

**Requirement:** 04-REQ-6.2
**Type:** unit
**Description:** Verify that an unlock action is accepted as a valid action value.

**Preconditions:**
- None.

**Input:**
- `'{"command_id":"def-456","action":"unlock","doors":["driver"]}'`

**Expected:**
- Returns `Ok(cmd)` with `cmd.action == "unlock"`.

**Assertion pseudocode:**
```
payload = '{"command_id":"def-456","action":"unlock","doors":["driver"]}'
result = validate_command_payload(payload)
ASSERT result is Ok(cmd)
ASSERT cmd.action == "unlock"
```

### TS-04-6: Command validation does not validate door values

**Requirement:** 04-REQ-6.4
**Type:** unit
**Description:** Verify that arbitrary door values pass validation without being checked.

**Preconditions:**
- None.

**Input:**
- `'{"command_id":"abc","action":"lock","doors":["unknown-door","another"]}'`

**Expected:**
- Returns `Ok(cmd)` with `cmd.doors == ["unknown-door", "another"]`.

**Assertion pseudocode:**
```
payload = '{"command_id":"abc","action":"lock","doors":["unknown-door","another"]}'
result = validate_command_payload(payload)
ASSERT result is Ok(cmd)
ASSERT cmd.doors == ["unknown-door", "another"]
```

### TS-04-7: Telemetry state produces JSON on first update

**Requirement:** 04-REQ-8.1, 04-REQ-8.2
**Type:** unit
**Description:** Verify that the first signal update produces a telemetry JSON with the updated field and omits unset fields.

**Preconditions:**
- `TelemetryState` initialized with `vin = "VIN-001"`, no prior updates.

**Input:**
- `state.update(SignalUpdate::IsLocked(true))`

**Expected:**
- Returns `Some(json)` containing `"vin":"VIN-001"`, `"is_locked":true`, `"timestamp"`, and no `latitude`, `longitude`, or `parking_active` fields.

**Assertion pseudocode:**
```
state = TelemetryState::new("VIN-001")
result = state.update(SignalUpdate::IsLocked(true))
ASSERT result is Some(json)
parsed = parse_json(json)
ASSERT parsed.vin == "VIN-001"
ASSERT parsed.is_locked == true
ASSERT "timestamp" IN parsed
ASSERT "latitude" NOT IN parsed
ASSERT "longitude" NOT IN parsed
ASSERT "parking_active" NOT IN parsed
```

### TS-04-8: Telemetry state omits unset fields

**Requirement:** 04-REQ-8.3
**Type:** unit
**Description:** Verify that only the updated signal appears in telemetry; other signals are omitted.

**Preconditions:**
- `TelemetryState` initialized with `vin = "VIN-001"`, no prior updates.

**Input:**
- `state.update(SignalUpdate::Latitude(48.1351))`

**Expected:**
- Returns `Some(json)` containing `"latitude":48.1351` and no `is_locked`, `longitude`, or `parking_active` fields.

**Assertion pseudocode:**
```
state = TelemetryState::new("VIN-001")
result = state.update(SignalUpdate::Latitude(48.1351))
ASSERT result is Some(json)
parsed = parse_json(json)
ASSERT parsed.latitude == 48.1351
ASSERT "is_locked" NOT IN parsed
ASSERT "longitude" NOT IN parsed
ASSERT "parking_active" NOT IN parsed
```

### TS-04-9: Telemetry state includes all known fields

**Requirement:** 04-REQ-8.2
**Type:** unit
**Description:** Verify that after multiple signal updates, all known fields appear in the telemetry payload.

**Preconditions:**
- `TelemetryState` initialized with `vin = "VIN-001"`.
- Prior updates: `IsLocked(true)`, `Latitude(48.1351)`, `Longitude(11.582)`.

**Input:**
- `state.update(SignalUpdate::ParkingActive(true))`

**Expected:**
- Returns `Some(json)` containing all four signal fields with their latest values.

**Assertion pseudocode:**
```
state = TelemetryState::new("VIN-001")
state.update(SignalUpdate::IsLocked(true))
state.update(SignalUpdate::Latitude(48.1351))
state.update(SignalUpdate::Longitude(11.582))
result = state.update(SignalUpdate::ParkingActive(true))
ASSERT result is Some(json)
parsed = parse_json(json)
ASSERT parsed.is_locked == true
ASSERT parsed.latitude == 48.1351
ASSERT parsed.longitude == 11.582
ASSERT parsed.parking_active == true
```

### TS-04-10: Registration message format

**Requirement:** 04-REQ-4.1
**Type:** unit
**Description:** Verify that the serialized registration message contains the correct VIN, status, and timestamp fields.

**Preconditions:**
- None.

**Input:**
- `RegistrationMessage { vin: "VIN-001", status: "online", timestamp: <current_unix_ts> }`

**Expected:**
- Serialized JSON contains `"vin":"VIN-001"`, `"status":"online"`, and a `"timestamp"` field.

**Assertion pseudocode:**
```
msg = RegistrationMessage { vin: "VIN-001", status: "online", timestamp: now() }
json = serialize(msg)
ASSERT json contains "vin":"VIN-001"
ASSERT json contains "status":"online"
ASSERT json contains "timestamp"
```

### TS-04-11: End-to-end command flow

**Requirement:** 04-REQ-2.3, 04-REQ-5.2, 04-REQ-6.3
**Type:** integration
**Description:** Verify that a valid command published on NATS is forwarded to DATA_BROKER.

**Preconditions:**
- NATS container running on `localhost:4222`.
- DATA_BROKER container running on `localhost:55556`.
- CLOUD_GATEWAY_CLIENT running with `VIN="E2E-VIN"`.

**Input:**
- Publish NATS message to `vehicles.E2E-VIN.commands` with header `Authorization: Bearer demo-token` and payload `'{"command_id":"cmd-1","action":"lock","doors":["driver"],"source":"companion_app","vin":"E2E-VIN","timestamp":1700000000}'`.

**Expected:**
- Within 2 seconds, `Vehicle.Command.Door.Lock` in DATA_BROKER contains the command payload.

**Assertion pseudocode:**
```
nats.publish("vehicles.E2E-VIN.commands", payload, headers={"Authorization": "Bearer demo-token"})
wait_up_to(2s)
value = databroker.get("Vehicle.Command.Door.Lock")
ASSERT value == payload
```

### TS-04-12: End-to-end response relay

**Requirement:** 04-REQ-7.1, 04-REQ-7.2
**Type:** integration
**Description:** Verify that a command response written to DATA_BROKER is relayed verbatim to NATS.

**Preconditions:**
- NATS container running on `localhost:4222`.
- DATA_BROKER container running on `localhost:55556`.
- CLOUD_GATEWAY_CLIENT running with `VIN="E2E-VIN"`.
- NATS subscriber listening on `vehicles.E2E-VIN.command_responses`.

**Input:**
- Set `Vehicle.Command.Door.Response` in DATA_BROKER to `'{"command_id":"cmd-1","status":"success","timestamp":1700000001}'`.

**Expected:**
- Within 2 seconds, the NATS subscriber receives the response JSON verbatim.

**Assertion pseudocode:**
```
response_json = '{"command_id":"cmd-1","status":"success","timestamp":1700000001}'
databroker.set("Vehicle.Command.Door.Response", response_json)
msg = nats_sub.next(timeout=2s)
ASSERT msg.data == response_json
```

### TS-04-13: End-to-end telemetry on signal change

**Requirement:** 04-REQ-8.1, 04-REQ-8.2
**Type:** integration
**Description:** Verify that a DATA_BROKER signal change triggers a telemetry message on NATS.

**Preconditions:**
- NATS container running on `localhost:4222`.
- DATA_BROKER container running on `localhost:55556`.
- CLOUD_GATEWAY_CLIENT running with `VIN="E2E-VIN"`.
- NATS subscriber listening on `vehicles.E2E-VIN.telemetry`.

**Input:**
- Set `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` to `true` in DATA_BROKER.

**Expected:**
- Within 2 seconds, a telemetry JSON is received on NATS containing `"vin":"E2E-VIN"` and `"is_locked":true`.

**Assertion pseudocode:**
```
databroker.set("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", true)
msg = nats_sub.next(timeout=2s)
parsed = parse_json(msg.data)
ASSERT parsed.vin == "E2E-VIN"
ASSERT parsed.is_locked == true
```

### TS-04-14: Self-registration on startup

**Requirement:** 04-REQ-4.1, 04-REQ-4.2
**Type:** integration
**Description:** Verify that the service publishes a registration message on startup.

**Preconditions:**
- NATS container running on `localhost:4222`.
- NATS subscriber listening on `vehicles.REG-VIN.status`.

**Input:**
- Start CLOUD_GATEWAY_CLIENT with `VIN="REG-VIN"`.

**Expected:**
- Within 5 seconds, a registration message is received containing `"vin":"REG-VIN"` and `"status":"online"`.

**Assertion pseudocode:**
```
nats_sub = nats.subscribe("vehicles.REG-VIN.status")
start_service(VIN="REG-VIN")
msg = nats_sub.next(timeout=5s)
parsed = parse_json(msg.data)
ASSERT parsed.vin == "REG-VIN"
ASSERT parsed.status == "online"
```

### TS-04-15: Command rejected with invalid token

**Requirement:** 04-REQ-5.E2
**Type:** integration
**Description:** Verify that a command with an invalid bearer token is rejected and not forwarded.

**Preconditions:**
- NATS container running on `localhost:4222`.
- DATA_BROKER container running on `localhost:55556`.
- CLOUD_GATEWAY_CLIENT running with `VIN="E2E-VIN"`.

**Input:**
- Publish NATS message to `vehicles.E2E-VIN.commands` with header `Authorization: Bearer wrong-token` and payload `'{"command_id":"cmd-2","action":"lock","doors":["driver"]}'`.

**Expected:**
- `Vehicle.Command.Door.Lock` in DATA_BROKER is NOT updated.
- No message published to `vehicles.E2E-VIN.command_responses`.

**Assertion pseudocode:**
```
nats.publish("vehicles.E2E-VIN.commands", payload, headers={"Authorization": "Bearer wrong-token"})
wait(1s)
ASSERT databroker.get("Vehicle.Command.Door.Lock") is unchanged
ASSERT nats_sub("vehicles.E2E-VIN.command_responses").next(timeout=1s) is None
```

### TS-04-16: NATS reconnection with exponential backoff

**Requirement:** 04-REQ-2.2, 04-REQ-2.E1
**Type:** integration
**Description:** Verify that the service retries NATS connection with exponential backoff and exits after exhausting retries.

**Preconditions:**
- NATS server is NOT running.

**Input:**
- Start CLOUD_GATEWAY_CLIENT.

**Expected:**
- The service attempts to connect at t=0, t~1s, t~3s, t~7s, t~15s.
- After 5 failed attempts, the service exits with code 1.

**Assertion pseudocode:**
```
stop_nats_server()
start_time = now()
result = start_service()
ASSERT result.exit_code == 1
ASSERT elapsed(start_time) >= 15s
```

## Property Test Cases

### TS-04-P1: Command Authentication Integrity

**Property:** Property 1 from design.md
**Validates:** 04-REQ-5.1, 04-REQ-5.2, 04-REQ-5.E1, 04-REQ-5.E2
**Type:** property
**Description:** For any NATS message, the system accepts it only if the Authorization header matches the configured bearer token.

**For any:** NATS message with arbitrary header values (missing, empty, wrong prefix, wrong token, valid token)
**Invariant:** `validate_bearer_token` returns `Ok` if and only if the header value is exactly `"Bearer <configured_token>"`

**Assertion pseudocode:**
```
FOR ANY header_value IN arbitrary_strings_and_none:
    headers = {"Authorization": header_value} if header_value is not None else {}
    result = validate_bearer_token(headers, expected_token)
    IF header_value == "Bearer " + expected_token:
        ASSERT result is Ok
    ELSE:
        ASSERT result is Err
```

### TS-04-P2: Command Structural Validity

**Property:** Property 2 from design.md
**Validates:** 04-REQ-6.1, 04-REQ-6.2, 04-REQ-6.3, 04-REQ-6.E1, 04-REQ-6.E2, 04-REQ-6.E3
**Type:** property
**Description:** For any command that passes authentication, the system writes to DATA_BROKER only if the payload is valid JSON containing a non-empty command_id, a valid action, and a doors array.

**For any:** authenticated NATS message with arbitrary payload bytes
**Invariant:** DATA_BROKER write occurs if and only if payload is valid JSON with non-empty command_id, action in {"lock","unlock"}, and doors array present

**Assertion pseudocode:**
```
FOR ANY payload IN arbitrary_json_variants:
    result = validate_command_payload(payload)
    IF payload.is_valid_json AND payload.command_id != "" AND payload.action IN ["lock","unlock"] AND payload.has_doors:
        ASSERT result is Ok
    ELSE:
        ASSERT result is Err
```

### TS-04-P3: Command Passthrough Fidelity

**Property:** Property 3 from design.md
**Validates:** 04-REQ-6.3, 04-REQ-6.4
**Type:** property
**Description:** For any validated command, the payload written to Vehicle.Command.Door.Lock in DATA_BROKER is identical to the original payload received from NATS.

**For any:** validated command payload
**Invariant:** The DATA_BROKER signal value equals the original NATS payload byte-for-byte

**Assertion pseudocode:**
```
FOR ANY payload IN valid_command_payloads:
    send_to_nats(payload)
    broker_value = databroker.get("Vehicle.Command.Door.Lock")
    ASSERT broker_value == payload
```

### TS-04-P4: Response Relay Fidelity

**Property:** Property 4 from design.md
**Validates:** 04-REQ-7.1, 04-REQ-7.2
**Type:** property
**Description:** For any change to Vehicle.Command.Door.Response in DATA_BROKER, the JSON value is published verbatim to NATS without modification.

**For any:** JSON string written to Vehicle.Command.Door.Response
**Invariant:** The NATS message payload on vehicles.{VIN}.command_responses equals the DATA_BROKER signal value

**Assertion pseudocode:**
```
FOR ANY response_json IN valid_response_json_strings:
    databroker.set("Vehicle.Command.Door.Response", response_json)
    nats_msg = nats.subscribe("vehicles.{VIN}.command_responses").next(timeout=2s)
    ASSERT nats_msg.data == response_json
```

### TS-04-P5: Telemetry Completeness

**Property:** Property 5 from design.md
**Validates:** 04-REQ-8.1, 04-REQ-8.2, 04-REQ-8.3
**Type:** property
**Description:** For any change to a subscribed telemetry signal, the system publishes an aggregated JSON message that includes all currently known signal values and omits signals never set.

**For any:** sequence of signal updates to telemetry-subscribed signals
**Invariant:** Published JSON contains exactly the set of previously updated fields, each with its latest value

**Assertion pseudocode:**
```
FOR ANY updates IN sequences_of_signal_updates:
    state = TelemetryState::new("VIN")
    known = {}
    FOR update IN updates:
        known[update.field] = update.value
        result = state.update(update)
        IF result is Some(json):
            parsed = parse_json(result)
            FOR field, value IN known:
                ASSERT parsed[field] == value
            FOR field NOT IN known:
                ASSERT field NOT IN parsed
```

### TS-04-P6: Startup Determinism

**Property:** Property 6 from design.md
**Validates:** 04-REQ-9.1, 04-REQ-9.2
**Type:** property
**Description:** For any execution of the service, initialization proceeds in strict order and a failure at any step prevents subsequent steps from executing.

**For any:** startup execution with a failure injected at step N
**Invariant:** Steps 1..N-1 complete, step N fails, steps N+1..end do not execute

**Assertion pseudocode:**
```
FOR ANY failure_step IN [config, nats_connect, broker_connect, registration, processing]:
    inject_failure_at(failure_step)
    result = start_service()
    ASSERT steps_before(failure_step) all completed
    ASSERT steps_after(failure_step) none executed
    ASSERT result.exit_code != 0
```

## Edge Case Tests

### TS-04-E1: Config fails when VIN is missing

**Requirement:** 04-REQ-1.E1
**Type:** unit
**Description:** Verify that the service returns an error when VIN is not set.

**Preconditions:**
- Environment variable `VIN` is not set.

**Input:**
- Call `Config::from_env()`.

**Expected:**
- Returns `Err(ConfigError::MissingVin)`.

**Assertion pseudocode:**
```
unset_env("VIN")
result = Config::from_env()
ASSERT result is Err(ConfigError::MissingVin)
```

### TS-04-E2: Bearer token validation rejects missing header

**Requirement:** 04-REQ-5.E1
**Type:** unit
**Description:** Verify that a missing Authorization header causes authentication to fail.

**Preconditions:**
- NATS message headers do not contain `"Authorization"`.

**Input:**
- Empty headers, `expected_token = "demo-token"`.

**Expected:**
- Returns `Err(AuthError::MissingHeader)`.

**Assertion pseudocode:**
```
headers = {}
result = validate_bearer_token(headers, "demo-token")
ASSERT result is Err(AuthError::MissingHeader)
```

### TS-04-E3: Bearer token validation rejects wrong token

**Requirement:** 04-REQ-5.E2
**Type:** unit
**Description:** Verify that an incorrect bearer token is rejected.

**Preconditions:**
- NATS message headers contain `"Authorization" = "Bearer wrong-token"`.

**Input:**
- Headers with wrong token, `expected_token = "demo-token"`.

**Expected:**
- Returns `Err(AuthError::InvalidToken)`.

**Assertion pseudocode:**
```
headers = {"Authorization": "Bearer wrong-token"}
result = validate_bearer_token(headers, "demo-token")
ASSERT result is Err(AuthError::InvalidToken)
```

### TS-04-E4: Bearer token validation rejects malformed header

**Requirement:** 04-REQ-5.E2
**Type:** unit
**Description:** Verify that a malformed Authorization header (wrong prefix) is rejected.

**Preconditions:**
- NATS message headers contain `"Authorization" = "NotBearer demo-token"`.

**Input:**
- Headers with malformed prefix, `expected_token = "demo-token"`.

**Expected:**
- Returns `Err(AuthError::InvalidToken)`.

**Assertion pseudocode:**
```
headers = {"Authorization": "NotBearer demo-token"}
result = validate_bearer_token(headers, "demo-token")
ASSERT result is Err(AuthError::InvalidToken)
```

### TS-04-E5: Command validation rejects invalid JSON

**Requirement:** 04-REQ-6.E1
**Type:** unit
**Description:** Verify that a non-JSON payload is rejected.

**Preconditions:**
- None.

**Input:**
- `'not-valid-json{{'`

**Expected:**
- Returns `Err(ValidationError::InvalidJson(_))`.

**Assertion pseudocode:**
```
result = validate_command_payload("not-valid-json{{")
ASSERT result is Err(ValidationError::InvalidJson(_))
```

### TS-04-E6: Command validation rejects missing command_id

**Requirement:** 04-REQ-6.E2
**Type:** unit
**Description:** Verify that a payload missing command_id is rejected.

**Preconditions:**
- None.

**Input:**
- `'{"action":"lock","doors":["driver"]}'`

**Expected:**
- Returns `Err(ValidationError::MissingField("command_id"))`.

**Assertion pseudocode:**
```
result = validate_command_payload('{"action":"lock","doors":["driver"]}')
ASSERT result is Err(ValidationError::MissingField("command_id"))
```

### TS-04-E7: Command validation rejects empty command_id

**Requirement:** 04-REQ-6.E2
**Type:** unit
**Description:** Verify that a payload with an empty command_id is rejected.

**Preconditions:**
- None.

**Input:**
- `'{"command_id":"","action":"lock","doors":["driver"]}'`

**Expected:**
- Returns `Err(ValidationError::MissingField("command_id"))`.

**Assertion pseudocode:**
```
result = validate_command_payload('{"command_id":"","action":"lock","doors":["driver"]}')
ASSERT result is Err(ValidationError::MissingField("command_id"))
```

### TS-04-E8: Command validation rejects missing action

**Requirement:** 04-REQ-6.E2
**Type:** unit
**Description:** Verify that a payload missing the action field is rejected.

**Preconditions:**
- None.

**Input:**
- `'{"command_id":"abc","doors":["driver"]}'`

**Expected:**
- Returns `Err(ValidationError::MissingField("action"))`.

**Assertion pseudocode:**
```
result = validate_command_payload('{"command_id":"abc","doors":["driver"]}')
ASSERT result is Err(ValidationError::MissingField("action"))
```

### TS-04-E9: Command validation rejects invalid action

**Requirement:** 04-REQ-6.E3
**Type:** unit
**Description:** Verify that an action value other than "lock" or "unlock" is rejected.

**Preconditions:**
- None.

**Input:**
- `'{"command_id":"abc","action":"open","doors":["driver"]}'`

**Expected:**
- Returns `Err(ValidationError::InvalidAction("open"))`.

**Assertion pseudocode:**
```
result = validate_command_payload('{"command_id":"abc","action":"open","doors":["driver"]}')
ASSERT result is Err(ValidationError::InvalidAction("open"))
```

### TS-04-E10: Command validation rejects missing doors

**Requirement:** 04-REQ-6.E2
**Type:** unit
**Description:** Verify that a payload missing the doors field is rejected.

**Preconditions:**
- None.

**Input:**
- `'{"command_id":"abc","action":"lock"}'`

**Expected:**
- Returns `Err(ValidationError::MissingField("doors"))`.

**Assertion pseudocode:**
```
result = validate_command_payload('{"command_id":"abc","action":"lock"}')
ASSERT result is Err(ValidationError::MissingField("doors"))
```

### TS-04-E11: NATS connection retries exhausted

**Requirement:** 04-REQ-2.E1
**Type:** unit
**Description:** Verify that the service exits with code 1 after exhausting all NATS connection retry attempts.

**Preconditions:**
- NATS server is unreachable at the configured URL.

**Input:**
- Start CLOUD_GATEWAY_CLIENT with `NATS_URL` pointing to an unreachable address.

**Expected:**
- The service retries 5 times with exponential backoff (1s, 2s, 4s) and then exits with code 1.
- An error log message indicates the NATS server is unreachable.

**Assertion pseudocode:**
```
set_env("NATS_URL", "nats://unreachable:4222")
result = start_service()
ASSERT result.exit_code == 1
ASSERT logs contain "unreachable" or "retries exhausted"
```

### TS-04-E12: DATA_BROKER connection failure at startup

**Requirement:** 04-REQ-3.E1
**Type:** unit
**Description:** Verify that the service exits with code 1 when the DATA_BROKER connection cannot be established.

**Preconditions:**
- NATS server is running (startup step 2 succeeds).
- DATA_BROKER is unreachable at the configured address.

**Input:**
- Start CLOUD_GATEWAY_CLIENT with `DATABROKER_ADDR` pointing to an unreachable address.

**Expected:**
- The service exits with code 1 and logs an error message about the DATA_BROKER connection failure.

**Assertion pseudocode:**
```
set_env("DATABROKER_ADDR", "http://unreachable:55556")
result = start_service()
ASSERT result.exit_code == 1
ASSERT logs contain "DATA_BROKER" or "connection failed"
```

### TS-04-E13: Response relay skips invalid JSON from DATA_BROKER

**Requirement:** 04-REQ-7.E1
**Type:** unit
**Description:** Verify that an invalid JSON value in Vehicle.Command.Door.Response is logged as an error and not published to NATS.

**Preconditions:**
- CLOUD_GATEWAY_CLIENT is running and connected to both NATS and DATA_BROKER.
- NATS subscriber listening on `vehicles.{VIN}.command_responses`.

**Input:**
- Set `Vehicle.Command.Door.Response` in DATA_BROKER to `"not-valid-json{{"`.

**Expected:**
- An error is logged.
- No message is published to `vehicles.{VIN}.command_responses` on NATS.

**Assertion pseudocode:**
```
databroker.set("Vehicle.Command.Door.Response", "not-valid-json{{")
wait(1s)
ASSERT nats_sub("vehicles.{VIN}.command_responses").next(timeout=1s) is None
ASSERT logs contain ERROR level entry about invalid JSON
```

## Coverage Matrix

| Requirement | Test Spec Entry | Type |
|-------------|-----------------|------|
| 04-REQ-1.1 | TS-04-1 | unit |
| 04-REQ-1.2 | TS-04-2 | unit |
| 04-REQ-1.3 | TS-04-2 | unit |
| 04-REQ-1.4 | TS-04-2 | unit |
| 04-REQ-1.E1 | TS-04-E1 | unit |
| 04-REQ-2.1 | TS-04-11 | integration |
| 04-REQ-2.2 | TS-04-16 | integration |
| 04-REQ-2.3 | TS-04-11 | integration |
| 04-REQ-2.E1 | TS-04-E11, TS-04-16 | unit, integration |
| 04-REQ-3.1 | TS-04-11 | integration |
| 04-REQ-3.2 | TS-04-13 | integration |
| 04-REQ-3.3 | TS-04-12 | integration |
| 04-REQ-3.E1 | TS-04-E12 | unit |
| 04-REQ-4.1 | TS-04-10, TS-04-14 | unit, integration |
| 04-REQ-4.2 | TS-04-14 | integration |
| 04-REQ-5.1 | TS-04-3, TS-04-P1 | unit, property |
| 04-REQ-5.2 | TS-04-3, TS-04-P1, TS-04-11 | unit, property, integration |
| 04-REQ-5.E1 | TS-04-E2, TS-04-P1 | unit, property |
| 04-REQ-5.E2 | TS-04-E3, TS-04-E4, TS-04-P1, TS-04-15 | unit, property, integration |
| 04-REQ-6.1 | TS-04-4, TS-04-P2 | unit, property |
| 04-REQ-6.2 | TS-04-4, TS-04-5, TS-04-P2 | unit, property |
| 04-REQ-6.3 | TS-04-P2, TS-04-P3, TS-04-11 | property, integration |
| 04-REQ-6.4 | TS-04-6, TS-04-P3 | unit, property |
| 04-REQ-6.E1 | TS-04-E5, TS-04-P2 | unit, property |
| 04-REQ-6.E2 | TS-04-E6, TS-04-E7, TS-04-E8, TS-04-E10, TS-04-P2 | unit, property |
| 04-REQ-6.E3 | TS-04-E9, TS-04-P2 | unit, property |
| 04-REQ-7.1 | TS-04-P4, TS-04-12 | property, integration |
| 04-REQ-7.2 | TS-04-P4, TS-04-12 | property, integration |
| 04-REQ-7.E1 | TS-04-E13 | unit |
| 04-REQ-8.1 | TS-04-7, TS-04-P5, TS-04-13 | unit, property, integration |
| 04-REQ-8.2 | TS-04-7, TS-04-9, TS-04-P5, TS-04-13 | unit, property, integration |
| 04-REQ-8.3 | TS-04-8, TS-04-P5 | unit, property |
| 04-REQ-9.1 | TS-04-P6, TS-04-14 | property, integration |
| 04-REQ-9.2 | TS-04-E1, TS-04-P6, TS-04-16 | unit, property, integration |
| 04-REQ-10.1 | TS-04-SMOKE-1 | integration |
| 04-REQ-10.2 | TS-04-SMOKE-1 | integration |
| 04-REQ-10.3 | TS-04-E2, TS-04-E3 | unit |
| 04-REQ-10.4 | TS-04-SMOKE-1 | integration |

## Integration Smoke Tests

### TS-04-SMOKE-1: Startup execution path

**Execution Path:** Path 1 from design.md
**Description:** Verify that the service starts, connects to NATS and DATA_BROKER, and publishes a registration message.

**Setup:** Real NATS container on `localhost:4222`, real DATA_BROKER container on `localhost:55556`. No stubs for any component in the execution path.

**Trigger:** Execute the `cloud-gateway-client` binary with `VIN="SMOKE-VIN"`.

**Expected side effects:**
- Process starts without error.
- Logs contain "Connected to NATS" (or equivalent).
- Logs contain "Connected to DATA_BROKER" (or equivalent).
- Registration message published to `vehicles.SMOKE-VIN.status`.

**Must NOT satisfy with:** Mocking `NatsClient::connect`, `BrokerClient::connect`, or `NatsClient::publish_registration`.

**Assertion pseudocode:**
```
nats_sub = nats.subscribe("vehicles.SMOKE-VIN.status")
process = start_binary(VIN="SMOKE-VIN")
ASSERT process.is_running()
msg = nats_sub.next(timeout=5s)
ASSERT msg is not None
ASSERT parse_json(msg.data).status == "online"
```

### TS-04-SMOKE-2: Command processing execution path

**Execution Path:** Path 2 from design.md
**Description:** Verify the full inbound command path from NATS to DATA_BROKER.

**Setup:** Real NATS container, real DATA_BROKER container. No stubs for `NatsClient`, `command_validator`, or `BrokerClient`.

**Trigger:** Publish an authenticated command to `vehicles.SMOKE-VIN.commands`.

**Expected side effects:**
- `Vehicle.Command.Door.Lock` in DATA_BROKER contains the command payload.

**Must NOT satisfy with:** Mocking `validate_bearer_token`, `validate_command_payload`, or `BrokerClient::write_command`.

**Assertion pseudocode:**
```
payload = '{"command_id":"smoke-1","action":"lock","doors":["driver"]}'
nats.publish("vehicles.SMOKE-VIN.commands", payload, headers={"Authorization": "Bearer demo-token"})
wait_up_to(2s)
value = databroker.get("Vehicle.Command.Door.Lock")
ASSERT value == payload
```

### TS-04-SMOKE-3: Response relay execution path

**Execution Path:** Path 3 from design.md
**Description:** Verify the full outbound response path from DATA_BROKER to NATS.

**Setup:** Real NATS container, real DATA_BROKER container. No stubs for `BrokerClient::subscribe_responses` or `NatsClient::publish_response`.

**Trigger:** Set `Vehicle.Command.Door.Response` in DATA_BROKER.

**Expected side effects:**
- Response JSON appears verbatim on `vehicles.SMOKE-VIN.command_responses` NATS subject.

**Must NOT satisfy with:** Mocking `BrokerClient::subscribe_responses` or `NatsClient::publish_response`.

**Assertion pseudocode:**
```
nats_sub = nats.subscribe("vehicles.SMOKE-VIN.command_responses")
response = '{"command_id":"smoke-1","status":"success","timestamp":1700000001}'
databroker.set("Vehicle.Command.Door.Response", response)
msg = nats_sub.next(timeout=2s)
ASSERT msg.data == response
```

### TS-04-SMOKE-4: Telemetry publishing execution path

**Execution Path:** Path 4 from design.md
**Description:** Verify the full outbound telemetry path from DATA_BROKER signal changes to NATS.

**Setup:** Real NATS container, real DATA_BROKER container. No stubs for `BrokerClient::subscribe_telemetry`, `TelemetryState::update`, or `NatsClient::publish_telemetry`.

**Trigger:** Set `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` to `true` in DATA_BROKER.

**Expected side effects:**
- Telemetry JSON appears on `vehicles.SMOKE-VIN.telemetry` NATS subject with `is_locked: true`.

**Must NOT satisfy with:** Mocking `BrokerClient::subscribe_telemetry`, `TelemetryState::update`, or `NatsClient::publish_telemetry`.

**Assertion pseudocode:**
```
nats_sub = nats.subscribe("vehicles.SMOKE-VIN.telemetry")
databroker.set("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", true)
msg = nats_sub.next(timeout=2s)
parsed = parse_json(msg.data)
ASSERT parsed.vin == "SMOKE-VIN"
ASSERT parsed.is_locked == true
```
