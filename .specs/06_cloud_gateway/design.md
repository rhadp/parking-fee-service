# Design: CLOUD_GATEWAY (Spec 06)

> Design document for the CLOUD_GATEWAY service.
> Implements requirements from `.specs/06_cloud_gateway/requirements.md`.

## Architecture Overview

The CLOUD_GATEWAY is a Go service with two communication interfaces:

1. **REST API** (port 8081) -- serves COMPANION_APPs with command and status endpoints, protected by bearer token authentication.
2. **NATS client** -- connects to a NATS server to relay commands to vehicles and receive responses and telemetry.

The service acts as a protocol translator: it receives REST requests from mobile apps, converts them to NATS messages for vehicle delivery, waits for NATS responses, and returns the result over REST.

```
COMPANION_APP                CLOUD_GATEWAY                    CLOUD_GATEWAY_CLIENT
     |                            |                                  |
     |-- POST /commands --------->|                                  |
     |                            |-- NATS PUB vehicles.V.commands ->|
     |                            |                                  |
     |                            |<- NATS SUB vehicles.V.cmd_resp --|
     |<-- 200 OK {status} -------|                                  |
```

### Request-Response Correlation

Each command carries a unique `command_id` (UUID). When the CLOUD_GATEWAY publishes a command to NATS, it stores the `command_id` in a pending-request map with an associated response channel. When a message arrives on the `command_responses` subject, the CLOUD_GATEWAY extracts the `command_id`, looks up the pending request, and delivers the response through the channel. The REST handler blocks on this channel with a timeout.

```
pendingRequests map[string]chan CommandResponse
```

### Telemetry Aggregation

The CLOUD_GATEWAY subscribes to `vehicles.*.telemetry` and maintains an in-memory map of the latest vehicle state per VIN. The `GET /vehicles/{vin}/status` endpoint reads from this map.

```
vehicleStates map[string]*VehicleState  // keyed by VIN
```

## Module Structure

```
backend/cloud-gateway/
  go.mod
  go.sum
  main.go                  # Entry point: wires up server, NATS client, starts HTTP
  server.go                # HTTP server setup, routing, middleware registration
  auth.go                  # Bearer token validation middleware
  handlers.go              # REST endpoint handlers (command, status)
  nats_client.go           # NATS connection, publish, subscribe, reconnection
  protocol.go              # REST <-> NATS message translation
  correlation.go           # Pending request map, response correlation, timeout
  types.go                 # Shared types (CommandRequest, CommandResponse, VehicleState)
  main_test.go             # Integration test wiring
  server_test.go           # HTTP handler unit tests
  auth_test.go             # Auth middleware unit tests
  handlers_test.go         # Handler unit tests
  nats_client_test.go      # NATS client unit tests (with embedded NATS server or mock)
  protocol_test.go         # Protocol translation unit tests
  correlation_test.go      # Correlation and timeout unit tests
```

## REST API Specification

### POST /vehicles/{vin}/commands

Sends a lock or unlock command to a vehicle.

**Request:**
```
POST /vehicles/{vin}/commands HTTP/1.1
Host: localhost:8081
Authorization: Bearer <token>
Content-Type: application/json

{
  "command_id": "550e8400-e29b-41d4-a716-446655440000",
  "type": "lock",
  "doors": ["driver"]
}
```

**Success Response (200 OK):**
```json
{
  "command_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "success"
}
```

**Error Responses:**

| Status | Body | Condition |
|--------|------|-----------|
| 400 | `{"error": "invalid_request_body", "message": "..."}` | Malformed JSON or missing required fields |
| 400 | `{"error": "invalid_command_type"}` | `type` is not `"lock"` or `"unlock"` |
| 400 | `{"error": "invalid_vin_format"}` | VIN does not match alphanumeric 5-20 chars |
| 401 | `{"error": "missing_authorization"}` | No Authorization header |
| 401 | `{"error": "invalid_token"}` | Token not in allowed set |
| 401 | `{"error": "invalid_auth_scheme"}` | Not Bearer scheme |
| 503 | `{"error": "nats_unavailable", "message": "..."}` | NATS connection down |
| 504 | `{"command_id": "...", "status": "failed", "error": "command_timeout", "message": "..."}` | No vehicle response within 10s |

### GET /vehicles/{vin}/status

Returns the latest known state of a vehicle.

**Request:**
```
GET /vehicles/{vin}/status HTTP/1.1
Host: localhost:8081
Authorization: Bearer <token>
```

**Success Response (200 OK):**
```json
{
  "vin": "VIN12345",
  "locked": true,
  "parking_active": false,
  "latitude": 48.1351,
  "longitude": 11.5820,
  "last_updated": "2026-03-05T10:30:00Z"
}
```

**Error Responses:**

| Status | Body | Condition |
|--------|------|-----------|
| 400 | `{"error": "invalid_vin_format"}` | VIN does not match expected format |
| 401 | `{"error": "missing_authorization"}` | No Authorization header |
| 401 | `{"error": "invalid_token"}` | Invalid token |

If no telemetry has been received for the VIN, `last_updated` is `null` and all state fields are zero/default values.

## NATS Subject Model

| Subject | Direction | Purpose |
|---------|-----------|---------|
| `vehicles.{VIN}.commands` | CLOUD_GATEWAY publishes | Relay lock/unlock commands to vehicle |
| `vehicles.{VIN}.command_responses` | CLOUD_GATEWAY subscribes | Receive command execution results from vehicle |
| `vehicles.{VIN}.telemetry` | CLOUD_GATEWAY subscribes | Receive vehicle state updates (lock, parking, location) |

### NATS Command Message

Published to `vehicles.{VIN}.commands`:
```json
{
  "command_id": "550e8400-e29b-41d4-a716-446655440000",
  "action": "lock",
  "doors": ["driver"],
  "source": "companion_app",
  "vin": "VIN12345",
  "timestamp": 1772899800
}
```

### NATS Command Response Message

Received on `vehicles.{VIN}.command_responses`:
```json
{
  "command_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "success",
  "timestamp": 1772899801
}
```

Or on failure:
```json
{
  "command_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "failed",
  "reason": "vehicle_moving",
  "timestamp": 1772899801
}
```

### NATS Telemetry Message

Received on `vehicles.{VIN}.telemetry`:
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

## Bearer Token Validation

For the demo, authentication uses a static set of valid bearer tokens configured via the `AUTH_TOKENS` environment variable (comma-separated list). If `AUTH_TOKENS` is not set, a default demo token `demo-token-001` is used.

The auth middleware:
1. Extracts the `Authorization` header.
2. Checks that the scheme is `Bearer`.
3. Validates the token against the allowed set.
4. On failure, returns the appropriate 401 error before the handler runs.

## Command Timeout Handling

The command flow is synchronous from the REST caller's perspective but asynchronous internally:

1. REST handler receives the command.
2. Handler generates a response channel and registers it in the pending-request map keyed by `command_id`.
3. Handler publishes the command to NATS.
4. Handler blocks on the response channel with a 10-second timeout (`context.WithTimeout`).
5. On receiving a correlated response, the channel delivers the result.
6. On timeout, the handler removes the pending entry and returns `504 Gateway Timeout`.
7. In all cases, the pending entry is cleaned up after the handler returns.

## Correctness Properties

### CP-1: Protocol Translation Integrity

Every field in a REST command request shall appear in the corresponding NATS message, with `type` mapped to `action`. No fields shall be added, removed, or modified beyond this mapping and the addition of `source`, `vin`, and `timestamp`.

### CP-2: Authentication Enforcement

No REST request shall reach a handler without passing through the bearer token validation middleware. All endpoints require valid authentication.

### CP-3: Response Correlation Accuracy

A NATS response shall only be delivered to the REST caller whose `command_id` matches. No response shall be delivered to the wrong caller, and no response shall be lost due to a race condition (the pending map is protected by a mutex).

### CP-4: Timeout Guarantee

Every pending command request shall resolve within 10 seconds -- either by receiving a correlated NATS response or by returning a timeout error. No REST request shall block indefinitely.

### CP-5: NATS Disconnection Visibility

While the NATS connection is disconnected, all command requests shall receive a `503 Service Unavailable` response. The CLOUD_GATEWAY shall never silently drop commands.

### CP-6: Telemetry Freshness

The vehicle state map shall always reflect the most recent telemetry message received for each VIN. Older telemetry messages shall not overwrite newer ones (compared by `timestamp`).

### CP-7: Graceful Shutdown

When the CLOUD_GATEWAY receives a termination signal (SIGTERM, SIGINT), it shall stop accepting new requests, wait for in-flight requests to complete (with a 15-second grace period), close the NATS connection, and then exit.

## Error Handling

| Scenario | HTTP Status | Error Code | Action |
|----------|-------------|------------|--------|
| Missing Authorization header | 401 | `missing_authorization` | Reject before handler |
| Invalid bearer token | 401 | `invalid_token` | Reject before handler |
| Wrong auth scheme | 401 | `invalid_auth_scheme` | Reject before handler |
| Invalid VIN format | 400 | `invalid_vin_format` | Reject in handler |
| Malformed request body | 400 | `invalid_request_body` | Reject in handler |
| Invalid command type | 400 | `invalid_command_type` | Reject in handler |
| NATS connection down | 503 | `nats_unavailable` | Reject in handler |
| Command timeout (10s) | 504 | `command_timeout` | Clean up pending entry, return timeout |
| Invalid NATS response JSON | 502 | `invalid_vehicle_response` | Log error, return bad gateway |
| Unsupported HTTP method | 405 | `method_not_allowed` | Return method not allowed |

## Technology Stack

| Component | Technology |
|-----------|------------|
| Language | Go 1.22+ |
| HTTP router | `net/http` (standard library) with `http.ServeMux` |
| NATS client | `github.com/nats-io/nats.go` |
| JSON | `encoding/json` (standard library) |
| Logging | `log/slog` (standard library structured logging) |
| Testing | `testing` (standard library), `net/http/httptest` |
| Port | 8081 (configurable via `PORT` env var) |
| NATS URL | `nats://localhost:4222` (configurable via `NATS_URL` env var) |

## Configuration

| Environment Variable | Default | Description |
|---------------------|---------|-------------|
| `PORT` | `8081` | HTTP server listen port |
| `NATS_URL` | `nats://localhost:4222` | NATS server URL |
| `AUTH_TOKENS` | `demo-token-001` | Comma-separated list of valid bearer tokens |
| `COMMAND_TIMEOUT` | `10s` | Timeout for waiting on NATS command responses |

## Definition of Done

1. All REST endpoints return correct responses for valid and invalid inputs.
2. Bearer token authentication rejects unauthorized requests with appropriate error codes.
3. REST commands are correctly translated to NATS messages and published.
4. NATS responses are correlated with pending REST requests by `command_id`.
5. Command timeout returns `504 Gateway Timeout` after the configured timeout period.
6. Telemetry messages update the in-memory vehicle state map.
7. NATS connection failures are handled gracefully with reconnection and `503` responses.
8. All unit tests pass (`go test ./... -v`).
9. All integration tests pass with a running NATS server.
10. `go vet ./...` reports no issues.

## Testing Strategy

### Unit Tests

- **Auth middleware:** Test valid tokens, invalid tokens, missing headers, wrong scheme.
- **Handlers:** Test command and status endpoints using `httptest.NewRecorder` with mocked NATS client.
- **Protocol translation:** Test REST-to-NATS and NATS-to-REST field mapping.
- **Correlation:** Test pending request registration, response delivery, timeout, cleanup, concurrent access.

### Integration Tests

- **NATS round-trip:** Start an embedded or containerized NATS server, publish a command, simulate a response, verify end-to-end flow.
- **Telemetry ingestion:** Publish telemetry on NATS, verify status endpoint returns updated state.
- **Timeout:** Publish a command with no response, verify 504 after timeout.

### Test Commands

```bash
# Unit tests
cd backend/cloud-gateway && go test ./... -v

# Lint
cd backend/cloud-gateway && go vet ./...

# Integration tests (requires running NATS)
make infra-up
cd backend/cloud-gateway && go test ./... -v -tags=integration
make infra-down
```
