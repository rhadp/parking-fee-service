# Design Document: PARKING_FEE_SERVICE (Spec 05)

## Overview

The PARKING_FEE_SERVICE is a standalone Go HTTP server providing REST endpoints for parking operator discovery and adapter metadata retrieval. It loads zone and operator data from a JSON configuration file at startup, stores it in memory, and uses a geofence engine (ray-casting point-in-polygon plus Haversine-based proximity matching) to resolve location queries. No database or external runtime dependencies are required.

## Architecture

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
                     |  | Config / Store      |   |
                     |  | (Zones, Operators,  |   |
                     |  |  Adapter Metadata,  |   |
                     |  |  Settings)          |   |
                     |  +---------------------+   |
                     +----------------------------+
```

### Module Responsibilities

1. **main.go** -- Server entry point: loads configuration, wires dependencies, registers routes, starts HTTP server.
2. **config.go** -- Configuration loading: reads JSON config file or falls back to embedded default config; exposes typed config structs.
3. **model.go** -- Data model types: `Zone`, `Operator`, `AdapterMetadata`, `LatLon`, `ErrorResponse`, rate type constants.
4. **store.go** -- In-memory data store: holds zones, operators, and adapter metadata; provides lookup methods.
5. **geofence.go** -- Geofence engine: point-in-polygon (ray casting), Haversine distance, proximity matching.
6. **handler.go** -- HTTP handlers: request parsing, validation, response formatting, error handling, recovery middleware.

## Components and Interfaces

### Module Structure

```
backend/parking-fee-service/
  go.mod
  go.sum
  main.go
  config.go
  config_test.go
  model.go
  store.go
  store_test.go
  geofence.go
  geofence_test.go
  handler.go
  handler_test.go
  config.json              # Default configuration file (also embedded)
```

## Data Models

### LatLon

```go
type LatLon struct {
    Lat float64 `json:"lat"`
    Lon float64 `json:"lon"`
}
```

### Zone

```go
type Zone struct {
    ID      string   `json:"id"`
    Name    string   `json:"name"`
    Polygon []LatLon `json:"polygon"`
}
```

### RateType

```go
type RateType string

const (
    RatePerHour RateType = "per_hour"
    RateFlatFee RateType = "flat_fee"
)
```

### Operator

```go
type Operator struct {
    ID           string   `json:"operator_id"`
    Name         string   `json:"name"`
    ZoneID       string   `json:"zone_id"`
    RateType     RateType `json:"rate_type"`
    RateAmount   float64  `json:"rate_amount"`
    RateCurrency string   `json:"rate_currency"`
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

### Configuration File Format (JSON)

```json
{
  "settings": {
    "port": 8080,
    "proximity_threshold_meters": 500
  },
  "zones": [
    {
      "id": "zone-muc-central",
      "name": "Munich Central Station Area",
      "polygon": [
        {"lat": 48.1420, "lon": 11.5550},
        {"lat": 48.1420, "lon": 11.5700},
        {"lat": 48.1370, "lon": 11.5700},
        {"lat": 48.1370, "lon": 11.5550}
      ]
    },
    {
      "id": "zone-muc-airport",
      "name": "Munich Airport Area",
      "polygon": [
        {"lat": 48.3570, "lon": 11.7750},
        {"lat": 48.3570, "lon": 11.7950},
        {"lat": 48.3480, "lon": 11.7950},
        {"lat": 48.3480, "lon": 11.7750}
      ]
    }
  ],
  "operators": [
    {
      "operator_id": "muc-central",
      "name": "Munich Central Parking",
      "zone_id": "zone-muc-central",
      "rate_type": "per_hour",
      "rate_amount": 2.50,
      "rate_currency": "EUR",
      "adapter": {
        "image_ref": "europe-west1-docker.pkg.dev/rhadp-parking-demo/adapters/muc-central:v1.0.0",
        "checksum_sha256": "sha256:a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2",
        "version": "v1.0.0"
      }
    },
    {
      "operator_id": "muc-airport",
      "name": "Munich Airport Parking",
      "zone_id": "zone-muc-airport",
      "rate_type": "flat_fee",
      "rate_amount": 5.00,
      "rate_currency": "EUR",
      "adapter": {
        "image_ref": "europe-west1-docker.pkg.dev/rhadp-parking-demo/adapters/muc-airport:v1.0.0",
        "checksum_sha256": "sha256:f6e5d4c3b2a1f6e5d4c3b2a1f6e5d4c3b2a1f6e5d4c3b2a1f6e5d4c3b2a1f6e5",
        "version": "v1.0.0"
      }
    }
  ]
}
```

### Config Structs

```go
type Config struct {
    Settings  Settings          `json:"settings"`
    Zones     []Zone            `json:"zones"`
    Operators []OperatorConfig  `json:"operators"`
}

type Settings struct {
    Port                     int     `json:"port"`
    ProximityThresholdMeters float64 `json:"proximity_threshold_meters"`
}

type OperatorConfig struct {
    Operator
    Adapter AdapterMetadata `json:"adapter"`
}
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
    "zone_id": "zone-muc-central",
    "rate_type": "per_hour",
    "rate_amount": 2.50,
    "rate_currency": "EUR"
  }
]
```

Returns an empty array `[]` if no operators match.

**Error Responses:**

| Status | Condition                                    | Requirement |
|--------|----------------------------------------------|-------------|
| 400    | Missing, non-numeric, or out-of-range lat/lon | 05-REQ-1.E1, 05-REQ-1.E2 |

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

| Status | Condition             | Requirement |
|--------|-----------------------|-------------|
| 404    | Unknown operator ID   | 05-REQ-4.E1 |

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

Points on the polygon boundary are treated as inside (checked via distance-to-segment with a small epsilon).

### Near-Zone Buffer (Proximity Matching)

To support the "near a zone counts as a match" requirement:

1. Read the proximity threshold from configuration (default: 500 meters).
2. For points that fall outside the polygon, compute the minimum distance from the point to the nearest polygon edge.
3. If the minimum distance is less than or equal to the threshold, treat the point as a match.

Distance calculation uses the **Haversine formula** for geodesic distance between two lat/lon coordinate pairs.

### Implementation Constants

```go
const (
    DefaultProximityThresholdMeters = 500.0     // Default near-zone buffer distance
    EarthRadiusMeters               = 6371000.0 // Mean Earth radius in meters
    BoundaryEpsilonMeters           = 1.0        // Epsilon for boundary detection
)
```

### Key Functions

```go
// PointInPolygon returns true if point is inside the polygon (ray-casting).
func PointInPolygon(point LatLon, polygon []LatLon) bool

// HaversineDistance returns the geodesic distance in meters between two points.
func HaversineDistance(a, b LatLon) float64

// DistanceToSegment returns the minimum distance in meters from a point to a line segment.
func DistanceToSegment(point, segA, segB LatLon) float64

// MinDistanceToPolygon returns the minimum distance from a point to any edge of the polygon.
func MinDistanceToPolygon(point LatLon, polygon []LatLon) float64

// PointInOrNearPolygon returns true if the point is inside the polygon or within thresholdMeters of any edge.
func PointInOrNearPolygon(point LatLon, polygon []LatLon, thresholdMeters float64) bool
```

## Correctness Properties

### Property 1: Geofence Point-in-Polygon Accuracy

*For any* coordinate known to be geometrically inside a convex polygon (e.g., the centroid), `PointInPolygon` SHALL return true. *For any* coordinate known to be far outside (distance > proximity threshold), `PointInOrNearPolygon` SHALL return false.

**Validates: 05-REQ-2.1, 05-REQ-2.2**

### Property 2: Proximity Threshold Consistency

*For any* coordinate outside a polygon but within the configured proximity threshold distance of a polygon edge, `PointInOrNearPolygon` SHALL return true. *For any* coordinate outside a polygon and beyond the proximity threshold distance from all edges, `PointInOrNearPolygon` SHALL return false.

**Validates: 05-REQ-3.1, 05-REQ-3.2, 05-REQ-3.E1**

### Property 3: Response Format Consistency

*For any* API request to any defined endpoint, the response SHALL have `Content-Type: application/json` and a valid JSON body. Success responses return the resource directly; error responses return `{"error": "..."}`.

**Validates: 05-REQ-8.1, 05-REQ-8.2**

### Property 4: Operator-Adapter Integrity

*For any* operator returned by the location lookup endpoint, requesting `GET /operators/{operator_id}/adapter` SHALL return HTTP 200 with a valid adapter metadata object containing non-empty `image_ref`, a `checksum_sha256` matching the pattern `sha256:[0-9a-f]{64}`, and a non-empty `version`.

**Validates: 05-REQ-4.1, 05-REQ-4.2**

### Property 5: Parameter Validation Precedence

*For any* `GET /operators` request with invalid or missing `lat`/`lon` parameters, the service SHALL return HTTP 400 before performing any geofence computation.

**Validates: 05-REQ-1.E1, 05-REQ-1.E2, 05-REQ-8.3**

### Property 6: Complete Result Set

*For any* coordinate that falls inside or near multiple zones, the operator lookup SHALL return ALL operators associated with matching zones, not just the first match.

**Validates: 05-REQ-1.1, 05-REQ-2.E1**

### Property 7: Idempotent Reads

*For any* identical GET request issued multiple times, the service SHALL return identical responses (the data store is static, loaded once at startup).

**Validates: 05-REQ-1.1, 05-REQ-4.1, 05-REQ-5.1**

### Property 8: Configuration Validity

*For any* valid configuration file, all operators SHALL reference an existing zone ID, and all zones SHALL have at least 3 polygon points (minimum to form a closed polygon).

**Validates: 05-REQ-7.1, 05-REQ-7.2**

## Error Handling

| Error Condition | Behavior | Requirement |
|----------------|----------|-------------|
| Missing `lat` parameter | HTTP 400, JSON error | 05-REQ-1.E1 |
| Missing `lon` parameter | HTTP 400, JSON error | 05-REQ-1.E2 |
| Non-numeric `lat` | HTTP 400, JSON error | 05-REQ-1.E1 |
| Non-numeric `lon` | HTTP 400, JSON error | 05-REQ-1.E2 |
| `lat` out of range [-90, 90] | HTTP 400, JSON error | 05-REQ-1.E1 |
| `lon` out of range [-180, 180] | HTTP 400, JSON error | 05-REQ-1.E2 |
| Unknown operator ID | HTTP 404, JSON error | 05-REQ-4.E1 |
| Undefined route | HTTP 404, JSON error | 05-REQ-8.E1 |
| Internal panic | Recover, HTTP 500, JSON error | 05-REQ-8.E2 |
| Config file missing (when specified) | Exit with non-zero code, stderr message | 05-REQ-7.E1 |
| Config file invalid JSON | Exit with non-zero code, stderr message | 05-REQ-7.E1 |

## Technology Stack

| Component | Choice |
|-----------|--------|
| Language | Go 1.22+ |
| HTTP framework | `net/http` standard library (Go 1.22 `ServeMux` pattern matching) |
| Router | `net/http.ServeMux` with method and path pattern matching |
| JSON | `encoding/json` standard library |
| Configuration | JSON file, embedded default via `embed` package |
| Testing | `testing` standard library, `net/http/httptest` |
| Port | 8080 (configurable via config file) |
| External dependencies | None (standard library only) |

## Definition of Done

A task group is complete when ALL of the following are true:

1. All subtasks within the group are checked off (`[x]`)
2. All spec tests (`test_spec.md` entries) for the task group pass
3. All property tests for the task group pass
4. All previously passing tests still pass (no regressions)
5. No linter warnings or errors introduced
6. Code is committed on a feature branch and pushed to remote
7. Feature branch is merged back to `develop`
8. `tasks.md` checkboxes are updated to reflect completion

## Testing Strategy

### Unit Tests

- **Geofence engine** (`geofence_test.go`): Test point-in-polygon with known inside, outside, boundary, and near-zone coordinates. Table-driven tests for multiple polygons and points.
- **Store** (`store_test.go`): Verify operator lookup by location and adapter metadata retrieval for known and unknown IDs.
- **Config** (`config_test.go`): Verify configuration loading from file, embedded default fallback, and error handling for invalid configs.

### Integration Tests (via httptest)

- **Handler tests** (`handler_test.go`): Use `httptest.NewRecorder` to exercise full HTTP request/response cycle for each endpoint.
- **Validate:** Status codes, response bodies, Content-Type headers, error formats, and rate information fields.

### Property Tests

- **Geofence properties:** Vertex inclusion, centroid inclusion, distant point exclusion, proximity threshold consistency.
- **Response format:** All endpoints return `application/json` with valid JSON bodies.
- **Idempotency:** Repeated requests with the same parameters return identical responses.
