# Test Specification: Mock Apps

## Overview

This test specification defines test contracts for all six mock tools. Rust mock sensor tests use `std::process::Command` to invoke binaries and verify exit codes and output. Go mock app tests use table-driven unit tests and HTTP/gRPC test helpers. Integration tests in `tests/mock-apps/` test end-to-end behavior. Mock sensor unit tests run via `cd rhivos && cargo test -p mock-sensors`. Go tests run via `cd mock && go test -v ./...`. Integration tests run via `cd tests/mock-apps && go test -v ./...`.

## Test Cases

### TS-09-1: Location Sensor Publishes Coordinates

**Requirement:** 09-REQ-1.1
**Type:** integration
**Description:** location-sensor publishes Latitude and Longitude to DATA_BROKER and exits 0.

**Preconditions:**
- DATA_BROKER running on default port.

**Input:**
- `location-sensor --lat=48.1351 --lon=11.5820`

**Expected:**
- VSS `Vehicle.CurrentLocation.Latitude` = 48.1351
- VSS `Vehicle.CurrentLocation.Longitude` = 11.5820
- Exit code 0.

**Assertion pseudocode:**
```
proc = exec("location-sensor", "--lat=48.1351", "--lon=11.5820")
ASSERT proc.exit_code == 0
lat = databroker.get("Vehicle.CurrentLocation.Latitude")
lon = databroker.get("Vehicle.CurrentLocation.Longitude")
ASSERT lat == 48.1351
ASSERT lon == 11.5820
```

### TS-09-2: Speed Sensor Publishes Speed

**Requirement:** 09-REQ-2.1
**Type:** integration
**Description:** speed-sensor publishes Vehicle.Speed to DATA_BROKER and exits 0.

**Preconditions:**
- DATA_BROKER running on default port.

**Input:**
- `speed-sensor --speed=0.0`

**Expected:**
- VSS `Vehicle.Speed` = 0.0
- Exit code 0.

**Assertion pseudocode:**
```
proc = exec("speed-sensor", "--speed=0.0")
ASSERT proc.exit_code == 0
speed = databroker.get("Vehicle.Speed")
ASSERT speed == 0.0
```

### TS-09-3: Door Sensor Publishes Open State

**Requirement:** 09-REQ-3.1
**Type:** integration
**Description:** door-sensor publishes IsOpen=true to DATA_BROKER when invoked with --open.

**Preconditions:**
- DATA_BROKER running on default port.

**Input:**
- `door-sensor --open`

**Expected:**
- VSS `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` = true
- Exit code 0.

**Assertion pseudocode:**
```
proc = exec("door-sensor", "--open")
ASSERT proc.exit_code == 0
is_open = databroker.get("Vehicle.Cabin.Door.Row1.DriverSide.IsOpen")
ASSERT is_open == true
```

### TS-09-4: Door Sensor Publishes Closed State

**Requirement:** 09-REQ-3.1
**Type:** integration
**Description:** door-sensor publishes IsOpen=false to DATA_BROKER when invoked with --closed.

**Preconditions:**
- DATA_BROKER running on default port.

**Input:**
- `door-sensor --closed`

**Expected:**
- VSS `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` = false
- Exit code 0.

**Assertion pseudocode:**
```
proc = exec("door-sensor", "--closed")
ASSERT proc.exit_code == 0
is_open = databroker.get("Vehicle.Cabin.Door.Row1.DriverSide.IsOpen")
ASSERT is_open == false
```

### TS-09-5: Parking App CLI Lookup

**Requirement:** 09-REQ-4.1
**Type:** integration
**Description:** parking-app-cli lookup sends GET to PARKING_FEE_SERVICE and prints JSON.

**Preconditions:**
- Mock PARKING_FEE_SERVICE returning operator list.

**Input:**
- `parking-app-cli lookup --lat=48.1351 --lon=11.5820`

**Expected:**
- HTTP GET sent to `/operators?lat=48.1351&lon=11.5820`.
- JSON response printed to stdout.
- Exit code 0.

**Assertion pseudocode:**
```
mock_pfs = startMockHTTPServer(response=[{operator_id: "op-1", name: "Demo"}])
proc = exec("parking-app-cli", "lookup", "--lat=48.1351", "--lon=11.5820",
            "--service-addr=" + mock_pfs.addr)
ASSERT proc.exit_code == 0
ASSERT "op-1" IN proc.stdout
```

### TS-09-6: Parking App CLI Adapter Info

**Requirement:** 09-REQ-4.2
**Type:** integration
**Description:** parking-app-cli adapter-info queries adapter metadata from PARKING_FEE_SERVICE.

**Preconditions:**
- Mock PARKING_FEE_SERVICE returning adapter metadata.

**Input:**
- `parking-app-cli adapter-info --operator-id=op-1`

**Expected:**
- HTTP GET sent to `/operators/op-1/adapter`.
- JSON response printed to stdout.
- Exit code 0.

**Assertion pseudocode:**
```
mock_pfs = startMockHTTPServer(response={image_ref: "registry/adapter:v1", checksum: "sha256:abc"})
proc = exec("parking-app-cli", "adapter-info", "--operator-id=op-1",
            "--service-addr=" + mock_pfs.addr)
ASSERT proc.exit_code == 0
ASSERT "image_ref" IN proc.stdout
```

### TS-09-7: Parking App CLI Install Adapter

**Requirement:** 09-REQ-5.1
**Type:** integration
**Description:** parking-app-cli install calls InstallAdapter on UPDATE_SERVICE.

**Preconditions:**
- Mock UPDATE_SERVICE gRPC server.

**Input:**
- `parking-app-cli install --image-ref=registry/adapter:v1 --checksum=sha256:abc`

**Expected:**
- InstallAdapter RPC called.
- Response printed to stdout.
- Exit code 0.

**Assertion pseudocode:**
```
mock_us = startMockGRPCServer(InstallAdapter returns {job_id: "j1", adapter_id: "a1", state: DOWNLOADING})
proc = exec("parking-app-cli", "install", "--image-ref=registry/adapter:v1",
            "--checksum=sha256:abc", "--update-addr=" + mock_us.addr)
ASSERT proc.exit_code == 0
ASSERT "j1" IN proc.stdout
```

### TS-09-8: Parking App CLI List Adapters

**Requirement:** 09-REQ-5.2
**Type:** integration
**Description:** parking-app-cli list calls ListAdapters on UPDATE_SERVICE.

**Preconditions:**
- Mock UPDATE_SERVICE gRPC server.

**Input:**
- `parking-app-cli list`

**Expected:**
- ListAdapters RPC called.
- Response printed to stdout.
- Exit code 0.

**Assertion pseudocode:**
```
mock_us = startMockGRPCServer(ListAdapters returns {adapters: [{adapter_id: "a1"}]})
proc = exec("parking-app-cli", "list", "--update-addr=" + mock_us.addr)
ASSERT proc.exit_code == 0
ASSERT "a1" IN proc.stdout
```

### TS-09-9: Parking App CLI Start Session Override

**Requirement:** 09-REQ-6.1
**Type:** integration
**Description:** parking-app-cli start-session calls StartSession on PARKING_OPERATOR_ADAPTOR.

**Preconditions:**
- Mock PARKING_OPERATOR_ADAPTOR gRPC server.

**Input:**
- `parking-app-cli start-session --zone-id=zone-demo-1`

**Expected:**
- StartSession(zone_id="zone-demo-1") RPC called.
- Response printed to stdout.
- Exit code 0.

**Assertion pseudocode:**
```
mock_adaptor = startMockGRPCServer(StartSession returns {session_id: "s1", status: "active"})
proc = exec("parking-app-cli", "start-session", "--zone-id=zone-demo-1",
            "--adaptor-addr=" + mock_adaptor.addr)
ASSERT proc.exit_code == 0
ASSERT "s1" IN proc.stdout
```

### TS-09-10: Parking App CLI Stop Session Override

**Requirement:** 09-REQ-6.2
**Type:** integration
**Description:** parking-app-cli stop-session calls StopSession on PARKING_OPERATOR_ADAPTOR.

**Preconditions:**
- Mock PARKING_OPERATOR_ADAPTOR gRPC server.

**Input:**
- `parking-app-cli stop-session`

**Expected:**
- StopSession() RPC called.
- Response printed to stdout.
- Exit code 0.

**Assertion pseudocode:**
```
mock_adaptor = startMockGRPCServer(StopSession returns {session_id: "s1", status: "stopped"})
proc = exec("parking-app-cli", "stop-session", "--adaptor-addr=" + mock_adaptor.addr)
ASSERT proc.exit_code == 0
ASSERT "stopped" IN proc.stdout
```

### TS-09-11: Companion App CLI Lock

**Requirement:** 09-REQ-7.1
**Type:** integration
**Description:** companion-app-cli lock sends POST to CLOUD_GATEWAY with lock command.

**Preconditions:**
- Mock CLOUD_GATEWAY returning command response.

**Input:**
- `companion-app-cli lock --vin=VIN001 --token=test-token`

**Expected:**
- POST `/vehicles/VIN001/commands` with body `{"type":"lock","doors":["driver"]}`.
- Authorization header: `Bearer test-token`.
- Exit code 0.

**Assertion pseudocode:**
```
mock_cg = startMockHTTPServer(assertHeader("Authorization", "Bearer test-token"),
                              response={command_id: "cmd-1", status: "pending"})
proc = exec("companion-app-cli", "lock", "--vin=VIN001", "--token=test-token",
            "--gateway-addr=" + mock_cg.addr)
ASSERT proc.exit_code == 0
ASSERT "cmd-1" IN proc.stdout
ASSERT mock_cg.receivedBody.type == "lock"
```

### TS-09-12: Companion App CLI Unlock

**Requirement:** 09-REQ-7.2
**Type:** integration
**Description:** companion-app-cli unlock sends POST to CLOUD_GATEWAY with unlock command.

**Preconditions:**
- Mock CLOUD_GATEWAY returning command response.

**Input:**
- `companion-app-cli unlock --vin=VIN001 --token=test-token`

**Expected:**
- POST `/vehicles/VIN001/commands` with body `{"type":"unlock","doors":["driver"]}`.
- Exit code 0.

**Assertion pseudocode:**
```
mock_cg = startMockHTTPServer(response={command_id: "cmd-2", status: "pending"})
proc = exec("companion-app-cli", "unlock", "--vin=VIN001", "--token=test-token",
            "--gateway-addr=" + mock_cg.addr)
ASSERT proc.exit_code == 0
ASSERT mock_cg.receivedBody.type == "unlock"
```

### TS-09-13: Companion App CLI Status

**Requirement:** 09-REQ-7.3
**Type:** integration
**Description:** companion-app-cli status queries command status from CLOUD_GATEWAY.

**Preconditions:**
- Mock CLOUD_GATEWAY returning command status.

**Input:**
- `companion-app-cli status --vin=VIN001 --command-id=cmd-1 --token=test-token`

**Expected:**
- GET `/vehicles/VIN001/commands/cmd-1`.
- Exit code 0.

**Assertion pseudocode:**
```
mock_cg = startMockHTTPServer(response={command_id: "cmd-1", status: "success"})
proc = exec("companion-app-cli", "status", "--vin=VIN001", "--command-id=cmd-1",
            "--token=test-token", "--gateway-addr=" + mock_cg.addr)
ASSERT proc.exit_code == 0
ASSERT "success" IN proc.stdout
```

### TS-09-14: Parking Operator Start Session

**Requirement:** 09-REQ-8.2
**Type:** unit
**Description:** POST /parking/start creates a session and returns JSON with session_id and rate.

**Preconditions:**
- parking-operator serve running.

**Input:**
- POST `/parking/start` with `{"vehicle_id":"VIN001","zone_id":"zone-1","timestamp":1700000000}`

**Expected:**
- HTTP 200.
- Response contains `session_id` (UUID format), `status: "active"`, `rate.amount: 2.50`.

**Assertion pseudocode:**
```
resp = http.POST("/parking/start", {vehicle_id: "VIN001", zone_id: "zone-1", timestamp: 1700000000})
ASSERT resp.status == 200
body = resp.json()
ASSERT body.status == "active"
ASSERT body.rate.amount == 2.50
ASSERT body.rate.currency == "EUR"
ASSERT UUID_REGEX.matches(body.session_id)
```

### TS-09-15: Parking Operator Stop Session

**Requirement:** 09-REQ-8.3
**Type:** unit
**Description:** POST /parking/stop stops session and returns duration and total_amount.

**Preconditions:**
- Active session started at timestamp 1700000000.

**Input:**
- POST `/parking/stop` with `{"session_id":"<from start>","timestamp":1700003600}`

**Expected:**
- HTTP 200.
- `duration_seconds: 3600`, `total_amount: 2.50` (2.50/hr * 1hr).

**Assertion pseudocode:**
```
start_resp = http.POST("/parking/start", {vehicle_id: "VIN001", zone_id: "zone-1", timestamp: 1700000000})
session_id = start_resp.json().session_id
stop_resp = http.POST("/parking/stop", {session_id: session_id, timestamp: 1700003600})
ASSERT stop_resp.status == 200
body = stop_resp.json()
ASSERT body.duration_seconds == 3600
ASSERT body.total_amount == 2.50
ASSERT body.currency == "EUR"
```

### TS-09-16: Parking Operator Session Status

**Requirement:** 09-REQ-8.4
**Type:** unit
**Description:** GET /parking/status/{session_id} returns session state.

**Preconditions:**
- Active session exists.

**Input:**
- GET `/parking/status/{session_id}`

**Expected:**
- HTTP 200.
- Response contains session info.

**Assertion pseudocode:**
```
start_resp = http.POST("/parking/start", {vehicle_id: "VIN001", zone_id: "zone-1", timestamp: 1700000000})
session_id = start_resp.json().session_id
status_resp = http.GET("/parking/status/" + session_id)
ASSERT status_resp.status == 200
body = status_resp.json()
ASSERT body.session_id == session_id
ASSERT body.status == "active"
```

### TS-09-17: Parking Operator Graceful Shutdown

**Requirement:** 09-REQ-8.1
**Type:** integration
**Description:** parking-operator serve shuts down on SIGTERM with exit 0.

**Preconditions:**
- parking-operator serve running.

**Input:**
- Send SIGTERM.

**Expected:**
- Exit code 0.

**Assertion pseudocode:**
```
proc = exec("parking-operator", "serve", "--port=9999")
waitUntilListening(9999)
proc.Signal(SIGTERM)
ASSERT proc.Wait().exit_code == 0
```

## Edge Case Tests

### TS-09-E1: Location Sensor Missing Args

**Requirement:** 09-REQ-1.E1
**Type:** unit
**Description:** location-sensor with missing --lat or --lon exits 1 with usage error.

**Preconditions:**
- None.

**Input:**
- `location-sensor --lat=48.13` (missing --lon)

**Expected:**
- Exit code 1. Stderr contains error message.

**Assertion pseudocode:**
```
proc = exec("location-sensor", "--lat=48.13")
ASSERT proc.exit_code == 1
ASSERT proc.stderr.len() > 0
```

### TS-09-E2: Speed Sensor Missing Args

**Requirement:** 09-REQ-2.E1
**Type:** unit
**Description:** speed-sensor with missing --speed exits 1.

**Preconditions:**
- None.

**Input:**
- `speed-sensor` (no args)

**Expected:**
- Exit code 1. Stderr contains error message.

**Assertion pseudocode:**
```
proc = exec("speed-sensor")
ASSERT proc.exit_code == 1
ASSERT proc.stderr.len() > 0
```

### TS-09-E3: Door Sensor Missing Args

**Requirement:** 09-REQ-3.E1
**Type:** unit
**Description:** door-sensor with neither --open nor --closed exits 1.

**Preconditions:**
- None.

**Input:**
- `door-sensor` (no args)

**Expected:**
- Exit code 1. Stderr contains error message.

**Assertion pseudocode:**
```
proc = exec("door-sensor")
ASSERT proc.exit_code == 1
ASSERT proc.stderr.len() > 0
```

### TS-09-E4: Sensor Unreachable DATA_BROKER

**Requirement:** 09-REQ-1.E2, 09-REQ-2.E2, 09-REQ-3.E2
**Type:** unit
**Description:** Mock sensors exit 1 when DATA_BROKER is unreachable.

**Preconditions:**
- No DATA_BROKER running.

**Input:**
- `location-sensor --lat=48.13 --lon=11.58 --broker-addr=http://localhost:19999`

**Expected:**
- Exit code 1. Stderr contains connection error.

**Assertion pseudocode:**
```
proc = exec("location-sensor", "--lat=48.13", "--lon=11.58",
            "--broker-addr=http://localhost:19999")
ASSERT proc.exit_code == 1
ASSERT proc.stderr.len() > 0
```

### TS-09-E5: Companion App Missing Token

**Requirement:** 09-REQ-7.E2
**Type:** unit
**Description:** companion-app-cli exits 1 when no bearer token is provided.

**Preconditions:**
- CLOUD_GATEWAY_TOKEN env var not set.

**Input:**
- `companion-app-cli lock --vin=VIN001` (no --token, no env var)

**Expected:**
- Exit code 1. Stderr contains error about missing token.

**Assertion pseudocode:**
```
unsetenv("CLOUD_GATEWAY_TOKEN")
proc = exec("companion-app-cli", "lock", "--vin=VIN001")
ASSERT proc.exit_code == 1
ASSERT "token" IN proc.stderr.lower()
```

### TS-09-E6: Companion App Missing VIN

**Requirement:** 09-REQ-7.E1
**Type:** unit
**Description:** companion-app-cli exits 1 when --vin is missing.

**Preconditions:**
- None.

**Input:**
- `companion-app-cli lock --token=test-token` (no --vin)

**Expected:**
- Exit code 1. Stderr contains error.

**Assertion pseudocode:**
```
proc = exec("companion-app-cli", "lock", "--token=test-token")
ASSERT proc.exit_code == 1
ASSERT proc.stderr.len() > 0
```

### TS-09-E7: Parking Operator Stop Unknown Session

**Requirement:** 09-REQ-8.E1
**Type:** unit
**Description:** POST /parking/stop with unknown session_id returns 404.

**Preconditions:**
- parking-operator serve running with no sessions.

**Input:**
- POST `/parking/stop` with `{"session_id":"nonexistent","timestamp":1700000000}`

**Expected:**
- HTTP 404.

**Assertion pseudocode:**
```
resp = http.POST("/parking/stop", {session_id: "nonexistent", timestamp: 1700000000})
ASSERT resp.status == 404
```

### TS-09-E8: Parking Operator Status Unknown Session

**Requirement:** 09-REQ-8.E2
**Type:** unit
**Description:** GET /parking/status with unknown session_id returns 404.

**Preconditions:**
- parking-operator serve running with no sessions.

**Input:**
- GET `/parking/status/nonexistent`

**Expected:**
- HTTP 404.

**Assertion pseudocode:**
```
resp = http.GET("/parking/status/nonexistent")
ASSERT resp.status == 404
```

### TS-09-E9: Parking Operator Malformed Request

**Requirement:** 09-REQ-8.E3
**Type:** unit
**Description:** POST /parking/start with malformed body returns 400.

**Preconditions:**
- parking-operator serve running.

**Input:**
- POST `/parking/start` with `"not valid json"`

**Expected:**
- HTTP 400.

**Assertion pseudocode:**
```
resp = http.POST("/parking/start", "not valid json", contentType="text/plain")
ASSERT resp.status == 400
```

### TS-09-E10: Parking App CLI gRPC Error

**Requirement:** 09-REQ-5.E2, 09-REQ-6.E1
**Type:** unit
**Description:** parking-app-cli exits 1 and prints gRPC error on RPC failure.

**Preconditions:**
- No UPDATE_SERVICE running.

**Input:**
- `parking-app-cli install --image-ref=x --checksum=y --update-addr=localhost:19999`

**Expected:**
- Exit code 1. Stderr contains error.

**Assertion pseudocode:**
```
proc = exec("parking-app-cli", "install", "--image-ref=x", "--checksum=y",
            "--update-addr=localhost:19999")
ASSERT proc.exit_code == 1
ASSERT proc.stderr.len() > 0
```

### TS-09-E11: PARKING_FEE_SERVICE Non-2xx

**Requirement:** 09-REQ-4.E2
**Type:** unit
**Description:** parking-app-cli exits 1 on non-2xx HTTP response from PARKING_FEE_SERVICE.

**Preconditions:**
- Mock HTTP server returning 500.

**Input:**
- `parking-app-cli lookup --lat=0 --lon=0`

**Expected:**
- Exit code 1. Stderr contains HTTP status.

**Assertion pseudocode:**
```
mock_pfs = startMockHTTPServer(status=500, body="internal error")
proc = exec("parking-app-cli", "lookup", "--lat=0", "--lon=0",
            "--service-addr=" + mock_pfs.addr)
ASSERT proc.exit_code == 1
ASSERT "500" IN proc.stderr
```

## Property Test Cases

### TS-09-P1: Sensor Publish-and-Exit

**Property:** Property 1 from design.md
**Validates:** 09-REQ-1.1, 09-REQ-2.1, 09-REQ-3.1
**Type:** property
**Description:** For any valid sensor input, the tool publishes the correct VSS value and exits 0.

**For any:** Random lat/lon doubles, random speed floats, random bool for door.
**Invariant:** Exit code is 0 and DATA_BROKER contains the published value.

**Assertion pseudocode:**
```
FOR ANY lat IN random_doubles(-90, 90), lon IN random_doubles(-180, 180):
    proc = exec("location-sensor", "--lat=" + lat, "--lon=" + lon)
    ASSERT proc.exit_code == 0
    ASSERT databroker.get("Vehicle.CurrentLocation.Latitude") == lat
    ASSERT databroker.get("Vehicle.CurrentLocation.Longitude") == lon
```

### TS-09-P2: CLI Argument Validation

**Property:** Property 2 from design.md
**Validates:** 09-REQ-1.E1, 09-REQ-2.E1, 09-REQ-3.E1
**Type:** property
**Description:** For any invocation with missing required arguments, exit code is 1.

**For any:** Random subsets of required arguments (with at least one missing).
**Invariant:** Exit code is 1 and stderr is non-empty.

**Assertion pseudocode:**
```
FOR ANY missing_subset IN subsets_missing_required_args:
    proc = exec(tool, missing_subset)
    ASSERT proc.exit_code == 1
    ASSERT proc.stderr.len() > 0
```

### TS-09-P3: Parking Operator Session Integrity

**Property:** Property 4 from design.md
**Validates:** 09-REQ-8.2, 09-REQ-8.3, 09-REQ-8.5
**Type:** property
**Description:** For any start-stop sequence, duration and total_amount are calculated correctly.

**For any:** Random start timestamps and stop timestamps where stop > start.
**Invariant:** `duration_seconds == stop - start`, `total_amount == 2.50 * duration_hours`.

**Assertion pseudocode:**
```
FOR ANY start_ts IN random_timestamps, duration IN random_positive_ints:
    stop_ts = start_ts + duration
    start_resp = POST("/parking/start", {vehicle_id: "V1", zone_id: "z1", timestamp: start_ts})
    stop_resp = POST("/parking/stop", {session_id: start_resp.session_id, timestamp: stop_ts})
    ASSERT stop_resp.duration_seconds == duration
    expected_amount = 2.50 * (duration as f64 / 3600.0)
    ASSERT abs(stop_resp.total_amount - expected_amount) < 0.01
```

### TS-09-P4: Parking Operator Session Uniqueness

**Property:** Property 5 from design.md
**Validates:** 09-REQ-8.2, 09-REQ-8.5
**Type:** property
**Description:** For any number of start requests, all generated session_ids are unique UUIDs.

**For any:** N start requests (N = 1..100).
**Invariant:** All session_ids are distinct and match UUID format.

**Assertion pseudocode:**
```
ids = set()
FOR i IN 1..100:
    resp = POST("/parking/start", {vehicle_id: "V"+i, zone_id: "z1", timestamp: i})
    ASSERT UUID_REGEX.matches(resp.session_id)
    ASSERT resp.session_id NOT IN ids
    ids.add(resp.session_id)
```

### TS-09-P5: Parking Operator Session Uniqueness

**Property:** Property 5 from design.md
**Validates:** 09-REQ-8.2, 09-REQ-8.5
**Type:** property
**Description:** For any POST /parking/start request, the server generates a unique UUID-format session_id and stores the session in memory.

**For any:** N concurrent or sequential start requests (N = 1..100).
**Invariant:** All returned session_ids are distinct and match UUID format.

**Assertion pseudocode:**
```
ids = set()
FOR i IN 1..100:
    resp = POST("/parking/start", {vehicle_id: "V"+i, zone_id: "z1", timestamp: i})
    ASSERT resp.status == 200
    ASSERT UUID_REGEX.matches(resp.json().session_id)
    ASSERT resp.json().session_id NOT IN ids
    ids.add(resp.json().session_id)
ASSERT len(ids) == 100
```

### TS-09-P6: Bearer Token Enforcement

**Property:** Property 6 from design.md
**Validates:** 09-REQ-7.4, 09-REQ-7.E2
**Type:** property
**Description:** For any companion-app-cli invocation, the tool includes the bearer token from --token or CLOUD_GATEWAY_TOKEN in the Authorization header, and fails with exit code 1 if no token is available.

**For any:** Random valid tokens and invocations with no token provided.
**Invariant:** When a token is provided, the Authorization header equals "Bearer <token>". When no token is provided, the tool exits with code 1.

**Assertion pseudocode:**
```
// Token present: verify header propagation
FOR ANY token IN random_strings:
    mock_cg = startMockHTTPServer(captureHeaders=true, response={command_id: "x"})
    proc = exec("companion-app-cli", "lock", "--vin=VIN001", "--token=" + token,
                "--gateway-addr=" + mock_cg.addr)
    ASSERT proc.exit_code == 0
    ASSERT mock_cg.capturedHeader("Authorization") == "Bearer " + token

// Token absent: verify failure
unsetenv("CLOUD_GATEWAY_TOKEN")
proc = exec("companion-app-cli", "lock", "--vin=VIN001")
ASSERT proc.exit_code == 1
ASSERT "token" IN proc.stderr.lower()
```

## Integration Smoke Tests

### TS-09-SMOKE-1: End-to-End Sensor to DATA_BROKER

**Type:** smoke
**Description:** All three mock sensors publish values and a subscriber confirms receipt.

**Assertion pseudocode:**
```
start databroker
exec("location-sensor", "--lat=48.13", "--lon=11.58")
exec("speed-sensor", "--speed=0.0")
exec("door-sensor", "--closed")
ASSERT databroker.get("Vehicle.CurrentLocation.Latitude") == 48.13
ASSERT databroker.get("Vehicle.Speed") == 0.0
ASSERT databroker.get("Vehicle.Cabin.Door.Row1.DriverSide.IsOpen") == false
```

### TS-09-SMOKE-2: End-to-End Parking Operator Start-Stop

**Type:** smoke
**Description:** parking-operator serve handles a full start-stop lifecycle.

**Assertion pseudocode:**
```
proc = exec("parking-operator", "serve", "--port=9999")
waitUntilListening(9999)
start_resp = POST("http://localhost:9999/parking/start",
                  {vehicle_id: "V1", zone_id: "z1", timestamp: 1700000000})
ASSERT start_resp.status == 200
stop_resp = POST("http://localhost:9999/parking/stop",
                 {session_id: start_resp.json().session_id, timestamp: 1700003600})
ASSERT stop_resp.status == 200
ASSERT stop_resp.json().duration_seconds == 3600
proc.Signal(SIGTERM)
ASSERT proc.Wait().exit_code == 0
```

### TS-09-SMOKE-3: End-to-End Companion App Lock-Status

**Type:** smoke
**Description:** companion-app-cli sends lock command and queries status via CLOUD_GATEWAY.

**Assertion pseudocode:**
```
start cloud_gateway
proc_lock = exec("companion-app-cli", "lock", "--vin=VIN001", "--token=test-token")
ASSERT proc_lock.exit_code == 0
command_id = parse_json(proc_lock.stdout).command_id
proc_status = exec("companion-app-cli", "status", "--vin=VIN001",
                   "--command-id=" + command_id, "--token=test-token")
ASSERT proc_status.exit_code == 0
```

## Coverage Matrix

| Requirement | Test Spec Entry | Type |
|-------------|-----------------|------|
| 09-REQ-1.1 | TS-09-1 | integration |
| 09-REQ-1.2 | TS-09-1 | integration |
| 09-REQ-1.E1 | TS-09-E1 | unit |
| 09-REQ-1.E2 | TS-09-E4 | unit |
| 09-REQ-2.1 | TS-09-2 | integration |
| 09-REQ-2.2 | TS-09-2 | integration |
| 09-REQ-2.E1 | TS-09-E2 | unit |
| 09-REQ-2.E2 | TS-09-E4 | unit |
| 09-REQ-3.1 | TS-09-3, TS-09-4 | integration |
| 09-REQ-3.2 | TS-09-3 | integration |
| 09-REQ-3.E1 | TS-09-E3 | unit |
| 09-REQ-3.E2 | TS-09-E4 | unit |
| 09-REQ-4.1 | TS-09-5 | integration |
| 09-REQ-4.2 | TS-09-6 | integration |
| 09-REQ-4.3 | TS-09-5 | integration |
| 09-REQ-4.E1 | TS-09-E1 | unit |
| 09-REQ-4.E2 | TS-09-E11 | unit |
| 09-REQ-5.1 | TS-09-7 | integration |
| 09-REQ-5.2 | TS-09-8 | integration |
| 09-REQ-5.3 | — | manual |
| 09-REQ-5.4 | TS-09-8 | integration |
| 09-REQ-5.5 | TS-09-8 | integration |
| 09-REQ-5.6 | TS-09-7 | integration |
| 09-REQ-5.E1 | TS-09-E10 | unit |
| 09-REQ-5.E2 | TS-09-E10 | unit |
| 09-REQ-6.1 | TS-09-9 | integration |
| 09-REQ-6.2 | TS-09-10 | integration |
| 09-REQ-6.3 | TS-09-9 | integration |
| 09-REQ-6.E1 | TS-09-E10 | unit |
| 09-REQ-7.1 | TS-09-11 | integration |
| 09-REQ-7.2 | TS-09-12 | integration |
| 09-REQ-7.3 | TS-09-13 | integration |
| 09-REQ-7.4 | TS-09-11 | integration |
| 09-REQ-7.5 | TS-09-11 | integration |
| 09-REQ-7.E1 | TS-09-E6 | unit |
| 09-REQ-7.E2 | TS-09-E5 | unit |
| 09-REQ-7.E3 | TS-09-E11 | unit |
| 09-REQ-8.1 | TS-09-17 | integration |
| 09-REQ-8.2 | TS-09-14 | unit |
| 09-REQ-8.3 | TS-09-15 | unit |
| 09-REQ-8.4 | TS-09-16 | unit |
| 09-REQ-8.5 | TS-09-14, TS-09-15 | unit |
| 09-REQ-8.E1 | TS-09-E7 | unit |
| 09-REQ-8.E2 | TS-09-E8 | unit |
| 09-REQ-8.E3 | TS-09-E9 | unit |
| 09-REQ-9.1 | TS-09-E1 through TS-09-E11 | unit |
| 09-REQ-9.2 | TS-09-E1 through TS-09-E11 | unit |
| 09-REQ-9.3 | TS-09-1 through TS-09-17 | integration |
| 09-REQ-10.1 | TS-09-1 | integration |
| 09-REQ-10.2 | TS-09-1, TS-09-2, TS-09-3 | integration |
| Property 1 | TS-09-P1 | property |
| Property 2 | TS-09-P2 | property |
| Property 4 | TS-09-P3 | property |
| Property 5 | TS-09-P4, TS-09-P5 | property |
| Property 6 | TS-09-P6 | property |
