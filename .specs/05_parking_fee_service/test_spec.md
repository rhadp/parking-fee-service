# Test Specification: PARKING_FEE_SERVICE

## Overview

Tests cover geofence logic (unit), configuration loading (unit), data store lookups (unit), HTTP handlers (integration via httptest), and correctness properties (property tests). All tests run via `cd backend && go test -v ./parking-fee-service/...`. No external services are required -- the PARKING_FEE_SERVICE is self-contained.

## Test Cases

### TS-05-1: Operator Lookup Returns Matching Operators

**Requirement:** 05-REQ-1.1
**Type:** integration
**Description:** A GET request to `/operators?lat=&lon=` returns a JSON array of operators whose zones match the given coordinates.

**Preconditions:**
- Service is running with default Munich config (2 zones, 2 operators).

**Input:**
- `GET /operators?lat=48.1375&lon=11.5600` (inside munich-central zone)

**Expected:**
- HTTP 200
- JSON array containing one operator with `id: "parkhaus-munich"`, `zone_id: "munich-central"`
- Response includes `name`, `rate` fields

**Assertion pseudocode:**
```
resp = httptest.GET("/operators?lat=48.1375&lon=11.5600")
ASSERT resp.StatusCode == 200
body = json.Decode(resp.Body)
ASSERT len(body) >= 1
ASSERT body[0].id == "parkhaus-munich"
ASSERT body[0].zone_id == "munich-central"
```

### TS-05-2: Point-in-Polygon Ray Casting

**Requirement:** 05-REQ-1.2
**Type:** unit
**Description:** PointInPolygon correctly identifies coordinates inside and outside a convex polygon using ray casting.

**Preconditions:**
- A square polygon defined by coordinates: (48.14, 11.555), (48.14, 11.565), (48.135, 11.565), (48.135, 11.555).

**Input:**
- Point inside: (48.1375, 11.5600)
- Point outside: (48.1500, 11.5800)

**Expected:**
- Inside point returns `true`
- Outside point returns `false`

**Assertion pseudocode:**
```
polygon = [(48.14, 11.555), (48.14, 11.565), (48.135, 11.565), (48.135, 11.555)]
ASSERT geo.PointInPolygon({48.1375, 11.5600}, polygon) == true
ASSERT geo.PointInPolygon({48.1500, 11.5800}, polygon) == false
```

### TS-05-3: Proximity Matching Within Threshold

**Requirement:** 05-REQ-1.3
**Type:** unit
**Description:** FindMatchingZones includes zones where the point is outside the polygon but within the proximity threshold distance from the nearest edge.

**Preconditions:**
- Munich-central zone polygon loaded.
- Proximity threshold: 500 meters.

**Input:**
- A coordinate slightly outside the munich-central polygon but within 500m of its nearest edge.

**Expected:**
- The zone ID "munich-central" is included in the matching zones list.

**Assertion pseudocode:**
```
zones = [munich_central_zone]
// Point ~100m outside the polygon edge
nearPoint = {48.1405, 11.5600}
result = geo.FindMatchingZones(nearPoint, zones, 500.0)
ASSERT "munich-central" IN result
```

### TS-05-4: Multiple Operators Returned

**Requirement:** 05-REQ-1.4
**Type:** unit
**Description:** When multiple operators serve matching zones, all are returned.

**Preconditions:**
- Two operators both assigned to the same zone ID.

**Input:**
- A coordinate inside the shared zone.

**Expected:**
- Both operators are returned in the result array.

**Assertion pseudocode:**
```
store = NewStore(zones, [op1{zone_id: "z1"}, op2{zone_id: "z1"}])
result = store.GetOperatorsByZoneIDs(["z1"])
ASSERT len(result) == 2
```

### TS-05-5: Empty Array for No Matches

**Requirement:** 05-REQ-1.5
**Type:** integration
**Description:** When no operators match the given coordinates, the service returns an empty JSON array with HTTP 200.

**Preconditions:**
- Service is running with default Munich config.

**Input:**
- `GET /operators?lat=0.0&lon=0.0` (coordinates far from any zone)

**Expected:**
- HTTP 200
- Response body: `[]`

**Assertion pseudocode:**
```
resp = httptest.GET("/operators?lat=0.0&lon=0.0")
ASSERT resp.StatusCode == 200
body = json.Decode(resp.Body)
ASSERT len(body) == 0
```

### TS-05-6: Adapter Metadata Retrieval

**Requirement:** 05-REQ-2.1
**Type:** integration
**Description:** GET `/operators/{id}/adapter` returns adapter metadata with image_ref, checksum_sha256, and version.

**Preconditions:**
- Service is running with default config containing operator "parkhaus-munich".

**Input:**
- `GET /operators/parkhaus-munich/adapter`

**Expected:**
- HTTP 200
- JSON object with fields: `image_ref`, `checksum_sha256`, `version` (all non-empty strings)

**Assertion pseudocode:**
```
resp = httptest.GET("/operators/parkhaus-munich/adapter")
ASSERT resp.StatusCode == 200
body = json.Decode(resp.Body)
ASSERT body.image_ref != ""
ASSERT body.checksum_sha256 != ""
ASSERT body.version != ""
```

### TS-05-7: Adapter Metadata HTTP 200

**Requirement:** 05-REQ-2.2
**Type:** integration
**Description:** Successful adapter metadata retrieval returns HTTP 200.

**Preconditions:**
- Service is running with default config.

**Input:**
- `GET /operators/parkhaus-munich/adapter`

**Expected:**
- HTTP 200

**Assertion pseudocode:**
```
resp = httptest.GET("/operators/parkhaus-munich/adapter")
ASSERT resp.StatusCode == 200
```

### TS-05-8: Health Check

**Requirement:** 05-REQ-3.1
**Type:** integration
**Description:** GET `/health` returns HTTP 200 with `{"status":"ok"}`.

**Preconditions:**
- Service is running.

**Input:**
- `GET /health`

**Expected:**
- HTTP 200
- Body: `{"status":"ok"}`

**Assertion pseudocode:**
```
resp = httptest.GET("/health")
ASSERT resp.StatusCode == 200
body = json.Decode(resp.Body)
ASSERT body.status == "ok"
```

### TS-05-9: Config Loading from File

**Requirement:** 05-REQ-4.1
**Type:** unit
**Description:** LoadConfig reads configuration from the specified file path.

**Preconditions:**
- A temporary JSON config file with custom port, zones, and operators.

**Input:**
- Path to the temporary config file.

**Expected:**
- Config struct populated with values from the file.

**Assertion pseudocode:**
```
cfg, err = config.LoadConfig("/tmp/test-config.json")
ASSERT err == nil
ASSERT cfg.Port == 9090
ASSERT len(cfg.Zones) == 1
ASSERT len(cfg.Operators) == 1
```

### TS-05-10: Config Structure Validation

**Requirement:** 05-REQ-4.2
**Type:** unit
**Description:** The loaded configuration includes proximity threshold, port, zones with polygons, and operators with adapter metadata.

**Preconditions:**
- A valid config file with all required fields.

**Input:**
- Path to the config file.

**Expected:**
- Config has non-zero port, non-zero proximity threshold, zones with polygon coordinates, operators with zone associations and adapter metadata.

**Assertion pseudocode:**
```
cfg, err = config.LoadConfig(path)
ASSERT err == nil
ASSERT cfg.Port > 0
ASSERT cfg.ProximityThreshold > 0
ASSERT len(cfg.Zones) > 0
ASSERT len(cfg.Zones[0].Polygon) >= 3
ASSERT cfg.Operators[0].Adapter.ImageRef != ""
```

### TS-05-11: Proximity Threshold Used

**Requirement:** 05-REQ-4.3
**Type:** unit
**Description:** The configured proximity threshold is used for near-zone matching.

**Preconditions:**
- Config with proximity_threshold_meters = 100.

**Input:**
- A point 50m outside a zone polygon (within 100m threshold).
- A point 200m outside the same zone polygon (beyond 100m threshold).

**Expected:**
- 50m point matches the zone.
- 200m point does not match.

**Assertion pseudocode:**
```
zones = [zone_with_known_polygon]
near = geo.FindMatchingZones(point_50m_outside, zones, 100.0)
ASSERT len(near) == 1
far = geo.FindMatchingZones(point_200m_outside, zones, 100.0)
ASSERT len(far) == 0
```

### TS-05-12: Content-Type Header

**Requirement:** 05-REQ-5.1
**Type:** integration
**Description:** All responses set Content-Type: application/json.

**Preconditions:**
- Service is running.

**Input:**
- `GET /health`
- `GET /operators?lat=48.137&lon=11.575`
- `GET /operators/parkhaus-munich/adapter`

**Expected:**
- All responses have `Content-Type: application/json` header.

**Assertion pseudocode:**
```
FOR endpoint IN ["/health", "/operators?lat=48.137&lon=11.575", "/operators/parkhaus-munich/adapter"]:
    resp = httptest.GET(endpoint)
    ASSERT resp.Header("Content-Type") == "application/json"
```

### TS-05-13: Operator Lookup Response Fields

**Requirement:** 05-REQ-5.2
**Type:** integration
**Description:** Operator lookup response includes id, name, zone_id, and rate (with type, amount, currency) for each operator. The adapter field is NOT present.

**Preconditions:**
- Service is running with default config.

**Input:**
- `GET /operators?lat=48.1375&lon=11.5600` (inside munich-central)

**Expected:**
- Each operator object has: `id`, `name`, `zone_id`, `rate.type`, `rate.amount`, `rate.currency`.
- The `adapter` field is NOT present.

**Assertion pseudocode:**
```
resp = httptest.GET("/operators?lat=48.1375&lon=11.5600")
body = json.Decode(resp.Body)
op = body[0]
ASSERT op.id != ""
ASSERT op.name != ""
ASSERT op.zone_id != ""
ASSERT op.rate.type IN ["per-hour", "flat-fee"]
ASSERT op.rate.amount > 0
ASSERT op.rate.currency == "EUR"
ASSERT "adapter" NOT IN op
```

### TS-05-14: Error Response Format

**Requirement:** 05-REQ-5.3
**Type:** integration
**Description:** Error responses use the format `{"error":"<message>"}`.

**Preconditions:**
- Service is running.

**Input:**
- `GET /operators` (missing lat/lon)

**Expected:**
- HTTP 400
- Body contains `"error"` key with a string message.

**Assertion pseudocode:**
```
resp = httptest.GET("/operators")
ASSERT resp.StatusCode == 400
body = json.Decode(resp.Body)
ASSERT body.error != ""
```

### TS-05-15: Startup Logging

**Requirement:** 05-REQ-6.1
**Type:** integration
**Description:** On startup, the service logs version, port, zone count, operator count, and ready message.

**Preconditions:**
- Service starts with default config.

**Input:**
- Capture stdout/stderr during startup.

**Expected:**
- Log output contains port number, zone count, operator count.

**Assertion pseudocode:**
```
output = captureStartupLogs()
ASSERT "8080" IN output
ASSERT "zones" IN output
ASSERT "operators" IN output
```

### TS-05-16: Graceful Shutdown

**Requirement:** 05-REQ-6.2
**Type:** integration
**Description:** On SIGTERM or SIGINT, the service gracefully shuts down and exits with code 0.

**Preconditions:**
- Service is running.

**Input:**
- Send SIGTERM to the service process.

**Expected:**
- Service exits with code 0.

**Assertion pseudocode:**
```
proc = startService()
proc.Signal(SIGTERM)
exitCode = proc.Wait()
ASSERT exitCode == 0
```

## Edge Case Tests

### TS-05-E1: Missing lat/lon Parameters

**Requirement:** 05-REQ-1.E1
**Type:** integration
**Description:** Missing lat or lon query parameters return HTTP 400.

**Preconditions:**
- Service is running.

**Input:**
- `GET /operators` (no params)
- `GET /operators?lat=48.137` (missing lon)
- `GET /operators?lon=11.575` (missing lat)

**Expected:**
- HTTP 400
- Body: `{"error":"lat and lon query parameters are required"}`

**Assertion pseudocode:**
```
FOR url IN ["/operators", "/operators?lat=48.137", "/operators?lon=11.575"]:
    resp = httptest.GET(url)
    ASSERT resp.StatusCode == 400
    body = json.Decode(resp.Body)
    ASSERT body.error == "lat and lon query parameters are required"
```

### TS-05-E2: Invalid Coordinate Range

**Requirement:** 05-REQ-1.E2
**Type:** integration
**Description:** Coordinates outside valid ranges return HTTP 400.

**Preconditions:**
- Service is running.

**Input:**
- `GET /operators?lat=91.0&lon=11.575` (lat > 90)
- `GET /operators?lat=-91.0&lon=11.575` (lat < -90)
- `GET /operators?lat=48.137&lon=181.0` (lon > 180)
- `GET /operators?lat=48.137&lon=-181.0` (lon < -180)

**Expected:**
- HTTP 400
- Body: `{"error":"invalid coordinates"}`

**Assertion pseudocode:**
```
FOR params IN [("91.0","11.575"), ("-91.0","11.575"), ("48.137","181.0"), ("48.137","-181.0")]:
    resp = httptest.GET("/operators?lat=" + params[0] + "&lon=" + params[1])
    ASSERT resp.StatusCode == 400
    body = json.Decode(resp.Body)
    ASSERT body.error == "invalid coordinates"
```

### TS-05-E3: Non-Numeric Coordinates

**Requirement:** 05-REQ-1.E3
**Type:** integration
**Description:** Non-numeric lat or lon values return HTTP 400.

**Preconditions:**
- Service is running.

**Input:**
- `GET /operators?lat=abc&lon=11.575`
- `GET /operators?lat=48.137&lon=xyz`

**Expected:**
- HTTP 400
- Body: `{"error":"invalid coordinates"}`

**Assertion pseudocode:**
```
FOR params IN [("abc","11.575"), ("48.137","xyz")]:
    resp = httptest.GET("/operators?lat=" + params[0] + "&lon=" + params[1])
    ASSERT resp.StatusCode == 400
    body = json.Decode(resp.Body)
    ASSERT body.error == "invalid coordinates"
```

### TS-05-E4: Unknown Operator ID

**Requirement:** 05-REQ-2.E1
**Type:** integration
**Description:** Unknown operator ID returns HTTP 404.

**Preconditions:**
- Service is running with default config.

**Input:**
- `GET /operators/nonexistent-operator/adapter`

**Expected:**
- HTTP 404
- Body: `{"error":"operator not found"}`

**Assertion pseudocode:**
```
resp = httptest.GET("/operators/nonexistent-operator/adapter")
ASSERT resp.StatusCode == 404
body = json.Decode(resp.Body)
ASSERT body.error == "operator not found"
```

### TS-05-E5: Config File Missing Defaults

**Requirement:** 05-REQ-4.E1
**Type:** unit
**Description:** When config file does not exist, LoadConfig returns default configuration with Munich demo data.

**Preconditions:**
- No config file at the specified path.

**Input:**
- `config.LoadConfig("/nonexistent/path/config.json")`

**Expected:**
- Returns a valid Config with at least one zone and one operator.
- No error (uses defaults).

**Assertion pseudocode:**
```
cfg, err = config.LoadConfig("/nonexistent/path/config.json")
ASSERT err == nil
ASSERT len(cfg.Zones) >= 1
ASSERT len(cfg.Operators) >= 1
ASSERT cfg.Port == 8080
ASSERT cfg.ProximityThreshold == 500.0
```

### TS-05-E6: Invalid JSON Config

**Requirement:** 05-REQ-4.E2
**Type:** unit
**Description:** When config file contains invalid JSON, LoadConfig returns an error.

**Preconditions:**
- A temporary file containing `{invalid json`.

**Input:**
- Path to the invalid JSON file.

**Expected:**
- LoadConfig returns a non-nil error.

**Assertion pseudocode:**
```
cfg, err = config.LoadConfig("/tmp/invalid-config.json")
ASSERT err != nil
```

## Property Test Cases

### TS-05-P1: Point-in-Polygon Correctness

**Property:** Property 1 from design.md
**Validates:** 05-REQ-1.2, 05-REQ-1.5
**Type:** property
**Description:** For any coordinate inside a convex polygon, PointInPolygon returns true; for any coordinate outside the polygon by more than the threshold, FindMatchingZones returns empty.

**For any:** Random convex quadrilateral (4 vertices sorted by angle) and random test point.
**Invariant:** If the point's barycentric coordinates are all positive (inside), PointInPolygon returns true. If the point is >threshold from all edges, FindMatchingZones returns empty.

**Assertion pseudocode:**
```
FOR ANY polygon IN random_convex_quads, point IN random_coordinates:
    inside = point_is_geometrically_inside(polygon, point)
    IF inside:
        ASSERT geo.PointInPolygon(point, polygon) == true
    IF distance_to_nearest_edge(point, polygon) > threshold:
        ASSERT len(geo.FindMatchingZones(point, [zone], threshold)) == 0
```

### TS-05-P2: Proximity Matching

**Property:** Property 2 from design.md
**Validates:** 05-REQ-1.3
**Type:** property
**Description:** For any coordinate outside a zone but within threshold meters of the nearest edge, FindMatchingZones includes that zone.

**For any:** Random zone polygon and random point within threshold distance of an edge.
**Invariant:** The zone ID appears in the FindMatchingZones result.

**Assertion pseudocode:**
```
FOR ANY zone IN random_zones, point IN points_near_edge(zone, threshold):
    result = geo.FindMatchingZones(point, [zone], threshold)
    ASSERT zone.ID IN result
```

### TS-05-P3: Operator-Zone Association

**Property:** Property 3 from design.md
**Validates:** 05-REQ-1.4
**Type:** property
**Description:** For any set of zone IDs, GetOperatorsByZoneIDs returns all and only operators whose zone_id is in the set.

**For any:** Random subset of zone IDs from a set of operators.
**Invariant:** Every returned operator has zone_id in the input set, and every operator with zone_id in the set is returned.

**Assertion pseudocode:**
```
FOR ANY zoneIDs IN random_subsets(all_zone_ids):
    result = store.GetOperatorsByZoneIDs(zoneIDs)
    FOR op IN result:
        ASSERT op.ZoneID IN zoneIDs
    FOR op IN all_operators:
        IF op.ZoneID IN zoneIDs:
            ASSERT op IN result
```

### TS-05-P4: Coordinate Validation

**Property:** Property 4 from design.md
**Validates:** 05-REQ-1.E2, 05-REQ-1.E3
**Type:** property
**Description:** For any latitude outside [-90, 90] or longitude outside [-180, 180], the handler returns HTTP 400.

**For any:** Random float64 values for lat and lon, including out-of-range values.
**Invariant:** If lat is outside [-90,90] or lon is outside [-180,180], the response is 400.

**Assertion pseudocode:**
```
FOR ANY lat IN random_float64, lon IN random_float64:
    resp = handler("/operators?lat=" + lat + "&lon=" + lon)
    IF lat < -90 OR lat > 90 OR lon < -180 OR lon > 180:
        ASSERT resp.StatusCode == 400
```

### TS-05-P5: Adapter Metadata Completeness

**Property:** Property 5 from design.md
**Validates:** 05-REQ-2.1
**Type:** property
**Description:** For any valid operator ID, GetOperator returns an operator with non-empty image_ref, checksum_sha256, and version.

**For any:** All operator IDs in the config.
**Invariant:** The operator's adapter fields are all non-empty strings.

**Assertion pseudocode:**
```
FOR ANY opID IN all_operator_ids:
    op, found = store.GetOperator(opID)
    ASSERT found == true
    ASSERT op.Adapter.ImageRef != ""
    ASSERT op.Adapter.ChecksumSHA256 != ""
    ASSERT op.Adapter.Version != ""
```

### TS-05-P6: Config Defaults

**Property:** Property 6 from design.md
**Validates:** 05-REQ-4.E1
**Type:** property
**Description:** For any missing or nonexistent config file path, LoadConfig returns a valid default configuration.

**For any:** Random nonexistent file paths.
**Invariant:** The returned config has at least one zone and one operator, valid port, and valid proximity threshold.

**Assertion pseudocode:**
```
FOR ANY path IN random_nonexistent_paths:
    cfg, err = config.LoadConfig(path)
    ASSERT err == nil
    ASSERT len(cfg.Zones) >= 1
    ASSERT len(cfg.Operators) >= 1
    ASSERT cfg.Port > 0
    ASSERT cfg.ProximityThreshold > 0
```

## Coverage Matrix

| Requirement | Test Spec Entry | Type |
|-------------|-----------------|------|
| 05-REQ-1.1 | TS-05-1 | integration |
| 05-REQ-1.2 | TS-05-2 | unit |
| 05-REQ-1.3 | TS-05-3 | unit |
| 05-REQ-1.4 | TS-05-4 | unit |
| 05-REQ-1.5 | TS-05-5 | integration |
| 05-REQ-1.E1 | TS-05-E1 | integration |
| 05-REQ-1.E2 | TS-05-E2 | integration |
| 05-REQ-1.E3 | TS-05-E3 | integration |
| 05-REQ-2.1 | TS-05-6 | integration |
| 05-REQ-2.2 | TS-05-7 | integration |
| 05-REQ-2.E1 | TS-05-E4 | integration |
| 05-REQ-3.1 | TS-05-8 | integration |
| 05-REQ-4.1 | TS-05-9 | unit |
| 05-REQ-4.2 | TS-05-10 | unit |
| 05-REQ-4.3 | TS-05-11 | unit |
| 05-REQ-4.E1 | TS-05-E5 | unit |
| 05-REQ-4.E2 | TS-05-E6 | unit |
| 05-REQ-5.1 | TS-05-12 | integration |
| 05-REQ-5.2 | TS-05-13 | integration |
| 05-REQ-5.3 | TS-05-14 | integration |
| 05-REQ-6.1 | TS-05-15 | integration |
| 05-REQ-6.2 | TS-05-16 | integration |
| Property 1 | TS-05-P1 | property |
| Property 2 | TS-05-P2 | property |
| Property 3 | TS-05-P3 | property |
| Property 4 | TS-05-P4 | property |
| Property 5 | TS-05-P5 | property |
| Property 6 | TS-05-P6 | property |

## Integration Smoke Tests

### TS-05-SMOKE-1: End-to-End Operator Discovery

**Description:** Start the service binary, query an operator by Munich coordinates, retrieve its adapter metadata, and verify the health endpoint -- all via live HTTP requests.

**Preconditions:**
- The service binary is built and available.
- No config file present (service uses built-in defaults).

**Steps:**
1. Start the service binary as a subprocess on a free port.
2. Wait for the ready log message.
3. `GET /health` -- assert HTTP 200, body `{"status":"ok"}`.
4. `GET /operators?lat=48.1375&lon=11.5600` -- assert HTTP 200, response contains at least one operator with `zone_id: "munich-central"`.
5. Extract the operator `id` from step 4.
6. `GET /operators/{id}/adapter` -- assert HTTP 200, response contains non-empty `image_ref`, `checksum_sha256`, `version`.
7. Send SIGTERM to the subprocess.
8. Assert the process exits with code 0.

**Assertion pseudocode:**
```
proc = startServiceBinary(port=FREE_PORT)
waitForReady(proc)

resp1 = http.GET("http://localhost:{port}/health")
ASSERT resp1.StatusCode == 200

resp2 = http.GET("http://localhost:{port}/operators?lat=48.1375&lon=11.5600")
ASSERT resp2.StatusCode == 200
operators = json.Decode(resp2.Body)
ASSERT len(operators) >= 1
opID = operators[0].id

resp3 = http.GET("http://localhost:{port}/operators/{opID}/adapter")
ASSERT resp3.StatusCode == 200
adapter = json.Decode(resp3.Body)
ASSERT adapter.image_ref != ""

proc.Signal(SIGTERM)
ASSERT proc.Wait() == 0
```

### TS-05-SMOKE-2: Custom Config File

**Description:** Start the service with a custom config file via CONFIG_PATH and verify it uses the custom data.

**Preconditions:**
- A custom config JSON file with a single zone "test-zone" and operator "test-op".

**Steps:**
1. Write a temporary config file with one custom zone and operator.
2. Start the service binary with `CONFIG_PATH` set to the temporary file.
3. `GET /operators?lat={inside-test-zone}&lon={inside-test-zone}` -- assert the response contains `id: "test-op"`.
4. `GET /operators/test-op/adapter` -- assert HTTP 200 with custom adapter metadata.
5. Shut down the service.

**Assertion pseudocode:**
```
writeConfigFile("/tmp/smoke-config.json", custom_config)
proc = startServiceBinary(port=FREE_PORT, env={"CONFIG_PATH": "/tmp/smoke-config.json"})
waitForReady(proc)

resp = http.GET("http://localhost:{port}/operators?lat={test_lat}&lon={test_lon}")
ASSERT resp.StatusCode == 200
operators = json.Decode(resp.Body)
ASSERT operators[0].id == "test-op"

resp2 = http.GET("http://localhost:{port}/operators/test-op/adapter")
ASSERT resp2.StatusCode == 200

proc.Signal(SIGTERM)
ASSERT proc.Wait() == 0
```

### TS-05-SMOKE-3: Error Paths

**Description:** Verify error responses via live HTTP requests against the running service.

**Preconditions:**
- Service binary is running with defaults.

**Steps:**
1. `GET /operators` (no params) -- assert HTTP 400.
2. `GET /operators?lat=999&lon=999` -- assert HTTP 400.
3. `GET /operators?lat=abc&lon=def` -- assert HTTP 400.
4. `GET /operators/does-not-exist/adapter` -- assert HTTP 404.

**Assertion pseudocode:**
```
proc = startServiceBinary(port=FREE_PORT)
waitForReady(proc)

resp1 = http.GET("http://localhost:{port}/operators")
ASSERT resp1.StatusCode == 400

resp2 = http.GET("http://localhost:{port}/operators?lat=999&lon=999")
ASSERT resp2.StatusCode == 400

resp3 = http.GET("http://localhost:{port}/operators?lat=abc&lon=def")
ASSERT resp3.StatusCode == 400

resp4 = http.GET("http://localhost:{port}/operators/does-not-exist/adapter")
ASSERT resp4.StatusCode == 404

proc.Signal(SIGTERM)
ASSERT proc.Wait() == 0
```
