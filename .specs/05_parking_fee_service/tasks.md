# Implementation Plan: PARKING_FEE_SERVICE

<!-- AGENT INSTRUCTIONS
- Implement exactly ONE top-level task group per session
- Follow the git-flow: feature branch from develop -> implement -> test -> merge to develop -> push
- Update checkbox states as you go: [-] in progress, [x] complete
- The geo module should be built first — all zone matching depends on it
- REST handlers depend on the zone store, which depends on geo
- CLI extensions and integration tests come last
-->

## Overview

This plan implements the PARKING_FEE_SERVICE in dependency order:

1. Geospatial utilities (point-in-polygon, Haversine distance).
2. Zone data store with seed data.
3. REST API handlers.
4. Mock PARKING_APP CLI extensions.
5. Integration tests.

The service is a straightforward Go REST API with no external dependencies
beyond the standard library. All zone data is hardcoded.

## Test Commands

- Go unit tests (PFS): `cd backend/parking-fee-service && go test ./...`
- Go unit tests (CLI): `cd mock/parking-app-cli && go test ./...`
- All tests: `make test`
- Go linter: `cd backend/parking-fee-service && go vet ./...`
- All linters: `make lint`
- Build all: `make build`
- Integration tests: requires PARKING_FEE_SERVICE running + optionally
  UPDATE_SERVICE + podman (see group 6)

## Tasks

- [ ] 1. Geospatial Utilities
  - [ ] 1.1 Implement Haversine distance function
    - Create `backend/parking-fee-service/geo/geo.go`
    - `HaversineDistance(lat1, lon1, lat2, lon2 float64) float64` → meters
    - Use Earth radius 6371000m
    - _Requirements: 05-REQ-1.3 (prerequisite)_

  - [ ] 1.2 Implement point-in-polygon function
    - `PointInPolygon(lat, lon float64, polygon []LatLon) bool`
    - Ray-casting algorithm: count edge crossings
    - Handle edge cases: points on vertex, points on edge
    - _Requirements: 05-REQ-1.2_

  - [ ] 1.3 Implement distance-to-polygon function
    - `DistanceToPolygon(lat, lon float64, polygon []LatLon) float64` → meters
    - For each polygon edge, find closest point on segment, compute
      Haversine distance, return minimum
    - _Requirements: 05-REQ-1.3_

  - [ ] 1.4 Write geo unit tests
    - Haversine: known distances (Munich to Berlin ≈ 504 km), symmetry
    - Point-in-polygon: inside rectangle, outside rectangle, on edge, on
      vertex, concave polygon
    - Distance-to-polygon: point at known offset from edge, point far away,
      point at vertex distance
    - **Property 1: Point-in-Polygon Accuracy**
    - **Property 6: Haversine Symmetry**
    - **Validates: 05-REQ-1.2, 05-REQ-1.3**

  - [ ] 1.V Verify task group 1
    - [ ] `cd backend/parking-fee-service && go test ./geo/...` passes
    - [ ] `go vet ./...` clean
    - [ ] Haversine symmetry holds for all test cases
    - [ ] Point-in-polygon correct for inside/outside/boundary

- [ ] 2. Zone Data Store and Seed Data
  - [ ] 2.1 Create zone data model
    - Create `backend/parking-fee-service/zones/store.go`
    - Define `Zone`, `LatLon`, `ZoneMatch`, `AdapterMetadata` structs
    - Define `Store` with `map[string]*Zone` keyed by zone_id
    - `GetByID(zoneID) (*Zone, bool)`
    - _Requirements: 05-REQ-2.1, 05-REQ-3.1 (prerequisite)_

  - [ ] 2.2 Implement zone lookup by location
    - `FindByLocation(lat, lon float64) []ZoneMatch`
    - Step 1: point-in-polygon test for each zone → exact matches (distance=0)
    - Step 2: if no exact matches, check zones within 200m fuzzy radius
    - Sort results by distance ascending
    - _Requirements: 05-REQ-1.1, 05-REQ-1.2, 05-REQ-1.3, 05-REQ-1.5_

  - [ ] 2.3 Create hardcoded seed data
    - Create `backend/parking-fee-service/zones/seed.go`
    - Define 3 Munich demo zones: Marienplatz, Olympiapark, Sendlinger Tor
    - Each with realistic polygon coordinates, rate config, adapter metadata
    - `LoadSeedData() *Store` function
    - Validate polygons on load (≥3 points), log warning for malformed
    - _Requirements: 05-REQ-4.1, 05-REQ-4.2, 05-REQ-4.3, 05-REQ-4.4,
      05-REQ-4.E1_

  - [ ] 2.4 Write store unit tests
    - FindByLocation: point inside Marienplatz → returns zone with distance=0
    - FindByLocation: point 100m from Marienplatz → returns zone with
      distance ≈ 100m
    - FindByLocation: point 500m from all zones → empty result
    - FindByLocation: point equidistant to two zones → both returned, sorted
    - GetByID: known zone → found; unknown zone → not found
    - Seed data: verify 3 zones loaded, verify polygon validity
    - **Property 2: Fuzzy Radius Boundary**
    - **Property 3: No-Match Safety**
    - **Property 5: Sort Order Invariant**
    - **Validates: 05-REQ-1.1–1.5, 05-REQ-1.E1, 05-REQ-4.1–4.4**

  - [ ] 2.V Verify task group 2
    - [ ] `cd backend/parking-fee-service && go test ./zones/...` passes
    - [ ] `go vet ./...` clean
    - [ ] All 3 seed zones load correctly
    - [ ] Location matching correct for inside, fuzzy, and no-match cases
    - [ ] Results sorted by distance

- [ ] 3. REST API Handlers
  - [ ] 3.1 Create REST handlers
    - Create `backend/parking-fee-service/api/handlers.go`
    - `GET /healthz` → 200 `{}`
    - `GET /api/v1/zones?lat=X&lon=Y` → parse params, call
      `store.FindByLocation`, return JSON array
    - `GET /api/v1/zones/{zone_id}` → call `store.GetByID`, return zone or 404
    - `GET /api/v1/zones/{zone_id}/adapter` → call `store.GetByID`, return
      adapter metadata or 404
    - Validate lat/lon: missing → 400, non-numeric → 400
    - Add request logging middleware
    - _Requirements: 05-REQ-1.1, 05-REQ-1.4, 05-REQ-1.E2, 05-REQ-2.1,
      05-REQ-2.2, 05-REQ-2.E1, 05-REQ-3.1, 05-REQ-3.2, 05-REQ-3.E1,
      05-REQ-5.2, 05-REQ-5.4_

  - [ ] 3.2 Wire up main.go
    - Replace spec 01 skeleton with real implementation
    - Parse config: `--listen-addr` (default `:8080`)
    - Initialize: seed data → store → register routes → start HTTP server
    - Graceful shutdown on SIGINT/SIGTERM
    - _Requirements: 05-REQ-5.1, 05-REQ-5.3_

  - [ ] 3.3 Write handler unit tests
    - Use `httptest.Server` for all endpoint tests
    - Zone lookup: valid coords → 200 with zones, no match → 200 empty array,
      missing lat → 400, invalid lon → 400
    - Zone details: known zone → 200 with full data, unknown → 404
    - Adapter metadata: known zone → 200 with image_ref/checksum, unknown → 404
    - Health check: → 200 `{}`
    - **Property 4: Adapter Metadata Consistency**
    - **Validates: 05-REQ-1.1, 05-REQ-1.E1, 05-REQ-1.E2, 05-REQ-2.1,
      05-REQ-2.E1, 05-REQ-3.1, 05-REQ-3.E1, 05-REQ-5.1, 05-REQ-5.2**

  - [ ] 3.V Verify task group 3
    - [ ] `cd backend/parking-fee-service && go test ./...` passes
    - [ ] `go vet ./...` clean
    - [ ] `go build` produces binary
    - [ ] All REST endpoints return correct responses
    - [ ] Requirements 05-REQ-1.1, 05-REQ-2.1, 05-REQ-3.1, 05-REQ-5.1–5.4 met

- [ ] 4. Checkpoint — PARKING_FEE_SERVICE Complete
  - All geo utilities, zone store, and REST handlers working
  - Commit and verify clean state

- [ ] 5. Mock PARKING_APP CLI Extensions
  - [ ] 5.1 Add PARKING_FEE_SERVICE subcommands
    - Add to `mock/parking-app-cli/main.go`:
    - `lookup-zones --lat <lat> --lon <lon>` → GET /api/v1/zones?lat=X&lon=Y,
      print results as table
    - `zone-info --zone-id <id>` → GET /api/v1/zones/{id}, print zone details
    - `adapter-info --zone-id <id>` → GET /api/v1/zones/{id}/adapter, print
      adapter metadata
    - _Requirements: 05-REQ-6.1, 05-REQ-6.2, 05-REQ-6.3_

  - [ ] 5.2 Add flag and error handling
    - `--parking-fee-service-addr` flag (default: `http://localhost:8080`)
    - Service unreachable → error message + non-zero exit
    - HTTP error responses (400, 404) → meaningful error messages
    - _Requirements: 05-REQ-6.4, 05-REQ-6.E1_

  - [ ] 5.3 Write CLI tests
    - Argument parsing for each new subcommand
    - HTTP request construction using `httptest.Server`
    - Error handling: unreachable service
    - **Validates: 05-REQ-6.1, 05-REQ-6.2, 05-REQ-6.3, 05-REQ-6.4,
      05-REQ-6.E1**

  - [ ] 5.V Verify task group 5
    - [ ] `cd mock/parking-app-cli && go test ./...` passes
    - [ ] `go vet ./...` clean
    - [ ] All new subcommands appear in `--help`
    - [ ] Requirements 05-REQ-6.1–6.4, 05-REQ-6.E1 met

- [ ] 6. Integration Tests
  - [ ] 6.1 Test zone discovery
    - Start PARKING_FEE_SERVICE
    - Lookup with Marienplatz coordinates → verify zone returned
    - Lookup with coordinates 100m from Olympiapark → verify fuzzy match
    - Lookup with coordinates far from all zones → verify empty result
    - _Requirements: 05-REQ-7.1, 05-REQ-7.2_

  - [ ] 6.2 Test adapter metadata to install flow
    - Start PARKING_FEE_SERVICE + UPDATE_SERVICE (requires podman)
    - Get adapter metadata from PFS
    - Call UPDATE_SERVICE InstallAdapter with returned image_ref/checksum
    - Verify adapter state transitions to RUNNING
    - Skip if UPDATE_SERVICE or podman unavailable
    - _Requirements: 05-REQ-7.3_

  - [ ] 6.3 Test full discovery flow via CLI
    - Use parking-app-cli to: `lookup-zones` → `adapter-info` →
      `install-adapter` → `list-adapters` → verify RUNNING
    - End-to-end validation of the discovery-to-install workflow
    - Skip if infrastructure unavailable
    - _Requirements: 05-REQ-7.4, 05-REQ-7.E1_

  - [ ] 6.V Verify task group 6
    - [ ] Zone discovery tests pass
    - [ ] Adapter install flow tests pass (with podman)
    - [ ] Tests skip cleanly without infrastructure
    - [ ] Requirements 05-REQ-7.1–7.4 met

- [ ] 7. Final Verification and Documentation
  - [ ] 7.1 Run full test suite
    - `make build && make test && make lint`
    - Verify no regressions in specs 01–04 tests
    - _Requirements: all_

  - [ ] 7.2 Run integration tests
    - Start all infrastructure and services
    - Run all integration tests
    - Verify all pass

  - [ ] 7.3 Update documentation
    - Document PARKING_FEE_SERVICE REST API in `docs/parking-fee-service-api.md`
    - Document demo zones and the discovery flow in `docs/zone-discovery.md`
    - Update README if needed

  - [ ] 7.V Verify task group 7
    - [ ] `make build` succeeds
    - [ ] `make test` passes
    - [ ] `make lint` clean
    - [ ] Integration tests pass
    - [ ] No regressions from specs 01–04
    - [ ] All 05-REQ requirements verified

### Checkbox States

| Syntax   | Meaning                |
|----------|------------------------|
| `- [ ]`  | Not started (required) |
| `- [ ]*` | Not started (optional) |
| `- [x]`  | Completed              |
| `- [-]`  | In progress            |
| `- [~]`  | Queued                 |

## Traceability

| Requirement | Implemented By Task | Verified By Test |
|-------------|---------------------|------------------|
| 05-REQ-1.1 | 2.2, 3.1 | Store test (2.4), handler test (3.3) |
| 05-REQ-1.2 | 1.2, 2.2 | Geo test (1.4), store test (2.4) |
| 05-REQ-1.3 | 1.1, 1.3, 2.2 | Geo test (1.4), store test (2.4) |
| 05-REQ-1.4 | 3.1 | Handler test (3.3) |
| 05-REQ-1.5 | 2.2 | Store test (2.4) |
| 05-REQ-1.E1 | 2.2, 3.1 | Store test (2.4), handler test (3.3) |
| 05-REQ-1.E2 | 3.1 | Handler test (3.3) |
| 05-REQ-2.1 | 3.1 | Handler test (3.3) |
| 05-REQ-2.2 | 3.1 | Handler test (3.3) |
| 05-REQ-2.E1 | 3.1 | Handler test (3.3) |
| 05-REQ-3.1 | 3.1 | Handler test (3.3) |
| 05-REQ-3.2 | 3.1 | Handler test (3.3) |
| 05-REQ-3.E1 | 3.1 | Handler test (3.3) |
| 05-REQ-4.1 | 2.3 | Seed data test (2.4) |
| 05-REQ-4.2 | 2.3 | Seed data test (2.4) |
| 05-REQ-4.3 | 2.3 | Seed data test (2.4) |
| 05-REQ-4.4 | 2.3, 3.2 | Seed data test (2.4) |
| 05-REQ-4.E1 | 2.3 | Seed data test (2.4) |
| 05-REQ-5.1 | 3.2 | Handler test (3.3) |
| 05-REQ-5.2 | 3.1 | Handler test (3.3) |
| 05-REQ-5.3 | 3.1 | (No auth to test) |
| 05-REQ-5.4 | 3.1 | Handler test (3.3) — verify logging |
| 05-REQ-6.1 | 5.1 | CLI test (5.3) |
| 05-REQ-6.2 | 5.1 | CLI test (5.3) |
| 05-REQ-6.3 | 5.1 | CLI test (5.3) |
| 05-REQ-6.4 | 5.2 | CLI test (5.3) |
| 05-REQ-6.E1 | 5.2 | CLI test (5.3) |
| 05-REQ-7.1 | 6.1 | Integration test `test_zone_lookup` |
| 05-REQ-7.2 | 6.1 | Integration test `test_fuzzy_match` |
| 05-REQ-7.3 | 6.2 | Integration test `test_adapter_install` |
| 05-REQ-7.4 | 6.3 | Integration test `test_full_discovery_flow` |
| 05-REQ-7.E1 | 6.1, 6.2, 6.3 | Test skip behavior |

## Notes

- **Standard library only:** The PARKING_FEE_SERVICE uses only Go standard
  library packages (net/http, encoding/json, math, log/slog). No external
  dependencies needed for a REST API with in-memory data.
- **Geospatial precision:** The ray-casting point-in-polygon algorithm is
  sufficient for the demo's rectangular/simple polygons. For production with
  complex polygons, consider using a spatial library (e.g., S2 geometry).
- **Fuzzy radius:** The 200m threshold is hardcoded. This is fine for the
  demo. Production would make it configurable per zone or globally.
- **Adapter image references:** All demo zones use the same adapter image
  (`localhost/parking-operator-adaptor:latest`). In production, different
  operators would have different adapters.
- **No state changes:** PARKING_FEE_SERVICE is read-only for the demo.
  All data is loaded from seed on startup. No mutations occur at runtime.
- **Spec 04 CLI compatibility:** The new subcommands are added alongside
  existing spec 04 subcommands in `mock/parking-app-cli`. The new
  `--parking-fee-service-addr` flag does not conflict with existing flags.
