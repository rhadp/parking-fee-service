package handler

import (
	"encoding/json"
	"fmt"
	"math"
	"math/rand"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/model"
	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/store"
)

// Default Munich test data matching the design doc.
var testZones = []model.Zone{
	{
		ID:   "munich-central",
		Name: "Munich Central Station Area",
		Polygon: []model.Coordinate{
			{Lat: 48.1400, Lon: 11.5550},
			{Lat: 48.1400, Lon: 11.5650},
			{Lat: 48.1350, Lon: 11.5650},
			{Lat: 48.1350, Lon: 11.5550},
		},
	},
	{
		ID:   "munich-marienplatz",
		Name: "Marienplatz Area",
		Polygon: []model.Coordinate{
			{Lat: 48.1380, Lon: 11.5730},
			{Lat: 48.1380, Lon: 11.5790},
			{Lat: 48.1350, Lon: 11.5790},
			{Lat: 48.1350, Lon: 11.5730},
		},
	},
}

var testOperators = []model.Operator{
	{
		ID:     "parkhaus-munich",
		Name:   "Parkhaus Muenchen GmbH",
		ZoneID: "munich-central",
		Rate:   model.Rate{Type: "per-hour", Amount: 2.50, Currency: "EUR"},
		Adapter: model.AdapterMeta{
			ImageRef:       "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0",
			ChecksumSHA256: "sha256:abc123def456",
			Version:        "1.0.0",
		},
	},
	{
		ID:     "city-park-munich",
		Name:   "CityPark Muenchen",
		ZoneID: "munich-marienplatz",
		Rate:   model.Rate{Type: "flat-fee", Amount: 5.00, Currency: "EUR"},
		Adapter: model.AdapterMeta{
			ImageRef:       "us-docker.pkg.dev/sdv-demo/adapters/citypark-munich:v1.0.0",
			ChecksumSHA256: "sha256:789ghi012jkl",
			Version:        "1.0.0",
		},
	},
}

// setupTestServer creates an httptest.Server with all routes wired up
// using default Munich test data and a 500m proximity threshold.
func setupTestServer() *httptest.Server {
	s := store.NewStore(testZones, testOperators)
	mux := http.NewServeMux()
	mux.HandleFunc("GET /operators", NewOperatorHandler(s, testZones, 500.0))
	mux.HandleFunc("GET /operators/{id}/adapter", NewAdapterHandler(s))
	mux.HandleFunc("GET /health", HealthHandler())
	return httptest.NewServer(mux)
}

// TS-05-1: Operator lookup returns matching operators for coordinates inside a zone.
func TestOperatorLookup(t *testing.T) {
	ts := setupTestServer()
	defer ts.Close()

	resp, err := http.Get(ts.URL + "/operators?lat=48.1375&lon=11.5600")
	if err != nil {
		t.Fatalf("request failed: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		t.Errorf("expected status 200, got %d", resp.StatusCode)
	}

	var operators []model.OperatorResponse
	if err := json.NewDecoder(resp.Body).Decode(&operators); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}
	if len(operators) < 1 {
		t.Fatal("expected at least 1 operator in response")
	}
	if operators[0].ID != "parkhaus-munich" {
		t.Errorf("expected operator id parkhaus-munich, got %s", operators[0].ID)
	}
	if operators[0].ZoneID != "munich-central" {
		t.Errorf("expected zone_id munich-central, got %s", operators[0].ZoneID)
	}
}

// TS-05-5: When no operators match, the service returns an empty JSON array
// with HTTP 200.
func TestEmptyArrayNoMatches(t *testing.T) {
	ts := setupTestServer()
	defer ts.Close()

	resp, err := http.Get(ts.URL + "/operators?lat=0.0&lon=0.0")
	if err != nil {
		t.Fatalf("request failed: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		t.Errorf("expected status 200, got %d", resp.StatusCode)
	}

	var operators []model.OperatorResponse
	if err := json.NewDecoder(resp.Body).Decode(&operators); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}
	if len(operators) != 0 {
		t.Errorf("expected empty array, got %d operators", len(operators))
	}
}

// TS-05-6: GET /operators/{id}/adapter returns adapter metadata with image_ref,
// checksum_sha256, and version.
func TestAdapterMetadataRetrieval(t *testing.T) {
	ts := setupTestServer()
	defer ts.Close()

	resp, err := http.Get(ts.URL + "/operators/parkhaus-munich/adapter")
	if err != nil {
		t.Fatalf("request failed: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		t.Errorf("expected status 200, got %d", resp.StatusCode)
	}

	var adapter model.AdapterMeta
	if err := json.NewDecoder(resp.Body).Decode(&adapter); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}
	if adapter.ImageRef == "" {
		t.Error("expected non-empty image_ref")
	}
	if adapter.ChecksumSHA256 == "" {
		t.Error("expected non-empty checksum_sha256")
	}
	if adapter.Version == "" {
		t.Error("expected non-empty version")
	}
}

// TS-05-7: Successful adapter metadata retrieval returns HTTP 200.
func TestAdapterMetadataHTTP200(t *testing.T) {
	ts := setupTestServer()
	defer ts.Close()

	resp, err := http.Get(ts.URL + "/operators/parkhaus-munich/adapter")
	if err != nil {
		t.Fatalf("request failed: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		t.Errorf("expected status 200, got %d", resp.StatusCode)
	}
}

// TS-05-8: GET /health returns HTTP 200 with {"status":"ok"}.
func TestHealthCheck(t *testing.T) {
	ts := setupTestServer()
	defer ts.Close()

	resp, err := http.Get(ts.URL + "/health")
	if err != nil {
		t.Fatalf("request failed: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		t.Errorf("expected status 200, got %d", resp.StatusCode)
	}

	var body map[string]string
	if err := json.NewDecoder(resp.Body).Decode(&body); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}
	if body["status"] != "ok" {
		t.Errorf("expected status ok, got %q", body["status"])
	}
}

// TS-05-12: All responses set Content-Type: application/json.
func TestContentTypeHeader(t *testing.T) {
	ts := setupTestServer()
	defer ts.Close()

	endpoints := []string{
		"/health",
		"/operators?lat=48.137&lon=11.575",
		"/operators/parkhaus-munich/adapter",
	}

	for _, endpoint := range endpoints {
		t.Run(endpoint, func(t *testing.T) {
			resp, err := http.Get(ts.URL + endpoint)
			if err != nil {
				t.Fatalf("request failed: %v", err)
			}
			defer resp.Body.Close()

			ct := resp.Header.Get("Content-Type")
			if ct != "application/json" {
				t.Errorf("expected Content-Type application/json, got %q", ct)
			}
		})
	}
}

// TS-05-13: Operator lookup response includes id, name, zone_id, rate (with type,
// amount, currency). The adapter field is NOT present.
func TestOperatorResponseFields(t *testing.T) {
	ts := setupTestServer()
	defer ts.Close()

	resp, err := http.Get(ts.URL + "/operators?lat=48.1375&lon=11.5600")
	if err != nil {
		t.Fatalf("request failed: %v", err)
	}
	defer resp.Body.Close()

	// Decode as raw JSON to check field presence.
	var rawOps []map[string]interface{}
	if err := json.NewDecoder(resp.Body).Decode(&rawOps); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}
	if len(rawOps) < 1 {
		t.Fatal("expected at least 1 operator in response")
	}

	op := rawOps[0]

	// Required fields.
	if _, ok := op["id"]; !ok {
		t.Error("missing id field")
	}
	if _, ok := op["name"]; !ok {
		t.Error("missing name field")
	}
	if _, ok := op["zone_id"]; !ok {
		t.Error("missing zone_id field")
	}
	rate, ok := op["rate"]
	if !ok {
		t.Fatal("missing rate field")
	}

	// Rate sub-fields.
	rateMap, ok := rate.(map[string]interface{})
	if !ok {
		t.Fatal("rate field is not an object")
	}
	rateType, _ := rateMap["type"].(string)
	if rateType != "per-hour" && rateType != "flat-fee" {
		t.Errorf("expected rate type per-hour or flat-fee, got %q", rateType)
	}
	amount, _ := rateMap["amount"].(float64)
	if amount <= 0 {
		t.Errorf("expected rate amount > 0, got %f", amount)
	}
	currency, _ := rateMap["currency"].(string)
	if currency != "EUR" {
		t.Errorf("expected rate currency EUR, got %q", currency)
	}

	// Adapter field must NOT be present.
	if _, ok := op["adapter"]; ok {
		t.Error("adapter field should NOT be present in operator lookup response")
	}
}

// TS-05-14: Error responses use the format {"error":"<message>"}.
func TestErrorResponseFormat(t *testing.T) {
	ts := setupTestServer()
	defer ts.Close()

	resp, err := http.Get(ts.URL + "/operators")
	if err != nil {
		t.Fatalf("request failed: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusBadRequest {
		t.Errorf("expected status 400, got %d", resp.StatusCode)
	}

	var body map[string]string
	if err := json.NewDecoder(resp.Body).Decode(&body); err != nil {
		t.Fatalf("failed to decode error response: %v", err)
	}
	if body["error"] == "" {
		t.Error("expected non-empty error message")
	}
}

// TS-05-E1: Missing lat or lon query parameters return HTTP 400.
func TestMissingLatLon(t *testing.T) {
	ts := setupTestServer()
	defer ts.Close()

	testCases := []struct {
		name string
		url  string
	}{
		{"no params", "/operators"},
		{"missing lon", "/operators?lat=48.137"},
		{"missing lat", "/operators?lon=11.575"},
	}

	for _, tc := range testCases {
		t.Run(tc.name, func(t *testing.T) {
			resp, err := http.Get(ts.URL + tc.url)
			if err != nil {
				t.Fatalf("request failed: %v", err)
			}
			defer resp.Body.Close()

			if resp.StatusCode != http.StatusBadRequest {
				t.Errorf("expected status 400, got %d", resp.StatusCode)
			}

			var body map[string]string
			if err := json.NewDecoder(resp.Body).Decode(&body); err != nil {
				t.Fatalf("failed to decode error response: %v", err)
			}
			if body["error"] != "lat and lon query parameters are required" {
				t.Errorf("expected error 'lat and lon query parameters are required', got %q",
					body["error"])
			}
		})
	}
}

// TS-05-E2: Coordinates outside valid ranges return HTTP 400.
func TestInvalidCoordinateRange(t *testing.T) {
	ts := setupTestServer()
	defer ts.Close()

	testCases := []struct {
		name string
		lat  string
		lon  string
	}{
		{"lat > 90", "91.0", "11.575"},
		{"lat < -90", "-91.0", "11.575"},
		{"lon > 180", "48.137", "181.0"},
		{"lon < -180", "48.137", "-181.0"},
	}

	for _, tc := range testCases {
		t.Run(tc.name, func(t *testing.T) {
			url := fmt.Sprintf("%s/operators?lat=%s&lon=%s", ts.URL, tc.lat, tc.lon)
			resp, err := http.Get(url)
			if err != nil {
				t.Fatalf("request failed: %v", err)
			}
			defer resp.Body.Close()

			if resp.StatusCode != http.StatusBadRequest {
				t.Errorf("expected status 400, got %d", resp.StatusCode)
			}

			var body map[string]string
			if err := json.NewDecoder(resp.Body).Decode(&body); err != nil {
				t.Fatalf("failed to decode error response: %v", err)
			}
			if body["error"] != "invalid coordinates" {
				t.Errorf("expected error 'invalid coordinates', got %q", body["error"])
			}
		})
	}
}

// TS-05-E3: Non-numeric lat or lon values return HTTP 400.
func TestNonNumericCoordinates(t *testing.T) {
	ts := setupTestServer()
	defer ts.Close()

	testCases := []struct {
		name string
		lat  string
		lon  string
	}{
		{"non-numeric lat", "abc", "11.575"},
		{"non-numeric lon", "48.137", "xyz"},
	}

	for _, tc := range testCases {
		t.Run(tc.name, func(t *testing.T) {
			url := fmt.Sprintf("%s/operators?lat=%s&lon=%s", ts.URL, tc.lat, tc.lon)
			resp, err := http.Get(url)
			if err != nil {
				t.Fatalf("request failed: %v", err)
			}
			defer resp.Body.Close()

			if resp.StatusCode != http.StatusBadRequest {
				t.Errorf("expected status 400, got %d", resp.StatusCode)
			}

			var body map[string]string
			if err := json.NewDecoder(resp.Body).Decode(&body); err != nil {
				t.Fatalf("failed to decode error response: %v", err)
			}
			if body["error"] != "invalid coordinates" {
				t.Errorf("expected error 'invalid coordinates', got %q", body["error"])
			}
		})
	}
}

// TS-05-E4: Unknown operator ID returns HTTP 404.
func TestUnknownOperatorID(t *testing.T) {
	ts := setupTestServer()
	defer ts.Close()

	resp, err := http.Get(ts.URL + "/operators/nonexistent-operator/adapter")
	if err != nil {
		t.Fatalf("request failed: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusNotFound {
		t.Errorf("expected status 404, got %d", resp.StatusCode)
	}

	var body map[string]string
	if err := json.NewDecoder(resp.Body).Decode(&body); err != nil {
		t.Fatalf("failed to decode error response: %v", err)
	}
	if body["error"] != "operator not found" {
		t.Errorf("expected error 'operator not found', got %q", body["error"])
	}
}

// TS-05-P4: Property test — for any latitude outside [-90, 90] or longitude
// outside [-180, 180], the handler returns HTTP 400.
func TestPropertyCoordinateValidation(t *testing.T) {
	s := store.NewStore(testZones, testOperators)
	h := NewOperatorHandler(s, testZones, 500.0)

	rng := rand.New(rand.NewSource(42))

	for i := 0; i < 200; i++ {
		// Generate random float64 in a wide range.
		lat := (rng.Float64() - 0.5) * 400 // range: [-200, 200]
		lon := (rng.Float64() - 0.5) * 800 // range: [-400, 400]

		// Skip NaN and Inf.
		if math.IsNaN(lat) || math.IsNaN(lon) || math.IsInf(lat, 0) || math.IsInf(lon, 0) {
			continue
		}

		outOfRange := lat < -90 || lat > 90 || lon < -180 || lon > 180
		if !outOfRange {
			continue // only test invalid coordinates
		}

		url := fmt.Sprintf("/operators?lat=%f&lon=%f", lat, lon)
		req := httptest.NewRequest("GET", url, nil)
		w := httptest.NewRecorder()
		h.ServeHTTP(w, req)

		if w.Code != http.StatusBadRequest {
			t.Errorf("expected 400 for lat=%f lon=%f, got %d", lat, lon, w.Code)
		}
	}
}
