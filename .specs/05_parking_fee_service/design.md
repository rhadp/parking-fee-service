# Design Document: PARKING_FEE_SERVICE

## Overview

The PARKING_FEE_SERVICE is a Go HTTP server (`backend/parking-fee-service`) providing three REST endpoints for parking operator discovery, adapter metadata retrieval, and health checks. It loads operator/zone data from a JSON config file at startup, stores it in memory, and uses point-in-polygon + proximity matching for location queries. Built entirely with Go standard library -- no external dependencies.

## Architecture

```mermaid
flowchart TD
    PA["PARKING_APP"] -->|"GET /operators?lat=&lon="| PFS["PARKING_FEE_SERVICE\n:8080"]
    PA -->|"GET /operators/{id}/adapter"| PFS
    PA -->|"GET /health"| PFS

    PFS -->|"loads at startup"| CFG["config.json"]
    PFS -->|"in-memory"| Store["Zones + Operators\n(geofence polygons)"]

    subgraph PARKING_FEE_SERVICE
        Handler["HTTP Handlers"]
        Geo["Geofence Engine\n(ray casting + Haversine)"]
        Store2["Config Store"]
    end

    Handler --> Geo
    Handler --> Store2
```

```mermaid
sequenceDiagram
    participant PA as PARKING_APP
    participant PFS as PARKING_FEE_SERVICE
    participant Store as Config Store
    participant Geo as Geofence Engine

    PA->>PFS: GET /operators?lat=48.137&lon=11.575
    PFS->>PFS: Parse & validate lat/lon
    PFS->>Geo: FindMatchingZones(48.137, 11.575, threshold)
    Geo->>Store: Get all zones
    Store-->>Geo: zones[]
    Geo->>Geo: Point-in-polygon for each zone
    Geo->>Geo: Proximity check for non-matching zones
    Geo-->>PFS: matching zone IDs
    PFS->>Store: GetOperatorsByZoneIDs(zoneIDs)
    Store-->>PFS: operators[]
    PFS-->>PA: 200 [{id, name, zone_id, rate}]
```

### Module Responsibilities

1. **main** -- Entry point: loads config, sets up HTTP routes, starts server, handles shutdown signals.
2. **config** -- Configuration loading and parsing: reads JSON file, provides defaults, validates structure.
3. **handler** -- HTTP request handlers: operator lookup, adapter metadata, health check.
4. **geo** -- Geofence engine: point-in-polygon (ray casting), Haversine distance, proximity matching.
5. **store** -- In-memory data store: zones and operators indexed for fast lookup by zone ID and operator ID.
6. **model** -- Core data types: Zone, Operator, Rate, AdapterMeta, Coordinate.

## Execution Paths

### Path 1: Operator Lookup (GET /operators?lat=&lon=)

1. `handler.NewOperatorHandler` receives the HTTP request.
2. `handler.NewOperatorHandler` extracts `lat` and `lon` from `r.URL.Query()` -- returns `(string, string)`.
3. `handler.NewOperatorHandler` parses both strings via `strconv.ParseFloat(s, 64)` -- returns `(float64, error)`. On error: writes `{"error":"invalid coordinates"}` with HTTP 400.
4. `handler.NewOperatorHandler` validates lat in [-90, 90] and lon in [-180, 180]. On failure: writes `{"error":"invalid coordinates"}` with HTTP 400.
5. `geo.FindMatchingZones(model.Coordinate{lat, lon}, zones, threshold)` -- returns `[]string` (matching zone IDs).
   - 5a. For each zone: `geo.PointInPolygon(point, zone.Polygon)` -- returns `bool`. If true, zone ID is added.
   - 5b. For non-matching zones: `geo.DistanceToPolygonEdge(point, zone.Polygon)` -- returns `float64` (meters). If <= threshold, zone ID is added.
6. `store.GetOperatorsByZoneIDs(zoneIDs)` -- returns `[]model.Operator`.
7. `handler.NewOperatorHandler` converts each `model.Operator` to `model.OperatorResponse` (strips adapter field).
8. `handler.NewOperatorHandler` writes JSON array with HTTP 200.

### Path 2: Adapter Metadata (GET /operators/{id}/adapter)

1. `handler.NewAdapterHandler` receives the HTTP request.
2. `handler.NewAdapterHandler` extracts `{id}` from `r.PathValue("id")` -- returns `string`.
3. `store.GetOperator(id)` -- returns `(*model.Operator, bool)`. On not found: writes `{"error":"operator not found"}` with HTTP 404.
4. `handler.NewAdapterHandler` writes `model.AdapterMeta` as JSON with HTTP 200.

### Path 3: Health Check (GET /health)

1. `handler.HealthHandler` receives the HTTP request.
2. `handler.HealthHandler` writes `{"status":"ok"}` with HTTP 200.

### Path 4: Startup

1. `main` reads `CONFIG_PATH` env var via `os.Getenv("CONFIG_PATH")` -- returns `string`, defaults to `"config.json"`.
2. `config.LoadConfig(path)` -- returns `(*model.Config, error)`. On file-not-found: returns `DefaultConfig()`, logs warning. On parse error: returns `(nil, error)`.
3. `main` exits with non-zero code if `LoadConfig` returns error.
4. `store.NewStore(cfg.Zones, cfg.Operators)` -- returns `*store.Store`.
5. `main` registers routes on `http.NewServeMux()`:
   - `GET /operators` -> `handler.NewOperatorHandler(...)`
   - `GET /operators/{id}/adapter` -> `handler.NewAdapterHandler(...)`
   - `GET /health` -> `handler.HealthHandler()`
6. `main` starts `http.Server` on configured port.
7. `main` logs version, port, zone count, operator count, ready message via `log/slog`.

### Path 5: Shutdown

1. `main` receives SIGTERM or SIGINT via `os/signal.Notify`.
2. `main` calls `http.Server.Shutdown(ctx)` with a timeout context -- returns `error`.
3. `main` logs shutdown message and exits with code 0.

## Components and Interfaces

### REST API

| Method | Path | Request | Response (200) | Errors |
|--------|------|---------|----------------|--------|
| GET | `/operators?lat={lat}&lon={lon}` | Query params | `[{OperatorResponse}]` | 400 |
| GET | `/operators/{id}/adapter` | Path param | `{AdapterMeta}` | 404 |
| GET | `/health` | -- | `{"status":"ok"}` | -- |

### Core Data Types

```go
type Coordinate struct {
    Lat float64 `json:"lat"`
    Lon float64 `json:"lon"`
}

type Zone struct {
    ID       string       `json:"id"`
    Name     string       `json:"name"`
    Polygon  []Coordinate `json:"polygon"`
}

type Rate struct {
    Type     string  `json:"type"`     // "per-hour" | "flat-fee"
    Amount   float64 `json:"amount"`
    Currency string  `json:"currency"` // "EUR"
}

type AdapterMeta struct {
    ImageRef       string `json:"image_ref"`
    ChecksumSHA256 string `json:"checksum_sha256"`
    Version        string `json:"version"`
}

type Operator struct {
    ID      string      `json:"id"`
    Name    string      `json:"name"`
    ZoneID  string      `json:"zone_id"`
    Rate    Rate        `json:"rate"`
    Adapter AdapterMeta `json:"adapter"`
}

type OperatorResponse struct {
    ID     string `json:"id"`
    Name   string `json:"name"`
    ZoneID string `json:"zone_id"`
    Rate   Rate   `json:"rate"`
}

type Config struct {
    Port               int        `json:"port"`
    ProximityThreshold float64    `json:"proximity_threshold_meters"`
    Zones              []Zone     `json:"zones"`
    Operators          []Operator `json:"operators"`
}
```

### Module Interfaces

```go
// config package
func LoadConfig(path string) (*model.Config, error)
func DefaultConfig() *model.Config

// geo package
func PointInPolygon(point model.Coordinate, polygon []model.Coordinate) bool
func HaversineDistance(a, b model.Coordinate) float64
func DistanceToPolygonEdge(point model.Coordinate, polygon []model.Coordinate) float64
func FindMatchingZones(point model.Coordinate, zones []model.Zone, threshold float64) []string

// store package
type Store struct { /* indexed zones and operators */ }
func NewStore(zones []model.Zone, operators []model.Operator) *Store
func (s *Store) GetZone(id string) (*model.Zone, bool)
func (s *Store) GetOperator(id string) (*model.Operator, bool)
func (s *Store) GetOperatorsByZoneIDs(zoneIDs []string) []model.Operator

// handler package
func NewOperatorHandler(store *store.Store, zones []model.Zone, threshold float64) http.HandlerFunc
func NewAdapterHandler(store *store.Store) http.HandlerFunc
func HealthHandler() http.HandlerFunc
```

## Data Models

### Configuration File (config.json)

```json
{
  "port": 8080,
  "proximity_threshold_meters": 500,
  "zones": [
    {
      "id": "munich-central",
      "name": "Munich Central Station Area",
      "polygon": [
        {"lat": 48.1400, "lon": 11.5550},
        {"lat": 48.1400, "lon": 11.5650},
        {"lat": 48.1350, "lon": 11.5650},
        {"lat": 48.1350, "lon": 11.5550}
      ]
    },
    {
      "id": "munich-marienplatz",
      "name": "Marienplatz Area",
      "polygon": [
        {"lat": 48.1380, "lon": 11.5730},
        {"lat": 48.1380, "lon": 11.5790},
        {"lat": 48.1350, "lon": 11.5790},
        {"lat": 48.1350, "lon": 11.5730}
      ]
    }
  ],
  "operators": [
    {
      "id": "parkhaus-munich",
      "name": "Parkhaus Muenchen GmbH",
      "zone_id": "munich-central",
      "rate": {"type": "per-hour", "amount": 2.50, "currency": "EUR"},
      "adapter": {
        "image_ref": "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0",
        "checksum_sha256": "sha256:abc123def456",
        "version": "1.0.0"
      }
    },
    {
      "id": "city-park-munich",
      "name": "CityPark Muenchen",
      "zone_id": "munich-marienplatz",
      "rate": {"type": "flat-fee", "amount": 5.00, "currency": "EUR"},
      "adapter": {
        "image_ref": "us-docker.pkg.dev/sdv-demo/adapters/citypark-munich:v1.0.0",
        "checksum_sha256": "sha256:789ghi012jkl",
        "version": "1.0.0"
      }
    }
  ]
}
```

### Operator Lookup Response

```json
[
  {
    "id": "parkhaus-munich",
    "name": "Parkhaus Muenchen GmbH",
    "zone_id": "munich-central",
    "rate": {"type": "per-hour", "amount": 2.50, "currency": "EUR"}
  }
]
```

Note: The adapter field is intentionally excluded from the lookup response -- the client must call `/operators/{id}/adapter` separately.

### Adapter Metadata Response

```json
{
  "image_ref": "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0",
  "checksum_sha256": "sha256:abc123def456",
  "version": "1.0.0"
}
```

### Error Response

```json
{"error": "lat and lon query parameters are required"}
```

## Correctness Properties

### Property 1: Point-in-Polygon Correctness

*For any* coordinate inside a convex polygon, `PointInPolygon` SHALL return `true`, and for any coordinate outside the polygon by more than the proximity threshold, `FindMatchingZones` SHALL return an empty list.

**Validates:** 05-REQ-1.2, 05-REQ-1.5

### Property 2: Proximity Matching

*For any* coordinate outside a zone's polygon but within `threshold` meters of the nearest edge, `FindMatchingZones` SHALL include that zone in the results.

**Validates:** 05-REQ-1.3

### Property 3: Operator-Zone Association

*For any* set of matching zone IDs, `GetOperatorsByZoneIDs` SHALL return all and only operators whose `zone_id` is in the set.

**Validates:** 05-REQ-1.4

### Property 4: Coordinate Validation

*For any* latitude outside [-90, 90] or longitude outside [-180, 180], the operator lookup handler SHALL return HTTP 400.

**Validates:** 05-REQ-1.E2, 05-REQ-1.E3

### Property 5: Adapter Metadata Completeness

*For any* valid operator ID, `GetOperator` SHALL return an operator with non-empty `image_ref`, `checksum_sha256`, and `version` fields.

**Validates:** 05-REQ-2.1

### Property 6: Config Defaults

*For any* missing or nonexistent config file path, `LoadConfig` SHALL return a valid default configuration with at least one zone and one operator.

**Validates:** 05-REQ-4.E1

## Error Handling

| Error Condition | Behavior | Requirement |
|----------------|----------|-------------|
| Missing lat/lon params | 400 with `{"error":"lat and lon query parameters are required"}` | 05-REQ-1.E1 |
| Invalid coordinates (range) | 400 with `{"error":"invalid coordinates"}` | 05-REQ-1.E2 |
| Non-numeric lat/lon | 400 with `{"error":"invalid coordinates"}` | 05-REQ-1.E3 |
| Unknown operator ID | 404 with `{"error":"operator not found"}` | 05-REQ-2.E1 |
| Config file missing | Start with defaults, log warning | 05-REQ-4.E1 |
| Config file invalid JSON | Exit non-zero, log error | 05-REQ-4.E2 |

## Technology Stack

| Technology | Version | Purpose |
|-----------|---------|---------|
| Go | 1.22+ | Service implementation |
| net/http | stdlib | HTTP server (Go 1.22 ServeMux patterns) |
| encoding/json | stdlib | JSON encoding/decoding |
| math | stdlib | Haversine distance calculations |
| strconv | stdlib | Coordinate string parsing |
| os/signal | stdlib | Graceful shutdown |
| log/slog | stdlib | Structured logging |

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

- **Unit tests:** Go `_test.go` files alongside source. The `geo`, `config`, `store`, and `handler` packages each have unit tests.
- **Property tests:** Go `testing/quick` or table-driven tests with boundary coordinates for geofence logic.
- **Integration tests:** `httptest` server for end-to-end HTTP request/response testing. No external dependencies needed -- the service is self-contained.
- **All tests run via:** `cd backend && go test -v ./parking-fee-service/...`

## Operational Readiness

- **Startup logging:** Logs version, port, zone count, operator count via `log/slog`.
- **Shutdown:** Handles SIGTERM/SIGINT, uses `http.Server.Shutdown()` for graceful drain.
- **Health:** `/health` endpoint returns `{"status":"ok"}` with HTTP 200.
- **No persistent state:** Rollback is a redeploy. No database migrations or cleanup needed.
- **Config fallback:** Service starts with built-in Munich demo data if config file is absent.
