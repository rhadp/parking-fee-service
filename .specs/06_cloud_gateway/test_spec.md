# Test Specification: CLOUD_GATEWAY (Spec 06)

> Test specifications for the CLOUD_GATEWAY service.
> Validates requirements from `.specs/06_cloud_gateway/requirements.md`.

## Test Notation

- **TS-06-N:** Nominal (happy path) tests
- **TS-06-PN:** Parametric nominal variants
- **TS-06-EN:** Error / edge case tests

## Test Cases

### TS-06-1: POST Command with Valid Token Is Relayed to NATS

**Requirement:** 06-REQ-1.1, 06-REQ-3.1, 06-REQ-5.1

**Description:** A valid lock command sent via REST with a correct bearer token is translated and published to the correct NATS subject.

**Preconditions:**
- CLOUD_GATEWAY is running on port 8081.
- NATS connection is established.
- A valid demo bearer token is configured.

**Steps:**
1. Send `POST /vehicles/VIN12345/commands` with header `Authorization: Bearer demo-token-001` and body:
   ```json
   {
     "command_id": "550e8400-e29b-41d4-a716-446655440000",
     "type": "lock",
     "doors": ["driver"]
   }
   ```
2. Subscribe to `vehicles.VIN12345.commands` on NATS before sending the request.
3. Simulate a success response on `vehicles.VIN12345.command_responses`:
   ```json
   {
     "command_id": "550e8400-e29b-41d4-a716-446655440000",
     "status": "success",
     "timestamp": 1772899801
   }
   ```

**Expected result:**
- The NATS message on `vehicles.VIN12345.commands` contains:
  - `command_id`: `"550e8400-e29b-41d4-a716-446655440000"`
  - `action`: `"lock"` (mapped from `type`)
  - `doors`: `["driver"]`
  - `source`: `"companion_app"`
  - `vin`: `"VIN12345"`
  - `timestamp`: a valid Unix timestamp
- REST response is `200 OK` with `{"command_id": "550e8400-e29b-41d4-a716-446655440000", "status": "success"}`.

---

### TS-06-2: POST Command with Invalid Token Returns 401

**Requirement:** 06-REQ-5.1

**Description:** A command request with an invalid bearer token is rejected with 401 Unauthorized.

**Preconditions:**
- CLOUD_GATEWAY is running.

**Steps:**
1. Send `POST /vehicles/VIN12345/commands` with header `Authorization: Bearer wrong-token-999` and a valid command body.

**Expected result:**
- HTTP status: `401 Unauthorized`.
- Response body: `{"error": "invalid_token"}`.
- No message is published to NATS.

---

### TS-06-E1: POST Command with Missing Authorization Returns 401

**Requirement:** 06-REQ-5.1

**Description:** A command request without an Authorization header is rejected.

**Preconditions:**
- CLOUD_GATEWAY is running.

**Steps:**
1. Send `POST /vehicles/VIN12345/commands` without an `Authorization` header, with a valid command body.

**Expected result:**
- HTTP status: `401 Unauthorized`.
- Response body: `{"error": "missing_authorization"}`.

---

### TS-06-E2: POST Command with Invalid VIN Format Returns 400

**Requirement:** 06-REQ-1.1

**Description:** A command request with a VIN that does not match the expected format is rejected.

**Preconditions:**
- CLOUD_GATEWAY is running.

**Steps:**
1. Send `POST /vehicles/AB!@/commands` with a valid bearer token and valid command body.

**Expected result:**
- HTTP status: `400 Bad Request`.
- Response body: `{"error": "invalid_vin_format"}`.

---

### TS-06-3: GET Status Returns Latest Vehicle State

**Requirement:** 06-REQ-2.1

**Description:** The status endpoint returns the most recent telemetry data for a vehicle.

**Preconditions:**
- CLOUD_GATEWAY is running with NATS connected.
- A telemetry message has been published on `vehicles.VIN12345.telemetry`:
  ```json
  {
    "vin": "VIN12345",
    "locked": true,
    "parking_active": false,
    "latitude": 48.1351,
    "longitude": 11.5820,
    "timestamp": 1772899800
  }
  ```

**Steps:**
1. Wait briefly for the CLOUD_GATEWAY to process the telemetry message.
2. Send `GET /vehicles/VIN12345/status` with header `Authorization: Bearer demo-token-001`.

**Expected result:**
- HTTP status: `200 OK`.
- Response body contains:
  - `vin`: `"VIN12345"`
  - `locked`: `true`
  - `parking_active`: `false`
  - `latitude`: `48.1351`
  - `longitude`: `11.5820`
  - `last_updated`: a non-null ISO 8601 timestamp.

---

### TS-06-P1: GET Status for Unknown VIN Returns Default State

**Requirement:** 06-REQ-2.1

**Description:** Requesting status for a VIN with no received telemetry returns default/zero values.

**Preconditions:**
- CLOUD_GATEWAY is running.
- No telemetry has been received for VIN `UNKNOWN999`.

**Steps:**
1. Send `GET /vehicles/UNKNOWN999/status` with a valid bearer token.

**Expected result:**
- HTTP status: `200 OK`.
- Response body contains:
  - `vin`: `"UNKNOWN999"`
  - `locked`: `false`
  - `parking_active`: `false`
  - `latitude`: `0`
  - `longitude`: `0`
  - `last_updated`: `null`.

---

### TS-06-4: NATS Response Is Correlated and Returned to REST Caller

**Requirement:** 06-REQ-4.1, 06-REQ-6.1

**Description:** A NATS command response with a matching command_id is correctly correlated with the pending REST request and returned.

**Preconditions:**
- CLOUD_GATEWAY is running with NATS connected.

**Steps:**
1. Send `POST /vehicles/VIN12345/commands` with a valid token and body:
   ```json
   {
     "command_id": "aaa-bbb-ccc",
     "type": "unlock",
     "doors": ["driver"]
   }
   ```
2. Within 2 seconds, publish on `vehicles.VIN12345.command_responses`:
   ```json
   {
     "command_id": "aaa-bbb-ccc",
     "status": "success",
     "timestamp": 1772899900
   }
   ```

**Expected result:**
- REST response is `200 OK` with `{"command_id": "aaa-bbb-ccc", "status": "success"}`.
- The response is returned promptly (well under the 10-second timeout).

---

### TS-06-E3: Command Timeout Returns 504

**Requirement:** 06-REQ-4.1

**Description:** When no NATS response is received within the timeout period, the REST caller receives a timeout error.

**Preconditions:**
- CLOUD_GATEWAY is running with NATS connected.
- No CLOUD_GATEWAY_CLIENT is responding to commands.
- Command timeout is set to a short value (e.g., 2 seconds) for test efficiency.

**Steps:**
1. Send `POST /vehicles/VIN12345/commands` with a valid token and body:
   ```json
   {
     "command_id": "timeout-test-001",
     "type": "lock",
     "doors": ["driver"]
   }
   ```
2. Do not publish any response on NATS.

**Expected result:**
- After the timeout period, HTTP status is `504 Gateway Timeout`.
- Response body contains:
  - `command_id`: `"timeout-test-001"`
  - `status`: `"failed"`
  - `error`: `"command_timeout"`

---

### TS-06-E4: Malformed Request Body Returns 400

**Requirement:** 06-REQ-1.1

**Description:** A POST request with an invalid JSON body is rejected.

**Preconditions:**
- CLOUD_GATEWAY is running.

**Steps:**
1. Send `POST /vehicles/VIN12345/commands` with a valid bearer token and body: `{invalid json`.

**Expected result:**
- HTTP status: `400 Bad Request`.
- Response body: `{"error": "invalid_request_body", "message": "..."}` where message contains a description of the parse error.

---

### TS-06-E5: Missing Required Fields Returns 400

**Requirement:** 06-REQ-1.1

**Description:** A POST request with valid JSON but missing required fields is rejected.

**Preconditions:**
- CLOUD_GATEWAY is running.

**Steps:**
1. Send `POST /vehicles/VIN12345/commands` with a valid bearer token and body:
   ```json
   {
     "doors": ["driver"]
   }
   ```

**Expected result:**
- HTTP status: `400 Bad Request`.
- Response body: `{"error": "invalid_request_body", "message": "..."}`.

---

### TS-06-5: Protocol Translation Preserves Command Fields

**Requirement:** 06-REQ-6.1

**Description:** All fields from the REST command body are correctly translated to the NATS message format.

**Preconditions:**
- CLOUD_GATEWAY is running with NATS connected.

**Steps:**
1. Subscribe to `vehicles.VIN12345.commands` on NATS.
2. Send `POST /vehicles/VIN12345/commands` with a valid token and body:
   ```json
   {
     "command_id": "proto-test-001",
     "type": "unlock",
     "doors": ["driver"]
   }
   ```
3. Capture the NATS message.

**Expected result:**
- NATS message contains exactly:
  - `command_id`: `"proto-test-001"` (preserved)
  - `action`: `"unlock"` (mapped from `type`)
  - `doors`: `["driver"]` (preserved)
  - `source`: `"companion_app"` (added)
  - `vin`: `"VIN12345"` (added from path)
  - `timestamp`: a valid Unix timestamp (added)
- No extra fields are present beyond those listed above.

---

### TS-06-E6: NATS Unavailable Returns 503

**Requirement:** 06-REQ-3.1, 06-REQ-7.1

**Description:** When the NATS connection is down, command requests return 503.

**Preconditions:**
- CLOUD_GATEWAY is running.
- NATS server is stopped or unreachable.

**Steps:**
1. Send `POST /vehicles/VIN12345/commands` with a valid token and a valid command body.

**Expected result:**
- HTTP status: `503 Service Unavailable`.
- Response body: `{"error": "nats_unavailable", "message": "Vehicle messaging service is temporarily unavailable"}`.

---

### TS-06-E7: Invalid Command Type Returns 400

**Requirement:** 06-REQ-1.1

**Description:** A command with a type other than "lock" or "unlock" is rejected.

**Preconditions:**
- CLOUD_GATEWAY is running.

**Steps:**
1. Send `POST /vehicles/VIN12345/commands` with a valid token and body:
   ```json
   {
     "command_id": "type-test-001",
     "type": "start_engine",
     "doors": ["driver"]
   }
   ```

**Expected result:**
- HTTP status: `400 Bad Request`.
- Response body: `{"error": "invalid_command_type"}`.

---

## Traceability Matrix

| Test ID | Requirement(s) | Category |
|---------|----------------|----------|
| TS-06-1 | 06-REQ-1.1, 06-REQ-3.1, 06-REQ-5.1 | Nominal |
| TS-06-2 | 06-REQ-5.1 | Nominal |
| TS-06-3 | 06-REQ-2.1 | Nominal |
| TS-06-4 | 06-REQ-4.1, 06-REQ-6.1 | Nominal |
| TS-06-5 | 06-REQ-6.1 | Nominal |
| TS-06-P1 | 06-REQ-2.1 | Parametric |
| TS-06-E1 | 06-REQ-5.1 | Error |
| TS-06-E2 | 06-REQ-1.1 | Error |
| TS-06-E3 | 06-REQ-4.1 | Error |
| TS-06-E4 | 06-REQ-1.1 | Error |
| TS-06-E5 | 06-REQ-1.1 | Error |
| TS-06-E6 | 06-REQ-3.1, 06-REQ-7.1 | Error |
| TS-06-E7 | 06-REQ-1.1 | Error |
