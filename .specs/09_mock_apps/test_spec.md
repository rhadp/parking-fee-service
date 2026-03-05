# Test Specification: Mock CLI Apps (Spec 09)

> Test specifications for the mock PARKING_APP CLI and mock COMPANION_APP CLI.
> Validates requirements from `.specs/09_mock_apps/requirements.md`.

## Test Environment

- **Language:** Go 1.22+
- **Framework:** Go standard `testing` package
- **Location:** Tests are co-located with source code in `mock/parking-app-cli/` and `mock/companion-app-cli/`
- **Run commands:**
  - `cd mock/parking-app-cli && go test ./... -v`
  - `cd mock/companion-app-cli && go test ./... -v`

## Test Categories

- **TS-09-n:** Subcommand dispatch and argument parsing tests
- **TS-09-Pn:** Positive / happy-path tests (correct requests and responses)
- **TS-09-En:** Error handling and edge case tests

---

## Subcommand Dispatch and Argument Parsing

### TS-09-1: Parking-app-cli dispatches to correct subcommand handler

**Requirement:** 09-REQ-1.1, 09-REQ-1.2, 09-REQ-1.3

**Preconditions:** The `parking-app-cli` binary is built.

**Steps:**
1. Invoke the binary with each known subcommand name: `lookup`, `adapter-info`, `install`, `watch`, `list`, `remove`, `status`, `start-session`, `stop-session`.
2. Verify that each subcommand invokes its corresponding handler (may produce a usage or connection error if required flags or services are missing, but must not dispatch to the wrong handler).

**Expected result:** Each subcommand name routes to the correct handler function. No cross-dispatch occurs.

---

### TS-09-2: Companion-app-cli dispatches to correct subcommand handler

**Requirement:** 09-REQ-2.1

**Preconditions:** The `companion-app-cli` binary is built.

**Steps:**
1. Invoke the binary with each known subcommand name: `lock`, `unlock`, `status`.
2. Verify that each subcommand invokes its corresponding handler.

**Expected result:** Each subcommand name routes to the correct handler function.

---

### TS-09-3: Unknown subcommand produces usage error

**Requirement:** 09-REQ-4.1

**Preconditions:** Either CLI binary is built.

**Steps:**
1. Invoke the binary with an unknown subcommand name (e.g., `foobar`).
2. Capture stderr and exit code.

**Expected result:** The CLI prints a usage message listing available subcommands to stderr and exits with a non-zero exit code.

---

### TS-09-4: No arguments produces usage message

**Requirement:** 09-REQ-4.1

**Preconditions:** Either CLI binary is built.

**Steps:**
1. Invoke the binary with no arguments.
2. Capture stderr and exit code.

**Expected result:** The CLI prints a usage message listing available subcommands to stderr and exits with a non-zero exit code.

---

### TS-09-5: Missing required flags produce usage error

**Requirement:** 09-REQ-1.1, 09-REQ-1.2, 09-REQ-1.3, 09-REQ-2.1

**Preconditions:** Either CLI binary is built.

**Steps:**
1. Invoke `parking-app-cli lookup` without `--lat` and `--lon`.
2. Invoke `parking-app-cli adapter-info` without `--operator-id`.
3. Invoke `parking-app-cli install` without `--image-ref` and `--checksum`.
4. Invoke `parking-app-cli remove` without `--adapter-id`.
5. Invoke `parking-app-cli status` without `--adapter-id`.
6. Invoke `parking-app-cli start-session` without `--zone-id`.
7. Invoke `parking-app-cli stop-session` without `--session-id`.
8. Invoke `companion-app-cli lock` without `--vin`.
9. Invoke `companion-app-cli unlock` without `--vin`.
10. Invoke `companion-app-cli status` without `--vin`.

**Expected result:** Each invocation prints a usage error to stderr mentioning the missing flag(s) and exits with a non-zero exit code.

---

## Positive / Happy-Path Tests

### TS-09-P1: lookup subcommand calls correct REST endpoint

**Requirement:** 09-REQ-1.1

**Preconditions:** An HTTP test server is started that records incoming requests and returns a mock JSON response `[{"operator_id": "op1", "name": "TestOperator"}]`.

**Steps:**
1. Set `PARKING_FEE_SERVICE_URL` to the test server's address.
2. Run `parking-app-cli lookup --lat=48.1351 --lon=11.5820`.
3. Inspect the recorded request on the test server.
4. Capture stdout from the CLI.

**Expected result:**
- The test server receives a GET request to `/operators?lat=48.1351&lon=11.5820`.
- The CLI prints the response JSON (indented) to stdout.
- The CLI exits with code 0.

---

### TS-09-P2: adapter-info subcommand calls correct REST endpoint

**Requirement:** 09-REQ-1.1

**Preconditions:** An HTTP test server is started that records incoming requests and returns `{"image_ref": "registry/adapter:v1", "checksum_sha256": "abc123", "version": "1.0"}`.

**Steps:**
1. Set `PARKING_FEE_SERVICE_URL` to the test server's address.
2. Run `parking-app-cli adapter-info --operator-id=op1`.
3. Inspect the recorded request.

**Expected result:**
- The test server receives a GET request to `/operators/op1/adapter`.
- The CLI prints the response JSON to stdout.
- The CLI exits with code 0.

---

### TS-09-P3: install subcommand calls correct gRPC method

**Requirement:** 09-REQ-1.2

**Preconditions:** A mock gRPC server implementing the `UpdateService.InstallAdapter` RPC is started. It records the request and returns `{job_id: "j1", adapter_id: "a1", state: DOWNLOADING}`.

**Steps:**
1. Set `UPDATE_SERVICE_ADDR` to the mock server's address.
2. Run `parking-app-cli install --image-ref=registry/adapter:v1 --checksum=abc123def456`.
3. Inspect the recorded gRPC request.

**Expected result:**
- The mock server receives an `InstallAdapter` call with `image_ref = "registry/adapter:v1"` and `checksum_sha256 = "abc123def456"`.
- The CLI prints the response fields (job_id, adapter_id, state) as JSON to stdout.
- The CLI exits with code 0.

---

### TS-09-P4: watch subcommand handles streaming responses

**Requirement:** 09-REQ-1.2

**Preconditions:** A mock gRPC server implementing `UpdateService.WatchAdapterStates` is started. It sends 3 `AdapterStateEvent` messages then closes the stream.

**Steps:**
1. Set `UPDATE_SERVICE_ADDR` to the mock server's address.
2. Run `parking-app-cli watch`.
3. Capture stdout.

**Expected result:**
- The CLI prints each event as a separate JSON line to stdout (3 events total).
- After the stream closes, the CLI exits with code 0.

---

### TS-09-P5: list subcommand calls ListAdapters

**Requirement:** 09-REQ-1.2

**Preconditions:** A mock gRPC server implementing `UpdateService.ListAdapters` is started. It returns a list of 2 adapters.

**Steps:**
1. Set `UPDATE_SERVICE_ADDR` to the mock server's address.
2. Run `parking-app-cli list`.

**Expected result:**
- The mock server receives a `ListAdapters` call.
- The CLI prints the adapter list as JSON to stdout.
- The CLI exits with code 0.

---

### TS-09-P6: lock subcommand sends correct command payload

**Requirement:** 09-REQ-2.1

**Preconditions:** An HTTP test server is started that records incoming requests and returns `{"command_id": "<uuid>", "status": "success"}`.

**Steps:**
1. Set `CLOUD_GATEWAY_URL` to the test server's address.
2. Set `BEARER_TOKEN` to `test-token-123`.
3. Run `companion-app-cli lock --vin=VIN12345`.
4. Inspect the recorded request.

**Expected result:**
- The test server receives a POST request to `/vehicles/VIN12345/commands`.
- The request body contains `{"command_id": "<uuid>", "type": "lock", "doors": ["driver"]}`.
- The `command_id` is a valid UUID.
- The `Authorization` header is `Bearer test-token-123`.
- The `Content-Type` header is `application/json`.
- The CLI prints the response JSON to stdout.
- The CLI exits with code 0.

---

### TS-09-P7: unlock subcommand sends correct command payload

**Requirement:** 09-REQ-2.1

**Preconditions:** Same as TS-09-P6.

**Steps:**
1. Set `CLOUD_GATEWAY_URL` to the test server's address.
2. Set `BEARER_TOKEN` to `test-token-123`.
3. Run `companion-app-cli unlock --vin=VIN12345`.

**Expected result:**
- The test server receives a POST request to `/vehicles/VIN12345/commands`.
- The request body contains `{"command_id": "<uuid>", "type": "unlock", "doors": ["driver"]}`.
- The `Authorization` header is `Bearer test-token-123`.
- The CLI exits with code 0.

---

### TS-09-P8: status subcommand (companion-app-cli) calls correct endpoint

**Requirement:** 09-REQ-2.1

**Preconditions:** An HTTP test server returns `{"vin": "VIN12345", "locked": true, "parking_active": false}`.

**Steps:**
1. Set `CLOUD_GATEWAY_URL` to the test server's address.
2. Set `BEARER_TOKEN` to `test-token-123`.
3. Run `companion-app-cli status --vin=VIN12345`.

**Expected result:**
- The test server receives a GET request to `/vehicles/VIN12345/status`.
- The `Authorization` header is `Bearer test-token-123`.
- The CLI prints the response JSON to stdout.
- The CLI exits with code 0.

---

### TS-09-P9: Bearer token is included in REST requests

**Requirement:** 09-REQ-2.1, 09-REQ-3.1

**Preconditions:** An HTTP test server records request headers.

**Steps:**
1. Set `BEARER_TOKEN` to `my-secret-token`.
2. Run any companion-app-cli subcommand (e.g., `status --vin=VIN1`).
3. Inspect the `Authorization` header on the recorded request.

**Expected result:** The request includes the header `Authorization: Bearer my-secret-token`.

---

### TS-09-P10: start-session subcommand calls StartSession gRPC method

**Requirement:** 09-REQ-1.3

**Preconditions:** A mock gRPC server implementing `ParkingAdaptor.StartSession` is started. It returns `{session_id: "s1", status: "active"}`.

**Steps:**
1. Set `PARKING_ADAPTOR_ADDR` to the mock server's address.
2. Run `parking-app-cli start-session --zone-id=zone-munich-01`.

**Expected result:**
- The mock server receives a `StartSession` call with `zone_id = "zone-munich-01"`.
- The CLI prints the response as JSON to stdout.
- The CLI exits with code 0.

---

### TS-09-P11: stop-session subcommand calls StopSession gRPC method

**Requirement:** 09-REQ-1.3

**Preconditions:** A mock gRPC server implementing `ParkingAdaptor.StopSession` is started. It returns `{session_id: "s1", status: "stopped"}`.

**Steps:**
1. Set `PARKING_ADAPTOR_ADDR` to the mock server's address.
2. Run `parking-app-cli stop-session --session-id=s1`.

**Expected result:**
- The mock server receives a `StopSession` call with `session_id = "s1"`.
- The CLI prints the response as JSON to stdout.
- The CLI exits with code 0.

---

## Error Handling Tests

### TS-09-E1: Service unreachable produces meaningful error (REST)

**Requirement:** 09-REQ-4.1

**Preconditions:** No HTTP server is running on the configured URL.

**Steps:**
1. Set `PARKING_FEE_SERVICE_URL` to `http://localhost:19999` (a port with no listener).
2. Run `parking-app-cli lookup --lat=48.1351 --lon=11.5820`.
3. Capture stderr and exit code.

**Expected result:**
- The CLI prints an error message to stderr containing the target URL and a connection error description (e.g., "connection refused").
- The CLI exits with a non-zero exit code.

---

### TS-09-E2: Service unreachable produces meaningful error (gRPC)

**Requirement:** 09-REQ-4.1

**Preconditions:** No gRPC server is running on the configured address.

**Steps:**
1. Set `UPDATE_SERVICE_ADDR` to `localhost:19998` (a port with no listener).
2. Run `parking-app-cli install --image-ref=test --checksum=abc`.
3. Capture stderr and exit code.

**Expected result:**
- The CLI prints an error message to stderr containing the target address and a gRPC connection error description.
- The CLI exits with a non-zero exit code.

---

### TS-09-E3: HTTP non-2xx response displays status and body

**Requirement:** 09-REQ-1.1, 09-REQ-4.1

**Preconditions:** An HTTP test server is configured to return `404 Not Found` with body `{"error": "operator_not_found"}`.

**Steps:**
1. Set `PARKING_FEE_SERVICE_URL` to the test server's address.
2. Run `parking-app-cli adapter-info --operator-id=nonexistent`.
3. Capture stderr and exit code.

**Expected result:**
- The CLI prints "error: HTTP 404: {\"error\": \"operator_not_found\"}" (or similar) to stderr.
- The CLI exits with a non-zero exit code.

---

### TS-09-E4: gRPC NOT_FOUND error displays status code and message

**Requirement:** 09-REQ-1.2, 09-REQ-4.1

**Preconditions:** A mock gRPC server is configured to return `status.Error(codes.NotFound, "adapter not found")` for `GetAdapterStatus`.

**Steps:**
1. Set `UPDATE_SERVICE_ADDR` to the mock server's address.
2. Run `parking-app-cli status --adapter-id=nonexistent`.
3. Capture stderr and exit code.

**Expected result:**
- The CLI prints "error: gRPC NOT_FOUND: adapter not found" (or similar) to stderr.
- The CLI exits with a non-zero exit code.

---

### TS-09-E5: Missing BEARER_TOKEN prints warning

**Requirement:** 09-REQ-2.1, 09-REQ-3.1

**Preconditions:** `BEARER_TOKEN` environment variable is not set. An HTTP test server is running.

**Steps:**
1. Unset `BEARER_TOKEN`.
2. Run `companion-app-cli lock --vin=VIN12345`.
3. Capture stderr.

**Expected result:**
- The CLI prints a warning to stderr indicating that no bearer token is configured.
- The request is still sent (without the `Authorization` header).

---

### TS-09-E6: Configuration flag overrides environment variable

**Requirement:** 09-REQ-3.1

**Preconditions:** Two HTTP test servers are running on different ports. `CLOUD_GATEWAY_URL` is set to test server A.

**Steps:**
1. Set `CLOUD_GATEWAY_URL` to test server A's address.
2. Run `companion-app-cli status --vin=VIN1 --cloud-gateway-url=<test server B address>`.
3. Check which server received the request.

**Expected result:** Test server B receives the request (flag overrides environment variable).

---

### TS-09-E7: HTTP request timeout produces error message

**Requirement:** 09-REQ-4.1

**Preconditions:** An HTTP test server is configured to delay its response by 15 seconds (exceeding the 10-second timeout).

**Steps:**
1. Set `PARKING_FEE_SERVICE_URL` to the test server's address.
2. Run `parking-app-cli lookup --lat=48.0 --lon=11.0`.
3. Capture stderr and exit code.

**Expected result:**
- The CLI prints a timeout error to stderr mentioning the request URL.
- The CLI exits with a non-zero exit code.
- The CLI does not hang beyond approximately 10 seconds.

---

### TS-09-E8: Companion-app-cli service unreachable

**Requirement:** 09-REQ-2.1, 09-REQ-4.1

**Preconditions:** No HTTP server is running on the configured CLOUD_GATEWAY_URL.

**Steps:**
1. Set `CLOUD_GATEWAY_URL` to `http://localhost:19997`.
2. Run `companion-app-cli lock --vin=VIN12345`.
3. Capture stderr and exit code.

**Expected result:**
- The CLI prints an error message to stderr containing the target URL and a connection error.
- The CLI exits with a non-zero exit code.

---

## Traceability

| Test ID | Requirement |
|---------|-------------|
| TS-09-1 | 09-REQ-1.1, 09-REQ-1.2, 09-REQ-1.3 |
| TS-09-2 | 09-REQ-2.1 |
| TS-09-3 | 09-REQ-4.1 |
| TS-09-4 | 09-REQ-4.1 |
| TS-09-5 | 09-REQ-1.1, 09-REQ-1.2, 09-REQ-1.3, 09-REQ-2.1 |
| TS-09-P1 | 09-REQ-1.1 |
| TS-09-P2 | 09-REQ-1.1 |
| TS-09-P3 | 09-REQ-1.2 |
| TS-09-P4 | 09-REQ-1.2 |
| TS-09-P5 | 09-REQ-1.2 |
| TS-09-P6 | 09-REQ-2.1 |
| TS-09-P7 | 09-REQ-2.1 |
| TS-09-P8 | 09-REQ-2.1 |
| TS-09-P9 | 09-REQ-2.1, 09-REQ-3.1 |
| TS-09-P10 | 09-REQ-1.3 |
| TS-09-P11 | 09-REQ-1.3 |
| TS-09-E1 | 09-REQ-4.1 |
| TS-09-E2 | 09-REQ-4.1 |
| TS-09-E3 | 09-REQ-1.1, 09-REQ-4.1 |
| TS-09-E4 | 09-REQ-1.2, 09-REQ-4.1 |
| TS-09-E5 | 09-REQ-2.1, 09-REQ-3.1 |
| TS-09-E6 | 09-REQ-3.1 |
| TS-09-E7 | 09-REQ-4.1 |
| TS-09-E8 | 09-REQ-2.1, 09-REQ-4.1 |
