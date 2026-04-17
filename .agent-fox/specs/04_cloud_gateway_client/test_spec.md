# Test Specification: CLOUD_GATEWAY_CLIENT

## Overview

This test specification defines the unit, edge case, property, and integration tests for the CLOUD_GATEWAY_CLIENT component. Tests validate configuration parsing, command authentication and validation, telemetry state aggregation, and end-to-end data flows through NATS and DATA_BROKER.

## Test Cases

### TS-04-1: Config reads VIN from environment

**Validates:** [04-REQ-1.1]

```
GIVEN env VIN="TEST-VIN-001"
GIVEN env NATS_URL not set
GIVEN env DATABROKER_ADDR not set
GIVEN env BEARER_TOKEN not set
WHEN Config::from_env() is called
THEN result is Ok(config)
  AND config.vin == "TEST-VIN-001"
  AND config.nats_url == "nats://localhost:4222"
  AND config.databroker_addr == "http://localhost:55556"
  AND config.bearer_token == "demo-token"
```

## Edge Case Tests

### TS-04-E1: Config fails when VIN is missing

**Validates:** [04-REQ-1.E1]

```
GIVEN env VIN is not set
WHEN Config::from_env() is called
THEN result is Err(ConfigError::MissingVin)
```

### TS-04-2: Config reads all custom environment variables

**Validates:** [04-REQ-1.2], [04-REQ-1.3], [04-REQ-1.4]

```
GIVEN env VIN="MY-VIN"
GIVEN env NATS_URL="nats://custom:9222"
GIVEN env DATABROKER_ADDR="http://custom:55557"
GIVEN env BEARER_TOKEN="secret-token"
WHEN Config::from_env() is called
THEN result is Ok(config)
  AND config.nats_url == "nats://custom:9222"
  AND config.databroker_addr == "http://custom:55557"
  AND config.bearer_token == "secret-token"
```

### TS-04-3: Bearer token validation accepts valid token

**Validates:** [04-REQ-5.1], [04-REQ-5.2]

```
GIVEN headers contain "Authorization" = "Bearer demo-token"
GIVEN expected_token = "demo-token"
WHEN validate_bearer_token(headers, expected_token) is called
THEN result is Ok(())
```

### TS-04-E2: Bearer token validation rejects missing header

**Validates:** [04-REQ-5.E1]

```
GIVEN headers do not contain "Authorization"
GIVEN expected_token = "demo-token"
WHEN validate_bearer_token(headers, expected_token) is called
THEN result is Err(AuthError::MissingHeader)
```

### TS-04-E3: Bearer token validation rejects wrong token

**Validates:** [04-REQ-5.E2]

```
GIVEN headers contain "Authorization" = "Bearer wrong-token"
GIVEN expected_token = "demo-token"
WHEN validate_bearer_token(headers, expected_token) is called
THEN result is Err(AuthError::InvalidToken)
```

### TS-04-E4: Bearer token validation rejects malformed header

**Validates:** [04-REQ-5.E2]

```
GIVEN headers contain "Authorization" = "NotBearer demo-token"
GIVEN expected_token = "demo-token"
WHEN validate_bearer_token(headers, expected_token) is called
THEN result is Err(AuthError::InvalidToken)
```

### TS-04-4: Command validation accepts valid payload

**Validates:** [04-REQ-6.1], [04-REQ-6.2]

```
GIVEN payload = '{"command_id":"abc-123","action":"lock","doors":["driver"],"source":"companion_app","vin":"VIN1","timestamp":1700000000}'
WHEN validate_command_payload(payload) is called
THEN result is Ok(cmd)
  AND cmd.command_id == "abc-123"
  AND cmd.action == "lock"
  AND cmd.doors == ["driver"]
```

### TS-04-5: Command validation accepts unlock action

**Validates:** [04-REQ-6.2]

```
GIVEN payload = '{"command_id":"def-456","action":"unlock","doors":["driver"]}'
WHEN validate_command_payload(payload) is called
THEN result is Ok(cmd)
  AND cmd.action == "unlock"
```

### TS-04-E5: Command validation rejects invalid JSON

**Validates:** [04-REQ-6.E1]

```
GIVEN payload = 'not-valid-json{{'
WHEN validate_command_payload(payload) is called
THEN result is Err(ValidationError::InvalidJson(_))
```

### TS-04-E6: Command validation rejects missing command_id

**Validates:** [04-REQ-6.E2]

```
GIVEN payload = '{"action":"lock","doors":["driver"]}'
WHEN validate_command_payload(payload) is called
THEN result is Err(ValidationError::MissingField("command_id"))
```

### TS-04-E7: Command validation rejects empty command_id

**Validates:** [04-REQ-6.E2]

```
GIVEN payload = '{"command_id":"","action":"lock","doors":["driver"]}'
WHEN validate_command_payload(payload) is called
THEN result is Err(ValidationError::MissingField("command_id"))
```

### TS-04-E8: Command validation rejects missing action

**Validates:** [04-REQ-6.E2]

```
GIVEN payload = '{"command_id":"abc","doors":["driver"]}'
WHEN validate_command_payload(payload) is called
THEN result is Err(ValidationError::MissingField("action"))
```

### TS-04-E9: Command validation rejects invalid action

**Validates:** [04-REQ-6.E3]

```
GIVEN payload = '{"command_id":"abc","action":"open","doors":["driver"]}'
WHEN validate_command_payload(payload) is called
THEN result is Err(ValidationError::InvalidAction("open"))
```

### TS-04-E10: Command validation rejects missing doors

**Validates:** [04-REQ-6.E2]

```
GIVEN payload = '{"command_id":"abc","action":"lock"}'
WHEN validate_command_payload(payload) is called
THEN result is Err(ValidationError::MissingField("doors"))
```

### TS-04-6: Command validation does not validate door values

**Validates:** [04-REQ-6.4]

```
GIVEN payload = '{"command_id":"abc","action":"lock","doors":["unknown-door","another"]}'
WHEN validate_command_payload(payload) is called
THEN result is Ok(cmd)
  AND cmd.doors == ["unknown-door", "another"]
```

### TS-04-7: Telemetry state produces JSON on first update

**Validates:** [04-REQ-8.1], [04-REQ-8.2]

```
GIVEN state = TelemetryState::new("VIN-001")
WHEN state.update(SignalUpdate::IsLocked(true)) is called
THEN result is Some(json)
  AND json contains "vin":"VIN-001"
  AND json contains "is_locked":true
  AND json contains "timestamp"
  AND json does not contain "latitude"
  AND json does not contain "longitude"
  AND json does not contain "parking_active"
```

### TS-04-8: Telemetry state omits unset fields

**Validates:** [04-REQ-8.3]

```
GIVEN state = TelemetryState::new("VIN-001")
WHEN state.update(SignalUpdate::Latitude(48.1351)) is called
THEN result is Some(json)
  AND json contains "latitude":48.1351
  AND json does not contain "is_locked"
  AND json does not contain "longitude"
  AND json does not contain "parking_active"
```

### TS-04-9: Telemetry state includes all known fields

**Validates:** [04-REQ-8.2]

```
GIVEN state = TelemetryState::new("VIN-001")
GIVEN state.update(SignalUpdate::IsLocked(true)) was called
GIVEN state.update(SignalUpdate::Latitude(48.1351)) was called
GIVEN state.update(SignalUpdate::Longitude(11.582)) was called
WHEN state.update(SignalUpdate::ParkingActive(true)) is called
THEN result is Some(json)
  AND json contains "is_locked":true
  AND json contains "latitude":48.1351
  AND json contains "longitude":11.582
  AND json contains "parking_active":true
```

## Property Test Cases

### TS-04-P1: Registration message format

**Validates:** [04-REQ-4.1]

```
GIVEN vin = "VIN-001"
WHEN RegistrationMessage is serialized
THEN json contains "vin":"VIN-001"
  AND json contains "status":"online"
  AND json contains "timestamp"
```

### TS-04-P2: Command Structural Validity

**Property:** Property 2 from design.md
**Validates:** [04-REQ-6.1], [04-REQ-6.2], [04-REQ-6.3], [04-REQ-6.E1], [04-REQ-6.E2], [04-REQ-6.E3]
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
**Validates:** [04-REQ-6.3], [04-REQ-6.4]
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
**Validates:** [04-REQ-7.1], [04-REQ-7.2]
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
**Validates:** [04-REQ-8.1], [04-REQ-8.2], [04-REQ-8.3]
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
            parsed = json.parse(result)
            FOR field, value IN known:
                ASSERT parsed[field] == value
            FOR field NOT IN known:
                ASSERT field NOT IN parsed
```

### TS-04-P6: Startup Determinism

**Property:** Property 6 from design.md
**Validates:** [04-REQ-9.1], [04-REQ-9.2]
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

## Integration Tests

### TS-04-10: End-to-end command flow

**Validates:** [04-REQ-5.2], [04-REQ-6.3], [04-REQ-2.3]

**Requires:** NATS container, DATA_BROKER container

```
GIVEN CLOUD_GATEWAY_CLIENT is running with VIN="E2E-VIN"
GIVEN NATS subscriber is listening on DATA_BROKER for Vehicle.Command.Door.Lock
WHEN a NATS message is published to "vehicles.E2E-VIN.commands"
  WITH header "Authorization" = "Bearer demo-token"
  WITH payload '{"command_id":"cmd-1","action":"lock","doors":["driver"],"source":"companion_app","vin":"E2E-VIN","timestamp":1700000000}'
THEN within 2 seconds, Vehicle.Command.Door.Lock in DATA_BROKER contains the command payload
```

### TS-04-11: End-to-end response relay

**Validates:** [04-REQ-7.1], [04-REQ-7.2]

**Requires:** NATS container, DATA_BROKER container

```
GIVEN CLOUD_GATEWAY_CLIENT is running with VIN="E2E-VIN"
GIVEN NATS subscriber is listening on "vehicles.E2E-VIN.command_responses"
WHEN Vehicle.Command.Door.Response is set to '{"command_id":"cmd-1","status":"success","timestamp":1700000001}' in DATA_BROKER
THEN within 2 seconds, the NATS subscriber receives the response JSON verbatim
```

### TS-04-12: End-to-end telemetry on signal change

**Validates:** [04-REQ-8.1], [04-REQ-8.2]

**Requires:** NATS container, DATA_BROKER container

```
GIVEN CLOUD_GATEWAY_CLIENT is running with VIN="E2E-VIN"
GIVEN NATS subscriber is listening on "vehicles.E2E-VIN.telemetry"
WHEN Vehicle.Cabin.Door.Row1.DriverSide.IsLocked is set to true in DATA_BROKER
THEN within 2 seconds, the NATS subscriber receives a telemetry JSON
  AND the JSON contains "vin":"E2E-VIN"
  AND the JSON contains "is_locked":true
```

### TS-04-13: Self-registration on startup

**Validates:** [04-REQ-4.1], [04-REQ-4.2]

**Requires:** NATS container

```
GIVEN NATS subscriber is listening on "vehicles.REG-VIN.status"
WHEN CLOUD_GATEWAY_CLIENT is started with VIN="REG-VIN"
THEN within 5 seconds, the NATS subscriber receives a registration message
  AND the JSON contains "vin":"REG-VIN"
  AND the JSON contains "status":"online"
```

### TS-04-14: Command rejected with invalid token

**Validates:** [04-REQ-5.E2]

**Requires:** NATS container, DATA_BROKER container

```
GIVEN CLOUD_GATEWAY_CLIENT is running with VIN="E2E-VIN"
WHEN a NATS message is published to "vehicles.E2E-VIN.commands"
  WITH header "Authorization" = "Bearer wrong-token"
  WITH payload '{"command_id":"cmd-2","action":"lock","doors":["driver"]}'
THEN Vehicle.Command.Door.Lock in DATA_BROKER is NOT updated
  AND no message is published to "vehicles.E2E-VIN.command_responses"
```

### TS-04-15: NATS reconnection with exponential backoff

**Validates:** [04-REQ-2.2], [04-REQ-2.E1]

**Requires:** NATS container (initially stopped)

```
GIVEN NATS server is not running
WHEN CLOUD_GATEWAY_CLIENT is started
THEN the service attempts to connect at t=0, t~1s, t~3s, t~7s, t~15s
  AND after 5 failed attempts, the service exits with code 1
```

## Coverage Matrix

| Requirement | Unit Tests | Integration Tests |
|-------------|-----------|-------------------|
| [04-REQ-1.1] | TS-04-1 | TS-04-10 |
| [04-REQ-1.2] | TS-04-2 | - |
| [04-REQ-1.3] | TS-04-2 | - |
| [04-REQ-1.4] | TS-04-2 | - |
| [04-REQ-1.E1] | TS-04-E1 | - |
| [04-REQ-2.1] | - | TS-04-10 |
| [04-REQ-2.2] | - | TS-04-15 |
| [04-REQ-2.3] | - | TS-04-10 |
| [04-REQ-2.E1] | - | TS-04-15 |
| [04-REQ-3.1] | - | TS-04-10 |
| [04-REQ-3.2] | - | TS-04-12 |
| [04-REQ-3.3] | - | TS-04-11 |
| [04-REQ-3.E1] | - | - |
| [04-REQ-4.1] | TS-04-P1 | TS-04-13 |
| [04-REQ-4.2] | - | TS-04-13 |
| [04-REQ-5.1] | TS-04-3 | TS-04-10 |
| [04-REQ-5.2] | TS-04-3 | TS-04-10 |
| [04-REQ-5.E1] | TS-04-E2 | - |
| [04-REQ-5.E2] | TS-04-E3, TS-04-E4 | TS-04-14 |
| [04-REQ-6.1] | TS-04-4, TS-04-P2 | TS-04-10 |
| [04-REQ-6.2] | TS-04-4, TS-04-5, TS-04-P2 | TS-04-10 |
| [04-REQ-6.3] | TS-04-P2, TS-04-P3 | TS-04-10 |
| [04-REQ-6.4] | TS-04-6, TS-04-P3 | - |
| [04-REQ-6.E1] | TS-04-E5, TS-04-P2 | - |
| [04-REQ-6.E2] | TS-04-E6, TS-04-E7, TS-04-E8, TS-04-E10, TS-04-P2 | - |
| [04-REQ-6.E3] | TS-04-E9, TS-04-P2 | - |
| [04-REQ-7.1] | TS-04-P4 | TS-04-11 |
| [04-REQ-7.2] | TS-04-P4 | TS-04-11 |
| [04-REQ-7.E1] | - | - |
| [04-REQ-8.1] | TS-04-7, TS-04-P5 | TS-04-12 |
| [04-REQ-8.2] | TS-04-7, TS-04-9, TS-04-P5 | TS-04-12 |
| [04-REQ-8.3] | TS-04-8, TS-04-P5 | - |
| [04-REQ-9.1] | TS-04-P6 | TS-04-13 |
| [04-REQ-9.2] | TS-04-E1, TS-04-P6 | TS-04-15 |
| [04-REQ-10.1] | - | - |
| [04-REQ-10.2] | - | - |
| [04-REQ-10.3] | - | - |
| [04-REQ-10.4] | - | - |

## Integration Smoke Tests

### TS-04-SMOKE-1: Service starts with valid configuration

```
GIVEN NATS container is running on localhost:4222
GIVEN DATA_BROKER container is running on localhost:55556
GIVEN env VIN="SMOKE-VIN"
WHEN CLOUD_GATEWAY_CLIENT binary is executed
THEN the process starts without error
  AND logs contain "Connected to NATS"
  AND logs contain "Connected to DATA_BROKER"
```

### TS-04-SMOKE-2: Service exits on missing VIN

```
GIVEN env VIN is not set
WHEN CLOUD_GATEWAY_CLIENT binary is executed
THEN the process exits with code 1
  AND stderr contains "VIN"
```

### TS-04-SMOKE-3: Service publishes registration on startup

```
GIVEN NATS container is running on localhost:4222
GIVEN DATA_BROKER container is running on localhost:55556
GIVEN NATS subscriber is listening on "vehicles.SMOKE-VIN.status"
GIVEN env VIN="SMOKE-VIN"
WHEN CLOUD_GATEWAY_CLIENT binary is executed
THEN within 5 seconds, a registration message is received on "vehicles.SMOKE-VIN.status"
```
