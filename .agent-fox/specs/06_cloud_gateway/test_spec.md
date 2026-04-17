# Test Specification: CLOUD_GATEWAY

## Overview

This test specification defines concrete test contracts for the CLOUD_GATEWAY, a Go service with REST and NATS interfaces for vehicle command routing. Tests are organized into unit tests (config, auth, store, handler packages), integration tests (httptest-based end-to-end and NATS-based), and integration smoke tests (full-stack REST-to-NATS flow). Unit and handler tests run via `cd backend && go test -v ./cloud-gateway/...`. Integration tests requiring NATS run via `cd backend && go test -v ./cloud-gateway/... -tags=integration`.

## Test Cases

### TS-06-1: Command Submission Success

**Requirement:** 06-REQ-1.1
**Type:** integration
**Description:** A POST to `/vehicles/{vin}/commands` with a valid token and body publishes to NATS and returns HTTP 202.

**Preconditions:**
- Service is running with config containing token "demo-token-001" mapped to VIN "VIN12345".
- NATS server is available.

**Input:**
- `POST /vehicles/VIN12345/commands`
- Header: `Authorization: Bearer demo-token-001`
- Body: `{"command_id":"cmd-001","type":"lock","doors":["driver"]}`

**Expected:**
- HTTP 202
- Response body: `{"command_id":"cmd-001","type":"lock","doors":["driver"]}`
- Command published to NATS subject `vehicles.VIN12345.commands`

**Assertion pseudocode:**
```
resp = httptest.POST("/vehicles/VIN12345/commands",
    header("Authorization", "Bearer demo-token-001"),
    body({"command_id":"cmd-001","type":"lock","doors":["driver"]}))
ASSERT resp.StatusCode == 202
body = json.Decode(resp.Body)
ASSERT body.command_id == "cmd-001"
ASSERT body.type == "lock"
ASSERT body.doors == ["driver"]
// Verify NATS message received on vehicles.VIN12345.commands
msg = nats.Subscribe("vehicles.VIN12345.commands").NextMsg(1s)
ASSERT msg != nil
ASSERT json.Decode(msg.Data).command_id == "cmd-001"
```

### TS-06-2: NATS Authorization Header

**Requirement:** 06-REQ-1.2
**Type:** integration
**Description:** Commands published to NATS include the bearer token as a NATS message header.

**Preconditions:**
- Service is running with valid config. NATS server is available.

**Input:**
- `POST /vehicles/VIN12345/commands` with bearer token "demo-token-001"

**Expected:**
- NATS message header contains `Authorization: Bearer demo-token-001`

**Assertion pseudocode:**
```
// Submit command via REST
httptest.POST("/vehicles/VIN12345/commands",
    header("Authorization", "Bearer demo-token-001"),
    body({"command_id":"cmd-002","type":"unlock","doors":["driver"]}))
// Check NATS message headers
msg = nats.Subscribe("vehicles.VIN12345.commands").NextMsg(1s)
ASSERT msg.Header.Get("Authorization") == "Bearer demo-token-001"
```

### TS-06-3: Command Timeout

**Requirement:** 06-REQ-1.3
**Type:** unit
**Description:** When no response is received within the timeout, the command status becomes "timeout".

**Preconditions:**
- Store is initialized. Timeout configured to 100ms (for fast test).

**Input:**
- `store.StartTimeout("cmd-003", 100ms)`
- Wait 200ms

**Expected:**
- `store.GetResponse("cmd-003")` returns `{command_id:"cmd-003", status:"timeout"}`

**Assertion pseudocode:**
```
store = NewStore()
store.StartTimeout("cmd-003", 100*time.Millisecond)
time.Sleep(200 * time.Millisecond)
resp, found = store.GetResponse("cmd-003")
ASSERT found == true
ASSERT resp.CommandID == "cmd-003"
ASSERT resp.Status == "timeout"
```

### TS-06-4: Command Status Query Success

**Requirement:** 06-REQ-2.1
**Type:** integration
**Description:** GET `/vehicles/{vin}/commands/{command_id}` returns the stored command response.

**Preconditions:**
- Service is running. A command response has been stored for "cmd-004".

**Input:**
- Store response: `{command_id:"cmd-004", status:"success"}`
- `GET /vehicles/VIN12345/commands/cmd-004` with valid bearer token

**Expected:**
- HTTP 200
- Body: `{"command_id":"cmd-004","status":"success"}`

**Assertion pseudocode:**
```
store.StoreResponse({CommandID:"cmd-004", Status:"success"})
resp = httptest.GET("/vehicles/VIN12345/commands/cmd-004",
    header("Authorization", "Bearer demo-token-001"))
ASSERT resp.StatusCode == 200
body = json.Decode(resp.Body)
ASSERT body.command_id == "cmd-004"
ASSERT body.status == "success"
```

### TS-06-5: Response Store Thread Safety

**Requirement:** 06-REQ-2.2
**Type:** unit
**Description:** Concurrent writes and reads to the response store do not cause data races.

**Preconditions:**
- Store is initialized.

**Input:**
- 100 concurrent goroutines each writing a unique command response.
- 100 concurrent goroutines each reading a random command response.

**Expected:**
- No data race detected (run with `-race` flag).
- All written responses are retrievable after goroutines complete.

**Assertion pseudocode:**
```
store = NewStore()
wg = sync.WaitGroup{}
FOR i IN 0..99:
    go func(i):
        store.StoreResponse({CommandID: fmt.Sprintf("cmd-%d", i), Status: "success"})
        wg.Done()
    go func(i):
        store.GetResponse(fmt.Sprintf("cmd-%d", i))
        wg.Done()
wg.Wait()
FOR i IN 0..99:
    resp, found = store.GetResponse(fmt.Sprintf("cmd-%d", i))
    ASSERT found == true
    ASSERT resp.Status == "success"
```

### TS-06-6: NATS Response Subscription

**Requirement:** 06-REQ-5.1, 06-REQ-5.2
**Type:** integration
**Description:** The service subscribes to command responses on NATS and stores them.

**Preconditions:**
- Service is running and connected to NATS.

**Input:**
- Publish to NATS subject `vehicles.VIN12345.command_responses`:
  `{"command_id":"cmd-005","status":"success"}`

**Expected:**
- `store.GetResponse("cmd-005")` returns `{command_id:"cmd-005", status:"success"}`

**Assertion pseudocode:**
```
nats.Publish("vehicles.VIN12345.command_responses",
    {"command_id":"cmd-005","status":"success"})
time.Sleep(100 * time.Millisecond)  // allow subscription processing
resp, found = store.GetResponse("cmd-005")
ASSERT found == true
ASSERT resp.Status == "success"
```

### TS-06-7: Telemetry Subscription Logging

**Requirement:** 06-REQ-5.3
**Type:** integration
**Description:** The service subscribes to telemetry on NATS and logs it without storing.

**Preconditions:**
- Service is running and connected to NATS. Log output captured.

**Input:**
- Publish to NATS subject `vehicles.VIN12345.telemetry`:
  `{"speed": 60, "location": {"lat": 48.137, "lon": 11.575}}`

**Expected:**
- Log output contains the telemetry data or VIN reference.
- No telemetry data in the response store.

**Assertion pseudocode:**
```
logBuf = captureLog()
nats.Publish("vehicles.VIN12345.telemetry",
    {"speed": 60, "location": {"lat": 48.137, "lon": 11.575}})
time.Sleep(100 * time.Millisecond)
ASSERT "VIN12345" IN logBuf.String()
ASSERT "telemetry" IN logBuf.String()
```

### TS-06-8: Bearer Token Validation

**Requirement:** 06-REQ-3.1
**Type:** unit
**Description:** The auth middleware extracts and validates bearer tokens.

**Preconditions:**
- Config contains token "demo-token-001" mapped to VIN "VIN12345".

**Input:**
- Request with `Authorization: Bearer demo-token-001` to `/vehicles/VIN12345/commands`

**Expected:**
- Request passes through middleware to handler.

**Assertion pseudocode:**
```
handler = auth.Middleware(cfg)(testHandler)
req = httptest.NewRequest("GET", "/vehicles/VIN12345/commands/x", nil)
req.Header.Set("Authorization", "Bearer demo-token-001")
rec = httptest.NewRecorder()
handler.ServeHTTP(rec, req)
ASSERT rec.Code != 401
ASSERT rec.Code != 403
```

### TS-06-9: VIN Authorization Check

**Requirement:** 06-REQ-3.2
**Type:** unit
**Description:** A valid token used against a different VIN returns HTTP 403.

**Preconditions:**
- Config contains token "demo-token-001" mapped to VIN "VIN12345".

**Input:**
- `POST /vehicles/VIN99999/commands` with `Authorization: Bearer demo-token-001`

**Expected:**
- HTTP 403
- Body: `{"error":"forbidden"}`

**Assertion pseudocode:**
```
handler = auth.Middleware(cfg)(testHandler)
req = httptest.NewRequest("POST", "/vehicles/VIN99999/commands", nil)
req.Header.Set("Authorization", "Bearer demo-token-001")
rec = httptest.NewRecorder()
handler.ServeHTTP(rec, req)
ASSERT rec.Code == 403
body = json.Decode(rec.Body)
ASSERT body.error == "forbidden"
```

### TS-06-10: Health Check

**Requirement:** 06-REQ-4.1
**Type:** integration
**Description:** GET `/health` returns HTTP 200 with `{"status":"ok"}`.

**Preconditions:**
- Service is running.

**Input:**
- `GET /health`

**Expected:**
- HTTP 200
- Body: `{"status":"ok"}`

**Assertion pseudocode:**
```
resp = httptest.GET("/health")
ASSERT resp.StatusCode == 200
body = json.Decode(resp.Body)
ASSERT body.status == "ok"
```

### TS-06-11: Config Loading

**Requirement:** 06-REQ-6.1, 06-REQ-6.2
**Type:** unit
**Description:** LoadConfig reads configuration from the specified file path with all required fields.

**Preconditions:**
- A temporary JSON config file with port, nats_url, command_timeout_seconds, and tokens.

**Input:**
- Path to the temporary config file.

**Expected:**
- Config struct populated with values from the file.

**Assertion pseudocode:**
```
cfg, err = config.LoadConfig("/tmp/test-config.json")
ASSERT err == nil
ASSERT cfg.Port == 8081
ASSERT cfg.NatsURL == "nats://localhost:4222"
ASSERT cfg.CommandTimeoutSeconds == 30
ASSERT len(cfg.Tokens) >= 1
ASSERT cfg.Tokens[0].Token != ""
ASSERT cfg.Tokens[0].VIN != ""
```

### TS-06-12: Config Token-VIN Lookup

**Requirement:** 06-REQ-6.2
**Type:** unit
**Description:** GetVINForToken returns the correct VIN for a configured token.

**Preconditions:**
- Config loaded with token "demo-token-001" mapped to VIN "VIN12345".

**Input:**
- `cfg.GetVINForToken("demo-token-001")`
- `cfg.GetVINForToken("unknown-token")`

**Expected:**
- First returns ("VIN12345", true)
- Second returns ("", false)

**Assertion pseudocode:**
```
vin, ok = cfg.GetVINForToken("demo-token-001")
ASSERT ok == true
ASSERT vin == "VIN12345"
vin, ok = cfg.GetVINForToken("unknown-token")
ASSERT ok == false
ASSERT vin == ""
```

### TS-06-13: Content-Type Header

**Requirement:** 06-REQ-7.1
**Type:** integration
**Description:** All REST responses set Content-Type: application/json.

**Preconditions:**
- Service is running.

**Input:**
- `GET /health`
- `POST /vehicles/VIN12345/commands` (with valid auth)
- `GET /vehicles/VIN12345/commands/nonexistent` (with valid auth)

**Expected:**
- All responses have `Content-Type: application/json` header.

**Assertion pseudocode:**
```
FOR endpoint IN [
    GET("/health"),
    POST("/vehicles/VIN12345/commands", valid_auth, valid_body),
    GET("/vehicles/VIN12345/commands/nonexistent", valid_auth)
]:
    ASSERT resp.Header("Content-Type") == "application/json"
```

### TS-06-14: Graceful Shutdown

**Requirement:** 06-REQ-8.2
**Type:** integration
**Description:** On SIGTERM, the service drains NATS and exits with code 0.

**Preconditions:**
- Service is running as a subprocess.

**Input:**
- Send SIGTERM to the service process.

**Expected:**
- Service exits with code 0.

**Assertion pseudocode:**
```
proc = startService()
proc.Signal(SIGTERM)
exitCode = proc.Wait()
ASSERT exitCode == 0
```

### TS-06-15: Startup Logging

**Requirement:** 06-REQ-8.1
**Type:** integration
**Description:** On startup, the service logs port, NATS URL, token count.

**Preconditions:**
- Service starts with test config.

**Input:**
- Capture stdout/stderr during startup.

**Expected:**
- Log output contains port, NATS URL, and token count.

**Assertion pseudocode:**
```
output = captureStartupLogs()
ASSERT "8081" IN output
ASSERT "nats://" IN output
ASSERT "token" IN output
```

## Edge Case Tests

### TS-06-E1: Invalid Command Payload

**Requirement:** 06-REQ-1.E1
**Type:** unit
**Description:** Missing required fields in command body return HTTP 400.

**Preconditions:**
- Handler is wired up with auth bypassed for test.

**Input:**
- POST with body `{}` (missing all fields)
- POST with body `{"command_id":"x"}` (missing type and doors)
- POST with body `{"command_id":"x","type":"lock"}` (missing doors)

**Expected:**
- HTTP 400 for all cases
- Body: `{"error":"invalid command payload"}`

**Assertion pseudocode:**
```
FOR body IN [{}, {"command_id":"x"}, {"command_id":"x","type":"lock"}]:
    resp = handler.SubmitCommand(body)
    ASSERT resp.StatusCode == 400
    ASSERT json.Decode(resp.Body).error == "invalid command payload"
```

### TS-06-E2: Invalid Command Type

**Requirement:** 06-REQ-1.E2
**Type:** unit
**Description:** Command type other than "lock" or "unlock" returns HTTP 400.

**Preconditions:**
- Handler is wired up.

**Input:**
- POST with body `{"command_id":"x","type":"start","doors":["driver"]}`

**Expected:**
- HTTP 400
- Body: `{"error":"invalid command type"}`

**Assertion pseudocode:**
```
resp = handler.SubmitCommand({"command_id":"x","type":"start","doors":["driver"]})
ASSERT resp.StatusCode == 400
ASSERT json.Decode(resp.Body).error == "invalid command type"
```

### TS-06-E3: Command Not Found

**Requirement:** 06-REQ-2.E1
**Type:** integration
**Description:** Querying a nonexistent command_id returns HTTP 404.

**Preconditions:**
- Service is running. No command "nonexistent" in store.

**Input:**
- `GET /vehicles/VIN12345/commands/nonexistent` with valid bearer token

**Expected:**
- HTTP 404
- Body: `{"error":"command not found"}`

**Assertion pseudocode:**
```
resp = httptest.GET("/vehicles/VIN12345/commands/nonexistent",
    header("Authorization", "Bearer demo-token-001"))
ASSERT resp.StatusCode == 404
body = json.Decode(resp.Body)
ASSERT body.error == "command not found"
```

### TS-06-E4: Missing Authorization Header

**Requirement:** 06-REQ-3.E1
**Type:** unit
**Description:** Requests without Authorization header return HTTP 401.

**Preconditions:**
- Auth middleware is configured.

**Input:**
- `POST /vehicles/VIN12345/commands` with no Authorization header
- `GET /vehicles/VIN12345/commands/x` with no Authorization header

**Expected:**
- HTTP 401
- Body: `{"error":"unauthorized"}`

**Assertion pseudocode:**
```
FOR method IN ["POST", "GET"]:
    req = httptest.NewRequest(method, "/vehicles/VIN12345/commands", nil)
    // No Authorization header
    rec = httptest.NewRecorder()
    handler.ServeHTTP(rec, req)
    ASSERT rec.Code == 401
    ASSERT json.Decode(rec.Body).error == "unauthorized"
```

### TS-06-E5: Invalid Token

**Requirement:** 06-REQ-3.E1
**Type:** unit
**Description:** Requests with an unrecognized token return HTTP 401.

**Preconditions:**
- Auth middleware configured with known tokens.

**Input:**
- `Authorization: Bearer invalid-token-999`

**Expected:**
- HTTP 401
- Body: `{"error":"unauthorized"}`

**Assertion pseudocode:**
```
req = httptest.NewRequest("POST", "/vehicles/VIN12345/commands", nil)
req.Header.Set("Authorization", "Bearer invalid-token-999")
rec = httptest.NewRecorder()
handler.ServeHTTP(rec, req)
ASSERT rec.Code == 401
ASSERT json.Decode(rec.Body).error == "unauthorized"
```

### TS-06-E6: NATS Connection Retry Exhaustion

**Requirement:** 06-REQ-5.E1
**Type:** unit
**Description:** When NATS is unreachable, the client retries with backoff and returns an error after max attempts.

**Preconditions:**
- No NATS server running on the target URL.

**Input:**
- `nats_client.Connect("nats://localhost:19999", 5)`

**Expected:**
- Returns error after 5 retry attempts.
- Total elapsed time >= 7s (1+2+4 seconds of backoff between attempts).

**Assertion pseudocode:**
```
start = time.Now()
_, err = nats_client.Connect("nats://localhost:19999", 5)
elapsed = time.Since(start)
ASSERT err != nil
ASSERT elapsed >= 7 * time.Second
```

### TS-06-E7: Config File Missing

**Requirement:** 06-REQ-6.E1
**Type:** unit
**Description:** Missing config file causes LoadConfig to return an error.

**Preconditions:**
- No file at the specified path.

**Input:**
- `config.LoadConfig("/nonexistent/config.json")`

**Expected:**
- Returns non-nil error.

**Assertion pseudocode:**
```
_, err = config.LoadConfig("/nonexistent/config.json")
ASSERT err != nil
```

### TS-06-E8: Config File Invalid JSON

**Requirement:** 06-REQ-6.E1
**Type:** unit
**Description:** Invalid JSON config causes LoadConfig to return an error.

**Preconditions:**
- A temporary file containing `{invalid json`.

**Input:**
- `config.LoadConfig("/tmp/invalid.json")`

**Expected:**
- Returns non-nil error.

**Assertion pseudocode:**
```
_, err = config.LoadConfig("/tmp/invalid.json")
ASSERT err != nil
```

### TS-06-E9: Error Response Format

**Requirement:** 06-REQ-7.2
**Type:** integration
**Description:** All error responses use the format `{"error":"<message>"}`.

**Preconditions:**
- Service is running.

**Input:**
- `POST /vehicles/VIN12345/commands` without auth (401)
- `POST /vehicles/VIN99999/commands` with valid token for VIN12345 (403)

**Expected:**
- All error responses contain an `"error"` key with a non-empty string.

**Assertion pseudocode:**
```
// 401 case
resp1 = httptest.POST("/vehicles/VIN12345/commands", no_auth)
ASSERT resp1.StatusCode == 401
ASSERT json.Decode(resp1.Body).error != ""

// 403 case
resp2 = httptest.POST("/vehicles/VIN99999/commands",
    header("Authorization", "Bearer demo-token-001"))
ASSERT resp2.StatusCode == 403
ASSERT json.Decode(resp2.Body).error != ""
```

## Property Test Cases

### TS-06-P1: Token-VIN Isolation

**Property:** Property 1 from design.md
**Validates:** 06-REQ-3.2
**Type:** property
**Description:** For any valid token mapped to VIN V, requests to a different VIN W always return 403.

**For any:** All token-VIN pairs in config, all VINs V != token's VIN.
**Invariant:** The middleware returns 403.

**Assertion pseudocode:**
```
FOR ANY token IN config.Tokens:
    FOR ANY otherVIN IN all_vins WHERE otherVIN != token.VIN:
        req = newRequest("POST", "/vehicles/" + otherVIN + "/commands")
        req.Header.Set("Authorization", "Bearer " + token.Token)
        rec = httptest.NewRecorder()
        middleware.ServeHTTP(rec, req)
        ASSERT rec.Code == 403
```

### TS-06-P2: Response Store Consistency

**Property:** Property 2 from design.md
**Validates:** 06-REQ-2.1, 06-REQ-2.2
**Type:** property
**Description:** For any stored response, GetResponse returns the same data.

**For any:** Random CommandResponse values.
**Invariant:** StoreResponse followed by GetResponse returns the identical response.

**Assertion pseudocode:**
```
FOR ANY resp IN random_command_responses:
    store.StoreResponse(resp)
    got, found = store.GetResponse(resp.CommandID)
    ASSERT found == true
    ASSERT got.CommandID == resp.CommandID
    ASSERT got.Status == resp.Status
    ASSERT got.Reason == resp.Reason
```

### TS-06-P3: Timeout Completeness

**Property:** Property 3 from design.md
**Validates:** 06-REQ-1.3
**Type:** property
**Description:** For any command with no response, after the timeout the status is "timeout".

**For any:** Random command IDs and short timeout durations.
**Invariant:** After waiting longer than the timeout, the response exists with status "timeout".

**Assertion pseudocode:**
```
FOR ANY cmdID IN random_uuids:
    store = NewStore()
    timeout = 50 * time.Millisecond
    store.StartTimeout(cmdID, timeout)
    time.Sleep(timeout + 50*time.Millisecond)
    resp, found = store.GetResponse(cmdID)
    ASSERT found == true
    ASSERT resp.Status == "timeout"
```

### TS-06-P4: Authentication Gate

**Property:** Property 4 from design.md
**Validates:** 06-REQ-3.E1
**Type:** property
**Description:** For any request without a valid token, the middleware returns 401.

**For any:** Random invalid token strings.
**Invariant:** The middleware returns 401.

**Assertion pseudocode:**
```
FOR ANY token IN random_strings:
    IF token NOT IN config.validTokens:
        req = newRequest("GET", "/vehicles/VIN12345/commands/x")
        req.Header.Set("Authorization", "Bearer " + token)
        rec = httptest.NewRecorder()
        middleware.ServeHTTP(rec, req)
        ASSERT rec.Code == 401
```

### TS-06-P5: Timeout Cancellation

**Property:** Property 6 from design.md
**Validates:** 06-REQ-1.3, 06-REQ-2.2
**Type:** property
**Description:** For any command that receives a response before timeout, the status is not "timeout".

**For any:** Random command IDs with response arriving before timeout.
**Invariant:** The stored status matches the NATS response, not "timeout".

**Assertion pseudocode:**
```
FOR ANY cmdID IN random_uuids:
    store = NewStore()
    store.StartTimeout(cmdID, 500*time.Millisecond)
    store.StoreResponse({CommandID: cmdID, Status: "success"})
    time.Sleep(600 * time.Millisecond)  // wait past timeout
    resp, _ = store.GetResponse(cmdID)
    ASSERT resp.Status == "success"  // NOT "timeout"
```

### TS-06-P6: NATS Header Propagation

**Property:** Property 5 from design.md
**Validates:** 06-REQ-1.2
**Type:** property
**Description:** For any command published to NATS, the message contains the bearer token from the originating REST request in the Authorization header.

**For any:** Random valid tokens from config and random valid commands.
**Invariant:** The NATS message header `Authorization` equals `Bearer <token>` for the token used in the REST request.

**Assertion pseudocode:**
```
FOR ANY token IN config.Tokens:
    cmd = {CommandID: random_uuid(), Type: "lock", Doors: ["driver"]}
    nats_client.PublishCommand(token.VIN, cmd, token.Token)
    msg = nats.Subscribe("vehicles." + token.VIN + ".commands").NextMsg(1s)
    ASSERT msg.Header.Get("Authorization") == "Bearer " + token.Token
```

## Integration Smoke Tests

### TS-06-SMOKE-1: End-to-End Command Flow

**Type:** integration (requires NATS)
**Description:** Full flow: submit command via REST, receive on NATS, publish response on NATS, query status via REST.

**Preconditions:**
- CLOUD_GATEWAY running and connected to NATS.
- A test NATS subscriber on `vehicles.VIN12345.commands`.

**Steps:**
1. Subscribe to `vehicles.VIN12345.commands` on NATS.
2. POST `/vehicles/VIN12345/commands` with `{"command_id":"smoke-001","type":"lock","doors":["driver"]}`.
3. Receive the command on NATS subscriber.
4. Publish response to `vehicles.VIN12345.command_responses`: `{"command_id":"smoke-001","status":"success"}`.
5. GET `/vehicles/VIN12345/commands/smoke-001`.

**Expected:**
- Step 2: HTTP 202
- Step 3: NATS message contains the command with Authorization header
- Step 5: HTTP 200 with `{"command_id":"smoke-001","status":"success"}`

**Assertion pseudocode:**
```
// 1. Subscribe
sub = nats.Subscribe("vehicles.VIN12345.commands")

// 2. Submit command
postResp = http.POST("/vehicles/VIN12345/commands",
    header("Authorization", "Bearer demo-token-001"),
    body({"command_id":"smoke-001","type":"lock","doors":["driver"]}))
ASSERT postResp.StatusCode == 202

// 3. Receive on NATS
msg = sub.NextMsg(2s)
ASSERT msg != nil
cmd = json.Decode(msg.Data)
ASSERT cmd.command_id == "smoke-001"
ASSERT msg.Header.Get("Authorization") == "Bearer demo-token-001"

// 4. Publish response
nats.Publish("vehicles.VIN12345.command_responses",
    {"command_id":"smoke-001","status":"success"})
time.Sleep(200 * time.Millisecond)

// 5. Query status
getResp = http.GET("/vehicles/VIN12345/commands/smoke-001",
    header("Authorization", "Bearer demo-token-001"))
ASSERT getResp.StatusCode == 200
result = json.Decode(getResp.Body)
ASSERT result.command_id == "smoke-001"
ASSERT result.status == "success"
```

### TS-06-SMOKE-2: Command Timeout End-to-End

**Type:** integration (requires NATS)
**Description:** Submit a command, do not send a response, verify timeout status.

**Preconditions:**
- CLOUD_GATEWAY running with command_timeout_seconds set to 1 (for fast test).

**Steps:**
1. POST `/vehicles/VIN12345/commands` with `{"command_id":"smoke-002","type":"unlock","doors":["driver"]}`.
2. Wait 2 seconds (past the 1s timeout).
3. GET `/vehicles/VIN12345/commands/smoke-002`.

**Expected:**
- Step 1: HTTP 202
- Step 3: HTTP 200 with `{"command_id":"smoke-002","status":"timeout"}`

**Assertion pseudocode:**
```
postResp = http.POST("/vehicles/VIN12345/commands",
    header("Authorization", "Bearer demo-token-001"),
    body({"command_id":"smoke-002","type":"unlock","doors":["driver"]}))
ASSERT postResp.StatusCode == 202

time.Sleep(2 * time.Second)

getResp = http.GET("/vehicles/VIN12345/commands/smoke-002",
    header("Authorization", "Bearer demo-token-001"))
ASSERT getResp.StatusCode == 200
result = json.Decode(getResp.Body)
ASSERT result.status == "timeout"
```

## Coverage Matrix

| Requirement | Test Spec Entry | Type |
|-------------|-----------------|------|
| 06-REQ-1.1 | TS-06-1 | integration |
| 06-REQ-1.2 | TS-06-2 | integration |
| 06-REQ-1.3 | TS-06-3 | unit |
| 06-REQ-1.E1 | TS-06-E1 | unit |
| 06-REQ-1.E2 | TS-06-E2 | unit |
| 06-REQ-2.1 | TS-06-4 | integration |
| 06-REQ-2.2 | TS-06-5 | unit |
| 06-REQ-2.E1 | TS-06-E3 | integration |
| 06-REQ-3.1 | TS-06-8 | unit |
| 06-REQ-3.2 | TS-06-9 | unit |
| 06-REQ-3.E1 | TS-06-E4, TS-06-E5 | unit |
| 06-REQ-4.1 | TS-06-10 | integration |
| 06-REQ-5.1 | TS-06-6 | integration |
| 06-REQ-5.2 | TS-06-6 | integration |
| 06-REQ-5.3 | TS-06-7 | integration |
| 06-REQ-5.E1 | TS-06-E6 | unit |
| 06-REQ-5.E2 | — (nats.go built-in) | — |
| 06-REQ-6.1 | TS-06-11 | unit |
| 06-REQ-6.2 | TS-06-11, TS-06-12 | unit |
| 06-REQ-6.3 | TS-06-3 | unit |
| 06-REQ-6.E1 | TS-06-E7, TS-06-E8 | unit |
| 06-REQ-7.1 | TS-06-13 | integration |
| 06-REQ-7.2 | TS-06-E9 | integration |
| 06-REQ-8.1 | TS-06-15 | integration |
| 06-REQ-8.2 | TS-06-14 | integration |
| Property 1 | TS-06-P1 | property |
| Property 2 | TS-06-P2 | property |
| Property 3 | TS-06-P3 | property |
| Property 4 | TS-06-P4 | property |
| Property 5 (NATS header) | TS-06-2, TS-06-P6 | integration, property |
| Property 6 | TS-06-P5 | property |
| Smoke (full flow) | TS-06-SMOKE-1 | integration |
| Smoke (timeout) | TS-06-SMOKE-2 | integration |
