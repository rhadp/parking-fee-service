# Requirements: PARKING_FEE_SERVICE (Spec 05)

> EARS-syntax requirements for the PARKING_FEE_SERVICE cloud REST API.
> Derived from the PRD at `.specs/05_parking_fee_service/prd.md` and the master PRD at `.specs/prd.md`.

## Introduction

The PARKING_FEE_SERVICE is a cloud-based Go REST API that enables parking operator discovery by vehicle location and provides adapter metadata for secure container provisioning. It uses geofence polygon matching with configurable proximity thresholds to determine which parking operators serve a given location. The service loads all operator, zone, and configuration data from a JSON configuration file at startup and serves it from memory.

## Glossary

| Term | Definition |
|------|-----------|
| Geofence | A virtual geographic boundary defined by a polygon of lat/lon coordinates |
| Proximity threshold | Configurable distance (in meters) within which a point outside a polygon is still considered a match |
| Point-in-polygon | Algorithm to determine whether a geographic coordinate falls inside a polygon |
| Operator | A parking service provider associated with one or more zones |
| Zone | A geographic area defined by a geofence polygon |
| Adapter | A containerized application (OCI image) that interfaces with a specific parking operator |
| Rate type | The billing model: either "per_hour" (charged by duration) or "flat_fee" (fixed amount per session) |
| OCI | Open Container Initiative -- standard for container image formats and registries |
| image_ref | A fully qualified OCI image reference (registry/repository:tag) |
| checksum_sha256 | SHA-256 hash of the OCI manifest digest, used for integrity verification |

## Requirements

### Requirement 1: Operator Lookup by Location

**User Story:** As a PARKING_APP, I want to query available parking operators by my vehicle's GPS coordinates, so that I can discover which operators serve the area where the vehicle is parked.

#### Acceptance Criteria

1. **05-REQ-1.1** WHEN a client sends a `GET /operators?lat={lat}&lon={lon}` request with valid latitude ([-90, 90]) and longitude ([-180, 180]) parameters, THE PARKING_FEE_SERVICE SHALL return HTTP 200 with a JSON array of operators whose geofence zone contains or is within the proximity threshold of the given coordinates.
2. **05-REQ-1.2** WHEN a client sends a `GET /operators?lat={lat}&lon={lon}` request with valid coordinates and no operators match, THE PARKING_FEE_SERVICE SHALL return HTTP 200 with an empty JSON array `[]`.
3. **05-REQ-1.3** WHEN a client sends a `GET /operators?lat={lat}&lon={lon}` request, THE PARKING_FEE_SERVICE SHALL include for each matching operator: `operator_id`, `name`, `zone_id`, `rate_type`, `rate_amount`, and `rate_currency`.

#### Edge Cases

1. **05-REQ-1.E1** IF the `lat` parameter is missing, not a number, or outside the range [-90, 90], THEN THE PARKING_FEE_SERVICE SHALL return HTTP 400 with a JSON error body containing a descriptive `error` message.
2. **05-REQ-1.E2** IF the `lon` parameter is missing, not a number, or outside the range [-180, 180], THEN THE PARKING_FEE_SERVICE SHALL return HTTP 400 with a JSON error body containing a descriptive `error` message.

### Requirement 2: Geofence Polygon Matching

**User Story:** As a PARKING_APP, I want the operator lookup to use precise geofence boundaries, so that the correct operators are returned for the vehicle's location.

#### Acceptance Criteria

1. **05-REQ-2.1** THE PARKING_FEE_SERVICE SHALL determine whether a coordinate is inside a zone by performing a point-in-polygon test against the zone's geofence polygon.
2. **05-REQ-2.2** THE PARKING_FEE_SERVICE SHALL treat a coordinate that is on the boundary of a geofence polygon as inside the zone.

#### Edge Cases

1. **05-REQ-2.E1** IF a coordinate falls within the geofence zones of multiple operators, THEN THE PARKING_FEE_SERVICE SHALL return all matching operators.

### Requirement 3: Proximity-Based Matching

**User Story:** As a PARKING_APP, I want the system to match operators when the vehicle is near (but not exactly inside) a zone, so that minor GPS inaccuracies or parking just outside a zone boundary do not prevent operator discovery.

#### Acceptance Criteria

1. **05-REQ-3.1** WHEN a coordinate is outside a geofence polygon but within the configured proximity threshold distance (in meters) of any polygon edge, THE PARKING_FEE_SERVICE SHALL treat the coordinate as a match for that zone's operators.
2. **05-REQ-3.2** THE PARKING_FEE_SERVICE SHALL load the proximity threshold value from the service configuration, with a default of 500 meters.

#### Edge Cases

1. **05-REQ-3.E1** IF a coordinate is outside a geofence polygon and beyond the proximity threshold distance from all polygon edges, THEN THE PARKING_FEE_SERVICE SHALL NOT include that zone's operators in the results.

### Requirement 4: Adapter Metadata Retrieval

**User Story:** As a PARKING_APP, I want to retrieve adapter metadata for a specific operator, so that I can securely download and verify the correct container image via UPDATE_SERVICE.

#### Acceptance Criteria

1. **05-REQ-4.1** WHEN a client sends a `GET /operators/{id}/adapter` request with a known operator ID, THE PARKING_FEE_SERVICE SHALL return HTTP 200 with a JSON object containing `image_ref` (OCI image reference), `checksum_sha256` (SHA-256 checksum string), and `version` (version string).
2. **05-REQ-4.2** THE PARKING_FEE_SERVICE SHALL return a `checksum_sha256` value that starts with `sha256:` followed by 64 hexadecimal characters.

#### Edge Cases

1. **05-REQ-4.E1** IF the operator ID does not match any known operator, THEN THE PARKING_FEE_SERVICE SHALL return HTTP 404 with a JSON error body containing a descriptive `error` message.

### Requirement 5: Health Check

**User Story:** As an operations team, I want a health check endpoint, so that I can monitor whether the service is running and responsive.

#### Acceptance Criteria

1. **05-REQ-5.1** WHEN a client sends a `GET /health` request, THE PARKING_FEE_SERVICE SHALL return HTTP 200 with a JSON object containing `"status": "ok"`.

#### Edge Cases

1. **05-REQ-5.E1** THE PARKING_FEE_SERVICE SHALL respond to `GET /health` regardless of query parameters or headers present on the request.

### Requirement 6: Rate Information

**User Story:** As a PARKING_APP, I want to know the rate type and amount for each operator, so that I can display pricing to the driver.

#### Acceptance Criteria

1. **05-REQ-6.1** THE PARKING_FEE_SERVICE SHALL support two rate types: `per_hour` and `flat_fee`.
2. **05-REQ-6.2** WHEN returning operator information, THE PARKING_FEE_SERVICE SHALL include `rate_type` (either `per_hour` or `flat_fee`), `rate_amount` (numeric value), and `rate_currency` (e.g., `EUR`).

### Requirement 7: Configuration Loading

**User Story:** As a developer, I want operator/zone data and service settings to be loaded from a configuration file, so that I can modify the demo data without changing code.

#### Acceptance Criteria

1. **05-REQ-7.1** WHEN the service starts, THE PARKING_FEE_SERVICE SHALL load zones, operators, and service settings from a JSON configuration file.
2. **05-REQ-7.2** THE PARKING_FEE_SERVICE SHALL use a default configuration embedded in the binary if no external configuration file is specified.

#### Edge Cases

1. **05-REQ-7.E1** IF the specified configuration file does not exist or contains invalid JSON, THEN THE PARKING_FEE_SERVICE SHALL exit with a non-zero exit code and a descriptive error message on stderr.

### Requirement 8: Error Responses

**User Story:** As a PARKING_APP developer, I want consistent error responses, so that I can handle errors programmatically.

#### Acceptance Criteria

1. **05-REQ-8.1** THE PARKING_FEE_SERVICE SHALL return well-formed JSON error responses with an `error` string field for all error conditions.
2. **05-REQ-8.2** THE PARKING_FEE_SERVICE SHALL set the `Content-Type` header to `application/json` for all API responses (success and error).
3. **05-REQ-8.3** THE PARKING_FEE_SERVICE SHALL use HTTP status codes: 400 for invalid parameters, 404 for unknown resources, and 500 for unexpected internal errors.

#### Edge Cases

1. **05-REQ-8.E1** IF a request is made to an undefined route, THEN THE PARKING_FEE_SERVICE SHALL return HTTP 404 with a JSON error body.
2. **05-REQ-8.E2** IF an unexpected panic occurs during request processing, THEN THE PARKING_FEE_SERVICE SHALL recover and return HTTP 500 with a JSON error body rather than dropping the connection.

## Traceability

| Requirement | PRD Section |
|-------------|-------------|
| 05-REQ-1 | REST API Endpoints: operator lookup by location |
| 05-REQ-2 | Geofence Matching |
| 05-REQ-3 | Geofence Matching: proximity threshold |
| 05-REQ-4 | REST API Endpoints: adapter metadata; Adapter Metadata |
| 05-REQ-5 | REST API Endpoints: health check |
| 05-REQ-6 | Rate Model |
| 05-REQ-7 | Configuration |
| 05-REQ-8 | Error handling, response format |
