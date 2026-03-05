# Test Specification: CLOUD_GATEWAY_CLIENT (Spec 04)

> Test specifications for the CLOUD_GATEWAY_CLIENT component.
> Verifies requirements from `.specs/04_cloud_gateway_client/requirements.md`.

## References

- Requirements: `.specs/04_cloud_gateway_client/requirements.md`
- Design: `.specs/04_cloud_gateway_client/design.md`

## Test ID Convention

| Prefix | Category |
|--------|----------|
| TS-04-n | Positive / happy-path tests |
| TS-04-Pn | Pipeline / integration tests |
| TS-04-En | Error / edge-case tests |

## Test Infrastructure

- **Unit tests:** No external dependencies. Mock NATS and DATA_BROKER interactions.
- **Integration tests:** Require running NATS server (`localhost:4222`) and Kuksa Databroker (UDS or `localhost:55556`). Start with `make infra-up`.

---

## TS-04-1: NATS Connection and Command Subscription

**Requirement:** 04-REQ-1.1

**Description:** Verify that the CLOUD_GATEWAY_CLIENT connects to the NATS server and subscribes to the command subject scoped to its VIN.

**Preconditions:**
- NATS server is running at `localhost:4222`.
- `VIN` environment variable is set to `TEST_VIN_001`.

**Steps:**
1. Start the CLOUD_GATEWAY_CLIENT with `VIN=TEST_VIN_001` and `NATS_URL=nats://localhost:4222`.
2. Using a separate NATS client, publish a test message to `vehicles.TEST_VIN_001.commands`.
3. Verify the CLOUD_GATEWAY_CLIENT receives and processes the message.

**Expected result:**
- The client connects to NATS successfully.
- The client is subscribed to `vehicles.TEST_VIN_001.commands`.
- The test message is received by the client.

**Test type:** Integration

---

## TS-04-2: VIN Environment Variable Required

**Requirement:** 04-REQ-1.1

**Description:** Verify that the service exits with a non-zero code if the `VIN` environment variable is not set.

**Preconditions:**
- `VIN` environment variable is unset.

**Steps:**
1. Start the CLOUD_GATEWAY_CLIENT without setting `VIN`.
2. Observe the exit code and stderr output.

**Expected result:**
- The service exits with a non-zero exit code.
- An error message indicates that `VIN` is required.

**Test type:** Unit

---

## TS-04-P1: Command Reception and DATA_BROKER Write

**Requirement:** 04-REQ-2.1

**Description:** Verify that a valid command received via NATS is written to `Vehicle.Command.Door.Lock` on DATA_BROKER.

**Preconditions:**
- NATS server and DATA_BROKER are running.
- CLOUD_GATEWAY_CLIENT is running with `VIN=TEST_VIN_001`.

**Steps:**
1. Publish a valid command JSON to `vehicles.TEST_VIN_001.commands` via a NATS test client:
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
2. Read `Vehicle.Command.Door.Lock` from DATA_BROKER via gRPC.

**Expected result:**
- `Vehicle.Command.Door.Lock` on DATA_BROKER contains the command JSON.
- The `command_id`, `action`, `doors`, `source`, `vin`, and `timestamp` fields are preserved.

**Test type:** Integration

---

## TS-04-P2: Command Response Relay from DATA_BROKER to NATS

**Requirement:** 04-REQ-3.1

**Description:** Verify that a command response written to DATA_BROKER is published to the NATS command_responses subject.

**Preconditions:**
- NATS server and DATA_BROKER are running.
- CLOUD_GATEWAY_CLIENT is running with `VIN=TEST_VIN_001`.
- A NATS test client is subscribed to `vehicles.TEST_VIN_001.command_responses`.

**Steps:**
1. Write a response JSON to `Vehicle.Command.Door.Response` on DATA_BROKER:
   ```json
   {
     "command_id": "550e8400-e29b-41d4-a716-446655440000",
     "status": "success",
     "timestamp": 1700000001
   }
   ```
2. Wait for the NATS subscriber to receive a message on `vehicles.TEST_VIN_001.command_responses`.

**Expected result:**
- The response JSON is received on `vehicles.TEST_VIN_001.command_responses`.
- The `command_id` and `status` fields match the DATA_BROKER values.

**Test type:** Integration

---

## TS-04-P3: Telemetry Publishing from DATA_BROKER to NATS

**Requirement:** 04-REQ-4.1

**Description:** Verify that vehicle state changes on DATA_BROKER are published as telemetry to NATS.

**Preconditions:**
- NATS server and DATA_BROKER are running.
- CLOUD_GATEWAY_CLIENT is running with `VIN=TEST_VIN_001`.
- A NATS test client is subscribed to `vehicles.TEST_VIN_001.telemetry`.

**Steps:**
1. Write `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = true` to DATA_BROKER.
2. Wait for the NATS subscriber to receive a message on `vehicles.TEST_VIN_001.telemetry`.

**Expected result:**
- A telemetry JSON message is received on `vehicles.TEST_VIN_001.telemetry`.
- The message contains the signal name (`Vehicle.Cabin.Door.Row1.DriverSide.IsLocked`), value (`true`), and a timestamp.

**Test type:** Integration

---

## TS-04-P4: Telemetry for Multiple Signals

**Requirement:** 04-REQ-4.1

**Description:** Verify that telemetry is published for all subscribed signals: lock status, latitude, longitude, and parking session active.

**Preconditions:**
- NATS server and DATA_BROKER are running.
- CLOUD_GATEWAY_CLIENT is running with `VIN=TEST_VIN_001`.
- A NATS test client is subscribed to `vehicles.TEST_VIN_001.telemetry`.

**Steps:**
1. Write `Vehicle.CurrentLocation.Latitude = 48.1351` to DATA_BROKER.
2. Write `Vehicle.CurrentLocation.Longitude = 11.5820` to DATA_BROKER.
3. Write `Vehicle.Parking.SessionActive = true` to DATA_BROKER.
4. Collect telemetry messages received on NATS.

**Expected result:**
- A telemetry message is received for each signal change.
- Each message contains the correct signal name and value.

**Test type:** Integration

---

## TS-04-E1: Malformed Command JSON

**Requirement:** 04-REQ-2.2

**Description:** Verify that malformed JSON on the command subject is handled gracefully.

**Preconditions:**
- NATS server and DATA_BROKER are running.
- CLOUD_GATEWAY_CLIENT is running with `VIN=TEST_VIN_001`.

**Steps:**
1. Publish `not valid json {{{` to `vehicles.TEST_VIN_001.commands` via NATS.
2. Publish a valid command JSON to `vehicles.TEST_VIN_001.commands`.
3. Read `Vehicle.Command.Door.Lock` from DATA_BROKER.

**Expected result:**
- The malformed message is discarded; no write to DATA_BROKER for the first message.
- A warning is logged indicating the parse failure.
- The service continues running and processes the subsequent valid command successfully.

**Test type:** Integration

---

## TS-04-E2: Command with Missing Required Fields

**Requirement:** 04-REQ-2.2

**Description:** Verify that a command JSON missing required fields is rejected.

**Preconditions:**
- CLOUD_GATEWAY_CLIENT is running.

**Steps:**
1. Publish a JSON message missing the `action` field to `vehicles.{VIN}.commands`:
   ```json
   {
     "command_id": "550e8400-e29b-41d4-a716-446655440000",
     "doors": ["driver"],
     "source": "companion_app",
     "vin": "TEST_VIN_001",
     "timestamp": 1700000000
   }
   ```
2. Check DATA_BROKER for any write to `Vehicle.Command.Door.Lock`.

**Expected result:**
- The message is not written to DATA_BROKER.
- A warning is logged indicating the missing `action` field.

**Test type:** Integration

---

## TS-04-E3: NATS Reconnection After Connection Loss

**Requirement:** 04-REQ-1.2

**Description:** Verify that the client reconnects to NATS and resumes operation after a connection loss.

**Preconditions:**
- NATS server and DATA_BROKER are running.
- CLOUD_GATEWAY_CLIENT is running with `VIN=TEST_VIN_001`.

**Steps:**
1. Verify the client is connected by publishing and processing a valid command.
2. Stop the NATS server (`make infra-down` or stop the NATS container).
3. Wait 2 seconds.
4. Restart the NATS server.
5. Publish a valid command to `vehicles.TEST_VIN_001.commands`.
6. Read `Vehicle.Command.Door.Lock` from DATA_BROKER.

**Expected result:**
- The client does not crash during the NATS outage.
- After NATS is restarted, the client reconnects automatically.
- The command published after reconnection is received and written to DATA_BROKER.

**Test type:** Integration

---

## TS-04-E4: VIN Isolation in NATS Subjects

**Requirement:** 04-REQ-1.1, 04-REQ-4.1

**Description:** Verify that the client only processes messages scoped to its own VIN.

**Preconditions:**
- NATS server and DATA_BROKER are running.
- CLOUD_GATEWAY_CLIENT is running with `VIN=VIN_AAA`.

**Steps:**
1. Publish a valid command to `vehicles.VIN_BBB.commands` (different VIN).
2. Publish a valid command to `vehicles.VIN_AAA.commands` (matching VIN).
3. Read `Vehicle.Command.Door.Lock` from DATA_BROKER.

**Expected result:**
- Only the command for `VIN_AAA` is written to DATA_BROKER.
- The command for `VIN_BBB` is not received or processed by this client instance.

**Test type:** Integration

---

## TS-04-E5: DATA_BROKER Unreachable During Command Processing

**Requirement:** 04-REQ-5.1

**Description:** Verify that the service handles DATA_BROKER unavailability gracefully when a command arrives.

**Preconditions:**
- NATS server is running.
- DATA_BROKER is stopped or unreachable.
- CLOUD_GATEWAY_CLIENT is running (attempting to connect to DATA_BROKER).

**Steps:**
1. Publish a valid command to `vehicles.{VIN}.commands` via NATS.
2. Observe service behavior and logs.

**Expected result:**
- The service does not crash.
- An error is logged indicating DATA_BROKER is unreachable.
- The command is discarded (not silently lost -- logged).

**Test type:** Integration

---

## Traceability Matrix

| Test ID | Requirement | Category |
|---------|-------------|----------|
| TS-04-1 | 04-REQ-1.1 | Connection |
| TS-04-2 | 04-REQ-1.1 | Configuration |
| TS-04-P1 | 04-REQ-2.1 | Command pipeline |
| TS-04-P2 | 04-REQ-3.1 | Response relay |
| TS-04-P3 | 04-REQ-4.1 | Telemetry |
| TS-04-P4 | 04-REQ-4.1 | Telemetry |
| TS-04-E1 | 04-REQ-2.2 | Error handling |
| TS-04-E2 | 04-REQ-2.2 | Error handling |
| TS-04-E3 | 04-REQ-1.2 | Reconnection |
| TS-04-E4 | 04-REQ-1.1, 04-REQ-4.1 | VIN isolation |
| TS-04-E5 | 04-REQ-5.1 | Error handling |
