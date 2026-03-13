# Test Specification: CLOUD_GATEWAY

## Overview

This test specification defines concrete test contracts for the CLOUD_GATEWAY, a Go HTTP server with dual REST/NATS interfaces. Tests are organized into unit tests (auth, config, store, handler, nats_client packages) and integration tests (httptest-based HTTP tests and NATS-connected end-to-end tests). Unit tests run via `cd backend && go test -v ./cloud-gateway/...`. Integration tests requiring NATS run in `tests/cloud-gateway/`.

## Test Cases

### TS-06-1: Command Submission via REST

**Requirement:** 06-REQ-1.1
**Type:** integration
**Description:** A valid POST to `/vehicles/{vin}/commands` publishes the command to NATS and returns HTTP 202.

**Preconditions:**
- Service running with NATS connected and demo token `demo-token-car1` mapped to VIN `VIN12345`.

**Input:**
- `POST /vehicles/VIN12345/commands`
- Header: `Authorization: Bearer demo-token-car1`
- Body: `{"command_id":"cmd-001","type":"lock","doors":["driver"]}`

**Expected:**
- HTTP 202
- Body contains `{"command_id":"cmd-001","status":"pending"}`
- Command published to NATS subject `vehicles.VIN12345.commands`

**Assertion pseudocode:**
```
resp = httptest.POST("/vehicles/VIN12345/commands", body, authHeader)
ASSERT resp.StatusCode == 202
body = json.Decode(resp.Body)
ASSERT body.command_id == "cmd-001"
ASSERT body.status == "pending"
```

### TS-06-2: Command Payload Structure

**Requirement:** 06-REQ-1.2
**Type:** unit
**Description:** The command payload is validated to contain command_id, type, and doors.

**Preconditions:**
- None.

**Input:**
- Valid: `{"command_id":"c1","type":"lock","doors":["driver"]}`
- Missing type: `{"command_id":"c1","doors":["driver"]}`
- Missing doors: `{"command_id":"c1","type":"lock"}`

**Expected:**
- Valid input parses successfully.
- Missing fields return validation error.

**Assertion pseudocode:**
```
cmd, err = parseCommand(validJSON)
ASSERT err == nil
ASSERT cmd.CommandID == "c1"

_, err = parseCommand(missingTypeJSON)
ASSERT err != nil
```

### TS-06-3: Bearer Token in NATS Header

**Requirement:** 06-REQ-1.3
**Type:** integration
**Description:** When publishing a command to NATS, the bearer token is included as a NATS message header.

**Preconditions:**
- NATS server running. Service connected.

**Input:**
- Submit command with token `demo-token-car1`.

**Expected:**
- NATS message on `vehicles.VIN12345.commands` has header `Authorization: Bearer demo-token-car1`.

**Assertion pseudocode:**
```
// Subscribe to NATS, submit command via REST
msg = nats.Subscribe("vehicles.VIN12345.commands")
submitCommand(token="demo-token-car1")
received = msg.Next()
ASSERT received.Header.Get("Authorization") == "Bearer demo-token-car1"
```

### TS-06-4: Command Stored as Pending

**Requirement:** 06-REQ-1.4
**Type:** unit
**Description:** After submission, the command is stored with status "pending" in the in-memory store.

**Preconditions:**
- Empty command store.

**Input:**
- Add command with ID "cmd-001".

**Expected:**
- Store.Get("cmd-001") returns status "pending".

**Assertion pseudocode:**
```
store = NewStore()
store.Add(CommandStatus{CommandID: "cmd-001", Status: "pending"})
cs, found = store.Get("cmd-001")
ASSERT found == true
ASSERT cs.Status == "pending"
```

### TS-06-5: Command Status Query

**Requirement:** 06-REQ-2.1
**Type:** integration
**Description:** GET `/vehicles/{vin}/commands/{command_id}` returns the current command status.

**Preconditions:**
- Command "cmd-001" exists in store with status "success".

**Input:**
- `GET /vehicles/VIN12345/commands/cmd-001`
- Header: `Authorization: Bearer demo-token-car1`

**Expected:**
- HTTP 200
- Body: `{"command_id":"cmd-001","status":"success"}`

**Assertion pseudocode:**
```
resp = httptest.GET("/vehicles/VIN12345/commands/cmd-001", authHeader)
ASSERT resp.StatusCode == 200
body = json.Decode(resp.Body)
ASSERT body.command_id == "cmd-001"
ASSERT body.status == "success"
```

### TS-06-6: Pending Status Before Response

**Requirement:** 06-REQ-2.2
**Type:** unit
**Description:** A newly submitted command returns status "pending".

**Preconditions:**
- Command "cmd-002" submitted, no response received.

**Input:**
- Query command "cmd-002".

**Expected:**
- Status is "pending".

**Assertion pseudocode:**
```
store.Add(CommandStatus{CommandID: "cmd-002", Status: "pending"})
cs, _ = store.Get("cmd-002")
ASSERT cs.Status == "pending"
```

### TS-06-7: Success and Failed Status

**Requirement:** 06-REQ-2.3
**Type:** unit
**Description:** After receiving a response, the command status reflects the response status.

**Preconditions:**
- Command "cmd-003" is pending.

**Input:**
- Update with response: `{command_id: "cmd-003", status: "failed", reason: "vehicle_moving"}`.

**Expected:**
- Status is "failed", reason is "vehicle_moving".

**Assertion pseudocode:**
```
store.Add(CommandStatus{CommandID: "cmd-003", Status: "pending"})
store.UpdateFromResponse(CommandResponse{CommandID: "cmd-003", Status: "failed", Reason: "vehicle_moving"})
cs, _ = store.Get("cmd-003")
ASSERT cs.Status == "failed"
ASSERT cs.Reason == "vehicle_moving"
```

### TS-06-8: NATS Response Subscription

**Requirement:** 06-REQ-3.1
**Type:** integration
**Description:** The service subscribes to `vehicles.*.command_responses` at startup.

**Preconditions:**
- NATS server running.

**Input:**
- Start service, publish a response to `vehicles.VIN12345.command_responses`.

**Expected:**
- The service receives and processes the response.

**Assertion pseudocode:**
```
startService()
nats.Publish("vehicles.VIN12345.command_responses", responseJSON)
// Verify command store updated
```

### TS-06-9: Response Updates Command Store

**Requirement:** 06-REQ-3.2
**Type:** integration
**Description:** A NATS response updates the corresponding command in the store.

**Preconditions:**
- Command "cmd-004" stored as "pending". NATS connected.

**Input:**
- NATS message on `vehicles.VIN12345.command_responses`: `{"command_id":"cmd-004","status":"success"}`

**Expected:**
- Store.Get("cmd-004") returns status "success".

**Assertion pseudocode:**
```
store.Add(cmd004_pending)
nats.Publish("vehicles.VIN12345.command_responses", successJSON)
wait()
cs, _ = store.Get("cmd-004")
ASSERT cs.Status == "success"
```

### TS-06-10: Response Payload Parsing

**Requirement:** 06-REQ-3.3
**Type:** unit
**Description:** NATS response payloads are parsed as JSON with command_id, status, and optional reason.

**Preconditions:**
- None.

**Input:**
- `{"command_id":"cmd-005","status":"success"}`
- `{"command_id":"cmd-006","status":"failed","reason":"door_open"}`

**Expected:**
- Both parse correctly. Reason is empty string when omitted.

**Assertion pseudocode:**
```
resp1 = parseResponse(successJSON)
ASSERT resp1.CommandID == "cmd-005"
ASSERT resp1.Status == "success"
ASSERT resp1.Reason == ""

resp2 = parseResponse(failedJSON)
ASSERT resp2.Reason == "door_open"
```

### TS-06-11: Command Timeout

**Requirement:** 06-REQ-4.1
**Type:** unit
**Description:** Commands pending beyond the timeout duration are marked as "timeout".

**Preconditions:**
- Command "cmd-007" added at time T. Timeout is 1 second (for testing).

**Input:**
- Wait 1.5 seconds, then call ExpireTimedOut.

**Expected:**
- Command "cmd-007" status is "timeout".

**Assertion pseudocode:**
```
store.Add(CommandStatus{CommandID: "cmd-007", Status: "pending", CreatedAt: time.Now().Add(-2*time.Second)})
store.ExpireTimedOut(1 * time.Second)
cs, _ = store.Get("cmd-007")
ASSERT cs.Status == "timeout"
```

### TS-06-12: Configurable Timeout

**Requirement:** 06-REQ-4.2
**Type:** unit
**Description:** The timeout duration is loaded from the config file.

**Preconditions:**
- Config file with `command_timeout_seconds: 60`.

**Input:**
- Load config.

**Expected:**
- Config.CommandTimeout == 60.

**Assertion pseudocode:**
```
cfg = LoadConfig(pathWithTimeout60)
ASSERT cfg.CommandTimeout == 60
```

### TS-06-13: Telemetry Subscription

**Requirement:** 06-REQ-5.1
**Type:** integration
**Description:** The service subscribes to `vehicles.*.telemetry` at startup.

**Preconditions:**
- NATS server running.

**Input:**
- Publish telemetry to `vehicles.VIN12345.telemetry`.

**Expected:**
- Service logs the telemetry (verify via log capture or test hook).

**Assertion pseudocode:**
```
startService()
nats.Publish("vehicles.VIN12345.telemetry", telemetryJSON)
ASSERT logContains("VIN12345")
```

### TS-06-14: Telemetry Logging

**Requirement:** 06-REQ-5.2
**Type:** integration
**Description:** Received telemetry is logged with the VIN extracted from the NATS subject.

**Preconditions:**
- Service running with NATS connected.

**Input:**
- Publish telemetry to `vehicles.VIN12345.telemetry`.

**Expected:**
- Log output includes "VIN12345" and telemetry content.

**Assertion pseudocode:**
```
nats.Publish("vehicles.VIN12345.telemetry", '{"speed": 0}')
ASSERT logContains("VIN12345")
```

### TS-06-15: Token Validation on All Endpoints

**Requirement:** 06-REQ-6.1
**Type:** integration
**Description:** Bearer token is validated on POST and GET command endpoints.

**Preconditions:**
- Service running with token config.

**Input:**
- POST and GET with valid token → success.
- POST and GET with no token → 401.

**Expected:**
- Valid token: 202/200 respectively.
- No token: 401 on both.

**Assertion pseudocode:**
```
resp1 = POST("/vehicles/VIN12345/commands", body, validAuth)
ASSERT resp1.StatusCode == 202

resp2 = POST("/vehicles/VIN12345/commands", body, noAuth)
ASSERT resp2.StatusCode == 401

resp3 = GET("/vehicles/VIN12345/commands/cmd-001", noAuth)
ASSERT resp3.StatusCode == 401
```

### TS-06-16: Token-VIN Loading from Config

**Requirement:** 06-REQ-6.2
**Type:** unit
**Description:** Token-to-VIN mappings are loaded from the JSON config file.

**Preconditions:**
- Config file with one token mapping.

**Input:**
- LoadConfig with token mapping `{token: "abc", vin: "VIN1"}`.

**Expected:**
- Config.Tokens has one entry with matching values.

**Assertion pseudocode:**
```
cfg = LoadConfig(pathWithToken)
ASSERT len(cfg.Tokens) == 1
ASSERT cfg.Tokens[0].Token == "abc"
ASSERT cfg.Tokens[0].VIN == "VIN1"
```

### TS-06-17: Token-VIN Authorization

**Requirement:** 06-REQ-6.3
**Type:** unit
**Description:** A token is verified against the VIN in the URL path.

**Preconditions:**
- Authenticator created with `{token: "t1", vin: "V1"}`.

**Input:**
- AuthorizeVIN("t1", "V1") → true
- AuthorizeVIN("t1", "V2") → false

**Expected:**
- Only the matching VIN returns true.

**Assertion pseudocode:**
```
auth = NewAuthenticator([{Token: "t1", VIN: "V1"}])
ASSERT auth.AuthorizeVIN("t1", "V1") == true
ASSERT auth.AuthorizeVIN("t1", "V2") == false
```

### TS-06-18: Config File Loading

**Requirement:** 06-REQ-7.1
**Type:** unit
**Description:** Configuration is loaded from the path specified by CONFIG_PATH env var.

**Preconditions:**
- A temporary config file.

**Input:**
- LoadConfig(path).

**Expected:**
- Config values match file contents.

**Assertion pseudocode:**
```
cfg = LoadConfig("/tmp/test-gw-config.json")
ASSERT cfg.Port == 9090
```

### TS-06-19: Config Fields

**Requirement:** 06-REQ-7.2
**Type:** unit
**Description:** Config includes port, NATS URL, command timeout, and token mappings.

**Preconditions:**
- A full config file.

**Input:**
- LoadConfig with all fields set.

**Expected:**
- All fields populated.

**Assertion pseudocode:**
```
cfg = LoadConfig(fullConfigPath)
ASSERT cfg.Port > 0
ASSERT cfg.NatsURL != ""
ASSERT cfg.CommandTimeout > 0
ASSERT len(cfg.Tokens) > 0
```

### TS-06-20: Config Defaults

**Requirement:** 06-REQ-7.3
**Type:** unit
**Description:** Missing config fields use defaults: port 8081, NATS nats://localhost:4222, timeout 30.

**Preconditions:**
- Config file with empty JSON `{}`.

**Input:**
- LoadConfig with empty object.

**Expected:**
- Defaults applied.

**Assertion pseudocode:**
```
cfg = LoadConfig(emptyConfigPath)
ASSERT cfg.Port == 8081
ASSERT cfg.NatsURL == "nats://localhost:4222"
ASSERT cfg.CommandTimeout == 30
```

### TS-06-21: NATS Connection

**Requirement:** 06-REQ-8.1
**Type:** integration
**Description:** The service connects to the configured NATS server URL at startup.

**Preconditions:**
- NATS server running on localhost:4222.

**Input:**
- Start service with NATS URL nats://localhost:4222.

**Expected:**
- Service connects successfully.

**Assertion pseudocode:**
```
nc, err = Connect("nats://localhost:4222", 5)
ASSERT err == nil
ASSERT nc.IsConnected()
```

### TS-06-22: NATS Subscriptions Active

**Requirement:** 06-REQ-8.2
**Type:** integration
**Description:** After connecting, the service subscribes to command_responses and telemetry.

**Preconditions:**
- NATS connected.

**Input:**
- Start service, check subscriptions are active.

**Expected:**
- Subscriptions to `vehicles.*.command_responses` and `vehicles.*.telemetry` are active.

**Assertion pseudocode:**
```
// Verified by publishing to both subjects and confirming messages are received
```

### TS-06-23: Health Check

**Requirement:** 06-REQ-9.1
**Type:** integration
**Description:** GET /health returns HTTP 200 with `{"status":"ok"}`.

**Preconditions:**
- Service running.

**Input:**
- `GET /health`

**Expected:**
- HTTP 200, body `{"status":"ok"}`

**Assertion pseudocode:**
```
resp = httptest.GET("/health")
ASSERT resp.StatusCode == 200
body = json.Decode(resp.Body)
ASSERT body.status == "ok"
```

### TS-06-24: Startup Logging

**Requirement:** 06-REQ-9.2
**Type:** integration
**Description:** On startup, the service logs version, port, NATS URL, and token count.

**Preconditions:**
- Service starts with default config.

**Input:**
- Capture startup logs.

**Expected:**
- Logs contain port, NATS URL, token count.

**Assertion pseudocode:**
```
output = captureStartupLogs()
ASSERT "8081" IN output
ASSERT "nats://" IN output
ASSERT "tokens" IN output
```

### TS-06-25: Graceful Shutdown

**Requirement:** 06-REQ-9.3
**Type:** integration
**Description:** SIGTERM causes graceful shutdown with exit code 0.

**Preconditions:**
- Service running.

**Input:**
- Send SIGTERM.

**Expected:**
- Service exits with code 0.

**Assertion pseudocode:**
```
proc = startService()
proc.Signal(SIGTERM)
exitCode = proc.Wait()
ASSERT exitCode == 0
```

### TS-06-26: Content-Type Header

**Requirement:** 06-REQ-10.1
**Type:** integration
**Description:** All responses set Content-Type: application/json.

**Preconditions:**
- Service running.

**Input:**
- GET /health, POST /vehicles/{vin}/commands, GET /vehicles/{vin}/commands/{id}

**Expected:**
- All have Content-Type: application/json.

**Assertion pseudocode:**
```
FOR endpoint IN [healthReq, commandPostReq, statusGetReq]:
    resp = httptest.Do(endpoint)
    ASSERT resp.Header("Content-Type") == "application/json"
```

### TS-06-27: Error Response Format

**Requirement:** 06-REQ-10.2
**Type:** integration
**Description:** Error responses use format `{"error":"<message>"}`.

**Preconditions:**
- Service running.

**Input:**
- Request without auth → 401.

**Expected:**
- Body: `{"error":"unauthorized"}`.

**Assertion pseudocode:**
```
resp = POST("/vehicles/VIN12345/commands", body, noAuth)
body = json.Decode(resp.Body)
ASSERT body.error == "unauthorized"
```

## Edge Case Tests

### TS-06-E1: Missing Authorization Header

**Requirement:** 06-REQ-1.E1
**Type:** integration
**Description:** Missing or malformed Authorization header returns HTTP 401.

**Preconditions:**
- Service running.

**Input:**
- POST with no Authorization header.
- POST with `Authorization: Basic abc`.

**Expected:**
- Both return HTTP 401 with `{"error":"unauthorized"}`.

**Assertion pseudocode:**
```
resp1 = POST("/vehicles/VIN12345/commands", body, noAuth)
ASSERT resp1.StatusCode == 401

resp2 = POST("/vehicles/VIN12345/commands", body, basicAuth)
ASSERT resp2.StatusCode == 401
```

### TS-06-E2: Token Not Authorized for VIN

**Requirement:** 06-REQ-1.E2
**Type:** integration
**Description:** A valid token used with a VIN it's not authorized for returns HTTP 403.

**Preconditions:**
- Token "demo-token-car1" mapped to VIN "VIN12345".

**Input:**
- POST to `/vehicles/VIN99999/commands` with token `demo-token-car1`.

**Expected:**
- HTTP 403 with `{"error":"forbidden"}`.

**Assertion pseudocode:**
```
resp = POST("/vehicles/VIN99999/commands", body, "Bearer demo-token-car1")
ASSERT resp.StatusCode == 403
```

### TS-06-E3: Invalid Command Payload

**Requirement:** 06-REQ-1.E3
**Type:** integration
**Description:** Invalid JSON or missing required fields returns HTTP 400.

**Preconditions:**
- Service running with valid auth.

**Input:**
- Body: `{invalid json}`
- Body: `{"command_id":"c1"}` (missing type and doors)

**Expected:**
- HTTP 400 with `{"error":"invalid command payload"}`.

**Assertion pseudocode:**
```
resp1 = POST(endpoint, "{invalid}", validAuth)
ASSERT resp1.StatusCode == 400

resp2 = POST(endpoint, '{"command_id":"c1"}', validAuth)
ASSERT resp2.StatusCode == 400
```

### TS-06-E4: Invalid Command Type

**Requirement:** 06-REQ-1.E4
**Type:** integration
**Description:** A command type other than "lock" or "unlock" returns HTTP 400.

**Preconditions:**
- Service running with valid auth.

**Input:**
- Body: `{"command_id":"c1","type":"open","doors":["driver"]}`

**Expected:**
- HTTP 400 with `{"error":"invalid command payload"}`.

**Assertion pseudocode:**
```
resp = POST(endpoint, invalidTypeBody, validAuth)
ASSERT resp.StatusCode == 400
```

### TS-06-E5: Unknown Command ID

**Requirement:** 06-REQ-2.E1
**Type:** integration
**Description:** Querying a nonexistent command ID returns HTTP 404.

**Preconditions:**
- Service running.

**Input:**
- GET `/vehicles/VIN12345/commands/nonexistent-id` with valid auth.

**Expected:**
- HTTP 404 with `{"error":"command not found"}`.

**Assertion pseudocode:**
```
resp = GET("/vehicles/VIN12345/commands/nonexistent-id", validAuth)
ASSERT resp.StatusCode == 404
```

### TS-06-E6: Auth on Status Query

**Requirement:** 06-REQ-2.E2
**Type:** integration
**Description:** Status query without valid auth returns 401 or 403.

**Preconditions:**
- Service running.

**Input:**
- GET status with no auth → 401.
- GET status with wrong VIN token → 403.

**Expected:**
- 401 and 403 respectively.

**Assertion pseudocode:**
```
resp1 = GET(statusEndpoint, noAuth)
ASSERT resp1.StatusCode == 401

resp2 = GET("/vehicles/VIN99999/commands/cmd-001", "Bearer demo-token-car1")
ASSERT resp2.StatusCode == 403
```

### TS-06-E7: Invalid NATS Response JSON

**Requirement:** 06-REQ-3.E1
**Type:** integration
**Description:** Invalid JSON in a NATS response is logged and discarded.

**Preconditions:**
- Service connected to NATS.

**Input:**
- Publish invalid JSON to `vehicles.VIN12345.command_responses`.

**Expected:**
- Service logs error, does not crash.

**Assertion pseudocode:**
```
nats.Publish("vehicles.VIN12345.command_responses", "{invalid json")
wait()
ASSERT service.IsRunning()
ASSERT logContains("error")
```

### TS-06-E8: Unknown Command ID in NATS Response

**Requirement:** 06-REQ-3.E2
**Type:** integration
**Description:** A NATS response with an unknown command_id is logged and discarded.

**Preconditions:**
- Service connected to NATS, store empty.

**Input:**
- Publish response with unknown command_id to NATS.

**Expected:**
- Service logs warning, does not crash.

**Assertion pseudocode:**
```
nats.Publish("vehicles.VIN12345.command_responses", '{"command_id":"unknown","status":"success"}')
wait()
ASSERT service.IsRunning()
```

### TS-06-E9: Invalid Telemetry JSON

**Requirement:** 06-REQ-5.E1
**Type:** integration
**Description:** Invalid JSON in telemetry is logged and discarded.

**Preconditions:**
- Service connected to NATS.

**Input:**
- Publish invalid JSON to `vehicles.VIN12345.telemetry`.

**Expected:**
- Service logs warning, does not crash.

**Assertion pseudocode:**
```
nats.Publish("vehicles.VIN12345.telemetry", "not json")
wait()
ASSERT service.IsRunning()
```

### TS-06-E10: Config File Missing

**Requirement:** 06-REQ-7.E1
**Type:** unit
**Description:** Missing config file returns default config with warning.

**Preconditions:**
- No config file at path.

**Input:**
- LoadConfig("/nonexistent/config.json").

**Expected:**
- Returns default config. No error.

**Assertion pseudocode:**
```
cfg, err = LoadConfig("/nonexistent/config.json")
ASSERT err == nil
ASSERT cfg.Port == 8081
ASSERT cfg.NatsURL == "nats://localhost:4222"
```

### TS-06-E11: Config File Invalid JSON

**Requirement:** 06-REQ-7.E2
**Type:** unit
**Description:** Invalid JSON config file returns an error.

**Preconditions:**
- A file containing `{invalid`.

**Input:**
- LoadConfig(invalidPath).

**Expected:**
- Returns non-nil error.

**Assertion pseudocode:**
```
_, err = LoadConfig(invalidJSONPath)
ASSERT err != nil
```

### TS-06-E12: NATS Unreachable at Startup

**Requirement:** 06-REQ-8.E1
**Type:** integration
**Description:** If NATS is unreachable, the service retries then exits non-zero.

**Preconditions:**
- No NATS server running on the configured URL.

**Input:**
- Attempt Connect("nats://localhost:19999", 3).

**Expected:**
- Returns error after retries.

**Assertion pseudocode:**
```
_, err = Connect("nats://localhost:19999", 3)
ASSERT err != nil
```

## Property Test Cases

### TS-06-P1: Command Routing Fidelity

**Property:** Property 1 from design.md
**Validates:** 06-REQ-1.1, 06-REQ-1.3, 06-REQ-1.4
**Type:** property
**Description:** For any valid command and authorized token, the command is published to the correct NATS subject with the token in headers, and stored as pending.

**For any:** Random command_id (UUID), type in ["lock","unlock"], random VIN, valid token.
**Invariant:** NATS subject equals `vehicles.{vin}.commands`, header contains token, store has command as pending.

**Assertion pseudocode:**
```
FOR ANY vin IN random_vins, cmd IN random_commands, token IN valid_tokens:
    subject = "vehicles." + vin + ".commands"
    // Verify subject construction
    ASSERT constructSubject(vin) == subject
    store.Add(CommandStatus{CommandID: cmd.CommandID, Status: "pending"})
    cs, _ = store.Get(cmd.CommandID)
    ASSERT cs.Status == "pending"
```

### TS-06-P2: Authentication Enforcement

**Property:** Property 2 from design.md
**Validates:** 06-REQ-1.E1, 06-REQ-1.E2, 06-REQ-6.1, 06-REQ-6.3
**Type:** property
**Description:** For any request, missing/malformed tokens yield 401 and mismatched VINs yield 403.

**For any:** Random header strings (empty, missing "Bearer ", valid format), random token-VIN pairs.
**Invariant:** ValidateToken returns error for malformed headers; AuthorizeVIN returns false for non-matching VINs.

**Assertion pseudocode:**
```
FOR ANY header IN random_strings:
    token, err = ValidateToken(header)
    IF NOT header.startsWith("Bearer "):
        ASSERT err != nil
    ELSE:
        ASSERT token == header[7:]

FOR ANY token IN known_tokens, vin IN random_vins:
    result = auth.AuthorizeVIN(token, vin)
    IF vin == token_to_vin[token]:
        ASSERT result == true
    ELSE:
        ASSERT result == false
```

### TS-06-P3: Response Status Update

**Property:** Property 3 from design.md
**Validates:** 06-REQ-3.2, 06-REQ-2.1, 06-REQ-2.3
**Type:** property
**Description:** For any response with a known command_id, the store is updated to match and subsequent queries return the updated status.

**For any:** Random command_id, status in ["success","failed"], random reason.
**Invariant:** After UpdateFromResponse, Get returns the updated status.

**Assertion pseudocode:**
```
FOR ANY cmdID IN random_ids, status IN ["success","failed"], reason IN random_strings:
    store.Add(CommandStatus{CommandID: cmdID, Status: "pending"})
    store.UpdateFromResponse(CommandResponse{CommandID: cmdID, Status: status, Reason: reason})
    cs, _ = store.Get(cmdID)
    ASSERT cs.Status == status
    ASSERT cs.Reason == reason
```

### TS-06-P4: Command Timeout

**Property:** Property 4 from design.md
**Validates:** 06-REQ-4.1, 06-REQ-4.2
**Type:** property
**Description:** For any command pending longer than the timeout, the status becomes "timeout".

**For any:** Random timeout durations, random command creation times.
**Invariant:** If now - createdAt > timeout, status is "timeout" after ExpireTimedOut.

**Assertion pseudocode:**
```
FOR ANY timeout IN random_durations, age IN random_ages:
    store.Add(CommandStatus{CommandID: "c", Status: "pending", CreatedAt: now.Add(-age)})
    store.ExpireTimedOut(timeout)
    cs, _ = store.Get("c")
    IF age > timeout:
        ASSERT cs.Status == "timeout"
    ELSE:
        ASSERT cs.Status == "pending"
```

### TS-06-P5: Payload Validation

**Property:** Property 5 from design.md
**Validates:** 06-REQ-1.E3, 06-REQ-1.E4
**Type:** property
**Description:** For any invalid payload, parsing returns an error.

**For any:** Random JSON objects missing one or more required fields, random invalid type values.
**Invariant:** parseCommand returns error for all invalid payloads.

**Assertion pseudocode:**
```
FOR ANY payload IN random_invalid_payloads:
    _, err = parseCommand(payload)
    ASSERT err != nil
```

### TS-06-P6: Config Defaults

**Property:** Property 6 from design.md
**Validates:** 06-REQ-7.1, 06-REQ-7.3, 06-REQ-7.E1
**Type:** property
**Description:** For any missing config file, defaults are applied.

**For any:** Random nonexistent file paths.
**Invariant:** LoadConfig returns port=8081, NatsURL=nats://localhost:4222, timeout=30.

**Assertion pseudocode:**
```
FOR ANY path IN random_nonexistent_paths:
    cfg, err = LoadConfig(path)
    ASSERT err == nil
    ASSERT cfg.Port == 8081
    ASSERT cfg.NatsURL == "nats://localhost:4222"
    ASSERT cfg.CommandTimeout == 30
```

### TS-06-P7: NATS Subject Correctness

**Property:** Property 7 from design.md
**Validates:** 06-REQ-1.1, 06-REQ-3.1, 06-REQ-5.1
**Type:** property
**Description:** For any VIN, the command subject is exactly `vehicles.{vin}.commands`.

**For any:** Random VIN strings.
**Invariant:** Subject construction produces the correct pattern.

**Assertion pseudocode:**
```
FOR ANY vin IN random_vin_strings:
    ASSERT commandSubject(vin) == "vehicles." + vin + ".commands"
    ASSERT responseSubject() == "vehicles.*.command_responses"
    ASSERT telemetrySubject() == "vehicles.*.telemetry"
```

## Coverage Matrix

| Requirement | Test Spec Entry | Type |
|-------------|-----------------|------|
| 06-REQ-1.1 | TS-06-1 | integration |
| 06-REQ-1.2 | TS-06-2 | unit |
| 06-REQ-1.3 | TS-06-3 | integration |
| 06-REQ-1.4 | TS-06-4 | unit |
| 06-REQ-1.E1 | TS-06-E1 | integration |
| 06-REQ-1.E2 | TS-06-E2 | integration |
| 06-REQ-1.E3 | TS-06-E3 | integration |
| 06-REQ-1.E4 | TS-06-E4 | integration |
| 06-REQ-2.1 | TS-06-5 | integration |
| 06-REQ-2.2 | TS-06-6 | unit |
| 06-REQ-2.3 | TS-06-7 | unit |
| 06-REQ-2.E1 | TS-06-E5 | integration |
| 06-REQ-2.E2 | TS-06-E6 | integration |
| 06-REQ-3.1 | TS-06-8 | integration |
| 06-REQ-3.2 | TS-06-9 | integration |
| 06-REQ-3.3 | TS-06-10 | unit |
| 06-REQ-3.E1 | TS-06-E7 | integration |
| 06-REQ-3.E2 | TS-06-E8 | integration |
| 06-REQ-4.1 | TS-06-11 | unit |
| 06-REQ-4.2 | TS-06-12 | unit |
| 06-REQ-5.1 | TS-06-13 | integration |
| 06-REQ-5.2 | TS-06-14 | integration |
| 06-REQ-5.E1 | TS-06-E9 | integration |
| 06-REQ-6.1 | TS-06-15 | integration |
| 06-REQ-6.2 | TS-06-16 | unit |
| 06-REQ-6.3 | TS-06-17 | unit |
| 06-REQ-7.1 | TS-06-18 | unit |
| 06-REQ-7.2 | TS-06-19 | unit |
| 06-REQ-7.3 | TS-06-20 | unit |
| 06-REQ-7.E1 | TS-06-E10 | unit |
| 06-REQ-7.E2 | TS-06-E11 | unit |
| 06-REQ-8.1 | TS-06-21 | integration |
| 06-REQ-8.2 | TS-06-22 | integration |
| 06-REQ-8.E1 | TS-06-E12 | integration |
| 06-REQ-9.1 | TS-06-23 | integration |
| 06-REQ-9.2 | TS-06-24 | integration |
| 06-REQ-9.3 | TS-06-25 | integration |
| 06-REQ-10.1 | TS-06-26 | integration |
| 06-REQ-10.2 | TS-06-27 | integration |
| Property 1 | TS-06-P1 | property |
| Property 2 | TS-06-P2 | property |
| Property 3 | TS-06-P3 | property |
| Property 4 | TS-06-P4 | property |
| Property 5 | TS-06-P5 | property |
| Property 6 | TS-06-P6 | property |
| Property 7 | TS-06-P7 | property |
