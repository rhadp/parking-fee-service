# Requirements: PARKING_FEE_SERVICE (Spec 05)

> EARS-syntax requirements for the PARKING_FEE_SERVICE cloud REST API.
> Derived from the PRD at `.specs/05_parking_fee_service/prd.md` and the master PRD at `.specs/prd.md`.

## Notation

Requirements use EARS (Easy Approach to Requirements Syntax) patterns:

| Pattern | Template |
|---------|----------|
| Ubiquitous | The system SHALL ... |
| Event-driven | WHEN [event], the system SHALL ... |
| State-driven | WHILE [state], the system SHALL ... |
| Conditional | IF [condition], THEN the system SHALL ... |
| Complex | WHILE [state], WHEN [event], the system SHALL ... |

## Requirements

### 05-REQ-1: Operator Lookup by Location

WHEN a client sends a `GET /operators?lat={lat}&lon={lon}` request with valid latitude and longitude parameters, the system SHALL return a JSON array of parking operators whose geofence zone contains or is near the given coordinates, with each entry including `operator_id`, `name`, `zone` (polygon coordinates), and `rate`.

**Edge cases:**

- IF the `lat` parameter is missing, not a number, or outside the range [-90, 90], THEN the system SHALL return HTTP 400 with a JSON error body containing a descriptive `error` message.
- IF the `lon` parameter is missing, not a number, or outside the range [-180, 180], THEN the system SHALL return HTTP 400 with a JSON error body containing a descriptive `error` message.
- IF no operators have a geofence zone that contains or is near the given coordinates, THEN the system SHALL return HTTP 200 with an empty JSON array `[]`.

### 05-REQ-2: Adapter Metadata Retrieval

WHEN a client sends a `GET /operators/{id}/adapter` request with a valid operator ID, the system SHALL return a JSON object containing `image_ref` (OCI image reference in Google Artifact Registry), `checksum_sha256` (SHA-256 checksum of the OCI manifest digest), and `version` (adapter version string).

**Edge cases:**

- IF the operator ID does not match any known operator, THEN the system SHALL return HTTP 404 with a JSON error body containing a descriptive `error` message.

### 05-REQ-3: Health Check

WHEN a client sends a `GET /health` request, the system SHALL return HTTP 200 with a JSON object containing `"status": "ok"`.

**Edge cases:**

- The health endpoint SHALL respond to any `GET /health` request regardless of query parameters or headers.

### 05-REQ-4: Geofence Matching with Fuzziness

The system SHALL determine whether a coordinate falls within an operator's zone by performing a point-in-polygon test against the zone's geofence polygon. IF the coordinate is outside the polygon but within a configurable buffer distance (near-zone), THEN the system SHALL treat the coordinate as a match for that operator.

**Edge cases:**

- IF the coordinate is exactly on the boundary of a polygon, THEN the system SHALL treat the coordinate as inside the zone.
- IF the coordinate falls within the near-zone buffer of multiple operators, THEN the system SHALL return all matching operators.

### 05-REQ-5: Error Responses

The system SHALL return well-formed JSON error responses with appropriate HTTP status codes: 400 for invalid or malformed request parameters, 404 for unknown resource identifiers, and 500 for unexpected internal errors. Each error response SHALL contain an `error` field with a human-readable message.

**Edge cases:**

- IF a request is made to an undefined route, THEN the system SHALL return HTTP 404 with a JSON error body.
- IF an internal panic or unexpected failure occurs during request processing, THEN the system SHALL recover and return HTTP 500 rather than dropping the connection.

### 05-REQ-6: Response Format

The system SHALL set the `Content-Type` header to `application/json` for all API responses (success and error). All JSON responses SHALL use a consistent structure: success responses return the resource directly, and error responses return an object with an `error` string field.

**Edge cases:**

- IF the client sends an `Accept` header that does not include `application/json`, the system SHALL still respond with `application/json` (the API only supports JSON).

## Traceability

| Requirement | PRD Section |
|-------------|-------------|
| 05-REQ-1 | REST API Endpoints: operator lookup by location |
| 05-REQ-2 | REST API Endpoints: adapter metadata; Adapter Metadata |
| 05-REQ-3 | REST API Endpoints: health check |
| 05-REQ-4 | Geofence Matching |
| 05-REQ-5 | Component Description: error handling |
| 05-REQ-6 | REST API Endpoints: response format |
