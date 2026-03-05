# Test Specification: Mock Apps (Spec 09)

> Test specifications for all mock/demo tools: mock sensors (Rust), mock CLI apps (Go), and mock PARKING_OPERATOR (Go).
> Validates requirements from `.specs/09_mock_apps/requirements.md`.

## Test ID Convention

| Prefix | Category |
|--------|----------|
| TS-09- | Functional tests |
| TS-09-P | Property tests |
| TS-09-E | Error/edge tests |

## Test Environment

- **Go test framework:** Go `testing` standard library
- **Rust test framework:** Rust built-in `#[test]` and `#[tokio::test]`
- **HTTP testing (Go):** `net/http/httptest` for handler-level tests
- **gRPC testing (Go):** Mock gRPC servers using `google.golang.org/grpc/test/bufconn`
- **Test locations:**
  - `mock/parking-app-cli/**/*_test.go`
  - `mock/companion-app-cli/**/*_test.go`
  - `mock/parking-operator/*_test.go`
  - `rhivos/location-sensor/src/main.rs` (inline tests)
  - `rhivos/speed-sensor/src/main.rs` (inline tests)
  - `rhivos/door-sensor/src/main.rs` (inline tests)
- **Run commands:**
  - `cd mock/parking-app-cli && go test ./... -v`
  - `cd mock/companion-app-cli && go test ./... -v`
  - `cd mock/parking-operator && go test ./... -v`
  - `cd rhivos && cargo test -p location-sensor -p speed-sensor -p door-sensor`

---

## Mock Sensor Tests

### TS-09-1: Location sensor writes correct VSS signals

**Requirement:** 09-REQ-1.1

**Description:** Invoking the location-sensor with `--lat` and `--lon` arguments writes the correct values to `Vehicle.CurrentLocation.Latitude` and `Vehicle.CurrentLocation.Longitude` in DATA_BROKER.

**Preconditions:** DATA_BROKER is running and accessible on the configured address.

**Steps:**

1. Start DATA_BROKER (or a mock gRPC server implementing kuksa.val.v1).
2. Run `location-sensor --lat=48.1351 --lon=11.5820 --broker-addr=http://localhost:<port>`.
3. Assert that the tool exits with code 0.
4. Read `Vehicle.CurrentLocation.Latitude` from DATA_BROKER. Assert value is `48.1351`.
5. Read `Vehicle.CurrentLocation.Longitude` from DATA_BROKER. Assert value is `11.5820`.

**Expected result:** Both signals are written with the specified values. Tool exits with code 0.

---

### TS-09-2: Speed sensor writes correct VSS signal

**Requirement:** 09-REQ-2.1

**Description:** Invoking the speed-sensor with `--speed` writes the correct value to `Vehicle.Speed` in DATA_BROKER.

**Preconditions:** DATA_BROKER is running and accessible.

**Steps:**

1. Start DATA_BROKER (or mock gRPC server).
2. Run `speed-sensor --speed=50.5 --broker-addr=http://localhost:<port>`.
3. Assert that the tool exits with code 0.
4. Read `Vehicle.Speed` from DATA_BROKER. Assert value is `50.5`.

**Expected result:** Signal is written with the specified value. Tool exits with code 0.

---

### TS-09-3: Door sensor writes correct VSS signal (open)

**Requirement:** 09-REQ-3.1

**Description:** Invoking the door-sensor with `--open` writes `true` to `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen`.

**Preconditions:** DATA_BROKER is running and accessible.

**Steps:**

1. Start DATA_BROKER (or mock gRPC server).
2. Run `door-sensor --open --broker-addr=http://localhost:<port>`.
3. Assert that the tool exits with code 0.
4. Read `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` from DATA_BROKER. Assert value is `true`.

**Expected result:** Signal is written as `true`. Tool exits with code 0.

---

### TS-09-4: Door sensor writes correct VSS signal (closed)

**Requirement:** 09-REQ-3.1

**Description:** Invoking the door-sensor with `--closed` writes `false` to `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen`.

**Preconditions:** DATA_BROKER is running and accessible.

**Steps:**

1. Start DATA_BROKER (or mock gRPC server).
2. Run `door-sensor --closed --broker-addr=http://localhost:<port>`.
3. Assert that the tool exits with code 0.
4. Read `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` from DATA_BROKER. Assert value is `false`.

**Expected result:** Signal is written as `false`. Tool exits with code 0.

---

### TS-09-5: Parking-app-cli dispatches to correct subcommand handler

**Requirement:** 09-REQ-4.1, 09-REQ-4.2, 09-REQ-4.3

**Description:** Each subcommand name routes to the correct handler function.

**Preconditions:** The parking-app-cli binary is built.

**Steps:**

1. Invoke the binary with each known subcommand name: `lookup`, `adapter-info`, `install`, `watch`, `list`, `remove`, `status`, `start-session`, `stop-session`.
2. Verify that each subcommand invokes its corresponding handler (may produce a usage or connection error if flags or services are missing, but must not dispatch to the wrong handler).

**Expected result:** Each subcommand name routes to the correct handler. No cross-dispatch.

---

### TS-09-6: Companion-app-cli dispatches to correct subcommand handler

**Requirement:** 09-REQ-5.1

**Description:** Each subcommand name routes to the correct handler.

**Preconditions:** The companion-app-cli binary is built.

**Steps:**

1. Invoke the binary with each known subcommand name: `lock`, `unlock`, `status`.
2. Verify that each subcommand invokes its corresponding handler.

**Expected result:** Each subcommand name routes to the correct handler.

---

### TS-09-7: Parking-operator handles POST /parking/start

**Requirement:** 09-REQ-6.1

**Description:** Sending a valid start request creates a parking session.

**Preconditions:** parking-operator server is running.

**Steps:**

1. Send `POST /parking/start` with body `{"vehicle_id": "VIN001", "zone_id": "muc-central", "timestamp": 1709640000}`.
2. Assert HTTP status is 200.
3. Assert `Content-Type` is `application/json`.
4. Parse JSON response.
5. Assert `session_id` is a non-empty string.
6. Assert `status` is `"active"`.

**Expected result:** 200 OK with session data.

---

### TS-09-8: Parking-operator handles POST /parking/stop

**Requirement:** 09-REQ-6.2

**Description:** Sending a valid stop request completes the parking session with correct duration and fee.

**Preconditions:** parking-operator server is running. A session was previously started via POST /parking/start.

**Steps:**

1. Start a session via `POST /parking/start`.
2. Record the returned `session_id`.
3. Wait briefly (or use a known time offset).
4. Send `POST /parking/stop` with body `{"session_id": "<session_id>"}`.
5. Assert HTTP status is 200.
6. Parse JSON response.
7. Assert `session_id` matches the started session.
8. Assert `duration_seconds` is a non-negative integer.
9. Assert `fee` is a non-negative number.
10. Assert `status` is `"completed"`.

**Expected result:** 200 OK with completed session data including duration and fee.

---

### TS-09-9: Parking-operator handles GET /parking/status

**Requirement:** 09-REQ-6.3

**Description:** The status endpoint returns all sessions.

**Preconditions:** parking-operator server is running. One or more sessions exist.

**Steps:**

1. Start a session via `POST /parking/start`.
2. Send `GET /parking/status`.
3. Assert HTTP status is 200.
4. Parse JSON response as an array.
5. Assert the array contains at least one entry with the started session's `session_id`.

**Expected result:** 200 OK with JSON array containing session data.

---

## Positive / Happy-Path Tests

### TS-09-P1: lookup subcommand calls correct REST endpoint

**Requirement:** 09-REQ-4.1

**Preconditions:** An HTTP test server records incoming requests and returns `[{"operator_id": "op1"}]`.

**Steps:**

1. Set `PARKING_FEE_SERVICE_URL` to the test server's address.
2. Run `parking-app-cli lookup --lat=48.1351 --lon=11.5820`.
3. Inspect the recorded request.

**Expected result:** Test server receives GET `/operators?lat=48.1351&lon=11.5820`. CLI prints JSON to stdout. Exit code 0.

---

### TS-09-P2: adapter-info subcommand calls correct REST endpoint

**Requirement:** 09-REQ-4.2

**Preconditions:** An HTTP test server returns `{"image_ref": "registry/adapter:v1", "checksum_sha256": "abc123", "version": "1.0"}`.

**Steps:**

1. Set `PARKING_FEE_SERVICE_URL` to the test server's address.
2. Run `parking-app-cli adapter-info --operator-id=op1`.

**Expected result:** Test server receives GET `/operators/op1/adapter`. CLI prints JSON to stdout. Exit code 0.

---

### TS-09-P3: install subcommand calls correct gRPC method

**Requirement:** 09-REQ-4.3

**Preconditions:** A mock gRPC server implementing `UpdateService.InstallAdapter` is started.

**Steps:**

1. Set `UPDATE_SERVICE_ADDR` to the mock server's address.
2. Run `parking-app-cli install --image-ref=registry/adapter:v1 --checksum=abc123def456`.

**Expected result:** Mock server receives `InstallAdapter` with correct fields. CLI prints response as JSON. Exit code 0.

---

### TS-09-P4: lock subcommand sends correct command payload

**Requirement:** 09-REQ-5.1

**Preconditions:** An HTTP test server records incoming requests.

**Steps:**

1. Set `CLOUD_GATEWAY_URL` to the test server's address.
2. Set `BEARER_TOKEN` to `test-token-123`.
3. Run `companion-app-cli lock --vin=VIN12345`.
4. Inspect the recorded request.

**Expected result:**

- POST to `/vehicles/VIN12345/commands`.
- Body contains `{"command_id": "<uuid>", "type": "lock", "doors": ["driver"]}`.
- `Authorization` header is `Bearer test-token-123`.
- `Content-Type` is `application/json`.
- Exit code 0.

---

### TS-09-P5: unlock subcommand sends correct command payload

**Requirement:** 09-REQ-5.1

**Preconditions:** Same as TS-09-P4.

**Steps:**

1. Run `companion-app-cli unlock --vin=VIN12345`.

**Expected result:** POST body contains `"type": "unlock"`. Other assertions same as TS-09-P4.

---

### TS-09-P6: status subcommand (companion-app-cli) calls correct endpoint

**Requirement:** 09-REQ-5.1

**Preconditions:** An HTTP test server returns `{"vin": "VIN12345", "locked": true}`.

**Steps:**

1. Run `companion-app-cli status --vin=VIN12345`.

**Expected result:** Test server receives GET `/vehicles/VIN12345/status` with bearer token header. Exit code 0.

---

## Error and Edge Case Tests

### TS-09-E1: Sensor missing required arguments

**Requirement:** 09-REQ-1.E1, 09-REQ-2.E1, 09-REQ-3.E1

**Description:** Sensors print usage errors when required arguments are missing.

**Test cases (table-driven):**

| Tool | Invocation | Missing |
|------|------------|---------|
| location-sensor | `location-sensor` | --lat and --lon |
| location-sensor | `location-sensor --lat=48.0` | --lon |
| speed-sensor | `speed-sensor` | --speed |
| door-sensor | `door-sensor` | --open or --closed |

**Expected result:** Each invocation prints a usage error to stderr and exits with a non-zero exit code.

---

### TS-09-E2: Sensor DATA_BROKER unreachable

**Requirement:** 09-REQ-1.E2, 09-REQ-2.E2, 09-REQ-3.E2

**Description:** Sensors print error when DATA_BROKER is unreachable.

**Steps:**

1. Run `location-sensor --lat=48.0 --lon=11.0 --broker-addr=http://localhost:19999`.
2. Capture stderr and exit code.

**Expected result:** Error message includes the target address. Non-zero exit code.

---

### TS-09-E3: CLI missing required flags

**Requirement:** 09-REQ-4.E1, 09-REQ-5.E1

**Description:** Mock CLIs print usage errors when required flags are missing.

**Test cases:**

| CLI | Subcommand | Missing Flag |
|-----|-----------|--------------|
| parking-app-cli | lookup | --lat, --lon |
| parking-app-cli | adapter-info | --operator-id |
| parking-app-cli | install | --image-ref, --checksum |
| parking-app-cli | remove | --adapter-id |
| parking-app-cli | status | --adapter-id |
| parking-app-cli | start-session | --zone-id |
| parking-app-cli | stop-session | --session-id |
| companion-app-cli | lock | --vin |
| companion-app-cli | unlock | --vin |
| companion-app-cli | status | --vin |

**Expected result:** Each invocation prints a usage error to stderr and exits with a non-zero exit code.

---

### TS-09-E4: Unknown subcommand produces usage error

**Requirement:** 09-REQ-4.E1

**Description:** Invoking either CLI with an unknown subcommand produces a usage listing.

**Steps:**

1. Run `parking-app-cli foobar`.
2. Run `companion-app-cli foobar`.

**Expected result:** Each prints available subcommands to stderr and exits with a non-zero exit code.

---

### TS-09-E5: Service unreachable produces meaningful error (REST)

**Requirement:** 09-REQ-4.E2, 09-REQ-5.E2

**Steps:**

1. Set `PARKING_FEE_SERVICE_URL` to `http://localhost:19999`.
2. Run `parking-app-cli lookup --lat=48.0 --lon=11.0`.

**Expected result:** Error message on stderr includes the target URL and a connection error. Non-zero exit code.

---

### TS-09-E6: Service unreachable produces meaningful error (gRPC)

**Requirement:** 09-REQ-4.E2

**Steps:**

1. Set `UPDATE_SERVICE_ADDR` to `localhost:19998`.
2. Run `parking-app-cli install --image-ref=test --checksum=abc`.

**Expected result:** Error message on stderr includes the target address and a gRPC connection error. Non-zero exit code.

---

### TS-09-E7: Parking-operator returns 400 for malformed body

**Requirement:** 09-REQ-6.E1

**Steps:**

1. Send `POST /parking/start` with body `{invalid json`.
2. Assert HTTP status is 400.
3. Assert response body contains `error` field.

**Expected result:** 400 with JSON error body.

---

### TS-09-E8: Parking-operator returns 404 for unknown session

**Requirement:** 09-REQ-6.E2

**Steps:**

1. Send `POST /parking/stop` with body `{"session_id": "nonexistent-session"}`.
2. Assert HTTP status is 404.
3. Assert response body contains `error` field.

**Expected result:** 404 with JSON error body.

---

### TS-09-E9: Parking-operator returns empty array when no sessions exist

**Requirement:** 09-REQ-6.3

**Steps:**

1. Start a fresh parking-operator instance.
2. Send `GET /parking/status`.
3. Assert HTTP status is 200.
4. Assert response body is `[]`.

**Expected result:** 200 OK with empty JSON array.

---

## Property Tests

### TS-09-P7: Sensor on-demand execution

**Requirement:** 09-REQ-7.1

**Description:** Each sensor tool exits after writing a single value. It does not continue running or publish additional values.

**Properties tested:**

1. After invocation, the process exits within 5 seconds.
2. DATA_BROKER receives exactly one write per invocation per signal path.

**Expected result:** All sensors are single-shot tools.

---

### TS-09-P8: Parking-operator session store consistency

**Requirement:** 09-REQ-6.1, 09-REQ-6.2, 09-REQ-6.3

**Description:** Starting and stopping sessions maintains consistent state.

**Properties tested:**

1. A started session appears in GET /parking/status with status "active".
2. A stopped session appears in GET /parking/status with status "completed".
3. Stopping the same session twice returns 404 on the second attempt (session already completed).
4. Duration and fee are non-negative.

**Expected result:** Session state transitions are consistent.

---

## Traceability

| Test ID | Requirement(s) | Category |
|---------|----------------|----------|
| TS-09-1 | 09-REQ-1.1 | Functional |
| TS-09-2 | 09-REQ-2.1 | Functional |
| TS-09-3 | 09-REQ-3.1 | Functional |
| TS-09-4 | 09-REQ-3.1 | Functional |
| TS-09-5 | 09-REQ-4.1, 09-REQ-4.2, 09-REQ-4.3 | Functional |
| TS-09-6 | 09-REQ-5.1 | Functional |
| TS-09-7 | 09-REQ-6.1 | Functional |
| TS-09-8 | 09-REQ-6.2 | Functional |
| TS-09-9 | 09-REQ-6.3 | Functional |
| TS-09-P1 | 09-REQ-4.1 | Property |
| TS-09-P2 | 09-REQ-4.2 | Property |
| TS-09-P3 | 09-REQ-4.3 | Property |
| TS-09-P4 | 09-REQ-5.1 | Property |
| TS-09-P5 | 09-REQ-5.1 | Property |
| TS-09-P6 | 09-REQ-5.1 | Property |
| TS-09-P7 | 09-REQ-7.1 | Property |
| TS-09-P8 | 09-REQ-6.1, 09-REQ-6.2, 09-REQ-6.3 | Property |
| TS-09-E1 | 09-REQ-1.E1, 09-REQ-2.E1, 09-REQ-3.E1 | Error/Edge |
| TS-09-E2 | 09-REQ-1.E2, 09-REQ-2.E2, 09-REQ-3.E2 | Error/Edge |
| TS-09-E3 | 09-REQ-4.E1, 09-REQ-5.E1 | Error/Edge |
| TS-09-E4 | 09-REQ-4.E1 | Error/Edge |
| TS-09-E5 | 09-REQ-4.E2, 09-REQ-5.E2 | Error/Edge |
| TS-09-E6 | 09-REQ-4.E2 | Error/Edge |
| TS-09-E7 | 09-REQ-6.E1 | Error/Edge |
| TS-09-E8 | 09-REQ-6.E2 | Error/Edge |
| TS-09-E9 | 09-REQ-6.3 | Error/Edge |
