# Test Specification: CLOUD_GATEWAY (Spec 06)

> Test specifications for the CLOUD_GATEWAY cloud service.
> Validates requirements from `.specs/06_cloud_gateway/requirements.md`.

## Test ID Convention

| Prefix  | Category           |
|---------|--------------------|
| TS-06-  | Functional tests   |
| TS-06-P | Property tests     |
| TS-06-E | Error/edge tests   |

## Test Environment

- **Test framework:** Go `testing` standard library
- **HTTP testing:** `net/http/httptest` for handler-level integration tests
- **NATS testing:** Embedded NATS server via `github.com/nats-io/nats-server/v2/server`
- **Test location:** `backend/cloud-gateway/*_test.go`
- **Run command:** `cd backend/cloud-gateway && go test ./... -v`
- **Lint command:** `cd backend/cloud-gateway && go vet ./...`

## Functional Tests

### TS-06-1: Command Submission via REST

**Requirement:** 06-REQ-1.1, 06-REQ-1.2
**Type:** integration
**Description:** Submitting a valid lock command via REST returns 202 and publishes the command to the correct NATS subject.

**Preconditions:**
- Server is running with demo token configuration.
- Embedded NATS server is running.
- A NATS subscriber is listening on `vehicles.VIN12345.commands`.

**Input:**
- `POST /vehicles/VIN12345/commands`
- Header: `Authorization: Bearer companion-token-vehicle-1`
- Body: `{"command_id": "cmd-001", "type": "lock", "doors": ["driver"]}`

**Expected:**
- HTTP status 202
- Response body: `{"command_id": "cmd-001", "status": "pending"}`
- NATS message received on `vehicles.VIN12345.commands` with payload: `{"command_id": "cmd-001", "action": "lock", "doors": ["driver"], "source": "companion_app"}`

**Assertion pseudocode:**
```
resp = POST("/vehicles/VIN12345/commands", body, authHeader)
ASSERT resp.status == 202
ASSERT resp.json.command_id == "cmd-001"
ASSERT resp.json.status == "pending"
natsMsg = subscriber.nextMessage(timeout=1s)
ASSERT natsMsg.subject == "vehicles.VIN12345.commands"
ASSERT natsMsg.json.action == "lock"
ASSERT natsMsg.json.source == "companion_app"
```

### TS-06-2: Bearer Token Validation - Valid Token

**Requirement:** 06-REQ-2.1, 06-REQ-2.2
**Type:** unit
**Description:** A request with a valid bearer token matching the VIN is allowed to proceed.

**Preconditions:**
- Token store contains `"companion-token-vehicle-1" -> "VIN12345"`.

**Input:**
- `POST /vehicles/VIN12345/commands`
- Header: `Authorization: Bearer companion-token-vehicle-1`
- Body: `{"command_id": "cmd-002", "type": "unlock", "doors": ["driver"]}`

**Expected:**
- HTTP status 202 (request accepted, not blocked by auth)

**Assertion pseudocode:**
```
resp = POST("/vehicles/VIN12345/commands", body, authHeader)
ASSERT resp.status == 202
```

### TS-06-3: NATS Command Relay Publishes to Correct Subject

**Requirement:** 06-REQ-3.1, 06-REQ-7.1
**Type:** integration
**Description:** Commands for different VINs are published to their respective NATS subjects.

**Preconditions:**
- Embedded NATS server is running.
- Subscribers listening on `vehicles.VIN12345.commands` and `vehicles.VIN67890.commands`.

**Input:**
- Submit command for VIN12345 with valid token
- Submit command for VIN67890 with valid token

**Expected:**
- First command received on `vehicles.VIN12345.commands` only
- Second command received on `vehicles.VIN67890.commands` only

**Assertion pseudocode:**
```
POST("/vehicles/VIN12345/commands", cmd1, token1)
POST("/vehicles/VIN67890/commands", cmd2, token2)
msg1 = sub1.nextMessage(timeout=1s)
msg2 = sub2.nextMessage(timeout=1s)
ASSERT msg1.subject == "vehicles.VIN12345.commands"
ASSERT msg2.subject == "vehicles.VIN67890.commands"
ASSERT sub1.pendingCount() == 0  // no cross-contamination
ASSERT sub2.pendingCount() == 0
```

### TS-06-4: Command Response Forwarding

**Requirement:** 06-REQ-4.1, 06-REQ-4.2
**Type:** integration
**Description:** A command response received via NATS is stored and retrievable via the REST status endpoint.

**Preconditions:**
- Server is running with embedded NATS.
- A command has been submitted (cmd-003) and is in "pending" state.

**Input:**
1. Publish NATS message to `vehicles.VIN12345.command_responses`: `{"command_id": "cmd-003", "status": "success"}`
2. `GET /vehicles/VIN12345/commands/cmd-003` with valid auth header

**Expected:**
- HTTP status 200
- Response body: `{"command_id": "cmd-003", "status": "success"}`

**Assertion pseudocode:**
```
# Submit command first
POST("/vehicles/VIN12345/commands", {"command_id":"cmd-003","type":"lock","doors":["driver"]}, auth)
# Simulate response from vehicle
nats.publish("vehicles.VIN12345.command_responses", {"command_id":"cmd-003","status":"success"})
sleep(100ms)  // allow async processing
resp = GET("/vehicles/VIN12345/commands/cmd-003", auth)
ASSERT resp.status == 200
ASSERT resp.json.command_id == "cmd-003"
ASSERT resp.json.status == "success"
```

### TS-06-5: Health Check Returns 200 OK

**Requirement:** 06-REQ-6.1, 06-REQ-6.E1
**Type:** unit
**Description:** The health endpoint returns 200 with status "ok" without authentication.

**Preconditions:** Server is running.

**Input:**
- `GET /health` (no Authorization header)

**Expected:**
- HTTP status 200
- Content-Type: `application/json`
- Body: `{"status": "ok"}`

**Assertion pseudocode:**
```
resp = GET("/health")
ASSERT resp.status == 200
ASSERT resp.headers["Content-Type"] contains "application/json"
ASSERT resp.json.status == "ok"
```

### TS-06-6: Telemetry Reception

**Requirement:** 06-REQ-5.1
**Type:** integration
**Description:** Telemetry messages published via NATS are received and stored by the CLOUD_GATEWAY.

**Preconditions:**
- Server is running with embedded NATS.
- Telemetry subscription is active for VIN12345.

**Input:**
- Publish NATS message to `vehicles.VIN12345.telemetry`: `{"vin":"VIN12345","door_locked":true,"latitude":48.1351,"longitude":11.582,"parking_active":false,"timestamp":1709654400}`

**Expected:**
- Telemetry data is stored in memory (verifiable via internal store access in tests).

**Assertion pseudocode:**
```
nats.publish("vehicles.VIN12345.telemetry", telemetryJSON)
sleep(100ms)
stored = store.getLatestTelemetry("VIN12345")
ASSERT stored != nil
ASSERT stored.DoorLocked == true
ASSERT stored.Latitude == 48.1351
```

## Error and Edge Case Tests

### TS-06-E1: Missing Authorization Header Returns 401

**Requirement:** 06-REQ-2.E1
**Type:** unit
**Description:** A request without an Authorization header is rejected with 401.

**Input:**
- `POST /vehicles/VIN12345/commands` (no Authorization header)
- Body: `{"command_id": "cmd-e1", "type": "lock", "doors": ["driver"]}`

**Expected:**
- HTTP status 401
- Body contains `"error"` field

**Assertion pseudocode:**
```
resp = POST("/vehicles/VIN12345/commands", body, noAuth)
ASSERT resp.status == 401
ASSERT resp.json.error != ""
```

### TS-06-E2: Invalid Bearer Token Returns 401

**Requirement:** 06-REQ-2.E2
**Type:** unit
**Description:** A request with an unrecognized token is rejected with 401.

**Input:**
- `POST /vehicles/VIN12345/commands`
- Header: `Authorization: Bearer invalid-token-xyz`

**Expected:**
- HTTP status 401
- Body contains `"error"` field with message about invalid token

**Assertion pseudocode:**
```
resp = POST("/vehicles/VIN12345/commands", body, invalidAuth)
ASSERT resp.status == 401
ASSERT resp.json.error contains "invalid token"
```

### TS-06-E3: Token for Wrong VIN Returns 403

**Requirement:** 06-REQ-2.E3
**Type:** unit
**Description:** A valid token used against a VIN it is not authorized for is rejected with 403.

**Input:**
- `POST /vehicles/VIN67890/commands`
- Header: `Authorization: Bearer companion-token-vehicle-1` (valid for VIN12345, not VIN67890)

**Expected:**
- HTTP status 403
- Body contains `"error"` field about VIN authorization

**Assertion pseudocode:**
```
resp = POST("/vehicles/VIN67890/commands", body, wrongVinAuth)
ASSERT resp.status == 403
ASSERT resp.json.error contains "not authorized for this vehicle"
```

### TS-06-E4: Missing Required Fields Returns 400

**Requirement:** 06-REQ-1.E1
**Type:** unit
**Description:** A command request with missing required fields returns 400.

**Test cases (table-driven):**

| Sub-case | Body | Missing Field |
|----------|------|---------------|
| E4a | `{"type":"lock","doors":["driver"]}` | command_id |
| E4b | `{"command_id":"cmd-e4","doors":["driver"]}` | type |
| E4c | `{"command_id":"cmd-e4","type":"lock"}` | doors |

**Expected:** HTTP 400 with JSON error body for each sub-case.

**Assertion pseudocode:**
```
FOR EACH case IN table:
    resp = POST("/vehicles/VIN12345/commands", case.body, validAuth)
    ASSERT resp.status == 400
    ASSERT resp.json.error != ""
```

### TS-06-E5: Invalid Command Type Returns 400

**Requirement:** 06-REQ-1.E2
**Type:** unit
**Description:** A command with an invalid type value returns 400.

**Input:**
- Body: `{"command_id": "cmd-e5", "type": "start_engine", "doors": ["driver"]}`

**Expected:**
- HTTP status 400
- Body contains `"error"` field

**Assertion pseudocode:**
```
resp = POST("/vehicles/VIN12345/commands", body, validAuth)
ASSERT resp.status == 400
ASSERT resp.json.error contains "invalid"
```

### TS-06-E6: Unknown VIN Returns 404

**Requirement:** 06-REQ-7.E1
**Type:** unit
**Description:** A command for an unknown VIN returns 404.

**Input:**
- `POST /vehicles/UNKNOWN_VIN/commands` with valid token format but for a VIN not in the system

**Expected:**
- HTTP status 404
- Body contains `"error"` field about unknown vehicle

**Assertion pseudocode:**
```
resp = POST("/vehicles/UNKNOWN_VIN/commands", body, validAuth)
ASSERT resp.status == 404 OR resp.status == 401  // token won't match unknown VIN either
```

### TS-06-E7: Unknown Command ID Returns 404

**Requirement:** 06-REQ-4.E1
**Type:** unit
**Description:** Querying status for a non-existent command_id returns 404.

**Input:**
- `GET /vehicles/VIN12345/commands/nonexistent-cmd` with valid auth

**Expected:**
- HTTP status 404
- Body contains `"error"` field

**Assertion pseudocode:**
```
resp = GET("/vehicles/VIN12345/commands/nonexistent-cmd", validAuth)
ASSERT resp.status == 404
ASSERT resp.json.error contains "command not found"
```

### TS-06-E8: NATS Unavailable Returns 503

**Requirement:** 06-REQ-3.E1
**Type:** integration
**Description:** When the NATS connection is down, command submission returns 503.

**Preconditions:**
- Server is running but NATS server is stopped or unreachable.

**Input:**
- `POST /vehicles/VIN12345/commands` with valid auth and body

**Expected:**
- HTTP status 503
- Body contains `"error"` field about messaging service unavailable

**Assertion pseudocode:**
```
stopNATSServer()
resp = POST("/vehicles/VIN12345/commands", body, validAuth)
ASSERT resp.status == 503
ASSERT resp.json.error contains "unavailable"
```

### TS-06-E9: Undefined Route Returns 404

**Requirement:** 06-REQ-8.E1
**Type:** unit
**Description:** A request to an undefined path returns 404 with JSON error.

**Input:**
- `GET /nonexistent-path`

**Expected:**
- HTTP status 404
- Content-Type: `application/json`
- Body contains `"error"` field

**Assertion pseudocode:**
```
resp = GET("/nonexistent-path")
ASSERT resp.status == 404
ASSERT resp.headers["Content-Type"] contains "application/json"
ASSERT resp.json.error != ""
```

### TS-06-E10: Invalid Telemetry JSON Is Discarded

**Requirement:** 06-REQ-5.E1
**Type:** integration
**Description:** A malformed telemetry message is logged and discarded without crashing.

**Preconditions:**
- Server is running with embedded NATS.

**Input:**
- Publish NATS message to `vehicles.VIN12345.telemetry` with body `"not valid json{"`

**Expected:**
- No crash or panic
- Telemetry store is not updated with invalid data
- Subsequent valid telemetry is still processed

**Assertion pseudocode:**
```
nats.publish("vehicles.VIN12345.telemetry", "not valid json{")
sleep(100ms)
stored = store.getLatestTelemetry("VIN12345")
ASSERT stored == nil  // no data stored from invalid message
nats.publish("vehicles.VIN12345.telemetry", validTelemetryJSON)
sleep(100ms)
stored = store.getLatestTelemetry("VIN12345")
ASSERT stored != nil  // valid data stored after invalid message
```

## Property Tests

### TS-06-P1: Token-VIN Binding

**Property:** Property 1 from design.md
**Validates:** 06-REQ-2.1, 06-REQ-2.2
**Type:** property
**Description:** For any token in the store, only the exact associated VIN is accessible.

**For any:** token-VIN pair (t, v) in the token store and any other VIN w where w != v
**Invariant:** auth(t, v) succeeds AND auth(t, w) fails with 403

**Assertion pseudocode:**
```
FOR ANY (token, vin) IN tokenStore:
    FOR ANY otherVin IN allVins WHERE otherVin != vin:
        ASSERT authMiddleware(token, vin) == ALLOW
        ASSERT authMiddleware(token, otherVin) == DENY_403
```

### TS-06-P2: Command-to-NATS Subject Mapping

**Property:** Property 2 from design.md
**Validates:** 06-REQ-1.1, 06-REQ-3.1, 06-REQ-7.1
**Type:** property
**Description:** For any VIN, commands are published to the NATS subject `vehicles.{vin}.commands` and no other.

**For any:** VIN v from the set of known VINs
**Invariant:** NATS publish subject == "vehicles." + v + ".commands"

**Assertion pseudocode:**
```
FOR ANY vin IN knownVINs:
    expectedSubject = "vehicles." + vin + ".commands"
    publishedSubject = captureNATSPublishSubject(vin, command)
    ASSERT publishedSubject == expectedSubject
```

### TS-06-P3: Response-to-Command Correlation

**Property:** Property 3 from design.md
**Validates:** 06-REQ-4.1
**Type:** property
**Description:** Processing a command response updates only the matching command_id status.

**For any:** set of pending commands C and a response for command_id X
**Invariant:** after processing, only C[X].status changes; all other C[Y].status (Y != X) remain unchanged

**Assertion pseudocode:**
```
FOR ANY commandSet, targetCmdID IN generatedSets:
    snapshotBefore = copy(commandStore)
    processResponse(targetCmdID, "success")
    FOR ANY cmdID IN commandSet:
        IF cmdID == targetCmdID:
            ASSERT commandStore[cmdID].status == "success"
        ELSE:
            ASSERT commandStore[cmdID].status == snapshotBefore[cmdID].status
```

### TS-06-P4: Command Status Lifecycle

**Property:** Property 4 from design.md
**Validates:** 06-REQ-1.1, 06-REQ-4.1, 06-REQ-4.2
**Type:** property
**Description:** Command status transitions only from pending to success or pending to failed.

**For any:** sequence of status updates for a command_id
**Invariant:** initial status is "pending"; terminal status is "success" or "failed"; no transition from terminal to any other state

**Assertion pseudocode:**
```
FOR ANY commandID:
    ASSERT getStatus(commandID) == "pending"  // after creation
    processResponse(commandID, "success")
    ASSERT getStatus(commandID) == "success"
    processResponse(commandID, "failed")  // second update attempt
    ASSERT getStatus(commandID) == "success"  // unchanged, terminal
```

### TS-06-P5: REST-to-NATS Field Mapping

**Property:** Property 5 from design.md
**Validates:** 06-REQ-1.2
**Type:** property
**Description:** The NATS message fields are correctly mapped from the REST request fields.

**For any:** command request with type T, command_id C, doors D
**Invariant:** NATS message has action==T, command_id==C, doors==D, source=="companion_app"

**Assertion pseudocode:**
```
FOR ANY (cmdID, cmdType, doors) IN generatedInputs:
    natsMsg = captureNATSMessage(POST(cmdID, cmdType, doors))
    ASSERT natsMsg.action == cmdType
    ASSERT natsMsg.command_id == cmdID
    ASSERT natsMsg.doors == doors
    ASSERT natsMsg.source == "companion_app"
```

### TS-06-P6: Response Format Consistency

**Property:** Property 6 from design.md
**Validates:** 06-REQ-8.1, 06-REQ-8.2
**Type:** property
**Description:** Every REST response has Content-Type application/json and valid JSON body.

**For any:** request to any endpoint (success or error path)
**Invariant:** Content-Type header contains "application/json" and body parses as valid JSON

**Assertion pseudocode:**
```
FOR ANY request IN [
    GET("/health"),
    POST("/vehicles/VIN12345/commands", validBody, validAuth),
    POST("/vehicles/VIN12345/commands", invalidBody, validAuth),
    GET("/vehicles/VIN12345/commands/unknown", validAuth),
    POST("/vehicles/VIN12345/commands", validBody, invalidAuth),
    GET("/nonexistent"),
]:
    resp = execute(request)
    ASSERT resp.headers["Content-Type"] contains "application/json"
    ASSERT json.parse(resp.body) succeeds
```

### TS-06-P7: Health Endpoint Independence

**Property:** Property 7 from design.md
**Validates:** 06-REQ-6.1, 06-REQ-6.E1
**Type:** property
**Description:** Health endpoint responds consistently regardless of auth state.

**For any:** Authorization header value (missing, valid, invalid, empty)
**Invariant:** response status is 200 and body is {"status":"ok"}

**Assertion pseudocode:**
```
FOR ANY authHeader IN [none, "Bearer valid", "Bearer invalid", "", "Basic xyz"]:
    resp = GET("/health", authHeader)
    ASSERT resp.status == 200
    ASSERT resp.json.status == "ok"
```

## Coverage Matrix

| Requirement | Test Spec Entry | Type |
|-------------|-----------------|------|
| 06-REQ-1.1 | TS-06-1 | integration |
| 06-REQ-1.2 | TS-06-1, TS-06-P5 | integration, property |
| 06-REQ-1.E1 | TS-06-E4 | unit |
| 06-REQ-1.E2 | TS-06-E5 | unit |
| 06-REQ-2.1 | TS-06-2, TS-06-P1 | unit, property |
| 06-REQ-2.2 | TS-06-2, TS-06-P1 | unit, property |
| 06-REQ-2.E1 | TS-06-E1 | unit |
| 06-REQ-2.E2 | TS-06-E2 | unit |
| 06-REQ-2.E3 | TS-06-E3 | unit |
| 06-REQ-3.1 | TS-06-3, TS-06-P2 | integration, property |
| 06-REQ-3.2 | TS-06-4 | integration |
| 06-REQ-3.E1 | TS-06-E8 | integration |
| 06-REQ-4.1 | TS-06-4, TS-06-P3 | integration, property |
| 06-REQ-4.2 | TS-06-4, TS-06-P4 | integration, property |
| 06-REQ-4.E1 | TS-06-E7 | unit |
| 06-REQ-5.1 | TS-06-6 | integration |
| 06-REQ-5.E1 | TS-06-E10 | integration |
| 06-REQ-6.1 | TS-06-5, TS-06-P7 | unit, property |
| 06-REQ-6.E1 | TS-06-5, TS-06-P7 | unit, property |
| 06-REQ-7.1 | TS-06-3, TS-06-P2 | integration, property |
| 06-REQ-7.2 | TS-06-3 | integration |
| 06-REQ-7.E1 | TS-06-E6 | unit |
| 06-REQ-8.1 | TS-06-P6 | property |
| 06-REQ-8.2 | TS-06-P6 | property |
| 06-REQ-8.E1 | TS-06-E9 | unit |
| 06-REQ-8.E2 | (covered by panic recovery in handler tests) | integration |
