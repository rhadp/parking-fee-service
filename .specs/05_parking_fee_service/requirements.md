# Requirements Document: Parking Fee Service (Phase 2.4)

## Introduction

This document specifies the requirements for the PARKING_FEE_SERVICE, the
cloud-based backend service responsible for parking operator discovery and
adapter provisioning metadata. The service provides a REST API that enables
vehicle systems to discover parking operators by GPS location using geofence
polygon matching, and to retrieve adapter metadata (OCI image reference,
checksum, version) for secure adapter installation.

This document also covers enhancements to the mock PARKING_APP CLI to support
operator lookup and adapter metadata retrieval via the PARKING_FEE_SERVICE.

## Glossary

| Term | Definition |
|------|-----------|
| Geofence | A virtual geographic boundary defined by a polygon of lat/lon vertices. Used to determine whether a vehicle is within a parking operator's service area. |
| Point-in-polygon | A computational geometry algorithm that determines whether a given point lies inside a polygon. The service uses the ray-casting algorithm. |
| Fuzziness buffer | A configurable distance (in meters) around a geofence polygon boundary. Points within this buffer distance of the polygon are treated as matches, even if they fall outside the polygon itself. |
| Operator | A parking operator (e.g., a city parking authority or private parking company) that manages parking zones and accepts parking fee payments. |
| Adapter metadata | Information needed to install a parking operator's adapter: OCI image reference, SHA-256 checksum, and version string. |
| OCI image reference | A fully-qualified reference to a container image in an OCI-compliant registry (e.g., `us-docker.pkg.dev/project/repo/adapter:v1.0`). |
| Bearer token | A simple authentication token sent in the HTTP `Authorization` header. Used for demo-scope access control. |
| Zone | A geographic area served by a parking operator, defined by a geofence polygon with associated parking rates. |

## Requirements

### Requirement 1: Operator Lookup by Location

**User Story:** As a vehicle parking system, I want to query available parking
operators by GPS coordinates, so that I can discover which operators serve the
vehicle's current location.

#### Acceptance Criteria

1. WHEN a client sends `GET /operators?lat={lat}&lon={lon}` with valid
   coordinates, THE service SHALL return a JSON array of operators whose zones
   contain or are near the given point. `05-REQ-1.1`
2. THE response body SHALL include for each matched operator: operator ID,
   operator name, zone ID, zone name, and hourly rate with currency.
   `05-REQ-1.2`
3. WHEN multiple operators have zones covering the same point, THE service
   SHALL return all matching operators in the response array. `05-REQ-1.3`
4. THE response SHALL use HTTP status 200 and Content-Type
   `application/json`. `05-REQ-1.4`

#### Edge Cases

1. IF no operators match the given coordinates, THEN the service SHALL return
   HTTP 200 with an empty JSON array `[]`. `05-REQ-1.E1`
2. IF the `lat` or `lon` query parameter is missing, THEN the service SHALL
   return HTTP 400 with a JSON error body containing a descriptive message.
   `05-REQ-1.E2`
3. IF the `lat` or `lon` query parameter is not a valid floating-point number,
   THEN the service SHALL return HTTP 400 with a JSON error body containing
   a descriptive message. `05-REQ-1.E3`
4. IF the `lat` value is outside the range [-90, 90] or the `lon` value is
   outside the range [-180, 180], THEN the service SHALL return HTTP 400 with
   a JSON error body describing the valid coordinate ranges. `05-REQ-1.E4`

---

### Requirement 2: Geofence Polygon Matching

**User Story:** As the parking fee service, I want to determine whether a GPS
point falls inside a geofence polygon, so that I can accurately match vehicles
to parking zones.

#### Acceptance Criteria

1. THE service SHALL use a point-in-polygon algorithm (ray-casting) to
   determine whether a coordinate falls within a zone's geofence polygon.
   `05-REQ-2.1`
2. THE geofence polygons SHALL be defined as ordered lists of lat/lon vertex
   pairs forming closed polygons (first and last vertex are implicitly
   connected). `05-REQ-2.2`
3. THE service SHALL support polygons with 3 or more vertices. `05-REQ-2.3`

#### Edge Cases

1. IF an operator's zone polygon has fewer than 3 vertices, THEN the service
   SHALL skip that zone during matching and not return it in results.
   `05-REQ-2.E1`

---

### Requirement 3: Near-Zone Fuzziness

**User Story:** As a vehicle parking system, I want operators to be matched
even when the vehicle is near but not exactly inside a zone boundary, so that
slight GPS inaccuracy does not prevent operator discovery.

#### Acceptance Criteria

1. THE service SHALL support a configurable fuzziness buffer distance (in
   meters) applied around each geofence polygon boundary. `05-REQ-3.1`
2. WHEN a point falls outside a polygon but within the fuzziness buffer
   distance of the polygon boundary, THE service SHALL include the
   corresponding operator in the results. `05-REQ-3.2`
3. THE default fuzziness buffer SHALL be 100 meters. `05-REQ-3.3`
4. THE fuzziness buffer distance SHALL be configurable via an environment
   variable `FUZZINESS_METERS`. `05-REQ-3.4`

#### Edge Cases

1. IF the fuzziness buffer is set to 0, THEN only exact point-in-polygon
   matches SHALL be returned (no near-zone matching). `05-REQ-3.E1`

---

### Requirement 4: Adapter Metadata Retrieval

**User Story:** As a vehicle parking system, I want to retrieve adapter
metadata for a specific operator, so that I can securely install the
correct adapter via UPDATE_SERVICE.

#### Acceptance Criteria

1. WHEN a client sends `GET /operators/{id}/adapter` with a valid operator ID,
   THE service SHALL return a JSON object containing the adapter's OCI image
   reference, SHA-256 checksum, and version. `05-REQ-4.1`
2. THE response body SHALL contain the fields: `image_ref` (string),
   `checksum_sha256` (string), and `version` (string). `05-REQ-4.2`
3. THE response SHALL use HTTP status 200 and Content-Type
   `application/json`. `05-REQ-4.3`

#### Edge Cases

1. IF the operator ID does not match any configured operator, THEN the service
   SHALL return HTTP 404 with a JSON error body containing a descriptive
   message. `05-REQ-4.E1`

---

### Requirement 5: Health Check Endpoint

**User Story:** As an operations engineer, I want a health check endpoint, so
that I can monitor the service's availability.

#### Acceptance Criteria

1. WHEN a client sends `GET /health`, THE service SHALL return HTTP 200 with
   a JSON body `{"status": "ok"}`. `05-REQ-5.1`
2. THE health check endpoint SHALL NOT require authentication. `05-REQ-5.2`

#### Edge Cases

(none)

---

### Requirement 6: Operator Data Configuration

**User Story:** As a service operator, I want parking operator data loaded from
a JSON configuration file, so that I can modify zone definitions and rates
without recompiling.

#### Acceptance Criteria

1. THE service SHALL load operator data (operator records, zone polygons,
   rates, adapter metadata) from an embedded default configuration or an
   external JSON file. `05-REQ-6.1`
2. THE operator data SHALL include at least two realistic demo operators with
   different zones (e.g., Munich city center, Munich airport). `05-REQ-6.2`
3. THE JSON configuration file path SHALL be configurable via the environment
   variable `OPERATORS_CONFIG`. `05-REQ-6.3`
4. IF the `OPERATORS_CONFIG` environment variable is not set, THEN the service
   SHALL use an embedded default operator dataset. `05-REQ-6.4`

#### Edge Cases

1. IF the configured JSON file does not exist or is malformed, THEN the
   service SHALL fail to start with a clear error message identifying the
   problem. `05-REQ-6.E1`

---

### Requirement 7: Authentication

**User Story:** As the parking fee service, I want to require bearer token
authentication on operator endpoints, so that only authorized clients can
discover operators and retrieve adapter metadata.

#### Acceptance Criteria

1. THE `/operators` and `/operators/{id}/adapter` endpoints SHALL require a
   valid bearer token in the `Authorization` header. `05-REQ-7.1`
2. THE service SHALL validate the bearer token against a configured set of
   accepted tokens. `05-REQ-7.2`
3. THE set of accepted tokens SHALL be configurable via the environment
   variable `AUTH_TOKENS` (comma-separated list). `05-REQ-7.3`

#### Edge Cases

1. IF the `Authorization` header is missing, THEN the service SHALL return
   HTTP 401 with a JSON error body `{"error": "missing authorization header"}`.
   `05-REQ-7.E1`
2. IF the bearer token is not in the configured set of accepted tokens, THEN
   the service SHALL return HTTP 401 with a JSON error body
   `{"error": "invalid token"}`. `05-REQ-7.E2`
3. IF the `Authorization` header does not use the `Bearer` scheme, THEN the
   service SHALL return HTTP 401 with a JSON error body
   `{"error": "invalid authorization scheme"}`. `05-REQ-7.E3`
