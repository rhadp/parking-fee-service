# Requirements: CLOUD_GATEWAY (Spec 06)

> EARS-syntax requirements for the CLOUD_GATEWAY service.
> Derived from the PRD at `.specs/06_cloud_gateway/prd.md` and the master PRD at `.specs/prd.md`.

## Notation

Requirements use the EARS (Easy Approach to Requirements Syntax) patterns:

- **Ubiquitous:** `The <system> shall <action>.`
- **Event-driven:** `When <trigger>, the <system> shall <action>.`
- **State-driven:** `While <state>, the <system> shall <action>.`
- **Unwanted behavior:** `If <condition>, then the <system> shall <action>.`
- **Option:** `Where <feature>, the <system> shall <action>.`

## Requirements

### 06-REQ-1.1: REST Command Endpoint

When a COMPANION_APP sends a `POST /vehicles/{vin}/commands` request with a valid bearer token and a JSON body containing `command_id` (UUID), `type` (`"lock"` or `"unlock"`), and `doors` (string array), the CLOUD_GATEWAY shall accept the command and return a `200 OK` response with `{"command_id": "<uuid>", "status": "<status>"}` after the command has been processed or timed out.

**Rationale:** The REST command endpoint is the entry point for COMPANION_APPs to issue remote lock/unlock commands to vehicles. The CLOUD_GATEWAY translates these REST requests into NATS messages for vehicle delivery.

**Acceptance criteria:**
- POST `/vehicles/{vin}/commands` accepts a JSON body with `command_id`, `type`, and `doors` fields.
- A `200 OK` response is returned with the `command_id` and a `status` of `"accepted"`, `"success"`, or `"failed"`.
- The `command_id` in the response matches the `command_id` from the request.

**Edge cases:**
- If the request body is missing required fields (`command_id`, `type`) or contains invalid JSON, then the CLOUD_GATEWAY shall return `400 Bad Request` with `{"error": "invalid_request_body", "message": "<details>"}`.
- If the `type` field is not `"lock"` or `"unlock"`, then the CLOUD_GATEWAY shall return `400 Bad Request` with `{"error": "invalid_command_type"}`.
- If the VIN in the path does not match the expected format (alphanumeric, 5-20 characters), then the CLOUD_GATEWAY shall return `400 Bad Request` with `{"error": "invalid_vin_format"}`.

---

### 06-REQ-2.1: REST Status Endpoint

When a COMPANION_APP sends a `GET /vehicles/{vin}/status` request with a valid bearer token, the CLOUD_GATEWAY shall return a `200 OK` response containing the latest known vehicle state including lock status, parking session status, and location.

**Rationale:** COMPANION_APPs need to query the current state of the vehicle to display status information to the user.

**Acceptance criteria:**
- GET `/vehicles/{vin}/status` returns a JSON body with fields: `vin`, `locked` (bool), `parking_active` (bool), `latitude` (float), `longitude` (float), `last_updated` (ISO 8601 timestamp).
- The response reflects the latest telemetry received from the vehicle via NATS.

**Edge cases:**
- If no telemetry has been received for the given VIN, then the CLOUD_GATEWAY shall return `200 OK` with default/zero values and `last_updated` set to `null`.
- If the VIN format is invalid, then the CLOUD_GATEWAY shall return `400 Bad Request` with `{"error": "invalid_vin_format"}`.

---

### 06-REQ-3.1: NATS Command Relay

When a valid command is received via the REST command endpoint, the CLOUD_GATEWAY shall publish a JSON message to the NATS subject `vehicles.{VIN}.commands` containing `command_id`, `action` (mapped from `type`), `doors`, `source` (`"companion_app"`), `vin`, and `timestamp` (Unix seconds).

**Rationale:** The CLOUD_GATEWAY translates REST commands into NATS messages so that the vehicle's CLOUD_GATEWAY_CLIENT can receive and process them through DATA_BROKER.

**Acceptance criteria:**
- The NATS message is published to `vehicles.{VIN}.commands` where `{VIN}` matches the VIN from the REST request path.
- The `type` field from the REST body is mapped to `action` in the NATS message.
- The `source` field is set to `"companion_app"`.
- A `timestamp` field with the current Unix timestamp is included.

**Edge cases:**
- If the NATS connection is unavailable at the time of publishing, then the CLOUD_GATEWAY shall return `503 Service Unavailable` with `{"error": "nats_unavailable", "message": "Vehicle messaging service is temporarily unavailable"}`.

---

### 06-REQ-4.1: NATS Response Handling

When the CLOUD_GATEWAY publishes a command to NATS, the CLOUD_GATEWAY shall subscribe to `vehicles.{VIN}.command_responses` and wait for a response message whose `command_id` matches the original command, then return the result to the REST caller.

**Rationale:** Commands sent to vehicles are processed asynchronously. The CLOUD_GATEWAY must correlate NATS responses with the originating REST request to deliver the result back to the COMPANION_APP.

**Acceptance criteria:**
- The CLOUD_GATEWAY subscribes to `vehicles.{VIN}.command_responses` to receive command results.
- Only the response with a matching `command_id` is correlated with the pending REST request.
- The `status` field from the NATS response (`"success"` or `"failed"`) is forwarded in the REST response.
- If the NATS response includes a `reason` field, it is included in the REST response.

**Edge cases:**
- If no matching response is received within 10 seconds, then the CLOUD_GATEWAY shall return `504 Gateway Timeout` with `{"command_id": "<uuid>", "status": "failed", "error": "command_timeout", "message": "Vehicle did not respond within the timeout period"}`.
- If multiple responses arrive with the same `command_id`, the CLOUD_GATEWAY shall use the first response and discard subsequent duplicates.

---

### 06-REQ-5.1: Bearer Token Authentication

The CLOUD_GATEWAY shall require a valid `Authorization: Bearer <token>` header on all REST API requests (`POST /vehicles/{vin}/commands` and `GET /vehicles/{vin}/status`).

**Rationale:** Bearer tokens authenticate COMPANION_APPs to prevent unauthorized access to vehicle commands and status. For the demo, static tokens are used.

**Acceptance criteria:**
- Requests with a valid bearer token are processed normally.
- The CLOUD_GATEWAY validates the token against a preconfigured set of static demo tokens.

**Edge cases:**
- If the `Authorization` header is missing, then the CLOUD_GATEWAY shall return `401 Unauthorized` with `{"error": "missing_authorization"}`.
- If the `Authorization` header is present but the token is invalid, then the CLOUD_GATEWAY shall return `401 Unauthorized` with `{"error": "invalid_token"}`.
- If the `Authorization` header does not use the `Bearer` scheme, then the CLOUD_GATEWAY shall return `401 Unauthorized` with `{"error": "invalid_auth_scheme"}`.

---

### 06-REQ-6.1: Protocol Translation Integrity

The CLOUD_GATEWAY shall perform bidirectional protocol translation between REST and NATS, preserving all command fields during translation and correctly mapping field names between the two protocols.

**Rationale:** Data integrity during protocol translation is critical. The REST `type` field must map to the NATS `action` field, and all other fields must be preserved without modification.

**Acceptance criteria:**
- REST `type` field is mapped to NATS `action` field (and vice versa for responses).
- `command_id`, `doors`, and all other fields are preserved without modification during translation.
- NATS response fields (`command_id`, `status`, `reason`) are correctly mapped to the REST response body.

**Edge cases:**
- If a NATS response message contains invalid JSON, then the CLOUD_GATEWAY shall log the error and return `502 Bad Gateway` with `{"command_id": "<uuid>", "status": "failed", "error": "invalid_vehicle_response"}`.
- If a NATS response message is missing the `command_id` field, then the CLOUD_GATEWAY shall discard the message and log a warning.

---

### 06-REQ-7.1: NATS Connection Management

When the CLOUD_GATEWAY starts, the CLOUD_GATEWAY shall connect to the configured NATS server and maintain the connection throughout its lifecycle, using automatic reconnection with exponential backoff.

**Rationale:** The NATS connection is essential for vehicle communication. The CLOUD_GATEWAY must handle connection failures gracefully and reconnect automatically to minimize downtime.

**Acceptance criteria:**
- The CLOUD_GATEWAY connects to the NATS server specified by the `NATS_URL` environment variable (default: `nats://localhost:4222`).
- The CLOUD_GATEWAY uses the nats.go client library for NATS connectivity.
- The CLOUD_GATEWAY subscribes to `vehicles.*.command_responses` and `vehicles.*.telemetry` for receiving vehicle messages.
- The HTTP server listens on port 8081 (configurable via `PORT` environment variable).

**Edge cases:**
- If the NATS server is unreachable at startup, then the CLOUD_GATEWAY shall retry the connection with exponential backoff and log each retry attempt.
- While the NATS connection is disconnected, the CLOUD_GATEWAY shall return `503 Service Unavailable` for all command requests.
- When the NATS connection is re-established, the CLOUD_GATEWAY shall re-subscribe to all required subjects and resume normal operation.

---

## Traceability

| Requirement | PRD Section |
|-------------|-------------|
| 06-REQ-1.1 | REST API: POST /vehicles/{vin}/commands |
| 06-REQ-2.1 | REST API: GET /vehicles/{vin}/status |
| 06-REQ-3.1 | NATS Interface: Publish vehicles.{VIN}.commands |
| 06-REQ-4.1 | NATS Interface: Subscribe vehicles.{VIN}.command_responses |
| 06-REQ-5.1 | Component Description: Bearer token authentication |
| 06-REQ-6.1 | Protocol Translation |
| 06-REQ-7.1 | Tech Stack: nats.go, Component Description: NATS interface |
