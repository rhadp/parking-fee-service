# Design: PARKING_FEE_SERVICE (Spec 05)

> Design document for the PARKING_FEE_SERVICE cloud REST API.
> Implements requirements from `.specs/05_parking_fee_service/requirements.md`.

## Architecture Overview

The PARKING_FEE_SERVICE is a standalone Go HTTP server providing REST endpoints for parking operator discovery and adapter metadata retrieval. It uses an in-memory data store with hardcoded demo data, a geofence engine for spatial matching, and standard library HTTP handlers.

```
                     +----------------------------+
                     |   PARKING_FEE_SERVICE      |
                     |   (Go HTTP server :8080)   |
                     +----------------------------+
                     |                            |
  GET /operators?    |  +---------------------+   |
  lat=..&lon=..  --->|  | Operator Handler    |   |
                     |  +---------------------+   |
                     |          |                  |
  GET /operators/    |  +---------------------+   |
  {id}/adapter   --->|  | Adapter Handler     |   |
                     |  +---------------------+   |
                     |          |                  |
  GET /health    --->|  | Health Handler      |   |
                     |  +---------------------+   |
                     |          |                  |
                     |  +---------------------+   |
                     |  | Geofence Engine     |   |
                     |  +---------------------+   |
                     |          |                  |
                     |  +---------------------+   |
                     |  | In-Memory Store     |   |
                     |  | (Operators, Zones,  |   |
                     |  |  Adapter Metadata)  |   |
                     |  +---------------------+   |
                     +----------------------------+
```

## Module Structure

```
backend/parking-fee-service/
  go.mod
  go.sum
  main.go                  # Server entry point, router setup
  handler.go               # HTTP handler functions
  handler_test.go          # Handler tests (integration-level via httptest)
  store.go                 # In-memory data store and demo data
  store_test.go            # Store unit tests
  geofence.go              # Point-in-polygon and near-zone logic
  geofence_test.go         # Geofence engine unit tests
  model.go                 # Data model types
```

## API Endpoint Specifications

### GET /operators?lat={lat}&lon={lon}

Looks up parking operators whose geofence zone contains or is near the given coordinates.

**Request:**

| Parameter | Type   | Required | Constraints                |
|-----------|--------|----------|----------------------------|
| lat       | float  | yes      | [-90, 90]                  |
| lon       | float  | yes      | [-180, 180]                |

**Response (200 OK):**

```json
[
  {
    "operator_id": "muc-central",
    "name": "Munich Central Parking",
    "zone": [
      {"lat": 48.1400, "lon": 11.5600},
      {"lat": 48.1400, "lon": 11.5900},
      {"lat": 48.1300, "lon": 11.5900},
      {"lat": 48.1300, "lon": 11.5600}
    ],
    "rate": "2.50 EUR/hr"
  }
]
```

Returns an empty array `[]` if no operators match.

**Error Responses:**

| Status | Condition                                    |
|--------|----------------------------------------------|
| 400    | Missing, non-numeric, or out-of-range lat/lon |

### GET /operators/{id}/adapter

Returns adapter metadata for the specified operator.

**Response (200 OK):**

```json
{
  "image_ref": "europe-west1-docker.pkg.dev/rhadp-parking-demo/adapters/muc-central:v1.0.0",
  "checksum_sha256": "sha256:a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2",
  "version": "v1.0.0"
}
```

**Error Responses:**

| Status | Condition             |
|--------|-----------------------|
| 404    | Unknown operator ID   |

### GET /health

Returns service health status.

**Response (200 OK):**

```json
{
  "status": "ok"
}
```

## Geofence Engine

### Point-in-Polygon Algorithm

The geofence engine uses the **ray-casting algorithm** to determine whether a point lies inside a polygon:

1. Cast a ray from the test point horizontally to the right (positive X direction).
2. Count the number of times the ray crosses polygon edges.
3. If the count is odd, the point is inside; if even, the point is outside.

Points on the polygon boundary are treated as inside.

### Near-Zone Buffer (Fuzziness)

To support the "near a zone counts as a match" requirement:

1. Define a configurable buffer distance (default: **500 meters**).
2. For points that fall outside the polygon, compute the minimum distance from the point to the nearest polygon edge.
3. If the minimum distance is less than or equal to the buffer distance, treat the point as a match.

Distance calculation uses the **Haversine formula** for geodesic distance between two lat/lon coordinate pairs.

### Implementation Constants

```go
const (
    DefaultBufferMeters = 500.0     // Near-zone buffer distance in meters
    EarthRadiusMeters   = 6371000.0 // Mean Earth radius in meters
)
```

## Data Model

### Operator

```go
type Operator struct {
    ID   string   `json:"operator_id"`
    Name string   `json:"name"`
    Zone []LatLon `json:"zone"`
    Rate string   `json:"rate"`
}
```

### LatLon

```go
type LatLon struct {
    Lat float64 `json:"lat"`
    Lon float64 `json:"lon"`
}
```

### AdapterMetadata

```go
type AdapterMetadata struct {
    ImageRef       string `json:"image_ref"`
    ChecksumSHA256 string `json:"checksum_sha256"`
    Version        string `json:"version"`
}
```

### ErrorResponse

```go
type ErrorResponse struct {
    Error string `json:"error"`
}
```

## Hardcoded Demo Data

Two operators with realistic Munich area geofence polygons:

### Operator 1: Munich Central Parking

- **ID:** `muc-central`
- **Name:** Munich Central Parking
- **Rate:** 2.50 EUR/hr
- **Zone polygon** (area around Munich Hauptbahnhof / central station):
  - (48.1420, 11.5550)
  - (48.1420, 11.5700)
  - (48.1370, 11.5700)
  - (48.1370, 11.5550)
  - (48.1420, 11.5550) *(closed polygon, first point repeated implicitly)*
- **Adapter:**
  - `image_ref`: `europe-west1-docker.pkg.dev/rhadp-parking-demo/adapters/muc-central:v1.0.0`
  - `checksum_sha256`: `sha256:a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2`
  - `version`: `v1.0.0`

### Operator 2: Munich Airport Parking

- **ID:** `muc-airport`
- **Name:** Munich Airport Parking
- **Rate:** 3.00 EUR/hr
- **Zone polygon** (area around Munich Airport / Flughafen):
  - (48.3570, 11.7750)
  - (48.3570, 11.7950)
  - (48.3480, 11.7950)
  - (48.3480, 11.7750)
  - (48.3570, 11.7750) *(closed polygon)*
- **Adapter:**
  - `image_ref`: `europe-west1-docker.pkg.dev/rhadp-parking-demo/adapters/muc-airport:v1.0.0`
  - `checksum_sha256`: `sha256:f6e5d4c3b2a1f6e5d4c3b2a1f6e5d4c3b2a1f6e5d4c3b2a1f6e5d4c3b2a1f6e5`
  - `version`: `v1.0.0`

## Correctness Properties

| ID | Property | Description |
|----|----------|-------------|
| CP-1 | Geofence accuracy | A coordinate known to be inside a polygon MUST be classified as inside. A coordinate known to be outside and beyond the buffer distance MUST NOT match. |
| CP-2 | Near-zone consistency | A coordinate within the buffer distance of a polygon edge MUST match. The buffer distance is applied uniformly to all polygon edges. |
| CP-3 | Response format consistency | Every API response MUST have `Content-Type: application/json`. Success responses return the resource; error responses return `{"error": "..."}`. |
| CP-4 | Operator-adapter mapping | Every operator in the store MUST have exactly one associated adapter metadata entry. `GET /operators/{id}/adapter` MUST return metadata for any operator that appears in lookup results. |
| CP-5 | Parameter validation | Invalid or missing `lat`/`lon` parameters MUST result in HTTP 400 before any geofence computation occurs. |
| CP-6 | Complete result set | The operator lookup MUST return ALL operators whose zone matches (inside or near), not just the first match. |
| CP-7 | Idempotent reads | All endpoints are GET requests. Repeated identical requests MUST return identical responses (the data store is static). |

## Error Handling

| Scenario | HTTP Status | Error Message Example |
|----------|-------------|----------------------|
| Missing `lat` parameter | 400 | `"missing required parameter: lat"` |
| Missing `lon` parameter | 400 | `"missing required parameter: lon"` |
| Non-numeric `lat` | 400 | `"invalid lat: must be a number"` |
| Non-numeric `lon` | 400 | `"invalid lon: must be a number"` |
| `lat` out of range | 400 | `"invalid lat: must be between -90 and 90"` |
| `lon` out of range | 400 | `"invalid lon: must be between -180 and 180"` |
| Unknown operator ID | 404 | `"operator not found: xyz"` |
| Undefined route | 404 | `"not found"` |
| Internal server error | 500 | `"internal server error"` |

## Technology Stack

| Component | Choice |
|-----------|--------|
| Language | Go 1.22+ |
| HTTP framework | `net/http` standard library (with `http.ServeMux` pattern matching from Go 1.22) |
| Router | `net/http.ServeMux` with method and path pattern matching |
| JSON | `encoding/json` standard library |
| Testing | `testing` standard library, `net/http/httptest` |
| Port | 8080 |
| External dependencies | None (standard library only) |

## Definition of Done

1. All endpoints (`GET /operators`, `GET /operators/{id}/adapter`, `GET /health`) are implemented and return correct responses.
2. Geofence engine correctly classifies points as inside, near, or outside operator zones.
3. All error cases return appropriate HTTP status codes with JSON error bodies.
4. All unit tests pass: `cd backend/parking-fee-service && go test ./... -v`.
5. All linting passes: `cd backend/parking-fee-service && go vet ./...`.
6. Hardcoded demo data contains at least 2 operators with realistic Munich area geofence polygons.
7. The service starts on port 8080 and responds to HTTP requests.

## Testing Strategy

### Unit Tests

- **Geofence engine:** Test point-in-polygon with known inside, outside, boundary, and near-zone coordinates. Use table-driven tests for multiple polygons and points.
- **Store:** Verify operator lookup and adapter metadata retrieval for known and unknown IDs.

### Integration Tests (via httptest)

- **Handler tests:** Use `httptest.NewServer` or `httptest.NewRecorder` to exercise full HTTP request/response cycle for each endpoint.
- **Validate:** Status codes, response bodies, Content-Type headers, and error formats.

### Property Tests

- **Geofence property:** For any polygon vertex, that vertex must be classified as inside or on-boundary.
- **Idempotency:** Repeated requests with the same parameters must return the same response.
