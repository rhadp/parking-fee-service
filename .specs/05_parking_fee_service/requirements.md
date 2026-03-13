# Requirements Document

## Introduction

This document specifies the requirements for the PARKING_FEE_SERVICE component (Phase 2.4) of the SDV Parking Demo System. The PARKING_FEE_SERVICE is a cloud-based Go REST API providing parking operator discovery via geofence-based location matching and adapter metadata retrieval. It uses in-memory configuration loaded from a JSON file at startup, with no external database dependencies.

## Glossary

- **PARKING_FEE_SERVICE:** A Go REST API for discovering parking operators by location and retrieving adapter metadata.
- **PARKING_OPERATOR:** A parking service provider associated with a geographic zone.
- **Geofence:** A geographic polygon defined by latitude/longitude coordinates that represents an operator's coverage zone.
- **Point-in-polygon:** A geometric test that determines whether a coordinate lies inside a polygon (ray casting algorithm).
- **Proximity threshold:** A configurable distance in meters; coordinates outside a polygon but within this distance from its nearest edge are treated as matches.
- **Haversine distance:** A formula for calculating great-circle distance between two points on a sphere using latitude/longitude.
- **Adapter metadata:** Information needed to download a PARKING_OPERATOR_ADAPTOR: OCI image reference, SHA-256 checksum, and version string.
- **OCI image reference:** A container image address in the format `registry/repository:tag` (e.g., `us-docker.pkg.dev/project/repo/adapter:v1`).
- **Rate type:** Either `per-hour` (hourly charge) or `flat-fee` (fixed charge per session).
- **Zone:** A named geographic area defined by a geofence polygon, associated with one or more operators.

## Requirements

### Requirement 1: Operator Lookup by Location

**User Story:** As a PARKING_APP, I want to find available parking operators for my current location, so that I can offer the driver relevant parking options.

#### Acceptance Criteria

1. [05-REQ-1.1] WHEN a GET request is made to `/operators` with query parameters `lat` and `lon`, THE service SHALL return a JSON array of operators whose zones contain or are near the given coordinates.
2. [05-REQ-1.2] THE service SHALL match coordinates that fall inside a zone's geofence polygon using point-in-polygon ray casting.
3. [05-REQ-1.3] THE service SHALL match coordinates that are outside a zone's geofence polygon but within the configured proximity threshold distance from the nearest polygon edge.
4. [05-REQ-1.4] WHEN multiple operators serve matching zones, THE service SHALL return all matching operators in the response array.
5. [05-REQ-1.5] WHEN no operators match the given coordinates, THE service SHALL return an empty JSON array `[]` with HTTP 200.

#### Edge Cases

1. [05-REQ-1.E1] IF the `lat` or `lon` query parameter is missing, THEN THE service SHALL return HTTP 400 with `{"error":"lat and lon query parameters are required"}`.
2. [05-REQ-1.E2] IF `lat` is not in the range [-90, 90] or `lon` is not in the range [-180, 180], THEN THE service SHALL return HTTP 400 with `{"error":"invalid coordinates"}`.
3. [05-REQ-1.E3] IF `lat` or `lon` cannot be parsed as a floating-point number, THEN THE service SHALL return HTTP 400 with `{"error":"invalid coordinates"}`.

### Requirement 2: Adapter Metadata Retrieval

**User Story:** As a PARKING_APP, I want to retrieve adapter metadata for a specific operator, so that I can trigger adapter download via UPDATE_SERVICE.

#### Acceptance Criteria

1. [05-REQ-2.1] WHEN a GET request is made to `/operators/{id}/adapter`, THE service SHALL return a JSON object containing `image_ref`, `checksum_sha256`, and `version` for the specified operator's adapter.
2. [05-REQ-2.2] THE response SHALL include HTTP 200 status code on success.

#### Edge Cases

1. [05-REQ-2.E1] IF the operator `{id}` does not exist, THEN THE service SHALL return HTTP 404 with `{"error":"operator not found"}`.

### Requirement 3: Health Check

**User Story:** As an operator, I want a health check endpoint, so that I can monitor service availability.

#### Acceptance Criteria

1. [05-REQ-3.1] WHEN a GET request is made to `/health`, THE service SHALL return HTTP 200 with `{"status":"ok"}`.

### Requirement 4: Configuration Loading

**User Story:** As a developer, I want the service to load operator/zone data from a JSON config file, so that I can modify demo data without code changes.

#### Acceptance Criteria

1. [05-REQ-4.1] WHEN the service starts, THE service SHALL load configuration from the file path specified by the `CONFIG_PATH` environment variable, defaulting to `config.json` in the working directory.
2. [05-REQ-4.2] THE configuration SHALL include a proximity threshold in meters, server port, list of zones with geofence polygons, and list of operators with zone associations and adapter metadata.
3. [05-REQ-4.3] THE service SHALL use the configured proximity threshold for near-zone matching.

#### Edge Cases

1. [05-REQ-4.E1] IF the configuration file does not exist, THEN THE service SHALL start with built-in default data (Munich demo data) and log a warning.
2. [05-REQ-4.E2] IF the configuration file contains invalid JSON, THEN THE service SHALL exit with a non-zero code and log a descriptive error.

### Requirement 5: Response Format

**User Story:** As a PARKING_APP developer, I want consistent JSON response formats, so that I can reliably parse API responses.

#### Acceptance Criteria

1. [05-REQ-5.1] THE service SHALL set `Content-Type: application/json` on all responses.
2. [05-REQ-5.2] THE operator lookup response SHALL include for each operator: `id`, `name`, `zone_id`, and `rate` (containing `type`, `amount`, `currency`).
3. [05-REQ-5.3] THE error responses SHALL use the format `{"error":"<message>"}`.

### Requirement 6: Graceful Lifecycle

**User Story:** As an operator, I want the service to start and stop cleanly.

#### Acceptance Criteria

1. [05-REQ-6.1] WHEN the service starts, THE service SHALL log its version, configured port, number of loaded zones and operators, and a ready message.
2. [05-REQ-6.2] WHEN the service receives SIGTERM or SIGINT, THE service SHALL gracefully shut down the HTTP server and exit with code 0.
