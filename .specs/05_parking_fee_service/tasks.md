# Implementation Plan: Parking Fee Service (Phase 2.4)

<!-- AGENT INSTRUCTIONS
- Implement exactly ONE top-level task group per session
- Task group 1 writes failing tests from test_spec.md — all subsequent groups
  implement code to make those tests pass
- Follow the git-flow: feature branch from develop -> implement -> test -> merge to develop -> push
- Update checkbox states as you go: [-] in progress, [x] complete
-->

## Overview

This plan implements the PARKING_FEE_SERVICE — a Go HTTP server providing
REST API for parking operator discovery (geofence-based) and adapter metadata
retrieval. It also enhances the mock PARKING_APP CLI to call the
PARKING_FEE_SERVICE for operator lookups and adapter metadata.

The approach is test-first: task group 1 creates failing tests based on
`test_spec.md`. Subsequent groups implement the service components to make
those tests pass incrementally.

Ordering rationale:
1. Tests first (red) — establishes the verification baseline
2. Data model and operator store — foundational data layer
3. Geofence engine — core matching algorithm, no HTTP dependency
4. REST server and handlers — HTTP layer on top of store and geo engine
5. Authentication middleware — security layer wrapping handlers
6. Mock CLI integration — end-to-end integration with PARKING_FEE_SERVICE
7. Integration tests and final verification

## Test Commands

- Unit tests: `cd backend/parking-fee-service && go test -v -count=1 ./...`
- Unit tests (geo only): `cd backend/parking-fee-service && go test -v -count=1 ./internal/geo/...`
- Unit tests (handler only): `cd backend/parking-fee-service && go test -v -count=1 ./internal/handler/...`
- Unit tests (store only): `cd backend/parking-fee-service && go test -v -count=1 ./internal/store/...`
- Unit tests (config only): `cd backend/parking-fee-service && go test -v -count=1 ./internal/config/...`
- Integration tests: `cd tests/integration/parking_fee_service && go test -v -count=1 -tags=integration ./...`
- Linter: `cd backend/parking-fee-service && go vet ./...`
- All Go tests: `make test` (from repo root)

## Tasks

- [x] 1. Write failing tests
  - [x] 1.1 Set up test infrastructure
    - Create `backend/parking-fee-service/internal/geo/polygon_test.go` with
      test helpers and test table structure
    - Create `backend/parking-fee-service/internal/handler/handler_test.go`
      with `setupTestServer` helper using `httptest`
    - Create `backend/parking-fee-service/internal/store/store_test.go`
    - Create `backend/parking-fee-service/internal/config/config_test.go`
    - Create `backend/parking-fee-service/testdata/operators.json` (valid
      sample config)
    - Create `backend/parking-fee-service/testdata/invalid.json` (malformed
      JSON for edge case)
    - _Test Spec: all (shared infrastructure)_

  - [x] 1.2 Write geofence engine tests
    - Translate TS-05-5 (point-in-polygon basic) into Go test
    - Translate TS-05-6 (implicit polygon close) into Go test
    - Translate TS-05-7 (triangle polygon) into Go test
    - Translate TS-05-8 (fuzziness configurable) into Go test
    - Translate TS-05-9 (near-boundary match) into Go test
    - Translate TS-05-E8 (degenerate polygon) into Go test
    - Translate TS-05-E9 (fuzziness zero) into Go test
    - Translate TS-05-P1 (determinism) into Go test
    - Translate TS-05-P2 (monotonicity) into Go test
    - Translate TS-05-P3 (interior points) into Go test
    - Translate TS-05-P4 (distant points) into Go test
    - Group under `Test*` naming conventions in `polygon_test.go`
    - _Test Spec: TS-05-5 through TS-05-9, TS-05-E8, TS-05-E9, TS-05-P1
      through TS-05-P4_

  - [x] 1.3 Write handler and auth tests
    - Translate TS-05-1 through TS-05-4 (operator lookup) into Go tests
    - Translate TS-05-12 through TS-05-14 (adapter metadata) into Go tests
    - Translate TS-05-15 through TS-05-16 (health check) into Go tests
    - Translate TS-05-21 through TS-05-23 (auth) into Go tests
    - Translate TS-05-E1 through TS-05-E7 (lookup edge cases) into Go tests
    - Translate TS-05-E10 (unknown operator) into Go test
    - Translate TS-05-E13 through TS-05-E15 (auth edge cases) into Go tests
    - Translate TS-05-P5 (adapter metadata consistency) into Go test
    - Translate TS-05-P6 (auth enforcement) into Go test
    - Translate TS-05-P7 (health availability) into Go test
    - Group under `Test*` naming conventions in `handler_test.go`
    - _Test Spec: TS-05-1 through TS-05-4, TS-05-12 through TS-05-16,
      TS-05-21 through TS-05-23, TS-05-E1 through TS-05-E7, TS-05-E10,
      TS-05-E13 through TS-05-E15, TS-05-P5 through TS-05-P7_

  - [x] 1.4 Write store and config tests
    - Translate TS-05-17 (load from JSON) into Go test
    - Translate TS-05-18 (default operators) into Go test
    - Translate TS-05-19 (config env var) into Go test
    - Translate TS-05-20 (default when no config) into Go test
    - Translate TS-05-10 (default fuzziness) into Go test
    - Translate TS-05-11 (fuzziness env var) into Go test
    - Translate TS-05-E11 (malformed config) into Go test
    - Translate TS-05-E12 (missing config file) into Go test
    - Group under `Test*` naming conventions
    - _Test Spec: TS-05-10, TS-05-11, TS-05-17 through TS-05-20,
      TS-05-E11, TS-05-E12_

  - [x] 1.5 Write integration test stubs
    - Create `tests/integration/parking_fee_service/` directory
    - Create `tests/integration/parking_fee_service/go.mod`
    - Create integration test file with `//go:build integration` tag
    - Translate TS-05-I1 (CLI lookup) into Go test stub
    - Translate TS-05-I2 (CLI adapter) into Go test stub
    - Translate TS-05-I3 (full discovery flow) into Go test stub
    - _Test Spec: TS-05-I1 through TS-05-I3_

  - [x] 1.V Verify task group 1
    - [x] All test files compile:
      `cd backend/parking-fee-service && go vet ./...`
    - [x] All tests FAIL (red) — no implementation yet:
      `cd backend/parking-fee-service && go test -count=1 ./... 2>&1 | grep -c FAIL`
    - [x] No linter warnings:
      `cd backend/parking-fee-service && go vet ./...`

- [x] 2. Data model and operator store
  - [x] 2.1 Create data model types
    - Create `backend/parking-fee-service/internal/model/operator.go`
    - Define types: Operator, Zone, Point, Rate, Adapter, OperatorsConfig
    - Add JSON struct tags matching the API response schema
    - _Requirements: 05-REQ-1.2, 05-REQ-4.2_

  - [x] 2.2 Create operator store
    - Create `backend/parking-fee-service/internal/store/store.go`
    - Implement `Store` interface: `ListOperators()`, `GetOperator(id string)`
    - Implement in-memory store backed by a `map[string]Operator`
    - _Requirements: 05-REQ-6.1_

  - [x] 2.3 Create default operator dataset
    - Create `backend/parking-fee-service/internal/store/default_data.go`
    - Embed the default operator dataset (Munich City Center, Munich Airport)
    - Include realistic polygon coordinates, rates, and adapter metadata
    - Implement `NewDefaultStore()` constructor
    - _Requirements: 05-REQ-6.2, 05-REQ-6.4_

  - [x] 2.4 Implement JSON config file loading
    - Implement `NewStoreFromFile(path string)` constructor
    - Parse JSON file into OperatorsConfig, populate store
    - Return clear error on missing or malformed file
    - _Requirements: 05-REQ-6.1, 05-REQ-6.3_

  - [x] 2.V Verify task group 2
    - [x] Store tests pass:
      `cd backend/parking-fee-service && go test -v -count=1 ./internal/store/...`
    - [x] Model compiles:
      `cd backend/parking-fee-service && go vet ./internal/model/...`
    - [x] No linter warnings:
      `cd backend/parking-fee-service && go vet ./...`
    - [x] Requirements 05-REQ-6.1 through 05-REQ-6.4, 05-REQ-6.E1 met

- [x] 3. Geofence matching engine
  - [x] 3.1 Implement point-in-polygon algorithm
    - Create `backend/parking-fee-service/internal/geo/polygon.go`
    - Implement `PointInPolygon(point Point, polygon []Point) bool` using
      ray-casting algorithm
    - Handle implicit polygon closing (last vertex connects to first)
    - _Requirements: 05-REQ-2.1, 05-REQ-2.2, 05-REQ-2.3_

  - [x] 3.2 Implement distance-to-polygon calculation
    - Implement `MinDistanceToPolygon(point Point, polygon []Point) float64`
    - Calculate minimum distance from point to any edge of the polygon
    - Use equirectangular approximation for distance at these scales
    - Return distance in meters
    - _Requirements: 05-REQ-3.1, 05-REQ-3.2_

  - [x] 3.3 Implement FindMatches function
    - Implement `FindMatches(lat, lon float64, operators []Operator, fuzzinessMeters float64) []Operator`
    - Skip operators with degenerate polygons (< 3 vertices)
    - First check point-in-polygon, then check fuzziness distance
    - _Requirements: 05-REQ-1.1, 05-REQ-2.E1, 05-REQ-3.1, 05-REQ-3.2_

  - [x] 3.V Verify task group 3
    - [x] Geo tests pass:
      `cd backend/parking-fee-service && go test -v -count=1 ./internal/geo/...`
    - [x] Property tests pass (P1-P4):
      `cd backend/parking-fee-service && go test -v -count=1 -run "TestProperty" ./internal/geo/...`
    - [x] Edge case tests pass (E8, E9):
      `cd backend/parking-fee-service && go test -v -count=1 -run "TestEdge" ./internal/geo/...`
    - [x] No linter warnings:
      `cd backend/parking-fee-service && go vet ./...`
    - [x] Requirements 05-REQ-2.1 through 05-REQ-2.3, 05-REQ-3.1 through
      05-REQ-3.4, 05-REQ-2.E1, 05-REQ-3.E1 met

- [x] 4. REST server and handlers
  - [x] 4.1 Create configuration loading
    - Create `backend/parking-fee-service/internal/config/config.go`
    - Implement `LoadConfig()` reading PORT, OPERATORS_CONFIG,
      FUZZINESS_METERS, AUTH_TOKENS from environment variables
    - Apply defaults: PORT=8080, FUZZINESS_METERS=100,
      AUTH_TOKENS=demo-token-1
    - _Requirements: 05-REQ-3.3, 05-REQ-3.4, 05-REQ-6.3, 05-REQ-7.3_

  - [x] 4.2 Create health handler
    - Create `backend/parking-fee-service/internal/handler/health.go`
    - Implement `GET /health` returning `{"status": "ok"}`
    - _Requirements: 05-REQ-5.1, 05-REQ-5.2_

  - [x] 4.3 Create operator lookup handler
    - Create `backend/parking-fee-service/internal/handler/operators.go`
    - Implement `GET /operators?lat={lat}&lon={lon}`
    - Parse and validate query parameters
    - Call geofence engine to find matches
    - Return JSON response with matched operators
    - Handle all error cases (missing params, invalid values, out of range)
    - _Requirements: 05-REQ-1.1 through 05-REQ-1.4, 05-REQ-1.E1 through
      05-REQ-1.E4_

  - [x] 4.4 Create adapter metadata handler
    - Create `backend/parking-fee-service/internal/handler/adapter.go`
    - Implement `GET /operators/{id}/adapter`
    - Extract operator ID from URL path
    - Lookup operator in store, return adapter metadata
    - Handle unknown operator ID (404)
    - _Requirements: 05-REQ-4.1 through 05-REQ-4.3, 05-REQ-4.E1_

  - [x] 4.5 Create auth middleware
    - Create `backend/parking-fee-service/internal/handler/middleware.go`
    - Implement `AuthMiddleware(next http.Handler, tokens []string) http.Handler`
    - Validate Authorization header presence, Bearer scheme, and token value
    - Return appropriate 401 error for each failure mode
    - _Requirements: 05-REQ-7.1, 05-REQ-7.2, 05-REQ-7.E1 through 05-REQ-7.E3_

  - [x] 4.6 Wire up main.go
    - Update `backend/parking-fee-service/main.go` to use new internal packages
    - Load config, create store, create handlers, apply middleware
    - Register routes and start HTTP server
    - Replace existing stub 501 responses with real handlers
    - _Requirements: all_

  - [x] 4.V Verify task group 4
    - [x] Handler tests pass:
      `cd backend/parking-fee-service && go test -v -count=1 ./internal/handler/...`
    - [x] Config tests pass:
      `cd backend/parking-fee-service && go test -v -count=1 ./internal/config/...`
    - [x] All unit tests pass:
      `cd backend/parking-fee-service && go test -v -count=1 ./...`
    - [x] Property tests pass (P5-P7):
      `cd backend/parking-fee-service && go test -v -count=1 -run "TestProperty" ./...`
    - [x] Edge case tests pass (E1-E7, E10, E13-E15):
      `cd backend/parking-fee-service && go test -v -count=1 -run "TestEdge" ./...`
    - [x] No linter warnings:
      `cd backend/parking-fee-service && go vet ./...`
    - [x] Requirements 05-REQ-1.1 through 05-REQ-7.3 and all edge cases met

- [x] 5. Checkpoint — service implementation complete
  - All unit tests pass for the PARKING_FEE_SERVICE.
  - The service starts and responds to all three endpoints correctly.
  - Verify end-to-end:
    - `cd backend/parking-fee-service && go test -v -count=1 ./...`
    - `cd backend/parking-fee-service && go vet ./...`
  - Ask the user if questions arise before proceeding to CLI integration.

- [ ] 6. Mock PARKING_APP CLI integration
  - [ ] 6.1 Implement `lookup` command
    - Update `mock/parking-app-cli/` to implement the `lookup` subcommand
    - Add `--lat` and `--lon` flags
    - Add `--token` global flag (default: `demo-token-1`)
    - Send `GET /operators?lat={lat}&lon={lon}` to PARKING_FEE_SERVICE
    - Include `Authorization: Bearer <token>` header
    - Parse JSON response and print operator list in human-readable format
    - Exit 0 on success, 1 on error
    - _Requirements: Mock CLI enhancements (lookup)_

  - [ ] 6.2 Implement `adapter` command
    - Add `adapter` subcommand to mock PARKING_APP CLI
    - Add `--operator-id` flag
    - Send `GET /operators/{id}/adapter` to PARKING_FEE_SERVICE
    - Include `Authorization: Bearer <token>` header
    - Parse JSON response and print adapter metadata
    - Exit 0 on success, 1 on error
    - _Requirements: Mock CLI enhancements (adapter)_

  - [ ] 6.3 Update mock CLI tests
    - Add unit tests for `lookup` command (mock HTTP server)
    - Add unit tests for `adapter` command (mock HTTP server)
    - Verify correct HTTP requests are sent
    - Verify output format matches specification
    - _Test Spec: related to TS-05-I1, TS-05-I2_

  - [ ] 6.V Verify task group 6
    - [ ] Mock CLI builds: `cd mock/parking-app-cli && go build ./...`
    - [ ] Mock CLI tests pass: `cd mock/parking-app-cli && go test -v -count=1 ./...`
    - [ ] CLI help shows new commands:
      `cd mock/parking-app-cli && go run . --help`
    - [ ] No linter warnings:
      `cd mock/parking-app-cli && go vet ./...`

- [ ] 7. Integration tests and final verification
  - [ ] 7.1 Write integration tests
    - Implement TS-05-I1 (CLI lookup integration)
    - Implement TS-05-I2 (CLI adapter integration)
    - Implement TS-05-I3 (full discovery flow)
    - Use `httptest.NewServer()` to start PARKING_FEE_SERVICE in-process
    - Build and exec the mock CLI binary against the test server
    - _Test Spec: TS-05-I1 through TS-05-I3_

  - [ ] 7.2 Run all tests and fix failures
    - Run full unit test suite:
      `cd backend/parking-fee-service && go test -v -count=1 ./...`
    - Run integration tests:
      `cd tests/integration/parking_fee_service && go test -v -count=1 -tags=integration ./...`
    - Run linter: `cd backend/parking-fee-service && go vet ./...`
    - Fix any remaining failures
    - _Test Spec: all_

  - [ ] 7.3 Update Makefile (if needed)
    - Ensure `make build` includes the parking-fee-service
    - Ensure `make test` includes the parking-fee-service tests
    - Verify `make lint` covers the parking-fee-service
    - _Requirements: build system integration_

  - [ ] 7.4 Run full quality gate
    - `make check` (build + test + lint)
    - `cd backend/parking-fee-service && go test -v -count=1 ./...`
    - `cd tests/integration/parking_fee_service && go test -v -count=1 -tags=integration ./...`
    - `git status` shows clean working tree

  - [ ] 7.V Verify task group 7
    - [ ] All 46 spec tests pass (23 acceptance + 15 edge + 7 property +
      3 integration - 2 overlap)
    - [ ] `make check` exits 0
    - [ ] No linter warnings
    - [ ] All changes committed and pushed
    - [ ] Feature branch merged to develop

### Checkbox States

| Syntax   | Meaning                |
|----------|------------------------|
| `- [ ]`  | Not started (required) |
| `- [ ]*` | Not started (optional) |
| `- [x]`  | Completed              |
| `- [-]`  | In progress            |
| `- [~]`  | Queued                 |

## Traceability

| Requirement | Test Spec Entry | Implemented By Task | Verified By Test |
|-------------|-----------------|---------------------|------------------|
| 05-REQ-1.1 | TS-05-1 | 4.3 | `TestOperatorLookup_InsideZone` |
| 05-REQ-1.2 | TS-05-2 | 4.3 | `TestOperatorLookup_ResponseFields` |
| 05-REQ-1.3 | TS-05-3 | 4.3 | `TestOperatorLookup_MultipleMatches` |
| 05-REQ-1.4 | TS-05-4 | 4.3 | `TestOperatorLookup_ContentType` |
| 05-REQ-1.E1 | TS-05-E1 | 4.3 | `TestEdge_NoMatchingOperators` |
| 05-REQ-1.E2 | TS-05-E2, E3 | 4.3 | `TestEdge_MissingLatParam`, `TestEdge_MissingLonParam` |
| 05-REQ-1.E3 | TS-05-E4, E5 | 4.3 | `TestEdge_InvalidLatValue`, `TestEdge_InvalidLonValue` |
| 05-REQ-1.E4 | TS-05-E6, E7 | 4.3 | `TestEdge_LatOutOfRange`, `TestEdge_LonOutOfRange` |
| 05-REQ-2.1 | TS-05-5 | 3.1 | `TestPointInPolygon_Basic` |
| 05-REQ-2.2 | TS-05-6 | 3.1 | `TestPointInPolygon_ImplicitClose` |
| 05-REQ-2.3 | TS-05-7 | 3.1 | `TestPointInPolygon_Triangle` |
| 05-REQ-2.E1 | TS-05-E8 | 3.3 | `TestEdge_DegeneratePolygon` |
| 05-REQ-3.1 | TS-05-8 | 3.2 | `TestFuzziness_Configurable` |
| 05-REQ-3.2 | TS-05-9 | 3.2 | `TestFuzziness_NearBoundary` |
| 05-REQ-3.3 | TS-05-10 | 4.1 | `TestConfig_DefaultFuzziness` |
| 05-REQ-3.4 | TS-05-11 | 4.1 | `TestConfig_FuzzinessEnvVar` |
| 05-REQ-3.E1 | TS-05-E9 | 3.3 | `TestEdge_FuzzinessZero` |
| 05-REQ-4.1 | TS-05-12 | 4.4 | `TestAdapterMetadata_ValidOperator` |
| 05-REQ-4.2 | TS-05-13 | 4.4 | `TestAdapterMetadata_AllFields` |
| 05-REQ-4.3 | TS-05-14 | 4.4 | `TestAdapterMetadata_ContentType` |
| 05-REQ-4.E1 | TS-05-E10 | 4.4 | `TestEdge_UnknownOperator` |
| 05-REQ-5.1 | TS-05-15 | 4.2 | `TestHealthCheck` |
| 05-REQ-5.2 | TS-05-16 | 4.2 | `TestHealthCheck_NoAuth` |
| 05-REQ-6.1 | TS-05-17 | 2.2, 2.4 | `TestStore_LoadFromJSON` |
| 05-REQ-6.2 | TS-05-18 | 2.3 | `TestStore_DefaultOperators` |
| 05-REQ-6.3 | TS-05-19 | 4.1 | `TestConfig_OperatorsConfigEnvVar` |
| 05-REQ-6.4 | TS-05-20 | 2.3 | `TestStore_DefaultWhenNoConfig` |
| 05-REQ-6.E1 | TS-05-E11, E12 | 2.4 | `TestEdge_MalformedConfig`, `TestEdge_MissingConfigFile` |
| 05-REQ-7.1 | TS-05-21 | 4.5 | `TestAuth_OperatorsWithToken` |
| 05-REQ-7.2 | TS-05-22 | 4.5 | `TestAuth_TokenValidation` |
| 05-REQ-7.3 | TS-05-23 | 4.1 | `TestConfig_AuthTokensEnvVar` |
| 05-REQ-7.E1 | TS-05-E13 | 4.5 | `TestEdge_MissingAuthHeader` |
| 05-REQ-7.E2 | TS-05-E14 | 4.5 | `TestEdge_InvalidToken` |
| 05-REQ-7.E3 | TS-05-E15 | 4.5 | `TestEdge_InvalidAuthScheme` |
| Property 1 | TS-05-P1 | 3.1 | `TestProperty_GeofenceDeterminism` |
| Property 2 | TS-05-P2 | 3.2 | `TestProperty_FuzzinessMonotonicity` |
| Property 3 | TS-05-P3 | 3.1 | `TestProperty_InteriorPointsMatch` |
| Property 4 | TS-05-P4 | 3.3 | `TestProperty_DistantPointsNeverMatch` |
| Property 5 | TS-05-P5 | 2.3 | `TestProperty_AdapterMetadataConsistency` |
| Property 6 | TS-05-P6 | 4.5 | `TestProperty_AuthEnforcement` |
| Property 7 | TS-05-P7 | 4.2 | `TestProperty_HealthAlwaysAvailable` |
| (integration) | TS-05-I1 | 6.1, 7.1 | `TestIntegration_CLILookup` |
| (integration) | TS-05-I2 | 6.2, 7.1 | `TestIntegration_CLIAdapter` |
| (integration) | TS-05-I3 | 7.1 | `TestIntegration_FullDiscoveryFlow` |

## Notes

- **Test implementation language:** All tests are Go tests. Unit tests live
  alongside the source code in the `backend/parking-fee-service/` module.
  Integration tests live in `tests/integration/parking_fee_service/` as a
  separate Go module.
- **No infrastructure required:** All unit tests use `httptest` and in-memory
  stores. No database, no container runtime, no external services needed.
- **Integration test prerequisites:** The integration tests require the mock
  PARKING_APP CLI binary to be built. The test setup should build it
  automatically using `go build`.
- **Geofence precision:** The equirectangular approximation used for distance
  calculations is accurate enough for demo purposes at the scale of city
  zones (~1km). Production use would require the Haversine formula or
  Vincenty's formulae.
- **Session sizing:** Task groups 1 and 2 are small. Group 3 (geofence engine)
  is medium. Group 4 (REST server) is the largest but consists of
  straightforward HTTP handler code. Groups 6 and 7 are medium.
- **Dependency on Phase 1.2:** This spec assumes the Go module skeleton at
  `backend/parking-fee-service/` exists with a basic `main.go` and `go.mod`.
  If it does not exist yet, task 4.6 should create it.
