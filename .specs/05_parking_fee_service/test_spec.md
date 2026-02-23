# Test Specification: Parking Fee Service (Phase 2.4)

## Overview

This test specification translates every acceptance criterion and correctness
property from the requirements and design documents into concrete, executable
test contracts. Tests are organized into three categories:

- **Acceptance criterion tests (TS-05-N):** One per acceptance criterion.
  Implemented as Go tests in `backend/parking-fee-service/` (unit tests) and
  `tests/integration/parking_fee_service/` (integration tests).
- **Property tests (TS-05-PN):** One per correctness property. Verify
  invariants that must hold across the service.
- **Edge case tests (TS-05-EN):** One per edge case requirement. Verify
  error handling and boundary behavior.

Tests use Go's standard `testing` package. HTTP handler tests use
`net/http/httptest`. Integration tests that exercise the mock CLI binary
are tagged with `//go:build integration`.

## Test Cases

### TS-05-1: Operator lookup returns matching operators

**Requirement:** 05-REQ-1.1
**Type:** unit
**Description:** Verify that `GET /operators?lat={lat}&lon={lon}` with
coordinates inside a known zone returns the matching operator(s).

**Preconditions:**
- Server configured with default demo operators (Munich City Center zone).

**Input:**
- `GET /operators?lat=48.1351&lon=11.5750` (inside Munich City Center polygon)
- Authorization: `Bearer demo-token-1`

**Expected:**
- HTTP 200.
- Response body contains JSON with `operators` array including an operator with
  `operator_id` = `op-munich-01`.

**Assertion pseudocode:**
```go
func TestOperatorLookup_InsideZone(t *testing.T) {
    srv := setupTestServer(t)
    req := httptest.NewRequest("GET", "/operators?lat=48.1351&lon=11.5750", nil)
    req.Header.Set("Authorization", "Bearer demo-token-1")
    rec := httptest.NewRecorder()
    srv.ServeHTTP(rec, req)
    ASSERT rec.Code == 200
    body := parseJSON(rec.Body)
    ASSERT len(body.Operators) >= 1
    ASSERT body.Operators[0].OperatorID == "op-munich-01"
}
```

---

### TS-05-2: Operator lookup response includes required fields

**Requirement:** 05-REQ-1.2
**Type:** unit
**Description:** Verify that each operator in the lookup response includes
operator ID, name, zone ID, zone name, and rate with currency.

**Preconditions:**
- Server configured with default demo operators.

**Input:**
- `GET /operators?lat=48.1351&lon=11.5750` inside a known zone.
- Authorization: `Bearer demo-token-1`

**Expected:**
- Each operator in response has non-empty: `operator_id`, `name`,
  `zone.zone_id`, `zone.name`, `rate.amount_per_hour` > 0, `rate.currency`.

**Assertion pseudocode:**
```go
func TestOperatorLookup_ResponseFields(t *testing.T) {
    srv := setupTestServer(t)
    req := httptest.NewRequest("GET", "/operators?lat=48.1351&lon=11.5750", nil)
    req.Header.Set("Authorization", "Bearer demo-token-1")
    rec := httptest.NewRecorder()
    srv.ServeHTTP(rec, req)
    ASSERT rec.Code == 200
    body := parseJSON(rec.Body)
    for _, op := range body.Operators {
        ASSERT op.OperatorID != ""
        ASSERT op.Name != ""
        ASSERT op.Zone.ZoneID != ""
        ASSERT op.Zone.Name != ""
        ASSERT op.Rate.AmountPerHour > 0
        ASSERT op.Rate.Currency != ""
    }
}
```

---

### TS-05-3: Operator lookup returns multiple matching operators

**Requirement:** 05-REQ-1.3
**Type:** unit
**Description:** Verify that when multiple operators have overlapping zones,
all are returned.

**Preconditions:**
- Server configured with two operators whose zones overlap at a specific point.

**Input:**
- `GET /operators?lat={lat}&lon={lon}` where coordinates are in the overlap area.
- Authorization: `Bearer demo-token-1`

**Expected:**
- Response contains 2 or more operators.

**Assertion pseudocode:**
```go
func TestOperatorLookup_MultipleMatches(t *testing.T) {
    // Configure test server with overlapping zones
    srv := setupTestServerWithOverlap(t)
    req := httptest.NewRequest("GET", "/operators?lat=48.1351&lon=11.5750", nil)
    req.Header.Set("Authorization", "Bearer demo-token-1")
    rec := httptest.NewRecorder()
    srv.ServeHTTP(rec, req)
    ASSERT rec.Code == 200
    body := parseJSON(rec.Body)
    ASSERT len(body.Operators) >= 2
}
```

---

### TS-05-4: Operator lookup response content type

**Requirement:** 05-REQ-1.4
**Type:** unit
**Description:** Verify the response uses HTTP 200 and Content-Type
application/json.

**Preconditions:**
- Server running with default config.

**Input:**
- `GET /operators?lat=48.1351&lon=11.5750`
- Authorization: `Bearer demo-token-1`

**Expected:**
- Status code 200.
- Content-Type header contains `application/json`.

**Assertion pseudocode:**
```go
func TestOperatorLookup_ContentType(t *testing.T) {
    srv := setupTestServer(t)
    req := httptest.NewRequest("GET", "/operators?lat=48.1351&lon=11.5750", nil)
    req.Header.Set("Authorization", "Bearer demo-token-1")
    rec := httptest.NewRecorder()
    srv.ServeHTTP(rec, req)
    ASSERT rec.Code == 200
    ASSERT strings.Contains(rec.Header().Get("Content-Type"), "application/json")
}
```

---

### TS-05-5: Point-in-polygon ray casting algorithm

**Requirement:** 05-REQ-2.1
**Type:** unit
**Description:** Verify the point-in-polygon algorithm correctly classifies
points as inside or outside a known polygon.

**Preconditions:**
- A simple rectangular polygon defined with known vertices.

**Input:**
- Point clearly inside the polygon.
- Point clearly outside the polygon.

**Expected:**
- Inside point returns true.
- Outside point returns false.

**Assertion pseudocode:**
```go
func TestPointInPolygon_Basic(t *testing.T) {
    polygon := []Point{{48.14, 11.56}, {48.14, 11.59}, {48.13, 11.59}, {48.13, 11.56}}
    ASSERT PointInPolygon(Point{48.135, 11.575}, polygon) == true   // inside
    ASSERT PointInPolygon(Point{48.10, 11.50}, polygon) == false    // outside
}
```

---

### TS-05-6: Polygon defined as ordered vertex list

**Requirement:** 05-REQ-2.2
**Type:** unit
**Description:** Verify polygons are handled as ordered vertex lists with
implicit closing edge.

**Preconditions:**
- A triangle polygon defined with 3 vertices (not explicitly closed).

**Input:**
- Point inside the triangle.
- Point outside the triangle.

**Expected:**
- Correct classification despite polygon not having duplicate first/last vertex.

**Assertion pseudocode:**
```go
func TestPointInPolygon_ImplicitClose(t *testing.T) {
    // Triangle that does not repeat first vertex
    triangle := []Point{{48.14, 11.56}, {48.14, 11.59}, {48.12, 11.575}}
    ASSERT PointInPolygon(Point{48.135, 11.575}, triangle) == true   // inside
    ASSERT PointInPolygon(Point{48.10, 11.50}, triangle) == false    // outside
}
```

---

### TS-05-7: Polygon with 3 or more vertices

**Requirement:** 05-REQ-2.3
**Type:** unit
**Description:** Verify polygons with exactly 3 vertices (minimum) work
correctly.

**Preconditions:**
- A triangle polygon.

**Input:**
- Point inside and outside the triangle.

**Expected:**
- Correct classification.

**Assertion pseudocode:**
```go
func TestPointInPolygon_Triangle(t *testing.T) {
    triangle := []Point{{0, 0}, {0, 10}, {10, 0}}
    ASSERT PointInPolygon(Point{1, 1}, triangle) == true
    ASSERT PointInPolygon(Point{9, 9}, triangle) == false
}
```

---

### TS-05-8: Fuzziness buffer configurable

**Requirement:** 05-REQ-3.1
**Type:** unit
**Description:** Verify the fuzziness buffer is applied during matching.

**Preconditions:**
- A point that is outside a polygon but within 50 meters of an edge.

**Input:**
- FindMatches with fuzziness = 100 meters (should match).
- FindMatches with fuzziness = 10 meters (should not match).

**Expected:**
- Point matches with 100m buffer.
- Point does not match with 10m buffer.

**Assertion pseudocode:**
```go
func TestFuzziness_Configurable(t *testing.T) {
    polygon := []Point{{48.14, 11.56}, {48.14, 11.59}, {48.13, 11.59}, {48.13, 11.56}}
    // Point ~50m outside the northern edge
    nearPoint := Point{Lat: 48.1405, Lon: 11.575}
    matches100 := FindMatches(nearPoint.Lat, nearPoint.Lon, operators, 100)
    ASSERT len(matches100) >= 1
    matches10 := FindMatches(nearPoint.Lat, nearPoint.Lon, operators, 10)
    ASSERT len(matches10) == 0
}
```

---

### TS-05-9: Near-zone point matched within buffer

**Requirement:** 05-REQ-3.2
**Type:** unit
**Description:** Verify a point outside a polygon but within fuzziness distance
of its boundary is matched.

**Preconditions:**
- Known polygon. Known point just outside boundary.

**Input:**
- Point outside polygon, within buffer distance.

**Expected:**
- Operator is included in results.

**Assertion pseudocode:**
```go
func TestFuzziness_NearBoundary(t *testing.T) {
    polygon := []Point{{48.14, 11.56}, {48.14, 11.59}, {48.13, 11.59}, {48.13, 11.56}}
    // Point just north of polygon edge, ~30m outside
    nearPoint := Point{Lat: 48.14027, Lon: 11.575}
    dist := MinDistanceToPolygon(nearPoint, polygon)
    ASSERT dist > 0      // outside polygon
    ASSERT dist < 100    // within default buffer
    matches := FindMatches(nearPoint.Lat, nearPoint.Lon, operators, 100)
    ASSERT len(matches) >= 1
}
```

---

### TS-05-10: Default fuzziness is 100 meters

**Requirement:** 05-REQ-3.3
**Type:** unit
**Description:** Verify the default fuzziness buffer is 100 meters when not
explicitly configured.

**Preconditions:**
- No `FUZZINESS_METERS` environment variable set.

**Input:**
- Load config with default settings.

**Expected:**
- Config fuzziness value is 100.

**Assertion pseudocode:**
```go
func TestConfig_DefaultFuzziness(t *testing.T) {
    os.Unsetenv("FUZZINESS_METERS")
    cfg := LoadConfig()
    ASSERT cfg.FuzzinessMeters == 100
}
```

---

### TS-05-11: Fuzziness configurable via environment variable

**Requirement:** 05-REQ-3.4
**Type:** unit
**Description:** Verify fuzziness buffer is configurable via FUZZINESS_METERS
environment variable.

**Preconditions:**
- None.

**Input:**
- Set `FUZZINESS_METERS=250`, load config.

**Expected:**
- Config fuzziness value is 250.

**Assertion pseudocode:**
```go
func TestConfig_FuzzinessEnvVar(t *testing.T) {
    t.Setenv("FUZZINESS_METERS", "250")
    cfg := LoadConfig()
    ASSERT cfg.FuzzinessMeters == 250
}
```

---

### TS-05-12: Adapter metadata retrieval

**Requirement:** 05-REQ-4.1
**Type:** unit
**Description:** Verify `GET /operators/{id}/adapter` returns adapter metadata
for a valid operator ID.

**Preconditions:**
- Server configured with default operators.

**Input:**
- `GET /operators/op-munich-01/adapter`
- Authorization: `Bearer demo-token-1`

**Expected:**
- HTTP 200.
- Response body contains `image_ref`, `checksum_sha256`, `version`.

**Assertion pseudocode:**
```go
func TestAdapterMetadata_ValidOperator(t *testing.T) {
    srv := setupTestServer(t)
    req := httptest.NewRequest("GET", "/operators/op-munich-01/adapter", nil)
    req.Header.Set("Authorization", "Bearer demo-token-1")
    rec := httptest.NewRecorder()
    srv.ServeHTTP(rec, req)
    ASSERT rec.Code == 200
    body := parseAdapterJSON(rec.Body)
    ASSERT body.ImageRef != ""
    ASSERT body.ChecksumSHA256 != ""
    ASSERT body.Version != ""
}
```

---

### TS-05-13: Adapter metadata response fields

**Requirement:** 05-REQ-4.2
**Type:** unit
**Description:** Verify the adapter response contains `image_ref`,
`checksum_sha256`, and `version` fields.

**Preconditions:**
- Server configured with default operators.

**Input:**
- `GET /operators/op-munich-01/adapter`
- Authorization: `Bearer demo-token-1`

**Expected:**
- JSON response has all three fields present and non-empty.

**Assertion pseudocode:**
```go
func TestAdapterMetadata_AllFields(t *testing.T) {
    srv := setupTestServer(t)
    req := httptest.NewRequest("GET", "/operators/op-munich-01/adapter", nil)
    req.Header.Set("Authorization", "Bearer demo-token-1")
    rec := httptest.NewRecorder()
    srv.ServeHTTP(rec, req)
    ASSERT rec.Code == 200
    var resp map[string]interface{}
    json.Unmarshal(rec.Body.Bytes(), &resp)
    ASSERT resp["image_ref"] != nil
    ASSERT resp["checksum_sha256"] != nil
    ASSERT resp["version"] != nil
}
```

---

### TS-05-14: Adapter metadata content type

**Requirement:** 05-REQ-4.3
**Type:** unit
**Description:** Verify the adapter response uses HTTP 200 and Content-Type
application/json.

**Preconditions:**
- Server configured with default operators.

**Input:**
- `GET /operators/op-munich-01/adapter`
- Authorization: `Bearer demo-token-1`

**Expected:**
- Status 200, Content-Type application/json.

**Assertion pseudocode:**
```go
func TestAdapterMetadata_ContentType(t *testing.T) {
    srv := setupTestServer(t)
    req := httptest.NewRequest("GET", "/operators/op-munich-01/adapter", nil)
    req.Header.Set("Authorization", "Bearer demo-token-1")
    rec := httptest.NewRecorder()
    srv.ServeHTTP(rec, req)
    ASSERT rec.Code == 200
    ASSERT strings.Contains(rec.Header().Get("Content-Type"), "application/json")
}
```

---

### TS-05-15: Health check returns 200 OK

**Requirement:** 05-REQ-5.1
**Type:** unit
**Description:** Verify `GET /health` returns 200 with `{"status": "ok"}`.

**Preconditions:**
- Server running.

**Input:**
- `GET /health` (no auth required).

**Expected:**
- HTTP 200, body contains `{"status": "ok"}`.

**Assertion pseudocode:**
```go
func TestHealthCheck(t *testing.T) {
    srv := setupTestServer(t)
    req := httptest.NewRequest("GET", "/health", nil)
    rec := httptest.NewRecorder()
    srv.ServeHTTP(rec, req)
    ASSERT rec.Code == 200
    ASSERT strings.Contains(rec.Body.String(), `"status"`)
    ASSERT strings.Contains(rec.Body.String(), `"ok"`)
}
```

---

### TS-05-16: Health check does not require auth

**Requirement:** 05-REQ-5.2
**Type:** unit
**Description:** Verify the health endpoint works without an Authorization
header.

**Preconditions:**
- Server running with auth enabled.

**Input:**
- `GET /health` without Authorization header.

**Expected:**
- HTTP 200 (not 401).

**Assertion pseudocode:**
```go
func TestHealthCheck_NoAuth(t *testing.T) {
    srv := setupTestServer(t)
    req := httptest.NewRequest("GET", "/health", nil)
    // No Authorization header
    rec := httptest.NewRecorder()
    srv.ServeHTTP(rec, req)
    ASSERT rec.Code == 200
}
```

---

### TS-05-17: Operator data loaded from config

**Requirement:** 05-REQ-6.1
**Type:** unit
**Description:** Verify the operator store can load data from embedded defaults
or an external JSON file.

**Preconditions:**
- A valid JSON config file in `testdata/operators.json`.

**Input:**
- Load store from JSON file.

**Expected:**
- Store contains operators defined in the file.

**Assertion pseudocode:**
```go
func TestStore_LoadFromJSON(t *testing.T) {
    store, err := NewStoreFromFile("testdata/operators.json")
    ASSERT err == nil
    ops := store.ListOperators()
    ASSERT len(ops) >= 1
}
```

---

### TS-05-18: At least two demo operators

**Requirement:** 05-REQ-6.2
**Type:** unit
**Description:** Verify the embedded default dataset includes at least two
operators with different zones.

**Preconditions:**
- Default store loaded without external config.

**Input:**
- Load default store.

**Expected:**
- At least 2 operators with distinct zone IDs.

**Assertion pseudocode:**
```go
func TestStore_DefaultOperators(t *testing.T) {
    store := NewDefaultStore()
    ops := store.ListOperators()
    ASSERT len(ops) >= 2
    ASSERT ops[0].Zone.ZoneID != ops[1].Zone.ZoneID
}
```

---

### TS-05-19: Config file path via environment variable

**Requirement:** 05-REQ-6.3
**Type:** unit
**Description:** Verify the JSON config file path is configurable via
`OPERATORS_CONFIG` environment variable.

**Preconditions:**
- A valid JSON config file exists.

**Input:**
- Set `OPERATORS_CONFIG=testdata/operators.json`, load config.

**Expected:**
- Config reflects the specified file path.

**Assertion pseudocode:**
```go
func TestConfig_OperatorsConfigEnvVar(t *testing.T) {
    t.Setenv("OPERATORS_CONFIG", "testdata/operators.json")
    cfg := LoadConfig()
    ASSERT cfg.OperatorsConfigPath == "testdata/operators.json"
}
```

---

### TS-05-20: Default embedded dataset when no config

**Requirement:** 05-REQ-6.4
**Type:** unit
**Description:** Verify the service uses embedded defaults when
`OPERATORS_CONFIG` is not set.

**Preconditions:**
- `OPERATORS_CONFIG` not set.

**Input:**
- Load store without environment variable.

**Expected:**
- Store uses embedded default data with known operators.

**Assertion pseudocode:**
```go
func TestStore_DefaultWhenNoConfig(t *testing.T) {
    os.Unsetenv("OPERATORS_CONFIG")
    store := NewDefaultStore()
    ops := store.ListOperators()
    ASSERT len(ops) >= 2
    ASSERT ops[0].ID == "op-munich-01" || ops[1].ID == "op-munich-01"
}
```

---

### TS-05-21: Bearer token required on /operators

**Requirement:** 05-REQ-7.1
**Type:** unit
**Description:** Verify the `/operators` endpoint requires a valid bearer
token.

**Preconditions:**
- Server running with auth tokens configured.

**Input:**
- `GET /operators?lat=48.135&lon=11.575` with valid bearer token.

**Expected:**
- HTTP 200 (not 401).

**Assertion pseudocode:**
```go
func TestAuth_OperatorsWithToken(t *testing.T) {
    srv := setupTestServer(t)
    req := httptest.NewRequest("GET", "/operators?lat=48.135&lon=11.575", nil)
    req.Header.Set("Authorization", "Bearer demo-token-1")
    rec := httptest.NewRecorder()
    srv.ServeHTTP(rec, req)
    ASSERT rec.Code == 200
}
```

---

### TS-05-22: Token validated against configured set

**Requirement:** 05-REQ-7.2
**Type:** unit
**Description:** Verify the service validates tokens against a configured set.

**Preconditions:**
- Server configured with tokens ["demo-token-1", "demo-token-2"].

**Input:**
- Request with `Bearer demo-token-1` (valid).
- Request with `Bearer invalid-token` (invalid).

**Expected:**
- Valid token: HTTP 200.
- Invalid token: HTTP 401.

**Assertion pseudocode:**
```go
func TestAuth_TokenValidation(t *testing.T) {
    srv := setupTestServerWithTokens(t, []string{"demo-token-1", "demo-token-2"})
    // Valid token
    req1 := httptest.NewRequest("GET", "/operators?lat=48.135&lon=11.575", nil)
    req1.Header.Set("Authorization", "Bearer demo-token-1")
    rec1 := httptest.NewRecorder()
    srv.ServeHTTP(rec1, req1)
    ASSERT rec1.Code == 200

    // Invalid token
    req2 := httptest.NewRequest("GET", "/operators?lat=48.135&lon=11.575", nil)
    req2.Header.Set("Authorization", "Bearer invalid-token")
    rec2 := httptest.NewRecorder()
    srv.ServeHTTP(rec2, req2)
    ASSERT rec2.Code == 401
}
```

---

### TS-05-23: Auth tokens configurable via environment

**Requirement:** 05-REQ-7.3
**Type:** unit
**Description:** Verify accepted tokens are configurable via `AUTH_TOKENS`.

**Preconditions:**
- None.

**Input:**
- Set `AUTH_TOKENS=token-a,token-b`, load config.

**Expected:**
- Config contains both tokens.

**Assertion pseudocode:**
```go
func TestConfig_AuthTokensEnvVar(t *testing.T) {
    t.Setenv("AUTH_TOKENS", "token-a,token-b")
    cfg := LoadConfig()
    ASSERT len(cfg.AuthTokens) == 2
    ASSERT cfg.AuthTokens[0] == "token-a"
    ASSERT cfg.AuthTokens[1] == "token-b"
}
```

---

## Edge Case Tests

### TS-05-E1: No operators match location

**Requirement:** 05-REQ-1.E1
**Type:** unit
**Description:** Verify empty array returned when no operators match
coordinates.

**Preconditions:**
- Server configured with default operators.

**Input:**
- `GET /operators?lat=0.0&lon=0.0` (middle of Gulf of Guinea — no zones).
- Authorization: `Bearer demo-token-1`

**Expected:**
- HTTP 200 with `{"operators": []}`.

**Assertion pseudocode:**
```go
func TestEdge_NoMatchingOperators(t *testing.T) {
    srv := setupTestServer(t)
    req := httptest.NewRequest("GET", "/operators?lat=0.0&lon=0.0", nil)
    req.Header.Set("Authorization", "Bearer demo-token-1")
    rec := httptest.NewRecorder()
    srv.ServeHTTP(rec, req)
    ASSERT rec.Code == 200
    body := parseJSON(rec.Body)
    ASSERT len(body.Operators) == 0
}
```

---

### TS-05-E2: Missing lat parameter

**Requirement:** 05-REQ-1.E2
**Type:** unit
**Description:** Verify HTTP 400 when `lat` query parameter is missing.

**Preconditions:**
- Server running.

**Input:**
- `GET /operators?lon=11.575` (missing lat).
- Authorization: `Bearer demo-token-1`

**Expected:**
- HTTP 400 with error body mentioning "lat".

**Assertion pseudocode:**
```go
func TestEdge_MissingLatParam(t *testing.T) {
    srv := setupTestServer(t)
    req := httptest.NewRequest("GET", "/operators?lon=11.575", nil)
    req.Header.Set("Authorization", "Bearer demo-token-1")
    rec := httptest.NewRecorder()
    srv.ServeHTTP(rec, req)
    ASSERT rec.Code == 400
    ASSERT strings.Contains(rec.Body.String(), "lat")
}
```

---

### TS-05-E3: Missing lon parameter

**Requirement:** 05-REQ-1.E2
**Type:** unit
**Description:** Verify HTTP 400 when `lon` query parameter is missing.

**Preconditions:**
- Server running.

**Input:**
- `GET /operators?lat=48.135` (missing lon).
- Authorization: `Bearer demo-token-1`

**Expected:**
- HTTP 400 with error body mentioning "lon".

**Assertion pseudocode:**
```go
func TestEdge_MissingLonParam(t *testing.T) {
    srv := setupTestServer(t)
    req := httptest.NewRequest("GET", "/operators?lat=48.135", nil)
    req.Header.Set("Authorization", "Bearer demo-token-1")
    rec := httptest.NewRecorder()
    srv.ServeHTTP(rec, req)
    ASSERT rec.Code == 400
    ASSERT strings.Contains(rec.Body.String(), "lon")
}
```

---

### TS-05-E4: Invalid lat value (non-numeric)

**Requirement:** 05-REQ-1.E3
**Type:** unit
**Description:** Verify HTTP 400 when `lat` is not a valid number.

**Preconditions:**
- Server running.

**Input:**
- `GET /operators?lat=abc&lon=11.575`
- Authorization: `Bearer demo-token-1`

**Expected:**
- HTTP 400 with error body mentioning "lat".

**Assertion pseudocode:**
```go
func TestEdge_InvalidLatValue(t *testing.T) {
    srv := setupTestServer(t)
    req := httptest.NewRequest("GET", "/operators?lat=abc&lon=11.575", nil)
    req.Header.Set("Authorization", "Bearer demo-token-1")
    rec := httptest.NewRecorder()
    srv.ServeHTTP(rec, req)
    ASSERT rec.Code == 400
    ASSERT strings.Contains(rec.Body.String(), "lat")
}
```

---

### TS-05-E5: Invalid lon value (non-numeric)

**Requirement:** 05-REQ-1.E3
**Type:** unit
**Description:** Verify HTTP 400 when `lon` is not a valid number.

**Preconditions:**
- Server running.

**Input:**
- `GET /operators?lat=48.135&lon=xyz`
- Authorization: `Bearer demo-token-1`

**Expected:**
- HTTP 400 with error body mentioning "lon".

**Assertion pseudocode:**
```go
func TestEdge_InvalidLonValue(t *testing.T) {
    srv := setupTestServer(t)
    req := httptest.NewRequest("GET", "/operators?lat=48.135&lon=xyz", nil)
    req.Header.Set("Authorization", "Bearer demo-token-1")
    rec := httptest.NewRecorder()
    srv.ServeHTTP(rec, req)
    ASSERT rec.Code == 400
    ASSERT strings.Contains(rec.Body.String(), "lon")
}
```

---

### TS-05-E6: Latitude out of range

**Requirement:** 05-REQ-1.E4
**Type:** unit
**Description:** Verify HTTP 400 when `lat` is outside [-90, 90].

**Preconditions:**
- Server running.

**Input:**
- `GET /operators?lat=91.0&lon=11.575`
- Authorization: `Bearer demo-token-1`

**Expected:**
- HTTP 400 with error body describing valid range.

**Assertion pseudocode:**
```go
func TestEdge_LatOutOfRange(t *testing.T) {
    srv := setupTestServer(t)
    req := httptest.NewRequest("GET", "/operators?lat=91.0&lon=11.575", nil)
    req.Header.Set("Authorization", "Bearer demo-token-1")
    rec := httptest.NewRecorder()
    srv.ServeHTTP(rec, req)
    ASSERT rec.Code == 400
    ASSERT strings.Contains(rec.Body.String(), "lat")
    ASSERT strings.Contains(rec.Body.String(), "-90") || strings.Contains(rec.Body.String(), "90")
}
```

---

### TS-05-E7: Longitude out of range

**Requirement:** 05-REQ-1.E4
**Type:** unit
**Description:** Verify HTTP 400 when `lon` is outside [-180, 180].

**Preconditions:**
- Server running.

**Input:**
- `GET /operators?lat=48.135&lon=200.0`
- Authorization: `Bearer demo-token-1`

**Expected:**
- HTTP 400 with error body describing valid range.

**Assertion pseudocode:**
```go
func TestEdge_LonOutOfRange(t *testing.T) {
    srv := setupTestServer(t)
    req := httptest.NewRequest("GET", "/operators?lat=48.135&lon=200.0", nil)
    req.Header.Set("Authorization", "Bearer demo-token-1")
    rec := httptest.NewRecorder()
    srv.ServeHTTP(rec, req)
    ASSERT rec.Code == 400
    ASSERT strings.Contains(rec.Body.String(), "lon")
}
```

---

### TS-05-E8: Degenerate polygon skipped

**Requirement:** 05-REQ-2.E1
**Type:** unit
**Description:** Verify operators with fewer than 3 polygon vertices are
skipped during matching.

**Preconditions:**
- Store includes an operator with a 2-vertex polygon.

**Input:**
- Find matches at a point that would be inside the degenerate polygon.

**Expected:**
- The degenerate-polygon operator is NOT in results.

**Assertion pseudocode:**
```go
func TestEdge_DegeneratePolygon(t *testing.T) {
    ops := []Operator{{
        ID:   "degenerate",
        Zone: Zone{Polygon: []Point{{48.14, 11.56}, {48.13, 11.59}}}, // only 2 vertices
    }}
    matches := FindMatches(48.135, 11.575, ops, 0)
    ASSERT len(matches) == 0
}
```

---

### TS-05-E9: Fuzziness zero disables near-zone matching

**Requirement:** 05-REQ-3.E1
**Type:** unit
**Description:** Verify fuzziness=0 returns only exact point-in-polygon
matches.

**Preconditions:**
- Point outside polygon but near its boundary.

**Input:**
- FindMatches with fuzziness = 0.

**Expected:**
- Near-boundary point is NOT matched.

**Assertion pseudocode:**
```go
func TestEdge_FuzzinessZero(t *testing.T) {
    polygon := []Point{{48.14, 11.56}, {48.14, 11.59}, {48.13, 11.59}, {48.13, 11.56}}
    // Point just outside polygon
    nearPoint := Point{Lat: 48.1405, Lon: 11.575}
    ASSERT PointInPolygon(nearPoint, polygon) == false  // confirm outside
    ops := []Operator{{ID: "test", Zone: Zone{Polygon: polygon}}}
    matches := FindMatches(nearPoint.Lat, nearPoint.Lon, ops, 0)
    ASSERT len(matches) == 0
}
```

---

### TS-05-E10: Unknown operator ID returns 404

**Requirement:** 05-REQ-4.E1
**Type:** unit
**Description:** Verify HTTP 404 when requesting adapter for unknown operator.

**Preconditions:**
- Server running with default operators.

**Input:**
- `GET /operators/op-nonexistent/adapter`
- Authorization: `Bearer demo-token-1`

**Expected:**
- HTTP 404 with error body.

**Assertion pseudocode:**
```go
func TestEdge_UnknownOperator(t *testing.T) {
    srv := setupTestServer(t)
    req := httptest.NewRequest("GET", "/operators/op-nonexistent/adapter", nil)
    req.Header.Set("Authorization", "Bearer demo-token-1")
    rec := httptest.NewRecorder()
    srv.ServeHTTP(rec, req)
    ASSERT rec.Code == 404
    ASSERT strings.Contains(rec.Body.String(), "op-nonexistent")
}
```

---

### TS-05-E11: Malformed config file prevents startup

**Requirement:** 05-REQ-6.E1
**Type:** unit
**Description:** Verify the service fails to load a malformed JSON config file.

**Preconditions:**
- A file `testdata/invalid.json` with invalid JSON content.

**Input:**
- Attempt to load store from `testdata/invalid.json`.

**Expected:**
- Error returned (non-nil).

**Assertion pseudocode:**
```go
func TestEdge_MalformedConfig(t *testing.T) {
    store, err := NewStoreFromFile("testdata/invalid.json")
    ASSERT err != nil
    ASSERT store == nil
}
```

---

### TS-05-E12: Missing config file prevents startup

**Requirement:** 05-REQ-6.E1
**Type:** unit
**Description:** Verify the service fails to load a non-existent config file.

**Preconditions:**
- No file at the specified path.

**Input:**
- Attempt to load store from `testdata/does_not_exist.json`.

**Expected:**
- Error returned (non-nil).

**Assertion pseudocode:**
```go
func TestEdge_MissingConfigFile(t *testing.T) {
    store, err := NewStoreFromFile("testdata/does_not_exist.json")
    ASSERT err != nil
    ASSERT store == nil
}
```

---

### TS-05-E13: Missing Authorization header

**Requirement:** 05-REQ-7.E1
**Type:** unit
**Description:** Verify HTTP 401 when Authorization header is missing.

**Preconditions:**
- Server running with auth enabled.

**Input:**
- `GET /operators?lat=48.135&lon=11.575` without Authorization header.

**Expected:**
- HTTP 401 with `{"error": "missing authorization header"}`.

**Assertion pseudocode:**
```go
func TestEdge_MissingAuthHeader(t *testing.T) {
    srv := setupTestServer(t)
    req := httptest.NewRequest("GET", "/operators?lat=48.135&lon=11.575", nil)
    rec := httptest.NewRecorder()
    srv.ServeHTTP(rec, req)
    ASSERT rec.Code == 401
    ASSERT strings.Contains(rec.Body.String(), "missing authorization header")
}
```

---

### TS-05-E14: Invalid bearer token

**Requirement:** 05-REQ-7.E2
**Type:** unit
**Description:** Verify HTTP 401 when bearer token is not in accepted set.

**Preconditions:**
- Server running with configured tokens.

**Input:**
- `GET /operators?lat=48.135&lon=11.575` with `Bearer wrong-token`.

**Expected:**
- HTTP 401 with `{"error": "invalid token"}`.

**Assertion pseudocode:**
```go
func TestEdge_InvalidToken(t *testing.T) {
    srv := setupTestServer(t)
    req := httptest.NewRequest("GET", "/operators?lat=48.135&lon=11.575", nil)
    req.Header.Set("Authorization", "Bearer wrong-token")
    rec := httptest.NewRecorder()
    srv.ServeHTTP(rec, req)
    ASSERT rec.Code == 401
    ASSERT strings.Contains(rec.Body.String(), "invalid token")
}
```

---

### TS-05-E15: Invalid authorization scheme

**Requirement:** 05-REQ-7.E3
**Type:** unit
**Description:** Verify HTTP 401 when Authorization header uses wrong scheme.

**Preconditions:**
- Server running.

**Input:**
- `GET /operators?lat=48.135&lon=11.575` with `Basic dXNlcjpwYXNz`.

**Expected:**
- HTTP 401 with `{"error": "invalid authorization scheme"}`.

**Assertion pseudocode:**
```go
func TestEdge_InvalidAuthScheme(t *testing.T) {
    srv := setupTestServer(t)
    req := httptest.NewRequest("GET", "/operators?lat=48.135&lon=11.575", nil)
    req.Header.Set("Authorization", "Basic dXNlcjpwYXNz")
    rec := httptest.NewRecorder()
    srv.ServeHTTP(rec, req)
    ASSERT rec.Code == 401
    ASSERT strings.Contains(rec.Body.String(), "invalid authorization scheme")
}
```

---

## Property Test Cases

### TS-05-P1: Geofence Determinism

**Property:** Property 1 from design.md
**Validates:** 05-REQ-2.1
**Type:** property
**Description:** For any point and polygon, PointInPolygon returns the same
result every time.

**For any:** Point P and polygon Q
**Invariant:** 100 consecutive calls to `PointInPolygon(P, Q)` all return the
same value.

**Assertion pseudocode:**
```go
func TestProperty_GeofenceDeterminism(t *testing.T) {
    polygon := defaultTestPolygon()
    points := []Point{{48.135, 11.575}, {48.10, 11.50}, {48.14, 11.56}}
    for _, p := range points {
        expected := PointInPolygon(p, polygon)
        for i := 0; i < 100; i++ {
            ASSERT PointInPolygon(p, polygon) == expected
        }
    }
}
```

---

### TS-05-P2: Fuzziness Monotonicity

**Property:** Property 2 from design.md
**Validates:** 05-REQ-3.1, 05-REQ-3.2
**Type:** property
**Description:** If a point is matched with fuzziness D1, it is also matched
with any D2 > D1.

**For any:** Near-boundary point P, fuzziness values D1 < D2
**Invariant:** match(P, D1) implies match(P, D2).

**Assertion pseudocode:**
```go
func TestProperty_FuzzinessMonotonicity(t *testing.T) {
    ops := defaultTestOperators()
    nearPoint := Point{Lat: 48.1405, Lon: 11.575}  // near polygon boundary
    for d1 := float64(10); d1 <= 500; d1 += 10 {
        m1 := FindMatches(nearPoint.Lat, nearPoint.Lon, ops, d1)
        m2 := FindMatches(nearPoint.Lat, nearPoint.Lon, ops, d1+50)
        if len(m1) > 0 {
            ASSERT len(m2) >= len(m1)
        }
    }
}
```

---

### TS-05-P3: Interior Points Always Match

**Property:** Property 3 from design.md
**Validates:** 05-REQ-2.1
**Type:** property
**Description:** Points strictly inside a polygon match regardless of
fuzziness (including 0).

**For any:** Point P strictly inside polygon Q
**Invariant:** PointInPolygon(P, Q) == true.

**Assertion pseudocode:**
```go
func TestProperty_InteriorPointsMatch(t *testing.T) {
    polygon := defaultTestPolygon()
    interiorPoints := []Point{
        {48.135, 11.575},   // center
        {48.1390, 11.5610}, // near corner but inside
        {48.1310, 11.5890}, // near opposite corner but inside
    }
    for _, p := range interiorPoints {
        ASSERT PointInPolygon(p, polygon) == true
        // Also verify with fuzziness = 0
        matches := FindMatches(p.Lat, p.Lon, testOps, 0)
        ASSERT len(matches) >= 1
    }
}
```

---

### TS-05-P4: Distant Points Never Match

**Property:** Property 4 from design.md
**Validates:** 05-REQ-2.1, 05-REQ-3.2
**Type:** property
**Description:** Points more than fuzzinessMeters from every polygon edge are
never matched.

**For any:** Point P where minDist(P, Q) > fuzzinessMeters
**Invariant:** FindMatches returns empty for P.

**Assertion pseudocode:**
```go
func TestProperty_DistantPointsNeverMatch(t *testing.T) {
    ops := defaultTestOperators()
    distantPoints := []Point{
        {0.0, 0.0},         // Gulf of Guinea
        {40.0, -74.0},      // New York
        {51.5, -0.1},       // London
    }
    for _, p := range distantPoints {
        matches := FindMatches(p.Lat, p.Lon, ops, 100)
        ASSERT len(matches) == 0
    }
}
```

---

### TS-05-P5: Adapter Metadata Consistency

**Property:** Property 5 from design.md
**Validates:** 05-REQ-4.2
**Type:** property
**Description:** For every operator, adapter metadata has non-empty required
fields.

**For any:** Operator O in the store
**Invariant:** O.Adapter.ImageRef, O.Adapter.ChecksumSHA256, O.Adapter.Version
are all non-empty strings.

**Assertion pseudocode:**
```go
func TestProperty_AdapterMetadataConsistency(t *testing.T) {
    store := NewDefaultStore()
    for _, op := range store.ListOperators() {
        ASSERT op.Adapter.ImageRef != ""
        ASSERT op.Adapter.ChecksumSHA256 != ""
        ASSERT op.Adapter.Version != ""
    }
}
```

---

### TS-05-P6: Authentication Enforcement

**Property:** Property 6 from design.md
**Validates:** 05-REQ-7.1, 05-REQ-7.2
**Type:** property
**Description:** All protected endpoints return 401 without a valid token and
never leak operator data.

**For any:** Protected endpoint E, request without valid token
**Invariant:** Response status is 401 and body does not contain operator IDs.

**Assertion pseudocode:**
```go
func TestProperty_AuthEnforcement(t *testing.T) {
    srv := setupTestServer(t)
    endpoints := []string{
        "/operators?lat=48.135&lon=11.575",
        "/operators/op-munich-01/adapter",
    }
    for _, ep := range endpoints {
        // No auth header
        req := httptest.NewRequest("GET", ep, nil)
        rec := httptest.NewRecorder()
        srv.ServeHTTP(rec, req)
        ASSERT rec.Code == 401
        ASSERT !strings.Contains(rec.Body.String(), "op-munich-01")

        // Wrong token
        req2 := httptest.NewRequest("GET", ep, nil)
        req2.Header.Set("Authorization", "Bearer wrong")
        rec2 := httptest.NewRecorder()
        srv.ServeHTTP(rec2, req2)
        ASSERT rec2.Code == 401
        ASSERT !strings.Contains(rec2.Body.String(), "op-munich-01")
    }
}
```

---

### TS-05-P7: Health Endpoint Availability

**Property:** Property 7 from design.md
**Validates:** 05-REQ-5.1
**Type:** property
**Description:** Health endpoint always returns 200 regardless of store state.

**For any:** Store state (default, empty, custom)
**Invariant:** `GET /health` returns HTTP 200.

**Assertion pseudocode:**
```go
func TestProperty_HealthAlwaysAvailable(t *testing.T) {
    // With default store
    srv1 := setupTestServer(t)
    req1 := httptest.NewRequest("GET", "/health", nil)
    rec1 := httptest.NewRecorder()
    srv1.ServeHTTP(rec1, req1)
    ASSERT rec1.Code == 200

    // With empty store
    srv2 := setupTestServerWithStore(t, NewEmptyStore())
    req2 := httptest.NewRequest("GET", "/health", nil)
    rec2 := httptest.NewRecorder()
    srv2.ServeHTTP(rec2, req2)
    ASSERT rec2.Code == 200
}
```

---

## Integration Test Cases

### TS-05-I1: Mock CLI operator lookup

**Type:** integration
**Description:** Verify the mock PARKING_APP CLI `lookup` command calls
PARKING_FEE_SERVICE and displays results.

**Preconditions:**
- PARKING_FEE_SERVICE running on a known port.
- Mock CLI binary built.

**Input:**
- `parking-app-cli lookup --lat=48.1351 --lon=11.5750 --pfs-url=http://localhost:<port> --token=demo-token-1`

**Expected:**
- Exit code 0.
- Stdout contains operator name and rate information.

**Assertion pseudocode:**
```go
func TestIntegration_CLILookup(t *testing.T) {
    srv := startTestPFS(t)
    result := exec("parking-app-cli", "lookup",
        "--lat=48.1351", "--lon=11.5750",
        "--pfs-url="+srv.URL, "--token=demo-token-1")
    ASSERT result.ExitCode == 0
    ASSERT strings.Contains(result.Stdout, "Munich City Parking")
    ASSERT strings.Contains(result.Stdout, "2.50")
}
```

---

### TS-05-I2: Mock CLI adapter metadata retrieval

**Type:** integration
**Description:** Verify the mock PARKING_APP CLI `adapter` command calls
PARKING_FEE_SERVICE and displays adapter metadata.

**Preconditions:**
- PARKING_FEE_SERVICE running.
- Mock CLI binary built.

**Input:**
- `parking-app-cli adapter --operator-id=op-munich-01 --pfs-url=http://localhost:<port> --token=demo-token-1`

**Expected:**
- Exit code 0.
- Stdout contains image reference and version.

**Assertion pseudocode:**
```go
func TestIntegration_CLIAdapter(t *testing.T) {
    srv := startTestPFS(t)
    result := exec("parking-app-cli", "adapter",
        "--operator-id=op-munich-01",
        "--pfs-url="+srv.URL, "--token=demo-token-1")
    ASSERT result.ExitCode == 0
    ASSERT strings.Contains(result.Stdout, "us-docker.pkg.dev")
    ASSERT strings.Contains(result.Stdout, "1.0.0")
}
```

---

### TS-05-I3: Full discovery flow

**Type:** integration
**Description:** Verify the end-to-end flow: lookup operators by location,
then retrieve adapter metadata for a discovered operator.

**Preconditions:**
- PARKING_FEE_SERVICE running.

**Input:**
1. `GET /operators?lat=48.1351&lon=11.5750` to discover operators.
2. Extract operator ID from response.
3. `GET /operators/{id}/adapter` to get adapter metadata.

**Expected:**
- Step 1 returns at least one operator.
- Step 3 returns valid adapter metadata matching the operator from step 1.

**Assertion pseudocode:**
```go
func TestIntegration_FullDiscoveryFlow(t *testing.T) {
    srv := startTestPFS(t)
    // Step 1: Lookup
    resp1 := httpGet(srv.URL+"/operators?lat=48.1351&lon=11.5750",
        "Bearer demo-token-1")
    ASSERT resp1.StatusCode == 200
    ops := parseOperators(resp1.Body)
    ASSERT len(ops) >= 1

    // Step 2: Get adapter for first operator
    opID := ops[0].OperatorID
    resp2 := httpGet(srv.URL+"/operators/"+opID+"/adapter",
        "Bearer demo-token-1")
    ASSERT resp2.StatusCode == 200
    adapter := parseAdapter(resp2.Body)
    ASSERT adapter.ImageRef != ""
    ASSERT adapter.ChecksumSHA256 != ""
    ASSERT adapter.Version != ""
}
```

---

## Coverage Matrix

| Requirement    | Test Spec Entry  | Type        |
|----------------|------------------|-------------|
| 05-REQ-1.1     | TS-05-1          | unit        |
| 05-REQ-1.2     | TS-05-2          | unit        |
| 05-REQ-1.3     | TS-05-3          | unit        |
| 05-REQ-1.4     | TS-05-4          | unit        |
| 05-REQ-1.E1    | TS-05-E1         | unit        |
| 05-REQ-1.E2    | TS-05-E2, E3     | unit        |
| 05-REQ-1.E3    | TS-05-E4, E5     | unit        |
| 05-REQ-1.E4    | TS-05-E6, E7     | unit        |
| 05-REQ-2.1     | TS-05-5          | unit        |
| 05-REQ-2.2     | TS-05-6          | unit        |
| 05-REQ-2.3     | TS-05-7          | unit        |
| 05-REQ-2.E1    | TS-05-E8         | unit        |
| 05-REQ-3.1     | TS-05-8          | unit        |
| 05-REQ-3.2     | TS-05-9          | unit        |
| 05-REQ-3.3     | TS-05-10         | unit        |
| 05-REQ-3.4     | TS-05-11         | unit        |
| 05-REQ-3.E1    | TS-05-E9         | unit        |
| 05-REQ-4.1     | TS-05-12         | unit        |
| 05-REQ-4.2     | TS-05-13         | unit        |
| 05-REQ-4.3     | TS-05-14         | unit        |
| 05-REQ-4.E1    | TS-05-E10        | unit        |
| 05-REQ-5.1     | TS-05-15         | unit        |
| 05-REQ-5.2     | TS-05-16         | unit        |
| 05-REQ-6.1     | TS-05-17         | unit        |
| 05-REQ-6.2     | TS-05-18         | unit        |
| 05-REQ-6.3     | TS-05-19         | unit        |
| 05-REQ-6.4     | TS-05-20         | unit        |
| 05-REQ-6.E1    | TS-05-E11, E12   | unit        |
| 05-REQ-7.1     | TS-05-21         | unit        |
| 05-REQ-7.2     | TS-05-22         | unit        |
| 05-REQ-7.3     | TS-05-23         | unit        |
| 05-REQ-7.E1    | TS-05-E13        | unit        |
| 05-REQ-7.E2    | TS-05-E14        | unit        |
| 05-REQ-7.E3    | TS-05-E15        | unit        |
| Property 1     | TS-05-P1         | property    |
| Property 2     | TS-05-P2         | property    |
| Property 3     | TS-05-P3         | property    |
| Property 4     | TS-05-P4         | property    |
| Property 5     | TS-05-P5         | property    |
| Property 6     | TS-05-P6         | property    |
| Property 7     | TS-05-P7         | property    |
| (integration)  | TS-05-I1         | integration |
| (integration)  | TS-05-I2         | integration |
| (integration)  | TS-05-I3         | integration |
