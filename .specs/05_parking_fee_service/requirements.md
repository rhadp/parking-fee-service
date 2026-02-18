# Requirements Document: PARKING_FEE_SERVICE

## Introduction

This specification defines the PARKING_FEE_SERVICE, a Go backend service that
provides a REST API for parking zone/operator lookup by geographic location
and adapter metadata retrieval. It enables the PARKING_APP to discover which
parking operators are available at the vehicle's current location and obtain
the adapter container image information needed for installation via
UPDATE_SERVICE.

## Glossary

| Term | Definition |
|------|-----------|
| Adapter metadata | Container image reference and checksum needed to install a PARKING_OPERATOR_ADAPTOR |
| Fuzzy match | Location lookup that returns zones within a configurable radius even if the point is outside the geofence polygon |
| Geofence | A polygon of geographic coordinates defining a parking zone boundary |
| Haversine distance | Formula for calculating the great-circle distance between two points on a sphere |
| PARKING_FEE_SERVICE | Go backend providing zone lookup and adapter metadata via REST API |
| Point-in-polygon | Algorithm that determines whether a geographic point lies inside a polygon |
| Seed data | Hardcoded demo zone data loaded on service startup |
| Zone | A parking area defined by a geofence polygon, associated with one operator and one adapter |

## Requirements

### Requirement 1: Zone Lookup by Location

**User Story:** As the PARKING_APP, I want to find available parking zones
near my current location, so that I can show the driver which parking
operators are available.

#### Acceptance Criteria

1. **05-REQ-1.1** WHEN the PARKING_FEE_SERVICE receives
   `GET /api/v1/zones?lat={latitude}&lon={longitude}`, THE service SHALL
   return a JSON array of zones matching or near the specified location.

2. **05-REQ-1.2** THE service SHALL perform a point-in-polygon test for each
   zone's geofence polygon. IF the point is inside a polygon, THEN that zone
   SHALL be included in the results with `distance_meters = 0`.

3. **05-REQ-1.3** IF no zone polygon contains the point, THEN the service
   SHALL find zones whose nearest polygon edge is within 200 meters of the
   point using Haversine distance calculation, and include those zones with
   their calculated `distance_meters`.

4. **05-REQ-1.4** THE response SHALL include for each matching zone:
   `zone_id`, `name`, `operator_name`, `rate_type`, `rate_amount`, `currency`,
   and `distance_meters`.

5. **05-REQ-1.5** THE results SHALL be sorted by `distance_meters` ascending
   (nearest first).

#### Edge Cases

1. **05-REQ-1.E1** IF no zones match (no polygon contains the point AND no
   zone is within 200m), THEN the service SHALL return an empty JSON array
   `[]` with HTTP 200 (not an error).

2. **05-REQ-1.E2** IF the `lat` or `lon` query parameters are missing or
   non-numeric, THEN the service SHALL return HTTP 400 with an error message.

---

### Requirement 2: Zone Details

**User Story:** As the PARKING_APP, I want to get full details of a specific
parking zone, so that I can display rate and operator information to the
driver.

#### Acceptance Criteria

1. **05-REQ-2.1** WHEN the PARKING_FEE_SERVICE receives
   `GET /api/v1/zones/{zone_id}`, THE service SHALL return the full zone
   details as JSON.

2. **05-REQ-2.2** THE response SHALL include: `zone_id`, `name`,
   `operator_name`, `rate_type`, `rate_amount`, `currency`, and `polygon`
   (array of `{latitude, longitude}` coordinates).

#### Edge Cases

1. **05-REQ-2.E1** IF the `zone_id` is unknown, THEN the service SHALL
   return HTTP 404 with an error message.

---

### Requirement 3: Adapter Metadata

**User Story:** As the PARKING_APP, I want to get the adapter container image
reference and checksum for a parking zone, so that I can request
UPDATE_SERVICE to install the correct adapter.

#### Acceptance Criteria

1. **05-REQ-3.1** WHEN the PARKING_FEE_SERVICE receives
   `GET /api/v1/zones/{zone_id}/adapter`, THE service SHALL return the
   adapter metadata as JSON.

2. **05-REQ-3.2** THE response SHALL include: `zone_id`, `image_ref`,
   and `checksum`.

#### Edge Cases

1. **05-REQ-3.E1** IF the `zone_id` is unknown, THEN the service SHALL
   return HTTP 404 with an error message.

---

### Requirement 4: Demo Zone Data

**User Story:** As a developer, I want realistic hardcoded parking zones for
the demo, so that the system works with plausible Munich location data.

#### Acceptance Criteria

1. **05-REQ-4.1** THE PARKING_FEE_SERVICE SHALL include at least 3 hardcoded
   parking zones with realistic Munich coordinates.

2. **05-REQ-4.2** EACH zone SHALL have a geofence polygon with at least 4
   coordinate points forming a closed area.

3. **05-REQ-4.3** EACH zone SHALL have rate configuration (`rate_type`,
   `rate_amount`, `currency`) and adapter metadata (`image_ref`, `checksum`).

4. **05-REQ-4.4** THE zones SHALL be loaded into the in-memory store on
   service startup without requiring external data sources.

#### Edge Cases

1. **05-REQ-4.E1** IF the seed data is malformed (e.g., polygon with fewer
   than 3 points), THEN the service SHALL log a warning and skip that zone.

---

### Requirement 5: Service Configuration and Health

**User Story:** As a developer, I want the PARKING_FEE_SERVICE to be
configurable and expose a health check, so that it integrates with the
existing infrastructure.

#### Acceptance Criteria

1. **05-REQ-5.1** THE PARKING_FEE_SERVICE SHALL accept a `--listen-addr`
   flag (default: `:8080`) for the REST listen address.

2. **05-REQ-5.2** THE service SHALL expose `GET /healthz` returning HTTP 200
   with an empty JSON object.

3. **05-REQ-5.3** THE service SHALL NOT require authentication for any
   endpoint.

4. **05-REQ-5.4** THE service SHALL log all incoming requests at INFO level.

---

### Requirement 6: Mock PARKING_APP CLI Extension

**User Story:** As a developer, I want the mock parking-app-cli to have
subcommands for PARKING_FEE_SERVICE, so that I can test the zone lookup and
adapter discovery workflow without a real Android app.

#### Acceptance Criteria

1. **05-REQ-6.1** THE mock `parking-app-cli` SHALL implement a
   `lookup-zones --lat <lat> --lon <lon>` subcommand that calls
   `GET /api/v1/zones?lat={lat}&lon={lon}` and prints the results.

2. **05-REQ-6.2** THE mock `parking-app-cli` SHALL implement a
   `zone-info --zone-id <id>` subcommand that calls
   `GET /api/v1/zones/{zone_id}` and prints the zone details.

3. **05-REQ-6.3** THE mock `parking-app-cli` SHALL implement an
   `adapter-info --zone-id <id>` subcommand that calls
   `GET /api/v1/zones/{zone_id}/adapter` and prints the adapter metadata.

4. **05-REQ-6.4** THE mock CLI SHALL accept a `--parking-fee-service-addr`
   flag (default: `http://localhost:8080`) for the PARKING_FEE_SERVICE
   address.

#### Edge Cases

1. **05-REQ-6.E1** IF the PARKING_FEE_SERVICE is unreachable, THEN the CLI
   SHALL print an error and exit with a non-zero exit code.

---

### Requirement 7: Integration Verification

**User Story:** As a developer, I want integration tests that verify the
end-to-end adapter discovery flow from zone lookup to adapter installation,
so that I can validate the PARKING_FEE_SERVICE integrates correctly with the
QM partition services.

#### Acceptance Criteria

1. **05-REQ-7.1** THE integration test SHALL verify: calling
   `GET /api/v1/zones?lat={lat}&lon={lon}` with coordinates inside a demo
   zone returns that zone in the results.

2. **05-REQ-7.2** THE integration test SHALL verify: calling
   `GET /api/v1/zones?lat={lat}&lon={lon}` with coordinates near (within
   200m) but outside a demo zone returns that zone with a non-zero
   `distance_meters`.

3. **05-REQ-7.3** THE integration test SHALL verify: the adapter metadata
   from `GET /api/v1/zones/{zone_id}/adapter` can be used to call
   `UPDATE_SERVICE.InstallAdapter(image_ref, checksum)` successfully.

4. **05-REQ-7.4** THE integration test SHALL verify the full discovery flow:
   lookup zones → select zone → get adapter metadata → install adapter →
   verify adapter state is RUNNING.

#### Edge Cases

1. **05-REQ-7.E1** IF required infrastructure is unavailable (e.g.,
   UPDATE_SERVICE or podman not running), THEN the integration test SHALL
   skip with a clear message.
