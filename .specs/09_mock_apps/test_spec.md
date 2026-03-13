# Test Specification: MOCK_APPS

## Overview

This test specification defines test contracts for the Mock Apps. Unit tests cover argument parsing, handler logic, store operations, and subcommand dispatch. Integration tests require running upstream services (DATA_BROKER, CLOUD_GATEWAY, etc.) and live in `tests/mock-apps/`. Rust sensor unit tests run via `cd rhivos && cargo test -p mock-sensors`. Go mock unit tests run via `cd mock && go test -v ./...`.

## Test Cases

### TS-09-1: Location Sensor Writes Lat/Lon

**Requirement:** 09-REQ-1.1
**Type:** integration
**Description:** location-sensor writes latitude and longitude to DATA_BROKER.

**Preconditions:**
- DATA_BROKER running on localhost:55556.

**Input:**
- `location-sensor --lat=48.1351 --lon=11.5820`

**Expected:**
- Exit code 0.
- DATA_BROKER contains Vehicle.CurrentLocation.Latitude = 48.1351, Vehicle.CurrentLocation.Longitude = 11.5820.

**Assertion pseudocode:**
```
exitCode = run("location-sensor", "--lat=48.1351", "--lon=11.5820")
ASSERT exitCode == 0
lat = broker.get("Vehicle.CurrentLocation.Latitude")
lon = broker.get("Vehicle.CurrentLocation.Longitude")
ASSERT lat == 48.1351
ASSERT lon == 11.5820
```

### TS-09-2: Speed Sensor Writes Speed

**Requirement:** 09-REQ-1.2
**Type:** integration
**Description:** speed-sensor writes Vehicle.Speed to DATA_BROKER.

**Preconditions:**
- DATA_BROKER running.

**Input:**
- `speed-sensor --speed=60.5`

**Expected:**
- Exit code 0.
- DATA_BROKER contains Vehicle.Speed = 60.5.

**Assertion pseudocode:**
```
exitCode = run("speed-sensor", "--speed=60.5")
ASSERT exitCode == 0
speed = broker.get("Vehicle.Speed")
ASSERT speed == 60.5
```

### TS-09-3: Door Sensor Writes Open/Closed

**Requirement:** 09-REQ-1.3
**Type:** integration
**Description:** door-sensor writes IsOpen true or false to DATA_BROKER.

**Preconditions:**
- DATA_BROKER running.

**Input:**
- `door-sensor --open` then `door-sensor --closed`

**Expected:**
- Exit code 0 for both.
- After --open: IsOpen = true. After --closed: IsOpen = false.

**Assertion pseudocode:**
```
exitCode = run("door-sensor", "--open")
ASSERT exitCode == 0
ASSERT broker.get("Vehicle.Cabin.Door.Row1.DriverSide.IsOpen") == true

exitCode = run("door-sensor", "--closed")
ASSERT exitCode == 0
ASSERT broker.get("Vehicle.Cabin.Door.Row1.DriverSide.IsOpen") == false
```

### TS-09-4: PARKING_OPERATOR Serve Starts Server

**Requirement:** 09-REQ-2.1
**Type:** integration
**Description:** parking-operator serve starts HTTP server on configured port.

**Preconditions:**
- Port 8080 available.

**Input:**
- `parking-operator serve --port=8080`

**Expected:**
- Server accepts HTTP connections.
- Startup log contains port number.

**Assertion pseudocode:**
```
proc = startProcess("parking-operator", "serve", "--port=8080")
waitForReady(proc, "8080")
resp = httpGet("http://localhost:8080/parking/status/nonexistent")
ASSERT resp.statusCode == 404
proc.Signal(SIGTERM)
```

### TS-09-5: PARKING_OPERATOR Start Session

**Requirement:** 09-REQ-2.2
**Type:** unit
**Description:** POST /parking/start creates session and returns correct response.

**Preconditions:**
- Handler initialized with empty store.

**Input:**
- POST /parking/start with `{"vehicle_id": "VIN001", "zone_id": "zone-1", "timestamp": 1700000000}`

**Expected:**
- HTTP 200.
- Response has session_id (UUID format), status="active", rate={per_hour, 2.50, EUR}.

**Assertion pseudocode:**
```
resp = httptest.POST("/parking/start", startRequest)
ASSERT resp.statusCode == 200
body = parseJSON(resp.body)
ASSERT body.session_id != ""
ASSERT body.status == "active"
ASSERT body.rate.rate_type == "per_hour"
ASSERT body.rate.amount == 2.50
ASSERT body.rate.currency == "EUR"
```

### TS-09-6: PARKING_OPERATOR Stop Session

**Requirement:** 09-REQ-2.3
**Type:** unit
**Description:** POST /parking/stop calculates duration and total_amount.

**Preconditions:**
- Store has active session "sess-001" started at timestamp 1700000000 with rate 2.50/hr.

**Input:**
- POST /parking/stop with `{"session_id": "sess-001", "timestamp": 1700003600}`

**Expected:**
- HTTP 200.
- duration_seconds=3600, total_amount=2.50, status="stopped".

**Assertion pseudocode:**
```
store.addSession("sess-001", startTime=1700000000, rate=2.50)
resp = httptest.POST("/parking/stop", stopRequest)
ASSERT resp.statusCode == 200
body = parseJSON(resp.body)
ASSERT body.duration_seconds == 3600
ASSERT body.total_amount == 2.50
ASSERT body.status == "stopped"
```

### TS-09-7: PARKING_OPERATOR Get Status

**Requirement:** 09-REQ-2.4
**Type:** unit
**Description:** GET /parking/status/{session_id} returns session info.

**Preconditions:**
- Store has active session "sess-001".

**Input:**
- GET /parking/status/sess-001

**Expected:**
- HTTP 200.
- Response contains session state as JSON.

**Assertion pseudocode:**
```
store.addSession("sess-001", ...)
resp = httptest.GET("/parking/status/sess-001")
ASSERT resp.statusCode == 200
body = parseJSON(resp.body)
ASSERT body.session_id == "sess-001"
ASSERT body.status == "active"
```

### TS-09-8: PARKING_OPERATOR Graceful Shutdown

**Requirement:** 09-REQ-2.5
**Type:** integration
**Description:** SIGTERM stops the server with exit code 0.

**Preconditions:**
- Server running.

**Input:**
- Send SIGTERM.

**Expected:**
- Exit code 0.

**Assertion pseudocode:**
```
proc = startProcess("parking-operator", "serve")
proc.Signal(SIGTERM)
exitCode = proc.Wait()
ASSERT exitCode == 0
```

### TS-09-9: COMPANION_APP Lock Command

**Requirement:** 09-REQ-3.1
**Type:** unit
**Description:** lock subcommand sends correct POST request to CLOUD_GATEWAY.

**Preconditions:**
- Mock HTTP server simulating CLOUD_GATEWAY.

**Input:**
- `companion-app-cli lock --vin=VIN001 --token=test-token --gateway-url=<mock-url>`

**Expected:**
- POST to /vehicles/VIN001/commands with lock payload and Bearer token.
- Response printed to stdout.

**Assertion pseudocode:**
```
mockServer = httptest.NewServer(captureHandler)
exitCode = run("companion-app-cli", "lock", "--vin=VIN001", "--token=test-token", "--gateway-url=" + mockServer.URL)
ASSERT exitCode == 0
ASSERT capturedRequest.method == "POST"
ASSERT capturedRequest.path == "/vehicles/VIN001/commands"
ASSERT capturedRequest.headers["Authorization"] == "Bearer test-token"
body = parseJSON(capturedRequest.body)
ASSERT body.type == "lock"
```

### TS-09-10: COMPANION_APP Unlock Command

**Requirement:** 09-REQ-3.2
**Type:** unit
**Description:** unlock subcommand sends correct POST request to CLOUD_GATEWAY.

**Preconditions:**
- Mock HTTP server simulating CLOUD_GATEWAY.

**Input:**
- `companion-app-cli unlock --vin=VIN001 --token=test-token`

**Expected:**
- POST to /vehicles/VIN001/commands with unlock payload.

**Assertion pseudocode:**
```
exitCode = run("companion-app-cli", "unlock", "--vin=VIN001", "--token=test-token", "--gateway-url=" + mockServer.URL)
ASSERT exitCode == 0
body = parseJSON(capturedRequest.body)
ASSERT body.type == "unlock"
```

### TS-09-11: COMPANION_APP Status Query

**Requirement:** 09-REQ-3.3
**Type:** unit
**Description:** status subcommand sends correct GET request.

**Preconditions:**
- Mock HTTP server.

**Input:**
- `companion-app-cli status --vin=VIN001 --command-id=cmd-123 --token=test-token`

**Expected:**
- GET to /vehicles/VIN001/commands/cmd-123.

**Assertion pseudocode:**
```
exitCode = run("companion-app-cli", "status", "--vin=VIN001", "--command-id=cmd-123", "--token=test-token", "--gateway-url=" + mockServer.URL)
ASSERT exitCode == 0
ASSERT capturedRequest.method == "GET"
ASSERT capturedRequest.path == "/vehicles/VIN001/commands/cmd-123"
```

### TS-09-12: PARKING_APP Lookup

**Requirement:** 09-REQ-4.1
**Type:** unit
**Description:** lookup subcommand queries PARKING_FEE_SERVICE.

**Preconditions:**
- Mock HTTP server simulating PARKING_FEE_SERVICE.

**Input:**
- `parking-app-cli lookup --lat=48.1351 --lon=11.5820`

**Expected:**
- GET to /operators?lat=48.1351&lon=11.5820.

**Assertion pseudocode:**
```
exitCode = run("parking-app-cli", "lookup", "--lat=48.1351", "--lon=11.5820")
ASSERT exitCode == 0
ASSERT capturedRequest.path == "/operators"
ASSERT capturedRequest.query["lat"] == "48.1351"
ASSERT capturedRequest.query["lon"] == "11.5820"
```

### TS-09-13: PARKING_APP Adapter Info

**Requirement:** 09-REQ-4.2
**Type:** unit
**Description:** adapter-info subcommand queries PARKING_FEE_SERVICE.

**Preconditions:**
- Mock HTTP server.

**Input:**
- `parking-app-cli adapter-info --operator-id=op-001`

**Expected:**
- GET to /operators/op-001/adapter.

**Assertion pseudocode:**
```
exitCode = run("parking-app-cli", "adapter-info", "--operator-id=op-001")
ASSERT exitCode == 0
ASSERT capturedRequest.path == "/operators/op-001/adapter"
```

### TS-09-14: PARKING_APP Install

**Requirement:** 09-REQ-4.3
**Type:** unit
**Description:** install subcommand calls UPDATE_SERVICE InstallAdapter gRPC.

**Preconditions:**
- Mock gRPC server simulating UPDATE_SERVICE.

**Input:**
- `parking-app-cli install --image-ref=ghcr.io/demo:v1 --checksum=abc123`

**Expected:**
- InstallAdapter called with image_ref and checksum.

**Assertion pseudocode:**
```
exitCode = run("parking-app-cli", "install", "--image-ref=ghcr.io/demo:v1", "--checksum=abc123")
ASSERT exitCode == 0
ASSERT mockGrpc.installCalled
ASSERT mockGrpc.capturedRequest.image_ref == "ghcr.io/demo:v1"
ASSERT mockGrpc.capturedRequest.checksum_sha256 == "abc123"
```

### TS-09-15: PARKING_APP Watch

**Requirement:** 09-REQ-4.4
**Type:** unit
**Description:** watch subcommand streams WatchAdapterStates events.

**Preconditions:**
- Mock gRPC server sends 2 events then closes.

**Input:**
- `parking-app-cli watch`

**Expected:**
- Two events printed to stdout.

**Assertion pseudocode:**
```
output = captureStdout(run("parking-app-cli", "watch"))
ASSERT countLines(output) >= 2
```

### TS-09-16: PARKING_APP List

**Requirement:** 09-REQ-4.5
**Type:** unit
**Description:** list subcommand calls UPDATE_SERVICE ListAdapters.

**Preconditions:**
- Mock gRPC server returns list with 1 adapter.

**Input:**
- `parking-app-cli list`

**Expected:**
- Adapter list printed.

**Assertion pseudocode:**
```
exitCode = run("parking-app-cli", "list")
ASSERT exitCode == 0
ASSERT mockGrpc.listCalled
```

### TS-09-17: PARKING_APP Remove

**Requirement:** 09-REQ-4.6
**Type:** unit
**Description:** remove subcommand calls UPDATE_SERVICE RemoveAdapter.

**Preconditions:**
- Mock gRPC server.

**Input:**
- `parking-app-cli remove --adapter-id=adapter-001`

**Expected:**
- RemoveAdapter called with adapter_id.

**Assertion pseudocode:**
```
exitCode = run("parking-app-cli", "remove", "--adapter-id=adapter-001")
ASSERT exitCode == 0
ASSERT mockGrpc.removeCalled
```

### TS-09-18: PARKING_APP Status

**Requirement:** 09-REQ-4.7
**Type:** unit
**Description:** status subcommand calls UPDATE_SERVICE GetAdapterStatus.

**Preconditions:**
- Mock gRPC server.

**Input:**
- `parking-app-cli status --adapter-id=adapter-001`

**Expected:**
- GetAdapterStatus called with adapter_id.

**Assertion pseudocode:**
```
exitCode = run("parking-app-cli", "status", "--adapter-id=adapter-001")
ASSERT exitCode == 0
ASSERT mockGrpc.getStatusCalled
```

### TS-09-19: PARKING_APP Start Session

**Requirement:** 09-REQ-4.8
**Type:** unit
**Description:** start-session subcommand calls PARKING_OPERATOR_ADAPTOR StartSession.

**Preconditions:**
- Mock gRPC server simulating PARKING_OPERATOR_ADAPTOR.

**Input:**
- `parking-app-cli start-session --zone-id=zone-demo-1`

**Expected:**
- StartSession called with zone_id.

**Assertion pseudocode:**
```
exitCode = run("parking-app-cli", "start-session", "--zone-id=zone-demo-1")
ASSERT exitCode == 0
ASSERT mockGrpc.startSessionCalled
ASSERT mockGrpc.capturedRequest.zone_id == "zone-demo-1"
```

### TS-09-20: PARKING_APP Stop Session

**Requirement:** 09-REQ-4.9
**Type:** unit
**Description:** stop-session subcommand calls PARKING_OPERATOR_ADAPTOR StopSession.

**Preconditions:**
- Mock gRPC server.

**Input:**
- `parking-app-cli stop-session`

**Expected:**
- StopSession called (no arguments).

**Assertion pseudocode:**
```
exitCode = run("parking-app-cli", "stop-session")
ASSERT exitCode == 0
ASSERT mockGrpc.stopSessionCalled
```

### TS-09-21: Sensor Config Default

**Requirement:** 09-REQ-5.1
**Type:** unit
**Description:** Sensors default to DATA_BROKER_ADDR=http://localhost:55556.

**Preconditions:**
- No DATA_BROKER_ADDR env var set.

**Input:**
- Parse config.

**Expected:**
- Default address used.

**Assertion pseudocode:**
```
clear_env("DATA_BROKER_ADDR")
addr = get_broker_addr()
ASSERT addr == "http://localhost:55556"
```

### TS-09-22: COMPANION_APP Config Default

**Requirement:** 09-REQ-5.2
**Type:** unit
**Description:** companion-app-cli defaults to CLOUD_GATEWAY_URL=http://localhost:8081.

**Preconditions:**
- No env vars set.

**Input:**
- Parse config.

**Expected:**
- Default gateway URL used.

**Assertion pseudocode:**
```
clear_env("CLOUD_GATEWAY_URL")
url = getGatewayURL()
ASSERT url == "http://localhost:8081"
```

### TS-09-23: PARKING_APP Config Defaults

**Requirement:** 09-REQ-5.3
**Type:** unit
**Description:** parking-app-cli uses correct default service addresses.

**Preconditions:**
- No env vars set.

**Input:**
- Parse config.

**Expected:**
- PARKING_FEE_SERVICE_URL=http://localhost:8080, UPDATE_SERVICE_ADDR=localhost:50052, ADAPTOR_ADDR=localhost:50053.

**Assertion pseudocode:**
```
cfg = loadConfig()
ASSERT cfg.feeServiceURL == "http://localhost:8080"
ASSERT cfg.updateServiceAddr == "localhost:50052"
ASSERT cfg.adaptorAddr == "localhost:50053"
```

### TS-09-24: PARKING_OPERATOR Config Default

**Requirement:** 09-REQ-5.4
**Type:** unit
**Description:** parking-operator defaults to port 8080.

**Preconditions:**
- No PORT env var set.

**Input:**
- Parse config.

**Expected:**
- Default port 8080.

**Assertion pseudocode:**
```
clear_env("PORT")
port = getPort()
ASSERT port == 8080
```

### TS-09-25: Help Flag

**Requirement:** 09-REQ-6.1
**Type:** unit
**Description:** --help prints usage and exits 0.

**Preconditions:**
- Any mock tool binary.

**Input:**
- `<tool> --help`

**Expected:**
- Exit code 0.
- Non-empty stdout or stderr containing usage text.

**Assertion pseudocode:**
```
FOR tool IN ["parking-app-cli", "companion-app-cli", "parking-operator"]:
    exitCode, output = run(tool, "--help")
    ASSERT exitCode == 0
    ASSERT "usage" IN lower(output) OR len(output) > 0
```

### TS-09-26: Connection Error Message

**Requirement:** 09-REQ-6.2
**Type:** unit
**Description:** Connection errors include target address in message.

**Preconditions:**
- No service running on target address.

**Input:**
- Run tool against unreachable address.

**Expected:**
- Exit code 1. Error message on stderr.

**Assertion pseudocode:**
```
exitCode, stderr = run("companion-app-cli", "lock", "--vin=VIN001", "--token=t", "--gateway-url=http://localhost:19999")
ASSERT exitCode == 1
ASSERT "19999" IN stderr OR "connection" IN lower(stderr)
```

### TS-09-27: Upstream Error Response

**Requirement:** 09-REQ-6.3
**Type:** unit
**Description:** Error responses from upstream are printed to stderr.

**Preconditions:**
- Mock server returning HTTP 403.

**Input:**
- Run tool against mock returning error.

**Expected:**
- Exit code 1. Error details on stderr.

**Assertion pseudocode:**
```
mockServer = httptest.NewServer(return403Handler)
exitCode, stderr = run("companion-app-cli", "lock", "--vin=VIN001", "--token=t", "--gateway-url=" + mockServer.URL)
ASSERT exitCode == 1
ASSERT len(stderr) > 0
```

## Edge Case Tests

### TS-09-E1: Sensor No Arguments

**Requirement:** 09-REQ-1.E1
**Type:** unit
**Description:** Sensor with no arguments prints usage and exits 1.

**Preconditions:**
- Binary built.

**Input:**
- `location-sensor` (no args)

**Expected:**
- Exit code 1.
- Usage message on stderr.

**Assertion pseudocode:**
```
exitCode, stderr = run("location-sensor")
ASSERT exitCode == 1
ASSERT len(stderr) > 0
```

### TS-09-E2: Sensor DATA_BROKER Unreachable

**Requirement:** 09-REQ-1.E2
**Type:** unit
**Description:** Sensor exits 1 when DATA_BROKER is unreachable.

**Preconditions:**
- No DATA_BROKER running.

**Input:**
- `speed-sensor --speed=10.0` with DATA_BROKER_ADDR=http://localhost:19999

**Expected:**
- Exit code 1.
- Error on stderr.

**Assertion pseudocode:**
```
set_env("DATA_BROKER_ADDR", "http://localhost:19999")
exitCode, stderr = run("speed-sensor", "--speed=10.0")
ASSERT exitCode == 1
ASSERT len(stderr) > 0
```

### TS-09-E3: PARKING_OPERATOR Stop Unknown Session

**Requirement:** 09-REQ-2.E1
**Type:** unit
**Description:** Stop with unknown session_id returns 404.

**Preconditions:**
- Empty store.

**Input:**
- POST /parking/stop with `{"session_id": "nonexistent", "timestamp": 1700000000}`

**Expected:**
- HTTP 404.

**Assertion pseudocode:**
```
resp = httptest.POST("/parking/stop", {"session_id": "nonexistent", "timestamp": 1700000000})
ASSERT resp.statusCode == 404
```

### TS-09-E4: PARKING_OPERATOR Status Unknown Session

**Requirement:** 09-REQ-2.E2
**Type:** unit
**Description:** Status with unknown session_id returns 404.

**Preconditions:**
- Empty store.

**Input:**
- GET /parking/status/nonexistent

**Expected:**
- HTTP 404.

**Assertion pseudocode:**
```
resp = httptest.GET("/parking/status/nonexistent")
ASSERT resp.statusCode == 404
```

### TS-09-E5: PARKING_OPERATOR Invalid JSON

**Requirement:** 09-REQ-2.E3
**Type:** unit
**Description:** Invalid JSON body returns 400.

**Preconditions:**
- Handler initialized.

**Input:**
- POST /parking/start with body "not json"

**Expected:**
- HTTP 400.

**Assertion pseudocode:**
```
resp = httptest.POST("/parking/start", "not json")
ASSERT resp.statusCode == 400
```

### TS-09-E6: COMPANION_APP Missing Token

**Requirement:** 09-REQ-3.E1
**Type:** unit
**Description:** Missing bearer token exits 1.

**Preconditions:**
- No CLOUD_GATEWAY_TOKEN env var, no --token flag.

**Input:**
- `companion-app-cli lock --vin=VIN001`

**Expected:**
- Exit code 1.
- Error on stderr mentioning token.

**Assertion pseudocode:**
```
clear_env("CLOUD_GATEWAY_TOKEN")
exitCode, stderr = run("companion-app-cli", "lock", "--vin=VIN001")
ASSERT exitCode == 1
ASSERT "token" IN lower(stderr)
```

### TS-09-E7: COMPANION_APP Gateway Unreachable

**Requirement:** 09-REQ-3.E2
**Type:** unit
**Description:** Unreachable CLOUD_GATEWAY exits 1.

**Preconditions:**
- No CLOUD_GATEWAY running.

**Input:**
- `companion-app-cli lock --vin=VIN001 --token=t --gateway-url=http://localhost:19999`

**Expected:**
- Exit code 1.

**Assertion pseudocode:**
```
exitCode = run("companion-app-cli", "lock", "--vin=VIN001", "--token=t", "--gateway-url=http://localhost:19999")
ASSERT exitCode == 1
```

### TS-09-E8: PARKING_APP Unknown Subcommand

**Requirement:** 09-REQ-4.E1
**Type:** unit
**Description:** Unknown subcommand prints usage and exits 1.

**Preconditions:**
- Binary built.

**Input:**
- `parking-app-cli foobar`

**Expected:**
- Exit code 1.
- Usage on stderr.

**Assertion pseudocode:**
```
exitCode, stderr = run("parking-app-cli", "foobar")
ASSERT exitCode == 1
ASSERT len(stderr) > 0
```

### TS-09-E9: PARKING_APP Missing Required Flag

**Requirement:** 09-REQ-4.E2
**Type:** unit
**Description:** Missing required flag exits 1.

**Preconditions:**
- Binary built.

**Input:**
- `parking-app-cli install` (no --image-ref or --checksum)

**Expected:**
- Exit code 1.
- Error on stderr.

**Assertion pseudocode:**
```
exitCode, stderr = run("parking-app-cli", "install")
ASSERT exitCode == 1
ASSERT len(stderr) > 0
```

### TS-09-E10: PARKING_APP Upstream Unreachable

**Requirement:** 09-REQ-4.E3
**Type:** unit
**Description:** Unreachable upstream service exits 1.

**Preconditions:**
- No service running.

**Input:**
- `parking-app-cli lookup --lat=48.0 --lon=11.0` with PARKING_FEE_SERVICE_URL=http://localhost:19999

**Expected:**
- Exit code 1.

**Assertion pseudocode:**
```
set_env("PARKING_FEE_SERVICE_URL", "http://localhost:19999")
exitCode = run("parking-app-cli", "lookup", "--lat=48.0", "--lon=11.0")
ASSERT exitCode == 1
```

## Property Test Cases

### TS-09-P1: Sensor Signal Type Correctness

**Property:** Property 1 from design.md
**Validates:** 09-REQ-1.1, 09-REQ-1.2, 09-REQ-1.3
**Type:** property
**Description:** For any valid sensor arguments, the correct VSS signal path and data type is used.

**For any:** Random valid lat/lon (doubles), speed (float), door state (bool).
**Invariant:** Signal path and data type match VSS specification.

**Assertion pseudocode:**
```
FOR ANY lat IN random_doubles(-90, 90), lon IN random_doubles(-180, 180):
    ASSERT location_sensor_signal_path(lat) == "Vehicle.CurrentLocation.Latitude"
    ASSERT location_sensor_signal_path(lon) == "Vehicle.CurrentLocation.Longitude"
FOR ANY speed IN random_floats(0, 300):
    ASSERT speed_sensor_signal_path() == "Vehicle.Speed"
FOR ANY open IN [true, false]:
    ASSERT door_sensor_signal_path() == "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen"
```

### TS-09-P2: PARKING_OPERATOR Session Lifecycle

**Property:** Property 2 from design.md
**Validates:** 09-REQ-2.2, 09-REQ-2.3, 09-REQ-2.4
**Type:** property
**Description:** For any start/stop sequence, duration and total_amount are correctly computed.

**For any:** Random start_time, stop_time (stop > start), rate amount.
**Invariant:** duration_seconds == stop_time - start_time, total_amount == rate * duration_hours.

**Assertion pseudocode:**
```
FOR ANY start_ts IN random_timestamps, duration IN random_ints(1, 86400):
    stop_ts = start_ts + duration
    store.start(vehicle_id, zone_id, start_ts)
    result = store.stop(session_id, stop_ts)
    ASSERT result.duration_seconds == duration
    expected_amount = 2.50 * (duration / 3600.0)
    ASSERT abs(result.total_amount - expected_amount) < 0.01
```

### TS-09-P3: CLI Subcommand Dispatch

**Property:** Property 3 from design.md
**Validates:** 09-REQ-4.1 through 09-REQ-4.9, 09-REQ-4.E1
**Type:** property
**Description:** Valid subcommands dispatch correctly; unknown subcommands exit 1.

**For any:** Random subcommand strings.
**Invariant:** Known subcommands dispatch to correct handler. Unknown strings produce exit code 1.

**Assertion pseudocode:**
```
known = ["lookup", "adapter-info", "install", "watch", "list", "remove", "status", "start-session", "stop-session"]
FOR ANY subcmd IN random_strings:
    IF subcmd IN known:
        ASSERT dispatches_to_correct_handler(subcmd)
    ELSE:
        ASSERT exit_code("parking-app-cli", subcmd) == 1
```

### TS-09-P4: Configuration Defaults

**Property:** Property 4 from design.md
**Validates:** 09-REQ-5.1, 09-REQ-5.2, 09-REQ-5.3, 09-REQ-5.4
**Type:** property
**Description:** Missing env vars use defaults; set env vars override defaults.

**For any:** Random subsets of env vars being set.
**Invariant:** Unset vars use defaults, set vars use provided values.

**Assertion pseudocode:**
```
defaults = {
    "DATA_BROKER_ADDR": "http://localhost:55556",
    "CLOUD_GATEWAY_URL": "http://localhost:8081",
    "PARKING_FEE_SERVICE_URL": "http://localhost:8080",
    "UPDATE_SERVICE_ADDR": "localhost:50052",
    "ADAPTOR_ADDR": "localhost:50053",
    "PORT": "8080"
}
FOR ANY subset IN random_subsets(defaults.keys()):
    clear_all_env()
    FOR var IN subset:
        set_env(var, "custom-" + var)
    cfg = loadConfig()
    FOR var IN defaults:
        IF var IN subset:
            ASSERT cfg[var] == "custom-" + var
        ELSE:
            ASSERT cfg[var] == defaults[var]
```

### TS-09-P5: Error Exit Code Consistency

**Property:** Property 5 from design.md
**Validates:** 09-REQ-1.E1, 09-REQ-1.E2, 09-REQ-3.E1, 09-REQ-3.E2, 09-REQ-4.E1, 09-REQ-4.E2, 09-REQ-4.E3, 09-REQ-6.2, 09-REQ-6.3
**Type:** property
**Description:** All error conditions produce exit code 1 with stderr output.

**For any:** Random error scenarios (missing args, unreachable services, missing tokens).
**Invariant:** Exit code is 1 and stderr is non-empty.

**Assertion pseudocode:**
```
error_scenarios = [
    ("location-sensor",),  // no args
    ("companion-app-cli", "lock", "--vin=X"),  // no token
    ("parking-app-cli", "foobar"),  // unknown subcommand
    ("parking-app-cli", "install"),  // missing flags
]
FOR scenario IN error_scenarios:
    exitCode, stderr = run(scenario...)
    ASSERT exitCode == 1
    ASSERT len(stderr) > 0
```

## Coverage Matrix

| Requirement | Test Spec Entry | Type |
|-------------|-----------------|------|
| 09-REQ-1.1 | TS-09-1 | integration |
| 09-REQ-1.2 | TS-09-2 | integration |
| 09-REQ-1.3 | TS-09-3 | integration |
| 09-REQ-1.E1 | TS-09-E1 | unit |
| 09-REQ-1.E2 | TS-09-E2 | unit |
| 09-REQ-2.1 | TS-09-4 | integration |
| 09-REQ-2.2 | TS-09-5 | unit |
| 09-REQ-2.3 | TS-09-6 | unit |
| 09-REQ-2.4 | TS-09-7 | unit |
| 09-REQ-2.5 | TS-09-8 | integration |
| 09-REQ-2.E1 | TS-09-E3 | unit |
| 09-REQ-2.E2 | TS-09-E4 | unit |
| 09-REQ-2.E3 | TS-09-E5 | unit |
| 09-REQ-3.1 | TS-09-9 | unit |
| 09-REQ-3.2 | TS-09-10 | unit |
| 09-REQ-3.3 | TS-09-11 | unit |
| 09-REQ-3.E1 | TS-09-E6 | unit |
| 09-REQ-3.E2 | TS-09-E7 | unit |
| 09-REQ-4.1 | TS-09-12 | unit |
| 09-REQ-4.2 | TS-09-13 | unit |
| 09-REQ-4.3 | TS-09-14 | unit |
| 09-REQ-4.4 | TS-09-15 | unit |
| 09-REQ-4.5 | TS-09-16 | unit |
| 09-REQ-4.6 | TS-09-17 | unit |
| 09-REQ-4.7 | TS-09-18 | unit |
| 09-REQ-4.8 | TS-09-19 | unit |
| 09-REQ-4.9 | TS-09-20 | unit |
| 09-REQ-4.E1 | TS-09-E8 | unit |
| 09-REQ-4.E2 | TS-09-E9 | unit |
| 09-REQ-4.E3 | TS-09-E10 | unit |
| 09-REQ-5.1 | TS-09-21 | unit |
| 09-REQ-5.2 | TS-09-22 | unit |
| 09-REQ-5.3 | TS-09-23 | unit |
| 09-REQ-5.4 | TS-09-24 | unit |
| 09-REQ-6.1 | TS-09-25 | unit |
| 09-REQ-6.2 | TS-09-26 | unit |
| 09-REQ-6.3 | TS-09-27 | unit |
| Property 1 | TS-09-P1 | property |
| Property 2 | TS-09-P2 | property |
| Property 3 | TS-09-P3 | property |
| Property 4 | TS-09-P4 | property |
| Property 5 | TS-09-P5 | property |
