# Requirements Document: Vehicle-to-Cloud Connectivity (Phase 2.2)

## Introduction

This document specifies the requirements for the vehicle-to-cloud connectivity
layer of the SDV Parking Demo System. The connectivity layer consists of the
CLOUD_GATEWAY backend service — which bridges REST requests from the
COMPANION_APP to MQTT messages for the vehicle — and enhancements to the mock
COMPANION_APP CLI that make it a functional integration testing tool. An
end-to-end integration test validates the full request-response cycle from CLI
through REST, MQTT, and back.

## Glossary

| Term | Definition |
|------|-----------|
| CLOUD_GATEWAY | Go backend service that translates between REST (towards COMPANION_APP) and MQTT (towards vehicle CLOUD_GATEWAY_CLIENT). |
| COMPANION_APP | Mobile application (or mock CLI) paired with a specific VIN, used to send lock/unlock commands and query vehicle status. |
| CLOUD_GATEWAY_CLIENT | Rust service on the vehicle (RHIVOS safety partition) that subscribes to MQTT commands and publishes telemetry. Implemented in spec 02. |
| VIN | Vehicle Identification Number. Unique identifier for each vehicle in the system. |
| Bearer token | A simple string token used for demo authentication. Not JWT or OIDC. |
| Command correlation | The practice of including a `command_id` (UUID) in a command request that is echoed in the response, allowing the caller to match responses to requests. |
| MQTT | Message Queuing Telemetry Transport — lightweight publish/subscribe messaging protocol. |
| Eclipse Mosquitto | Open-source MQTT broker used for local development. |
| REST | Representational State Transfer — HTTP-based API style used by COMPANION_APP to communicate with CLOUD_GATEWAY. |

## Requirements

### Requirement 1: CLOUD_GATEWAY REST API

**User Story:** As a COMPANION_APP developer, I want the CLOUD_GATEWAY to
expose a RESTful API for sending vehicle commands and querying status, so that
the mobile app can control the vehicle without knowing about MQTT.

#### Acceptance Criteria

1. THE CLOUD_GATEWAY SHALL expose a `POST /vehicles/{vin}/commands` endpoint
   that accepts a JSON body containing `command_id` (string), `type`
   (string: "lock" or "unlock"), and `doors` (array of strings). `03-REQ-1.1`
2. THE CLOUD_GATEWAY SHALL expose a `GET /vehicles/{vin}/status` endpoint that
   returns a JSON body containing `vin` (string), `locked` (boolean), and
   `timestamp` (integer, unix epoch). `03-REQ-1.2`
3. THE CLOUD_GATEWAY SHALL expose a `GET /health` endpoint that returns
   HTTP 200 with body `{"status": "ok"}` and requires no authentication.
   `03-REQ-1.3`
4. WHEN a request to `/vehicles/{vin}/commands` or `/vehicles/{vin}/status`
   does not include a valid `Authorization: Bearer <token>` header, THEN the
   CLOUD_GATEWAY SHALL respond with HTTP 401 Unauthorized. `03-REQ-1.4`
5. WHEN a `POST /vehicles/{vin}/commands` request contains a valid bearer token
   and well-formed body, THEN the CLOUD_GATEWAY SHALL respond with HTTP 202
   Accepted containing `{"command_id": "<id>", "status": "pending"}` and
   subsequently publish the command to MQTT. `03-REQ-1.5`

#### Edge Cases

1. IF the `POST /vehicles/{vin}/commands` request body is missing required
   fields or contains an invalid `type` value (not "lock" or "unlock"), THEN
   the CLOUD_GATEWAY SHALL respond with HTTP 400 Bad Request and a JSON body
   describing the validation error. `03-REQ-1.E1`
2. IF the `POST /vehicles/{vin}/commands` request body is not valid JSON, THEN
   the CLOUD_GATEWAY SHALL respond with HTTP 400 Bad Request. `03-REQ-1.E2`

---

### Requirement 2: CLOUD_GATEWAY MQTT Bridge

**User Story:** As a system integrator, I want the CLOUD_GATEWAY to bridge
between its REST API and the MQTT broker, so that commands from the
COMPANION_APP reach the vehicle and responses flow back.

#### Acceptance Criteria

1. THE CLOUD_GATEWAY SHALL connect to an MQTT broker on startup using a
   configurable broker address (default: `localhost:1883`). `03-REQ-2.1`
2. WHEN a valid command is received via REST `POST /vehicles/{vin}/commands`,
   THEN the CLOUD_GATEWAY SHALL publish a JSON message to the MQTT topic
   `vehicles/{vin}/commands` containing `command_id`, `action` (mapped from
   `type`), `doors`, and `source` ("companion_app"). `03-REQ-2.2`
3. THE CLOUD_GATEWAY SHALL subscribe to MQTT topic
   `vehicles/{vin}/command_responses` for each VIN that has issued a command,
   and use incoming messages to resolve pending command requests. `03-REQ-2.3`
4. THE CLOUD_GATEWAY SHALL subscribe to MQTT topic
   `vehicles/{vin}/telemetry` for each VIN that has been queried via the
   status endpoint, and cache the latest telemetry for status responses.
   `03-REQ-2.4`
5. WHEN an MQTT message is received on `vehicles/{vin}/command_responses` with
   a `command_id` matching a pending REST request, THEN the CLOUD_GATEWAY
   SHALL deliver the response to the waiting REST client. `03-REQ-2.5`

#### Edge Cases

1. IF the MQTT broker is unreachable on startup, THEN the CLOUD_GATEWAY SHALL
   retry the connection with exponential backoff and log each retry attempt,
   but SHALL still start the REST API server (degraded mode). `03-REQ-2.E1`
2. IF the MQTT broker connection is lost after startup, THEN the CLOUD_GATEWAY
   SHALL attempt automatic reconnection and log the disconnection event.
   `03-REQ-2.E2`
3. IF a command response is not received via MQTT within a configurable timeout
   (default: 30 seconds), THEN the CLOUD_GATEWAY SHALL respond to the waiting
   REST client with HTTP 504 Gateway Timeout containing
   `{"command_id": "<id>", "status": "timeout"}`. `03-REQ-2.E3`

---

### Requirement 3: Command Lifecycle

**User Story:** As a developer, I want every command to be uniquely identified
and traceable through the entire request-response cycle, so that I can
correlate REST requests with MQTT messages and debug issues.

#### Acceptance Criteria

1. WHEN the CLOUD_GATEWAY publishes a command to MQTT, THE published message
   SHALL preserve the `command_id` from the original REST request. `03-REQ-3.1`
2. WHEN the CLOUD_GATEWAY receives a command response from MQTT, IT SHALL match
   the response to the original REST request using the `command_id` field.
   `03-REQ-3.2`
3. THE MQTT command message published by CLOUD_GATEWAY SHALL conform to the
   JSON schema: `{"command_id": "<uuid>", "action": "lock"|"unlock",
   "doors": ["<door>", ...], "source": "companion_app"}`. `03-REQ-3.3`
4. THE MQTT command response message expected by CLOUD_GATEWAY SHALL conform to
   the JSON schema: `{"command_id": "<uuid>",
   "status": "success"|"failed", "reason": "<optional string>",
   "timestamp": <unix_epoch>}`. `03-REQ-3.4`

#### Edge Cases

1. IF a command response is received with a `command_id` that does not match
   any pending request, THEN the CLOUD_GATEWAY SHALL log a warning and discard
   the message. `03-REQ-3.E1`
2. IF multiple command responses are received for the same `command_id`, THEN
   the CLOUD_GATEWAY SHALL use the first response and ignore subsequent ones.
   `03-REQ-3.E2`

---

### Requirement 4: Mock COMPANION_APP CLI

**User Story:** As a developer, I want the mock COMPANION_APP CLI to send real
REST requests to the CLOUD_GATEWAY, so that I can manually test the
vehicle-to-cloud connectivity without a mobile application.

#### Acceptance Criteria

1. THE `lock` command SHALL send a `POST /vehicles/{vin}/commands` request to
   the CLOUD_GATEWAY with `type` set to "lock", a generated UUID as
   `command_id`, and `doors` set to `["driver"]`. `03-REQ-4.1`
2. THE `unlock` command SHALL send a `POST /vehicles/{vin}/commands` request to
   the CLOUD_GATEWAY with `type` set to "unlock", a generated UUID as
   `command_id`, and `doors` set to `["driver"]`. `03-REQ-4.2`
3. THE `status` command SHALL send a `GET /vehicles/{vin}/status` request to
   the CLOUD_GATEWAY and display the response to stdout. `03-REQ-4.3`
4. ALL commands SHALL include the bearer token from the `--token` flag in the
   `Authorization` header. `03-REQ-4.4`
5. ALL commands SHALL use the VIN from the `--vin` flag to construct the
   request URL path. `03-REQ-4.5`
6. WHEN a command succeeds, THE CLI SHALL print the JSON response to stdout
   and exit with code 0. `03-REQ-4.6`
7. WHEN a command fails (non-2xx HTTP response or network error), THE CLI
   SHALL print an error message to stderr and exit with a non-zero exit code.
   `03-REQ-4.7`

#### Edge Cases

1. IF the `--token` flag is not provided, THEN the CLI SHALL print an error
   message indicating that a token is required and exit with a non-zero exit
   code. `03-REQ-4.E1`
2. IF the CLOUD_GATEWAY is unreachable, THEN the CLI SHALL print a connection
   error message to stderr and exit with a non-zero exit code. `03-REQ-4.E2`

---

### Requirement 5: Multi-Vehicle Support

**User Story:** As a fleet operator, I want the CLOUD_GATEWAY to handle
commands for multiple vehicles simultaneously, so that the system supports
the multi-vehicle demo scenario.

#### Acceptance Criteria

1. THE CLOUD_GATEWAY SHALL support concurrent commands for different VINs,
   routing each command to the correct MQTT topic based on the VIN in the
   URL path. `03-REQ-5.1`
2. THE CLOUD_GATEWAY SHALL maintain separate command response tracking per
   VIN, so that responses for one vehicle do not interfere with another.
   `03-REQ-5.2`

#### Edge Cases

(none for this requirement)

---

### Requirement 6: Integration Testing

**User Story:** As a developer, I want an end-to-end integration test that
verifies the full request-response cycle from COMPANION_APP CLI through
CLOUD_GATEWAY to MQTT and back, so that I can catch regressions in the
connectivity layer.

#### Acceptance Criteria

1. THE integration test SHALL start a CLOUD_GATEWAY instance connected to a
   local Mosquitto broker, send a command via the mock COMPANION_APP CLI (or
   equivalent HTTP request), verify the command appears on the correct MQTT
   topic, publish a simulated response on the command_responses topic, and
   verify the REST response is returned correctly. `03-REQ-6.1`
2. THE integration test SHALL be implementable as a Go test that can be run
   with `go test` and requires only a running Mosquitto broker (provided by
   `make infra-up`). `03-REQ-6.2`
3. THE integration test SHALL verify command correlation: the `command_id` in
   the REST response matches the `command_id` in the original REST request.
   `03-REQ-6.3`

#### Edge Cases

1. IF the Mosquitto broker is not running when the integration test starts,
   THEN the test SHALL skip with a clear message rather than failing with an
   obscure error. `03-REQ-6.E1`
