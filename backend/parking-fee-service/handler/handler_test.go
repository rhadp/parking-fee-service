package handler

import (
	"encoding/json"
	"fmt"
	"math/rand"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"parking-fee-service/backend/parking-fee-service/model"
	"parking-fee-service/backend/parking-fee-service/store"
)

// Default test data matching Munich demo config.
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

func setupTestServer() *httptest.Server {
	s := store.NewStore(testZones, testOperators)
	mux := http.NewServeMux()
	mux.HandleFunc("GET /operators", NewOperatorHandler(s, testZones, 500.0))
	mux.HandleFunc("GET /operators/{id}/adapter", NewAdapterHandler(s))
	mux.HandleFunc("GET /health", HealthHandler())
	return httptest.NewServer(mux)
}

// TS-05-1: Operator lookup returns matching operators.
func TestOperatorLookup(t *testing.T) {
	srv := setupTestServer()
	defer srv.Close()

	resp, err := http.Get(srv.URL + "/operators?lat=48.1375&lon=11.5600")
	if err != nil {
		t.Fatalf("request failed: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		t.Errorf("expected status 200, got %d", resp.StatusCode)
	}

	var body []map[string]interface{}
	if err := json.NewDecoder(resp.Body).Decode(&body); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}
	if len(body) < 1 {
		t.Fatal("expected at least 1 operator in response")
	}

	found := false
	for _, op := range body {
		if op["id"] == "parkhaus-munich" && op["zone_id"] == "munich-central" {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("expected operator parkhaus-munich with zone munich-central, got %v", body)
	}
}

// TS-05-5: No matches returns empty array with HTTP 200.
func TestEmptyArrayNoMatches(t *testing.T) {
	srv := setupTestServer()
	defer srv.Close()

	resp, err := http.Get(srv.URL + "/operators?lat=0.0&lon=0.0")
	if err != nil {
		t.Fatalf("request failed: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		t.Errorf("expected status 200, got %d", resp.StatusCode)
	}

	var body []interface{}
	if err := json.NewDecoder(resp.Body).Decode(&body); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}
	if len(body) != 0 {
		t.Errorf("expected empty array, got %v", body)
	}
}

// TS-05-6: Adapter metadata retrieval.
func TestAdapterMetadataRetrieval(t *testing.T) {
	srv := setupTestServer()
	defer srv.Close()

	resp, err := http.Get(srv.URL + "/operators/parkhaus-munich/adapter")
	if err != nil {
		t.Fatalf("request failed: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		t.Errorf("expected status 200, got %d", resp.StatusCode)
	}

	var body map[string]interface{}
	if err := json.NewDecoder(resp.Body).Decode(&body); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}

	if body["image_ref"] == nil || body["image_ref"] == "" {
		t.Error("expected non-empty image_ref")
	}
	if body["checksum_sha256"] == nil || body["checksum_sha256"] == "" {
		t.Error("expected non-empty checksum_sha256")
	}
	if body["version"] == nil || body["version"] == "" {
		t.Error("expected non-empty version")
	}
}

// TS-05-7: Adapter metadata returns HTTP 200.
func TestAdapterMetadataHTTP200(t *testing.T) {
	srv := setupTestServer()
	defer srv.Close()

	resp, err := http.Get(srv.URL + "/operators/parkhaus-munich/adapter")
	if err != nil {
		t.Fatalf("request failed: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		t.Errorf("expected status 200, got %d", resp.StatusCode)
	}
}

// TS-05-8: Health check returns {"status":"ok"}.
func TestHealthCheck(t *testing.T) {
	srv := setupTestServer()
	defer srv.Close()

	resp, err := http.Get(srv.URL + "/health")
	if err != nil {
		t.Fatalf("request failed: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		t.Errorf("expected status 200, got %d", resp.StatusCode)
	}

	var body map[string]interface{}
	if err := json.NewDecoder(resp.Body).Decode(&body); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}
	if body["status"] != "ok" {
		t.Errorf("expected status 'ok', got %v", body["status"])
	}
}

// TS-05-12: Content-Type header is application/json on all responses.
func TestContentTypeHeader(t *testing.T) {
	srv := setupTestServer()
	defer srv.Close()

	endpoints := []string{
		"/health",
		"/operators?lat=48.137&lon=11.575",
		"/operators/parkhaus-munich/adapter",
	}

	for _, ep := range endpoints {
		resp, err := http.Get(srv.URL + ep)
		if err != nil {
			t.Fatalf("request to %s failed: %v", ep, err)
		}
		resp.Body.Close()

		ct := resp.Header.Get("Content-Type")
		if !strings.HasPrefix(ct, "application/json") {
			t.Errorf("endpoint %s: expected Content-Type application/json, got %q", ep, ct)
		}
	}
}

// TS-05-13: Operator lookup response fields include id, name, zone_id, rate; exclude adapter.
func TestOperatorResponseFields(t *testing.T) {
	srv := setupTestServer()
	defer srv.Close()

	resp, err := http.Get(srv.URL + "/operators?lat=48.1375&lon=11.5600")
	if err != nil {
		t.Fatalf("request failed: %v", err)
	}
	defer resp.Body.Close()

	var body []map[string]interface{}
	if err := json.NewDecoder(resp.Body).Decode(&body); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}

	if len(body) < 1 {
		t.Fatal("expected at least 1 operator")
	}

	op := body[0]

	// Required fields.
	if op["id"] == nil || op["id"] == "" {
		t.Error("missing or empty 'id' field")
	}
	if op["name"] == nil || op["name"] == "" {
		t.Error("missing or empty 'name' field")
	}
	if op["zone_id"] == nil || op["zone_id"] == "" {
		t.Error("missing or empty 'zone_id' field")
	}

	// Rate subfields.
	rate, ok := op["rate"].(map[string]interface{})
	if !ok {
		t.Fatal("'rate' field is missing or not an object")
	}
	rateType, _ := rate["type"].(string)
	if rateType != "per-hour" && rateType != "flat-fee" {
		t.Errorf("rate.type should be 'per-hour' or 'flat-fee', got %q", rateType)
	}
	if amount, ok := rate["amount"].(float64); !ok || amount <= 0 {
		t.Errorf("rate.amount should be a positive number, got %v", rate["amount"])
	}
	if rate["currency"] != "EUR" {
		t.Errorf("rate.currency should be 'EUR', got %v", rate["currency"])
	}

	// Adapter field MUST NOT be present.
	if _, exists := op["adapter"]; exists {
		t.Error("'adapter' field should NOT be present in operator lookup response")
	}
}

// TS-05-14: Error response format uses {"error":"<message>"}.
func TestErrorResponseFormat(t *testing.T) {
	srv := setupTestServer()
	defer srv.Close()

	resp, err := http.Get(srv.URL + "/operators")
	if err != nil {
		t.Fatalf("request failed: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusBadRequest {
		t.Errorf("expected status 400, got %d", resp.StatusCode)
	}

	var body map[string]interface{}
	if err := json.NewDecoder(resp.Body).Decode(&body); err != nil {
		t.Fatalf("failed to decode error response: %v", err)
	}
	if body["error"] == nil || body["error"] == "" {
		t.Error("expected non-empty 'error' field in response")
	}
}

// TS-05-E1: Missing lat/lon parameters.
func TestMissingLatLon(t *testing.T) {
	srv := setupTestServer()
	defer srv.Close()

	urls := []string{
		"/operators",
		"/operators?lat=48.137",
		"/operators?lon=11.575",
	}

	for _, path := range urls {
		resp, err := http.Get(srv.URL + path)
		if err != nil {
			t.Fatalf("request to %s failed: %v", path, err)
		}
		defer resp.Body.Close()

		if resp.StatusCode != http.StatusBadRequest {
			t.Errorf("%s: expected status 400, got %d", path, resp.StatusCode)
		}

		var body map[string]interface{}
		if err := json.NewDecoder(resp.Body).Decode(&body); err != nil {
			t.Errorf("%s: failed to decode response: %v", path, err)
			continue
		}
		if body["error"] != "lat and lon query parameters are required" {
			t.Errorf("%s: expected error 'lat and lon query parameters are required', got %v", path, body["error"])
		}
	}
}

// TS-05-E2: Invalid coordinate range.
func TestInvalidCoordinateRange(t *testing.T) {
	srv := setupTestServer()
	defer srv.Close()

	params := []struct{ lat, lon string }{
		{"91.0", "11.575"},
		{"-91.0", "11.575"},
		{"48.137", "181.0"},
		{"48.137", "-181.0"},
	}

	for _, p := range params {
		url := fmt.Sprintf("%s/operators?lat=%s&lon=%s", srv.URL, p.lat, p.lon)
		resp, err := http.Get(url)
		if err != nil {
			t.Fatalf("request failed: %v", err)
		}
		defer resp.Body.Close()

		if resp.StatusCode != http.StatusBadRequest {
			t.Errorf("lat=%s,lon=%s: expected status 400, got %d", p.lat, p.lon, resp.StatusCode)
		}

		var body map[string]interface{}
		if err := json.NewDecoder(resp.Body).Decode(&body); err != nil {
			t.Errorf("lat=%s,lon=%s: failed to decode response: %v", p.lat, p.lon, err)
			continue
		}
		if body["error"] != "invalid coordinates" {
			t.Errorf("lat=%s,lon=%s: expected error 'invalid coordinates', got %v", p.lat, p.lon, body["error"])
		}
	}
}

// TS-05-E3: Non-numeric coordinates.
func TestNonNumericCoordinates(t *testing.T) {
	srv := setupTestServer()
	defer srv.Close()

	params := []struct{ lat, lon string }{
		{"abc", "11.575"},
		{"48.137", "xyz"},
	}

	for _, p := range params {
		url := fmt.Sprintf("%s/operators?lat=%s&lon=%s", srv.URL, p.lat, p.lon)
		resp, err := http.Get(url)
		if err != nil {
			t.Fatalf("request failed: %v", err)
		}
		defer resp.Body.Close()

		if resp.StatusCode != http.StatusBadRequest {
			t.Errorf("lat=%s,lon=%s: expected status 400, got %d", p.lat, p.lon, resp.StatusCode)
		}

		var body map[string]interface{}
		if err := json.NewDecoder(resp.Body).Decode(&body); err != nil {
			t.Errorf("lat=%s,lon=%s: failed to decode response: %v", p.lat, p.lon, err)
			continue
		}
		if body["error"] != "invalid coordinates" {
			t.Errorf("lat=%s,lon=%s: expected error 'invalid coordinates', got %v", p.lat, p.lon, body["error"])
		}
	}
}

// TS-05-E4: Unknown operator ID returns 404.
func TestUnknownOperatorID(t *testing.T) {
	srv := setupTestServer()
	defer srv.Close()

	resp, err := http.Get(srv.URL + "/operators/nonexistent-operator/adapter")
	if err != nil {
		t.Fatalf("request failed: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusNotFound {
		t.Errorf("expected status 404, got %d", resp.StatusCode)
	}

	var body map[string]interface{}
	if err := json.NewDecoder(resp.Body).Decode(&body); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}
	if body["error"] != "operator not found" {
		t.Errorf("expected error 'operator not found', got %v", body["error"])
	}
}

// TS-05-P4: Property — invalid coordinates always return HTTP 400.
func TestPropertyCoordinateValidation(t *testing.T) {
	srv := setupTestServer()
	defer srv.Close()

	rng := rand.New(rand.NewSource(42))

	for i := 0; i < 50; i++ {
		lat := rng.Float64()*400 - 200 // range [-200, 200]
		lon := rng.Float64()*800 - 400 // range [-400, 400]

		outOfRange := lat < -90 || lat > 90 || lon < -180 || lon > 180

		url := fmt.Sprintf("%s/operators?lat=%f&lon=%f", srv.URL, lat, lon)
		resp, err := http.Get(url)
		if err != nil {
			t.Fatalf("request failed: %v", err)
		}
		resp.Body.Close()

		if outOfRange && resp.StatusCode != http.StatusBadRequest {
			t.Errorf("iteration %d: lat=%.2f, lon=%.2f out of range but got status %d (expected 400)",
				i, lat, lon, resp.StatusCode)
		}
	}
}
