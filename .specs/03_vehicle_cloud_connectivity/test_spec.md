# Test Specification: Vehicle-to-Cloud Connectivity (Phase 2.2)

## Overview

This test specification translates every acceptance criterion and correctness
property from the requirements and design documents into concrete, executable
test contracts. Tests are organized into three categories:

- **Acceptance criterion tests (TS-03-N):** One per acceptance criterion.
  Implemented as Go test functions in `tests/cloud_connectivity/` or as
  unit tests within the component packages.
- **Property tests (TS-03-PN):** One per correctness property. Verify
  invariants that must hold across the system.
- **Edge case tests (TS-03-EN):** One per edge case requirement. Verify
  error handling and boundary behavior.

Tests fall into two runtime categories:
- **Unit tests:** Run without external dependencies (no Mosquitto required).
  Use `httptest`, mock MQTT clients, and in-memory trackers.
- **Integration tests:** Require a running Mosquitto broker (`make infra-up`).
  Gated by build tag or runtime check. Test the full REST-MQTT-REST cycle.

## Test Cases

### TS-03-1: POST /vehicles/{vin}/commands endpoint exists

**Requirement:** 03-REQ-1.1
**Type:** unit
**Description:** Verify the CLOUD_GATEWAY accepts POST requests to
`/vehicles/{vin}/commands` with the correct JSON body schema.

**Preconditions:**
- CLOUD_GATEWAY HTTP handler is initialized.

**Input:**
- POST request to `/vehicles/VIN12345/commands` with bearer token.
- Body: `{"command_id":"abc-123","type":"lock","doors":["driver"]}`.
- Mock bridge that returns a pending response.

**Expected:**
- HTTP status 2xx (202 Accepted or 200 OK depending on sync/async mode).
- Response body contains `command_id` field.

**Assertion pseudocode:**
```
handler = newCommandHandler(mockBridge)
req = httptest.NewRequest("POST", "/vehicles/VIN12345/commands", jsonBody)
req.Header.Set("Authorization", "Bearer demo-token")
rec = httptest.NewRecorder()
handler.ServeHTTP(rec, req)
ASSERT rec.Code == 202 OR rec.Code == 200
body = json.Decode(rec.Body)
ASSERT body["command_id"] == "abc-123"
```

---

### TS-03-2: GET /vehicles/{vin}/status endpoint exists

**Requirement:** 03-REQ-1.2
**Type:** unit
**Description:** Verify the CLOUD_GATEWAY returns vehicle status from cached
telemetry.

**Preconditions:**
- CLOUD_GATEWAY handler initialized with telemetry cache containing data for
  VIN12345.

**Input:**
- GET request to `/vehicles/VIN12345/status` with bearer token.

**Expected:**
- HTTP 200 with JSON body containing `vin`, `locked`, `timestamp`.

**Assertion pseudocode:**
```
cache = newTelemetryCache()
cache.Update("VIN12345", TelemetryData{Locked: true, Timestamp: 1708700000})
handler = newStatusHandler(cache)
req = httptest.NewRequest("GET", "/vehicles/VIN12345/status", nil)
req.Header.Set("Authorization", "Bearer demo-token")
rec = httptest.NewRecorder()
handler.ServeHTTP(rec, req)
ASSERT rec.Code == 200
body = json.Decode(rec.Body)
ASSERT body["vin"] == "VIN12345"
ASSERT body["locked"] == true
ASSERT body["timestamp"] == 1708700000
```

---

### TS-03-3: GET /health returns 200

**Requirement:** 03-REQ-1.3
**Type:** unit
**Description:** Verify the health endpoint returns 200 OK with no auth.

**Preconditions:**
- CLOUD_GATEWAY HTTP handler initialized.

**Input:**
- GET request to `/health` with no Authorization header.

**Expected:**
- HTTP 200 with body `{"status":"ok"}`.

**Assertion pseudocode:**
```
handler = newRouter(...)
req = httptest.NewRequest("GET", "/health", nil)
rec = httptest.NewRecorder()
handler.ServeHTTP(rec, req)
ASSERT rec.Code == 200
body = json.Decode(rec.Body)
ASSERT body["status"] == "ok"
```

---

### TS-03-4: Unauthorized request returns 401

**Requirement:** 03-REQ-1.4
**Type:** unit
**Description:** Verify requests without valid bearer token get 401.

**Preconditions:**
- CLOUD_GATEWAY handler initialized with auth middleware.

**Input:**
- POST request to `/vehicles/VIN12345/commands` with no Authorization header.
- POST request with `Authorization: Bearer wrong-token`.

**Expected:**
- HTTP 401 for both cases.

**Assertion pseudocode:**
```
handler = newRouter(...)

req1 = httptest.NewRequest("POST", "/vehicles/VIN12345/commands", validBody)
rec1 = httptest.NewRecorder()
handler.ServeHTTP(rec1, req1)
ASSERT rec1.Code == 401

req2 = httptest.NewRequest("POST", "/vehicles/VIN12345/commands", validBody)
req2.Header.Set("Authorization", "Bearer wrong-token")
rec2 = httptest.NewRecorder()
handler.ServeHTTP(rec2, req2)
ASSERT rec2.Code == 401
```

---

### TS-03-5: Valid command returns 202 and publishes to MQTT

**Requirement:** 03-REQ-1.5
**Type:** unit
**Description:** Verify a valid command results in 202 Accepted and triggers
MQTT publish.

**Preconditions:**
- CLOUD_GATEWAY handler initialized with mock MQTT publisher.

**Input:**
- POST request with valid token and well-formed body.

**Expected:**
- HTTP 202 Accepted with `{"command_id":"...","status":"pending"}`.
- Mock MQTT publisher received one message for the correct topic.

**Assertion pseudocode:**
```
mockMQTT = newMockMQTTPublisher()
handler = newCommandHandler(mockMQTT)
req = httptest.NewRequest("POST", "/vehicles/VIN12345/commands", validBody)
req.Header.Set("Authorization", "Bearer demo-token")
rec = httptest.NewRecorder()
handler.ServeHTTP(rec, req)
ASSERT rec.Code == 202
ASSERT mockMQTT.PublishCount() == 1
ASSERT mockMQTT.LastTopic() == "vehicles/VIN12345/commands"
```

---

### TS-03-6: MQTT broker connection on startup

**Requirement:** 03-REQ-2.1
**Type:** integration
**Description:** Verify CLOUD_GATEWAY connects to MQTT broker on startup.

**Preconditions:**
- Mosquitto broker running on localhost:1883 (`make infra-up`).

**Input:**
- Start CLOUD_GATEWAY with `MQTT_BROKER_URL=tcp://localhost:1883`.

**Expected:**
- CLOUD_GATEWAY starts without error. MQTT connection is established.

**Assertion pseudocode:**
```
IF NOT portIsOpen(1883):
    t.Skip("Mosquitto not running")
gw = startCloudGateway(mqttURL="tcp://localhost:1883")
ASSERT gw.IsConnected()
gw.Shutdown()
```

---

### TS-03-7: REST command publishes to correct MQTT topic

**Requirement:** 03-REQ-2.2
**Type:** integration
**Description:** Verify a REST command is published to the correct MQTT topic
with the correct payload.

**Preconditions:**
- Mosquitto running. CLOUD_GATEWAY started and connected.

**Input:**
- Subscribe to `vehicles/VIN12345/commands` via test MQTT client.
- Send POST `/vehicles/VIN12345/commands` with valid command.

**Expected:**
- MQTT message received on `vehicles/VIN12345/commands`.
- Message contains `command_id`, `action`, `doors`, `source`.

**Assertion pseudocode:**
```
IF NOT portIsOpen(1883):
    t.Skip("Mosquitto not running")
gw = startCloudGateway()
testClient = newMQTTClient()
msgChan = testClient.Subscribe("vehicles/VIN12345/commands")
http.Post(gw.URL+"/vehicles/VIN12345/commands", validBody, authHeader)
msg = receive(msgChan, timeout=5s)
ASSERT msg != nil
payload = json.Decode(msg.Payload)
ASSERT payload["command_id"] == "abc-123"
ASSERT payload["action"] == "lock"
ASSERT payload["doors"] == ["driver"]
ASSERT payload["source"] == "companion_app"
```

---

### TS-03-8: Subscribe to command_responses topic

**Requirement:** 03-REQ-2.3
**Type:** integration
**Description:** Verify CLOUD_GATEWAY subscribes to command_responses and
resolves pending commands.

**Preconditions:**
- Mosquitto running. CLOUD_GATEWAY started.

**Input:**
- Send a command via REST (which registers a pending entry).
- Publish a matching response on `vehicles/VIN12345/command_responses`.

**Expected:**
- The REST response is delivered with the status from the MQTT response.

**Assertion pseudocode:**
```
IF NOT portIsOpen(1883):
    t.Skip("Mosquitto not running")
gw = startCloudGateway()
go func() {
    // Wait for MQTT command, then publish response
    testClient = newMQTTClient()
    testClient.Subscribe("vehicles/VIN12345/commands")
    msg = receive(cmdChan, 5s)
    cmdId = json.Decode(msg.Payload)["command_id"]
    testClient.Publish("vehicles/VIN12345/command_responses",
        json.Encode({"command_id": cmdId, "status": "success", "timestamp": now()}))
}()
resp = http.Post(gw.URL+"/vehicles/VIN12345/commands", validBody, authHeader)
ASSERT resp.StatusCode == 200
body = json.Decode(resp.Body)
ASSERT body["status"] == "success"
```

---

### TS-03-9: Subscribe to telemetry topic

**Requirement:** 03-REQ-2.4
**Type:** integration
**Description:** Verify CLOUD_GATEWAY subscribes to telemetry and caches data
for status endpoint.

**Preconditions:**
- Mosquitto running. CLOUD_GATEWAY started.

**Input:**
- Publish telemetry data on `vehicles/VIN12345/telemetry`.
- Wait briefly for processing.
- GET `/vehicles/VIN12345/status`.

**Expected:**
- Status response reflects the published telemetry.

**Assertion pseudocode:**
```
IF NOT portIsOpen(1883):
    t.Skip("Mosquitto not running")
gw = startCloudGateway()
testClient = newMQTTClient()
testClient.Publish("vehicles/VIN12345/telemetry",
    json.Encode({"vin":"VIN12345","locked":true,"timestamp":1234}))
sleep(500ms)
resp = http.Get(gw.URL+"/vehicles/VIN12345/status", authHeader)
ASSERT resp.StatusCode == 200
body = json.Decode(resp.Body)
ASSERT body["locked"] == true
```

---

### TS-03-10: MQTT response resolves pending command

**Requirement:** 03-REQ-2.5
**Type:** unit
**Description:** Verify the Command Tracker correctly resolves a pending
command when a matching MQTT response arrives.

**Preconditions:**
- Command Tracker initialized.

**Input:**
- Register a pending command with ID "cmd-001".
- Resolve "cmd-001" with a success response.

**Expected:**
- The response channel receives the response.
- The pending entry is removed.

**Assertion pseudocode:**
```
tracker = newCommandTracker()
ch = tracker.Register("cmd-001")
go func() {
    tracker.Resolve("cmd-001", CommandResponse{Status: "success"})
}()
resp = receive(ch, timeout=1s)
ASSERT resp.Status == "success"
ASSERT tracker.HasPending("cmd-001") == false
```

---

### TS-03-11: Command ID preserved in MQTT message

**Requirement:** 03-REQ-3.1
**Type:** unit
**Description:** Verify the command_id from REST is preserved in the MQTT
message.

**Preconditions:**
- Bridge module initialized with mock MQTT publisher.

**Input:**
- Submit command with `command_id = "preserve-me-123"`.

**Expected:**
- MQTT message payload contains `command_id = "preserve-me-123"`.

**Assertion pseudocode:**
```
mockMQTT = newMockMQTTPublisher()
bridge = newBridge(mockMQTT)
bridge.SendCommand("VIN12345", Command{
    CommandID: "preserve-me-123",
    Type: "lock",
    Doors: ["driver"]
})
published = mockMQTT.LastMessage()
payload = json.Decode(published.Payload)
ASSERT payload["command_id"] == "preserve-me-123"
```

---

### TS-03-12: Response matched by command_id

**Requirement:** 03-REQ-3.2
**Type:** unit
**Description:** Verify MQTT response is matched to the correct pending
request using command_id.

**Preconditions:**
- Command Tracker with two pending commands.

**Input:**
- Register "cmd-A" and "cmd-B".
- Resolve "cmd-B" first.

**Expected:**
- Only "cmd-B" receives the response. "cmd-A" remains pending.

**Assertion pseudocode:**
```
tracker = newCommandTracker()
chA = tracker.Register("cmd-A")
chB = tracker.Register("cmd-B")
tracker.Resolve("cmd-B", CommandResponse{Status: "success"})
respB = receive(chB, timeout=1s)
ASSERT respB.Status == "success"
ASSERT tracker.HasPending("cmd-A") == true
ASSERT tracker.HasPending("cmd-B") == false
```

---

### TS-03-13: MQTT command message schema

**Requirement:** 03-REQ-3.3
**Type:** unit
**Description:** Verify the MQTT command message conforms to the expected
JSON schema.

**Preconditions:**
- Bridge module initialized.

**Input:**
- Submit a lock command via the bridge.

**Expected:**
- Published MQTT payload has exactly the fields: `command_id`, `action`,
  `doors`, `source`.

**Assertion pseudocode:**
```
mockMQTT = newMockMQTTPublisher()
bridge = newBridge(mockMQTT)
bridge.SendCommand("VIN12345", Command{
    CommandID: "schema-test",
    Type: "lock",
    Doors: ["driver"]
})
payload = json.Decode(mockMQTT.LastMessage().Payload)
ASSERT hasKey(payload, "command_id")
ASSERT hasKey(payload, "action")
ASSERT hasKey(payload, "doors")
ASSERT hasKey(payload, "source")
ASSERT payload["action"] == "lock"
ASSERT payload["source"] == "companion_app"
```

---

### TS-03-14: MQTT response message schema validation

**Requirement:** 03-REQ-3.4
**Type:** unit
**Description:** Verify the bridge correctly parses MQTT response messages
conforming to the expected schema.

**Preconditions:**
- Bridge module initialized.

**Input:**
- Valid response JSON: `{"command_id":"x","status":"success","reason":"","timestamp":123}`.
- Invalid response JSON: missing `command_id`.

**Expected:**
- Valid message is parsed and resolved.
- Invalid message is logged and discarded.

**Assertion pseudocode:**
```
bridge = newBridge(...)
tracker = bridge.Tracker()
ch = tracker.Register("x")

bridge.HandleResponse([]byte(
    '{"command_id":"x","status":"success","reason":"","timestamp":123}'))
resp = receive(ch, 1s)
ASSERT resp.Status == "success"

// Invalid: no error, just logged and discarded
bridge.HandleResponse([]byte('{"status":"success"}'))
// No panic, no crash
```

---

### TS-03-15: CLI lock command sends correct REST request

**Requirement:** 03-REQ-4.1
**Type:** unit
**Description:** Verify the `lock` subcommand sends the correct POST request.

**Preconditions:**
- Mock HTTP server simulating CLOUD_GATEWAY.

**Input:**
- Execute `companion-app-cli lock --vin VIN12345 --token demo-token
  --gateway-url <mock-server-url>`.

**Expected:**
- Mock server receives POST to `/vehicles/VIN12345/commands`.
- Request body has `type: "lock"`, `doors: ["driver"]`, and a non-empty
  `command_id`.
- Authorization header is `Bearer demo-token`.

**Assertion pseudocode:**
```
var receivedReq *http.Request
server = httptest.NewServer(func(w, r) {
    receivedReq = r
    w.WriteHeader(200)
    w.Write('{"command_id":"test","status":"success"}')
})
exec("companion-app-cli", "lock",
    "--vin", "VIN12345",
    "--token", "demo-token",
    "--gateway-url", server.URL)
ASSERT receivedReq.Method == "POST"
ASSERT receivedReq.URL.Path == "/vehicles/VIN12345/commands"
body = json.Decode(receivedReq.Body)
ASSERT body["type"] == "lock"
ASSERT body["doors"] == ["driver"]
ASSERT body["command_id"] != ""
ASSERT receivedReq.Header.Get("Authorization") == "Bearer demo-token"
```

---

### TS-03-16: CLI unlock command sends correct REST request

**Requirement:** 03-REQ-4.2
**Type:** unit
**Description:** Verify the `unlock` subcommand sends the correct POST request.

**Preconditions:**
- Mock HTTP server simulating CLOUD_GATEWAY.

**Input:**
- Execute `companion-app-cli unlock --vin VIN12345 --token demo-token
  --gateway-url <mock-server-url>`.

**Expected:**
- Request body has `type: "unlock"`.

**Assertion pseudocode:**
```
var receivedBody map[string]interface{}
server = httptest.NewServer(func(w, r) {
    receivedBody = json.Decode(r.Body)
    w.WriteHeader(200)
    w.Write('{"command_id":"test","status":"success"}')
})
exec("companion-app-cli", "unlock",
    "--vin", "VIN12345",
    "--token", "demo-token",
    "--gateway-url", server.URL)
ASSERT receivedBody["type"] == "unlock"
```

---

### TS-03-17: CLI status command sends GET request

**Requirement:** 03-REQ-4.3
**Type:** unit
**Description:** Verify the `status` subcommand sends a GET request and
displays the response.

**Preconditions:**
- Mock HTTP server simulating CLOUD_GATEWAY.

**Input:**
- Execute `companion-app-cli status --vin VIN12345 --token demo-token
  --gateway-url <mock-server-url>`.

**Expected:**
- Mock server receives GET to `/vehicles/VIN12345/status`.
- CLI stdout contains the response data.

**Assertion pseudocode:**
```
var receivedReq *http.Request
server = httptest.NewServer(func(w, r) {
    receivedReq = r
    w.WriteHeader(200)
    w.Write('{"vin":"VIN12345","locked":true,"timestamp":1234}')
})
result = exec("companion-app-cli", "status",
    "--vin", "VIN12345",
    "--token", "demo-token",
    "--gateway-url", server.URL)
ASSERT receivedReq.Method == "GET"
ASSERT receivedReq.URL.Path == "/vehicles/VIN12345/status"
ASSERT result.exit_code == 0
ASSERT contains(result.stdout, "VIN12345")
ASSERT contains(result.stdout, "locked")
```

---

### TS-03-18: CLI includes bearer token in Authorization header

**Requirement:** 03-REQ-4.4
**Type:** unit
**Description:** Verify all CLI commands include the bearer token.

**Preconditions:**
- Mock HTTP server.

**Input:**
- Execute each CLI command with `--token my-secret-token`.

**Expected:**
- Authorization header is `Bearer my-secret-token` for all requests.

**Assertion pseudocode:**
```
FOR EACH cmd IN ["lock", "unlock", "status"]:
    var authHeader string
    server = httptest.NewServer(func(w, r) {
        authHeader = r.Header.Get("Authorization")
        w.WriteHeader(200)
        w.Write(validResponse)
    })
    exec("companion-app-cli", cmd,
        "--vin", "VIN12345",
        "--token", "my-secret-token",
        "--gateway-url", server.URL)
    ASSERT authHeader == "Bearer my-secret-token"
```

---

### TS-03-19: CLI uses VIN from --vin flag in URL

**Requirement:** 03-REQ-4.5
**Type:** unit
**Description:** Verify the CLI uses the --vin flag value in the request URL.

**Preconditions:**
- Mock HTTP server.

**Input:**
- Execute `companion-app-cli lock --vin CUSTOM_VIN_999 --token demo-token
  --gateway-url <mock-server-url>`.

**Expected:**
- Request URL path contains `CUSTOM_VIN_999`.

**Assertion pseudocode:**
```
var requestPath string
server = httptest.NewServer(func(w, r) {
    requestPath = r.URL.Path
    w.WriteHeader(200)
    w.Write(validResponse)
})
exec("companion-app-cli", "lock",
    "--vin", "CUSTOM_VIN_999",
    "--token", "demo-token",
    "--gateway-url", server.URL)
ASSERT contains(requestPath, "CUSTOM_VIN_999")
```

---

### TS-03-20: CLI success output

**Requirement:** 03-REQ-4.6
**Type:** unit
**Description:** Verify CLI prints JSON response to stdout on success.

**Preconditions:**
- Mock HTTP server returning 200.

**Input:**
- Execute `companion-app-cli lock --vin VIN12345 --token demo-token`.

**Expected:**
- Exit code 0. Stdout contains JSON response.

**Assertion pseudocode:**
```
server = httptest.NewServer(func(w, r) {
    w.WriteHeader(200)
    w.Write('{"command_id":"abc","status":"success"}')
})
result = exec("companion-app-cli", "lock",
    "--vin", "VIN12345",
    "--token", "demo-token",
    "--gateway-url", server.URL)
ASSERT result.exit_code == 0
ASSERT contains(result.stdout, "command_id")
ASSERT contains(result.stdout, "success")
```

---

### TS-03-21: CLI error output

**Requirement:** 03-REQ-4.7
**Type:** unit
**Description:** Verify CLI prints error to stderr and exits non-zero on
failure.

**Preconditions:**
- Mock HTTP server returning 401.

**Input:**
- Execute `companion-app-cli lock --vin VIN12345 --token wrong-token
  --gateway-url <mock-server-url>`.

**Expected:**
- Exit code != 0. Stderr contains error message.

**Assertion pseudocode:**
```
server = httptest.NewServer(func(w, r) {
    w.WriteHeader(401)
    w.Write('{"error":"unauthorized"}')
})
result = exec("companion-app-cli", "lock",
    "--vin", "VIN12345",
    "--token", "wrong-token",
    "--gateway-url", server.URL)
ASSERT result.exit_code != 0
ASSERT len(result.stderr) > 0
```

---

### TS-03-22: Multi-vehicle concurrent commands

**Requirement:** 03-REQ-5.1
**Type:** integration
**Description:** Verify concurrent commands for different VINs are routed to
the correct MQTT topics.

**Preconditions:**
- Mosquitto running. CLOUD_GATEWAY started.

**Input:**
- Subscribe to `vehicles/VIN_A/commands` and `vehicles/VIN_B/commands`.
- Send a lock command for VIN_A and an unlock command for VIN_B concurrently.

**Expected:**
- VIN_A topic receives lock command with correct command_id.
- VIN_B topic receives unlock command with correct command_id.
- No cross-contamination.

**Assertion pseudocode:**
```
IF NOT portIsOpen(1883):
    t.Skip("Mosquitto not running")
gw = startCloudGateway()
testClient = newMQTTClient()
chanA = testClient.Subscribe("vehicles/VIN_A/commands")
chanB = testClient.Subscribe("vehicles/VIN_B/commands")

go http.Post(gw.URL+"/vehicles/VIN_A/commands", lockBody("cmdA"), authHeader)
go http.Post(gw.URL+"/vehicles/VIN_B/commands", unlockBody("cmdB"), authHeader)

msgA = receive(chanA, 5s)
msgB = receive(chanB, 5s)
ASSERT json.Decode(msgA.Payload)["command_id"] == "cmdA"
ASSERT json.Decode(msgA.Payload)["action"] == "lock"
ASSERT json.Decode(msgB.Payload)["command_id"] == "cmdB"
ASSERT json.Decode(msgB.Payload)["action"] == "unlock"
```

---

### TS-03-23: Multi-vehicle response isolation

**Requirement:** 03-REQ-5.2
**Type:** unit
**Description:** Verify command response tracking is isolated per vehicle.

**Preconditions:**
- Command Tracker initialized.

**Input:**
- Register pending commands for VIN_A ("cmd-A") and VIN_B ("cmd-B").
- Resolve "cmd-A" with success.

**Expected:**
- Only "cmd-A" receives the response. "cmd-B" remains pending.

**Assertion pseudocode:**
```
tracker = newCommandTracker()
chA = tracker.Register("cmd-A")
chB = tracker.Register("cmd-B")
tracker.Resolve("cmd-A", CommandResponse{Status: "success"})
respA = receive(chA, 1s)
ASSERT respA.Status == "success"
ASSERT tracker.HasPending("cmd-B") == true
```

---

### TS-03-24: End-to-end integration test

**Requirement:** 03-REQ-6.1
**Type:** integration
**Description:** Full end-to-end test: CLI -> CLOUD_GATEWAY REST -> MQTT ->
simulated subscriber -> MQTT response -> REST response.

**Preconditions:**
- Mosquitto running on localhost:1883.

**Input:**
- Start CLOUD_GATEWAY connected to Mosquitto.
- Start a simulated CLOUD_GATEWAY_CLIENT (test MQTT subscriber) that echoes
  commands as success responses.
- Send a lock command via HTTP client (simulating CLI).

**Expected:**
- Command is received by the simulated subscriber via MQTT.
- Response is published back via MQTT.
- REST response contains matching `command_id` and `status: "success"`.

**Assertion pseudocode:**
```
IF NOT portIsOpen(1883):
    t.Skip("Mosquitto not running")
gw = startCloudGateway()

// Simulated CLOUD_GATEWAY_CLIENT
simulator = newMQTTClient()
simulator.Subscribe("vehicles/+/commands", func(msg) {
    cmd = json.Decode(msg.Payload)
    vin = extractVINFromTopic(msg.Topic)
    simulator.Publish("vehicles/"+vin+"/command_responses",
        json.Encode({"command_id": cmd["command_id"], "status": "success",
                     "timestamp": now()}))
})

cmdID = uuid.New()
resp = http.Post(gw.URL+"/vehicles/VIN12345/commands",
    json.Encode({"command_id": cmdID, "type": "lock", "doors": ["driver"]}),
    authHeader)
ASSERT resp.StatusCode == 200
body = json.Decode(resp.Body)
ASSERT body["command_id"] == cmdID
ASSERT body["status"] == "success"
gw.Shutdown()
```

---

### TS-03-25: Integration test runs with go test

**Requirement:** 03-REQ-6.2
**Type:** integration
**Description:** Verify the integration test is executable via `go test`.

**Preconditions:**
- Mosquitto running. Go toolchain installed.

**Input:**
- Run `go test -v -tags integration ./...` in the cloud-gateway module or
  integration test directory.

**Expected:**
- Tests execute and pass (or skip if Mosquitto is not running).

**Assertion pseudocode:**
```
result = exec("go test -v -count=1 -run TestIntegration ./...",
    cwd="backend/cloud-gateway/")
ASSERT result.exit_code == 0
ASSERT contains(result.stdout, "PASS") OR contains(result.stdout, "SKIP")
```

---

### TS-03-26: Integration test verifies command correlation

**Requirement:** 03-REQ-6.3
**Type:** integration
**Description:** Verify the integration test checks command_id correlation
end-to-end.

**Preconditions:**
- Mosquitto running. CLOUD_GATEWAY started.

**Input:**
- Send command with specific command_id. Simulate response with same ID.

**Expected:**
- REST response command_id matches the original request command_id.

**Assertion pseudocode:**
```
IF NOT portIsOpen(1883):
    t.Skip("Mosquitto not running")
gw = startCloudGateway()
setupSimulator()

originalID = "correlation-test-" + uuid.New()
resp = http.Post(gw.URL+"/vehicles/VIN12345/commands",
    json.Encode({"command_id": originalID, "type": "unlock", "doors": ["driver"]}),
    authHeader)
body = json.Decode(resp.Body)
ASSERT body["command_id"] == originalID
```

---

## Property Test Cases

### TS-03-P1: Command ID Preservation

**Property:** Property 1 from design.md
**Validates:** 03-REQ-3.1, 03-REQ-3.3
**Type:** property
**Description:** For any command submitted via REST, the command_id in the MQTT
message is identical to the one in the REST request.

**For any:** Command C with a random UUID as command_id
**Invariant:** `mqtt_message.command_id == rest_request.command_id`

**Assertion pseudocode:**
```
FOR i IN range(20):
    cmdID = uuid.New()
    mockMQTT = newMockMQTTPublisher()
    bridge = newBridge(mockMQTT)
    bridge.SendCommand("VIN-"+str(i), Command{CommandID: cmdID, Type: "lock"})
    published = json.Decode(mockMQTT.LastMessage().Payload)
    ASSERT published["command_id"] == cmdID
```

---

### TS-03-P2: Response Correlation Correctness

**Property:** Property 2 from design.md
**Validates:** 03-REQ-3.2, 03-REQ-2.5
**Type:** property
**Description:** For any MQTT response with a matching command_id, the REST
response contains the same command_id and status.

**For any:** Pending command P and response R where R.command_id == P.command_id
**Invariant:** REST response command_id == P.command_id AND REST response
status == R.status

**Assertion pseudocode:**
```
FOR EACH status IN ["success", "failed"]:
    tracker = newCommandTracker()
    cmdID = uuid.New()
    ch = tracker.Register(cmdID)
    tracker.Resolve(cmdID, CommandResponse{Status: status})
    resp = receive(ch, 1s)
    ASSERT resp.Status == status
```

---

### TS-03-P3: Authentication Enforcement

**Property:** Property 3 from design.md
**Validates:** 03-REQ-1.4
**Type:** property
**Description:** For any protected endpoint, requests without a valid token
always return 401.

**For any:** Endpoint E in {POST /vehicles/{vin}/commands,
GET /vehicles/{vin}/status}
**Invariant:** If request has no token or invalid token, response is 401.

**Assertion pseudocode:**
```
handler = newRouter(...)
FOR EACH endpoint IN [
    ("POST", "/vehicles/VIN12345/commands"),
    ("GET", "/vehicles/VIN12345/status")
]:
    // No token
    req1 = httptest.NewRequest(endpoint.method, endpoint.path, validBody)
    rec1 = httptest.NewRecorder()
    handler.ServeHTTP(rec1, req1)
    ASSERT rec1.Code == 401

    // Wrong token
    req2 = httptest.NewRequest(endpoint.method, endpoint.path, validBody)
    req2.Header.Set("Authorization", "Bearer invalid")
    rec2 = httptest.NewRecorder()
    handler.ServeHTTP(rec2, req2)
    ASSERT rec2.Code == 401

    // Missing "Bearer" prefix
    req3 = httptest.NewRequest(endpoint.method, endpoint.path, validBody)
    req3.Header.Set("Authorization", "demo-token")
    rec3 = httptest.NewRecorder()
    handler.ServeHTTP(rec3, req3)
    ASSERT rec3.Code == 401
```

---

### TS-03-P4: Topic Routing Correctness

**Property:** Property 4 from design.md
**Validates:** 03-REQ-2.2, 03-REQ-5.1
**Type:** property
**Description:** For any command for VIN V, the MQTT message is published to
`vehicles/V/commands` only.

**For any:** VIN V in {"VIN_A", "VIN_B", "VIN_XYZ123"}
**Invariant:** `mqtt_topic == "vehicles/" + V + "/commands"`

**Assertion pseudocode:**
```
FOR EACH vin IN ["VIN_A", "VIN_B", "VIN_XYZ123", "WBAPH5C55BA270000"]:
    mockMQTT = newMockMQTTPublisher()
    bridge = newBridge(mockMQTT)
    bridge.SendCommand(vin, Command{CommandID: uuid.New(), Type: "lock"})
    ASSERT mockMQTT.LastTopic() == "vehicles/" + vin + "/commands"
```

---

### TS-03-P5: Timeout Guarantee

**Property:** Property 5 from design.md
**Validates:** 03-REQ-2.E3
**Type:** unit
**Description:** For any pending command without an MQTT response, the REST
client receives 504 after timeout.

**For any:** Command C with no response within timeout
**Invariant:** REST response status is 504 with status "timeout"

**Assertion pseudocode:**
```
tracker = newCommandTracker(timeout=100ms)  // short timeout for testing
ch = tracker.Register("will-timeout")
resp = receive(ch, 200ms)
ASSERT resp.Status == "timeout"
```

---

### TS-03-P6: Multi-Vehicle Isolation

**Property:** Property 6 from design.md
**Validates:** 03-REQ-5.2
**Type:** unit
**Description:** For any two concurrent commands for different VINs, responses
are delivered to the correct pending request.

**For any:** VINs V1, V2 where V1 != V2
**Invariant:** Response for V1 is delivered to V1's pending request, not V2's.

**Assertion pseudocode:**
```
tracker = newCommandTracker()
chA = tracker.Register("cmd-for-VIN_A")
chB = tracker.Register("cmd-for-VIN_B")

// Resolve in reverse order
tracker.Resolve("cmd-for-VIN_B", CommandResponse{Status: "success", Reason: "B"})
tracker.Resolve("cmd-for-VIN_A", CommandResponse{Status: "failed", Reason: "A"})

respA = receive(chA, 1s)
respB = receive(chB, 1s)
ASSERT respA.Reason == "A"
ASSERT respB.Reason == "B"
```

---

### TS-03-P7: Graceful Degradation

**Property:** Property 7 from design.md
**Validates:** 03-REQ-2.E1, 03-REQ-2.E2
**Type:** unit
**Description:** For any state where the MQTT broker is unreachable, the REST
API remains responsive.

**For any:** CLOUD_GATEWAY state S where MQTT is disconnected
**Invariant:** REST API responds (does not hang or crash)

**Assertion pseudocode:**
```
// Start CLOUD_GATEWAY with unreachable MQTT broker
gw = startCloudGateway(mqttURL="tcp://localhost:19999")  // no broker here
waitForHTTP(gw.URL + "/health", timeout=5s)
resp = http.Get(gw.URL + "/health")
ASSERT resp.StatusCode == 200

// Command should fail gracefully, not hang
resp2 = http.Post(gw.URL + "/vehicles/VIN12345/commands", validBody, authHeader,
    timeout=5s)
ASSERT resp2 != nil  // got a response, did not hang
ASSERT resp2.StatusCode >= 400  // error response, not success
```

---

## Edge Case Tests

### TS-03-E1: Missing required fields in command body

**Requirement:** 03-REQ-1.E1
**Type:** unit
**Description:** Verify 400 Bad Request when required fields are missing.

**Preconditions:**
- CLOUD_GATEWAY handler initialized.

**Input:**
- POST with body missing `command_id`: `{"type":"lock","doors":["driver"]}`.
- POST with body missing `type`: `{"command_id":"x","doors":["driver"]}`.
- POST with body missing `doors`: `{"command_id":"x","type":"lock"}`.
- POST with invalid `type` value: `{"command_id":"x","type":"open","doors":["driver"]}`.

**Expected:**
- HTTP 400 for each. Response body contains `error` field.

**Assertion pseudocode:**
```
handler = newCommandHandler(...)
FOR EACH body IN [
    '{"type":"lock","doors":["driver"]}',           // missing command_id
    '{"command_id":"x","doors":["driver"]}',         // missing type
    '{"command_id":"x","type":"lock"}',              // missing doors
    '{"command_id":"x","type":"open","doors":["d"]}' // invalid type
]:
    req = httptest.NewRequest("POST", "/vehicles/VIN12345/commands",
        strings.NewReader(body))
    req.Header.Set("Authorization", "Bearer demo-token")
    rec = httptest.NewRecorder()
    handler.ServeHTTP(rec, req)
    ASSERT rec.Code == 400
    respBody = json.Decode(rec.Body)
    ASSERT hasKey(respBody, "error")
```

---

### TS-03-E2: Invalid JSON in command body

**Requirement:** 03-REQ-1.E2
**Type:** unit
**Description:** Verify 400 Bad Request when body is not valid JSON.

**Preconditions:**
- CLOUD_GATEWAY handler initialized.

**Input:**
- POST with body: `not json at all`.
- POST with body: `{malformed`.

**Expected:**
- HTTP 400 for both.

**Assertion pseudocode:**
```
handler = newCommandHandler(...)
FOR EACH body IN ["not json at all", "{malformed", ""]:
    req = httptest.NewRequest("POST", "/vehicles/VIN12345/commands",
        strings.NewReader(body))
    req.Header.Set("Authorization", "Bearer demo-token")
    rec = httptest.NewRecorder()
    handler.ServeHTTP(rec, req)
    ASSERT rec.Code == 400
```

---

### TS-03-E3: MQTT broker unreachable on startup

**Requirement:** 03-REQ-2.E1
**Type:** unit
**Description:** Verify CLOUD_GATEWAY starts REST API even when MQTT broker is
unreachable.

**Preconditions:**
- No MQTT broker running on the configured port.

**Input:**
- Start CLOUD_GATEWAY with `MQTT_BROKER_URL=tcp://localhost:19999`.

**Expected:**
- REST API starts and responds to health check.
- Log output contains retry attempt messages.

**Assertion pseudocode:**
```
gw = startCloudGateway(mqttURL="tcp://localhost:19999")
waitForHTTP(gw.URL + "/health", timeout=5s)
resp = http.Get(gw.URL + "/health")
ASSERT resp.StatusCode == 200
ASSERT contains(gw.LogOutput(), "retry") OR contains(gw.LogOutput(), "connect")
gw.Shutdown()
```

---

### TS-03-E4: MQTT broker connection lost after startup

**Requirement:** 03-REQ-2.E2
**Type:** integration
**Description:** Verify CLOUD_GATEWAY handles MQTT disconnection gracefully.

**Preconditions:**
- Mosquitto running. CLOUD_GATEWAY connected.

**Input:**
- Stop Mosquitto (simulate disconnection).
- Attempt to send a command.

**Expected:**
- CLOUD_GATEWAY does not crash.
- Command request receives an error response or timeout.
- Log output contains disconnection event.

**Assertion pseudocode:**
```
IF NOT portIsOpen(1883):
    t.Skip("Mosquitto not running")
gw = startCloudGateway()
ASSERT gw.IsConnected()

// This test requires ability to stop/start Mosquitto during test
// Implementation note: may need to use a dedicated test broker instance
exec("make infra-down")
sleep(2s)

resp = http.Post(gw.URL+"/vehicles/VIN/commands", validBody, authHeader,
    timeout=5s)
ASSERT resp != nil
ASSERT resp.StatusCode >= 400 OR resp.StatusCode == 504

exec("make infra-up")
```

---

### TS-03-E5: Command response timeout

**Requirement:** 03-REQ-2.E3
**Type:** unit
**Description:** Verify pending command times out and returns 504.

**Preconditions:**
- CLOUD_GATEWAY handler with short timeout (e.g., 200ms).

**Input:**
- Send a command. Do not publish any MQTT response.

**Expected:**
- After timeout, REST response is 504 with `{"command_id":"...","status":"timeout"}`.

**Assertion pseudocode:**
```
tracker = newCommandTracker(timeout=200ms)
handler = newCommandHandler(tracker, mockMQTT)
req = httptest.NewRequest("POST", "/vehicles/VIN12345/commands",
    json.Encode({"command_id":"timeout-test","type":"lock","doors":["driver"]}))
req.Header.Set("Authorization", "Bearer demo-token")
rec = httptest.NewRecorder()
handler.ServeHTTP(rec, req)
ASSERT rec.Code == 504
body = json.Decode(rec.Body)
ASSERT body["command_id"] == "timeout-test"
ASSERT body["status"] == "timeout"
```

---

### TS-03-E6: Unknown command_id in MQTT response

**Requirement:** 03-REQ-3.E1
**Type:** unit
**Description:** Verify unknown command_id responses are logged and discarded.

**Preconditions:**
- Command Tracker with no pending commands.

**Input:**
- Resolve a non-existent command_id "ghost-cmd".

**Expected:**
- No panic or crash. Resolve returns false or logs a warning.

**Assertion pseudocode:**
```
tracker = newCommandTracker()
resolved = tracker.Resolve("ghost-cmd", CommandResponse{Status: "success"})
ASSERT resolved == false
// No panic, no crash
```

---

### TS-03-E7: Duplicate command_id response

**Requirement:** 03-REQ-3.E2
**Type:** unit
**Description:** Verify only the first response for a command_id is used.

**Preconditions:**
- Command Tracker with one pending command.

**Input:**
- Register "cmd-dup".
- Resolve "cmd-dup" twice.

**Expected:**
- First resolve succeeds. Second resolve is ignored (returns false).

**Assertion pseudocode:**
```
tracker = newCommandTracker()
ch = tracker.Register("cmd-dup")
resolved1 = tracker.Resolve("cmd-dup", CommandResponse{Status: "success"})
ASSERT resolved1 == true
resp = receive(ch, 1s)
ASSERT resp.Status == "success"
resolved2 = tracker.Resolve("cmd-dup", CommandResponse{Status: "failed"})
ASSERT resolved2 == false
```

---

### TS-03-E8: CLI missing --token flag

**Requirement:** 03-REQ-4.E1
**Type:** unit
**Description:** Verify CLI exits with error when --token is not provided.

**Preconditions:**
- companion-app-cli binary built.

**Input:**
- Execute `companion-app-cli lock --vin VIN12345` (no --token).

**Expected:**
- Exit code != 0. Stderr mentions token is required.

**Assertion pseudocode:**
```
result = exec("companion-app-cli", "lock", "--vin", "VIN12345")
ASSERT result.exit_code != 0
ASSERT contains(result.stderr, "token")
```

---

### TS-03-E9: CLI cannot connect to CLOUD_GATEWAY

**Requirement:** 03-REQ-4.E2
**Type:** unit
**Description:** Verify CLI handles connection failure gracefully.

**Preconditions:**
- No service running on target port.

**Input:**
- Execute `companion-app-cli lock --vin VIN12345 --token demo-token
  --gateway-url http://localhost:19999`.

**Expected:**
- Exit code != 0. Stderr contains connection error.

**Assertion pseudocode:**
```
result = exec("companion-app-cli", "lock",
    "--vin", "VIN12345",
    "--token", "demo-token",
    "--gateway-url", "http://localhost:19999")
ASSERT result.exit_code != 0
ASSERT len(result.stderr) > 0
```

---

### TS-03-E10: Integration test skips without Mosquitto

**Requirement:** 03-REQ-6.E1
**Type:** unit
**Description:** Verify integration tests skip cleanly when Mosquitto is not
running.

**Preconditions:**
- Mosquitto not running.

**Input:**
- Run integration tests with Mosquitto stopped.

**Expected:**
- Tests skip (not fail) with a clear message.

**Assertion pseudocode:**
```
exec("make infra-down")
result = exec("go test -v -count=1 -run TestIntegration ./...",
    cwd="backend/cloud-gateway/")
ASSERT result.exit_code == 0
ASSERT contains(result.stdout, "SKIP") OR contains(result.stdout, "PASS")
ASSERT NOT contains(result.stdout, "FAIL")
```

---

## Coverage Matrix

| Requirement    | Test Spec Entry | Type        |
|----------------|-----------------|-------------|
| 03-REQ-1.1     | TS-03-1         | unit        |
| 03-REQ-1.2     | TS-03-2         | unit        |
| 03-REQ-1.3     | TS-03-3         | unit        |
| 03-REQ-1.4     | TS-03-4         | unit        |
| 03-REQ-1.5     | TS-03-5         | unit        |
| 03-REQ-1.E1    | TS-03-E1        | unit        |
| 03-REQ-1.E2    | TS-03-E2        | unit        |
| 03-REQ-2.1     | TS-03-6         | integration |
| 03-REQ-2.2     | TS-03-7         | integration |
| 03-REQ-2.3     | TS-03-8         | integration |
| 03-REQ-2.4     | TS-03-9         | integration |
| 03-REQ-2.5     | TS-03-10        | unit        |
| 03-REQ-2.E1    | TS-03-E3        | unit        |
| 03-REQ-2.E2    | TS-03-E4        | integration |
| 03-REQ-2.E3    | TS-03-E5        | unit        |
| 03-REQ-3.1     | TS-03-11        | unit        |
| 03-REQ-3.2     | TS-03-12        | unit        |
| 03-REQ-3.3     | TS-03-13        | unit        |
| 03-REQ-3.4     | TS-03-14        | unit        |
| 03-REQ-3.E1    | TS-03-E6        | unit        |
| 03-REQ-3.E2    | TS-03-E7        | unit        |
| 03-REQ-4.1     | TS-03-15        | unit        |
| 03-REQ-4.2     | TS-03-16        | unit        |
| 03-REQ-4.3     | TS-03-17        | unit        |
| 03-REQ-4.4     | TS-03-18        | unit        |
| 03-REQ-4.5     | TS-03-19        | unit        |
| 03-REQ-4.6     | TS-03-20        | unit        |
| 03-REQ-4.7     | TS-03-21        | unit        |
| 03-REQ-4.E1    | TS-03-E8        | unit        |
| 03-REQ-4.E2    | TS-03-E9        | unit        |
| 03-REQ-5.1     | TS-03-22        | integration |
| 03-REQ-5.2     | TS-03-23        | unit        |
| 03-REQ-6.1     | TS-03-24        | integration |
| 03-REQ-6.2     | TS-03-25        | integration |
| 03-REQ-6.3     | TS-03-26        | integration |
| 03-REQ-6.E1    | TS-03-E10       | unit        |
| Property 1     | TS-03-P1        | property    |
| Property 2     | TS-03-P2        | property    |
| Property 3     | TS-03-P3        | property    |
| Property 4     | TS-03-P4        | property    |
| Property 5     | TS-03-P5        | unit        |
| Property 6     | TS-03-P6        | unit        |
| Property 7     | TS-03-P7        | unit        |
