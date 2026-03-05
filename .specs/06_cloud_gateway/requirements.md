# Requirements: CLOUD_GATEWAY (Spec 06)

> EARS-syntax requirements for the CLOUD_GATEWAY cloud service.
> Derived from the PRD at `.specs/06_cloud_gateway/prd.md` and the master PRD at `.specs/prd.md`.

## Notation

Requirements use EARS (Easy Approach to Requirements Syntax) patterns:

| Pattern | Template |
|---------|----------|
| Ubiquitous | The system SHALL ... |
| Event-driven | WHEN [event], the system SHALL ... |
| State-driven | WHILE [state], the system SHALL ... |
| Conditional | IF [condition], THEN the system SHALL ... |
| Complex | WHEN [event] AND [condition], the system SHALL ... |

## Glossary

| Term | Definition |
|------|------------|
| CLOUD_GATEWAY | Cloud-based Go service that bridges REST (COMPANION_APP) and NATS (vehicle) interfaces |
| COMPANION_APP | Mobile application paired with a specific VIN, communicates via REST |
| CLOUD_GATEWAY_CLIENT | On-vehicle component that communicates with CLOUD_GATEWAY via NATS |
| VIN | Vehicle Identification Number, unique identifier for each vehicle |
| Bearer token | Authentication credential carried in the HTTP Authorization header |
| NATS | Lightweight messaging system used for vehicle-cloud communication |
| Command | A lock or unlock instruction sent from COMPANION_APP to a vehicle |
| Command response | The result of executing a command, relayed back from the vehicle |
| Telemetry | Vehicle state data (location, door status, parking state) published by the vehicle |

## Requirements

### 06-REQ-1: Command Submission via REST

**User Story:** As a COMPANION_APP user, I want to send lock/unlock commands to my vehicle via REST, so that I can control my car remotely.

#### Acceptance Criteria

1. WHEN a COMPANION_APP sends a `POST /vehicles/{vin}/commands` request with a valid bearer token and a JSON body containing `command_id` (UUID string), `type` ("lock" or "unlock"), and `doors` (string array), THE CLOUD_GATEWAY SHALL accept the command, return HTTP 202 Accepted with the `command_id` and initial status `"pending"`, and publish the command to the NATS subject `vehicles.{vin}.commands`.
2. WHEN a command is published to NATS, THE CLOUD_GATEWAY SHALL include the fields `command_id`, `action` (mapped from `type`), `doors`, and `source` set to `"companion_app"` in the NATS message payload.

#### Edge Cases

1. IF the request body is missing required fields (`command_id`, `type`, or `doors`), THEN THE CLOUD_GATEWAY SHALL return HTTP 400 with a JSON error body describing the missing field.
2. IF the `type` field is not `"lock"` or `"unlock"`, THEN THE CLOUD_GATEWAY SHALL return HTTP 400 with a JSON error body describing the invalid value.

### 06-REQ-2: Bearer Token Authentication

**User Story:** As a system operator, I want all REST requests authenticated via bearer tokens, so that only authorized COMPANION_APPs can issue commands.

#### Acceptance Criteria

1. WHEN a REST request is received, THE CLOUD_GATEWAY SHALL extract the bearer token from the `Authorization` header and validate it against a known set of token-to-VIN mappings.
2. WHEN a valid bearer token is presented, THE CLOUD_GATEWAY SHALL allow the request to proceed only if the token is associated with the VIN in the request path.

#### Edge Cases

1. IF the `Authorization` header is missing or does not start with `"Bearer "`, THEN THE CLOUD_GATEWAY SHALL return HTTP 401 with a JSON error body containing `"error": "missing or invalid authorization header"`.
2. IF the bearer token is not found in the known token set, THEN THE CLOUD_GATEWAY SHALL return HTTP 401 with a JSON error body containing `"error": "invalid token"`.
3. IF the bearer token is valid but associated with a different VIN than the one in the request path, THEN THE CLOUD_GATEWAY SHALL return HTTP 403 with a JSON error body containing `"error": "token not authorized for this vehicle"`.

### 06-REQ-3: NATS Command Relay

**User Story:** As a CLOUD_GATEWAY_CLIENT, I want to receive commands via NATS, so that I can relay them to the vehicle's DATA_BROKER.

#### Acceptance Criteria

1. THE CLOUD_GATEWAY SHALL maintain a connection to the NATS server and publish commands to the subject `vehicles.{vin}.commands` where `{vin}` is the target vehicle's VIN.
2. THE CLOUD_GATEWAY SHALL subscribe to the subject `vehicles.{vin}.command_responses` for each known VIN, to receive command execution results.

#### Edge Cases

1. IF the NATS connection is unavailable when a command is submitted, THEN THE CLOUD_GATEWAY SHALL return HTTP 503 with a JSON error body containing `"error": "messaging service unavailable"`.

### 06-REQ-4: Command Response Forwarding

**User Story:** As a COMPANION_APP user, I want to retrieve the result of my command, so that I know whether my lock/unlock succeeded.

#### Acceptance Criteria

1. WHEN a command response message is received on the NATS subject `vehicles.{vin}.command_responses`, THE CLOUD_GATEWAY SHALL parse the response and store it in an in-memory command status map keyed by `command_id`.
2. WHEN a COMPANION_APP sends a `GET /vehicles/{vin}/commands/{command_id}` request with a valid bearer token, THE CLOUD_GATEWAY SHALL return HTTP 200 with a JSON body containing `command_id`, `status` (one of `"pending"`, `"success"`, or `"failed"`), and optionally `reason`.

#### Edge Cases

1. IF the `command_id` is not found in the command status map, THEN THE CLOUD_GATEWAY SHALL return HTTP 404 with a JSON error body containing `"error": "command not found"`.

### 06-REQ-5: Vehicle Telemetry Reception

**User Story:** As a fleet operator, I want the CLOUD_GATEWAY to receive telemetry from vehicles, so that vehicle state is available in the cloud.

#### Acceptance Criteria

1. THE CLOUD_GATEWAY SHALL subscribe to the NATS subject `vehicles.{vin}.telemetry` for each known VIN and store the latest telemetry payload in memory.

#### Edge Cases

1. IF a telemetry message cannot be parsed as valid JSON, THEN THE CLOUD_GATEWAY SHALL log the error and discard the message without affecting other operations.

### 06-REQ-6: Health Check

**User Story:** As a system operator, I want a health check endpoint, so that I can monitor whether the CLOUD_GATEWAY is running.

#### Acceptance Criteria

1. WHEN a client sends a `GET /health` request, THE CLOUD_GATEWAY SHALL return HTTP 200 with a JSON body containing `"status": "ok"`.

#### Edge Cases

1. The health endpoint SHALL respond to any `GET /health` request regardless of authentication headers.

### 06-REQ-7: VIN Routing

**User Story:** As a system operator, I want commands routed to the correct vehicle by VIN, so that each vehicle only receives its own commands.

#### Acceptance Criteria

1. THE CLOUD_GATEWAY SHALL use the VIN from the request URL path to determine the NATS subject for publishing commands and subscribing to responses.
2. THE CLOUD_GATEWAY SHALL support multiple vehicles simultaneously, each identified by a unique VIN.

#### Edge Cases

1. IF the VIN in the request path does not match any known vehicle, THEN THE CLOUD_GATEWAY SHALL return HTTP 404 with a JSON error body containing `"error": "unknown vehicle"`.

### 06-REQ-8: Error Response Format

**User Story:** As a COMPANION_APP developer, I want consistent error responses, so that I can handle errors programmatically.

#### Acceptance Criteria

1. THE CLOUD_GATEWAY SHALL return well-formed JSON error responses with appropriate HTTP status codes: 400 for invalid requests, 401 for missing or invalid authentication, 403 for unauthorized VIN access, 404 for unknown resources, 503 for NATS unavailability, and 500 for unexpected internal errors.
2. THE CLOUD_GATEWAY SHALL set the `Content-Type` header to `application/json` for all API responses (success and error).

#### Edge Cases

1. IF a request is made to an undefined route, THEN THE CLOUD_GATEWAY SHALL return HTTP 404 with a JSON error body.
2. IF an internal panic or unexpected failure occurs during request processing, THEN THE CLOUD_GATEWAY SHALL recover and return HTTP 500 rather than dropping the connection.

## Traceability

| Requirement | PRD Section |
|-------------|-------------|
| 06-REQ-1 | REST API Endpoints: command submission; Command Flow steps 1-3 |
| 06-REQ-2 | Authentication |
| 06-REQ-3 | NATS Subject Hierarchy; Command Flow steps 3-4 |
| 06-REQ-4 | Command Flow steps 7-9; REST API Endpoints: command status |
| 06-REQ-5 | NATS Subject Hierarchy: telemetry |
| 06-REQ-6 | REST API Endpoints: health check |
| 06-REQ-7 | Component Description: multiple vehicles, VIN routing |
| 06-REQ-8 | Error handling, response format |
