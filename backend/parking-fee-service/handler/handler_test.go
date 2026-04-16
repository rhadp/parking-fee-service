package handler_test

import (
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"parking-fee-service/backend/parking-fee-service/config"
	"parking-fee-service/backend/parking-fee-service/handler"
	"parking-fee-service/backend/parking-fee-service/model"
	"parking-fee-service/backend/parking-fee-service/store"
)

// newTestServer creates an httptest.Server wired with the default Munich
// config so all integration tests work against a real server.
func newTestServer(t *testing.T) *httptest.Server {
	t.Helper()
	cfg := config.DefaultConfig()
	s := store.NewStore(cfg.Zones, cfg.Operators)

	mux := http.NewServeMux()
	mux.Handle("GET /operators", handler.NewOperatorHandler(s, cfg.Zones, cfg.ProximityThreshold))
	mux.Handle("GET /operators/{id}/adapter", handler.NewAdapterHandler(s))
	mux.Handle("GET /health", handler.HealthHandler())
	return httptest.NewServer(mux)
}

// ---- TS-05-1: Operator Lookup Returns Matching Operators ----

func TestOperatorLookup(t *testing.T) {
	srv := newTestServer(t)
	defer srv.Close()

	resp, err := http.Get(srv.URL + "/operators?lat=48.1375&lon=11.5600")
	if err != nil {
		t.Fatalf("GET /operators: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		t.Fatalf("status: want 200, got %d", resp.StatusCode)
	}

	var ops []map[string]any
	if err := json.NewDecoder(resp.Body).Decode(&ops); err != nil {
		t.Fatalf("decode: %v", err)
	}
	if len(ops) < 1 {
		t.Fatal("expected at least one operator")
	}

	// Find the expected operator
	found := false
	for _, op := range ops {
		if op["id"] == "parkhaus-munich" && op["zone_id"] == "munich-central" {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("expected parkhaus-munich in response, got %+v", ops)
	}
}

// ---- TS-05-5: Empty Array for No Matches ----

func TestEmptyArrayNoMatches(t *testing.T) {
	srv := newTestServer(t)
	defer srv.Close()

	resp, err := http.Get(srv.URL + "/operators?lat=0.0&lon=0.0")
	if err != nil {
		t.Fatalf("GET /operators: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		t.Fatalf("status: want 200, got %d", resp.StatusCode)
	}

	var ops []any
	if err := json.NewDecoder(resp.Body).Decode(&ops); err != nil {
		t.Fatalf("decode: %v", err)
	}
	if len(ops) != 0 {
		t.Errorf("expected empty array, got %v", ops)
	}
}

// ---- TS-05-6: Adapter Metadata Retrieval ----

func TestAdapterMetadataRetrieval(t *testing.T) {
	srv := newTestServer(t)
	defer srv.Close()

	resp, err := http.Get(srv.URL + "/operators/parkhaus-munich/adapter")
	if err != nil {
		t.Fatalf("GET /operators/parkhaus-munich/adapter: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		t.Fatalf("status: want 200, got %d", resp.StatusCode)
	}

	var meta model.AdapterMeta
	if err := json.NewDecoder(resp.Body).Decode(&meta); err != nil {
		t.Fatalf("decode: %v", err)
	}
	if meta.ImageRef == "" {
		t.Error("image_ref must not be empty")
	}
	if meta.ChecksumSHA256 == "" {
		t.Error("checksum_sha256 must not be empty")
	}
	if meta.Version == "" {
		t.Error("version must not be empty")
	}
}

// ---- TS-05-7: Adapter Metadata HTTP 200 ----

func TestAdapterMetadataHTTP200(t *testing.T) {
	srv := newTestServer(t)
	defer srv.Close()

	resp, err := http.Get(srv.URL + "/operators/parkhaus-munich/adapter")
	if err != nil {
		t.Fatal(err)
	}
	resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		t.Errorf("status: want 200, got %d", resp.StatusCode)
	}
}

// ---- TS-05-8: Health Check ----

func TestHealthCheck(t *testing.T) {
	srv := newTestServer(t)
	defer srv.Close()

	resp, err := http.Get(srv.URL + "/health")
	if err != nil {
		t.Fatal(err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		t.Fatalf("status: want 200, got %d", resp.StatusCode)
	}

	var body map[string]string
	if err := json.NewDecoder(resp.Body).Decode(&body); err != nil {
		t.Fatalf("decode: %v", err)
	}
	if body["status"] != "ok" {
		t.Errorf("status field: want \"ok\", got %q", body["status"])
	}
}

// ---- TS-05-12: Content-Type Header ----

func TestContentTypeHeader(t *testing.T) {
	srv := newTestServer(t)
	defer srv.Close()

	endpoints := []string{
		"/health",
		"/operators?lat=48.137&lon=11.575",
		"/operators/parkhaus-munich/adapter",
		// Error paths must also set Content-Type (per REQ-5.1)
		"/operators",                             // 400 missing params
		"/operators/nonexistent-operator/adapter", // 404
	}

	for _, ep := range endpoints {
		resp, err := http.Get(srv.URL + ep)
		if err != nil {
			t.Errorf("%s: request error: %v", ep, err)
			continue
		}
		resp.Body.Close()
		ct := resp.Header.Get("Content-Type")
		if !strings.HasPrefix(ct, "application/json") {
			t.Errorf("%s: Content-Type: want application/json, got %q", ep, ct)
		}
	}
}

// ---- TS-05-13: Operator Lookup Response Fields ----

func TestOperatorResponseFields(t *testing.T) {
	srv := newTestServer(t)
	defer srv.Close()

	resp, err := http.Get(srv.URL + "/operators?lat=48.1375&lon=11.5600")
	if err != nil {
		t.Fatal(err)
	}
	defer resp.Body.Close()

	var ops []map[string]any
	if err := json.NewDecoder(resp.Body).Decode(&ops); err != nil {
		t.Fatalf("decode: %v", err)
	}
	if len(ops) == 0 {
		t.Fatal("expected at least one operator")
	}

	op := ops[0]
	for _, field := range []string{"id", "name", "zone_id", "rate"} {
		if _, ok := op[field]; !ok {
			t.Errorf("operator response missing field %q", field)
		}
	}
	// adapter must NOT be present
	if _, ok := op["adapter"]; ok {
		t.Error("operator response must not include adapter field")
	}

	// Validate rate sub-fields
	rate, ok := op["rate"].(map[string]any)
	if !ok {
		t.Fatal("rate field is not an object")
	}
	for _, rf := range []string{"type", "amount", "currency"} {
		if _, ok := rate[rf]; !ok {
			t.Errorf("rate object missing field %q", rf)
		}
	}
	validTypes := map[any]bool{"per-hour": true, "flat-fee": true}
	if !validTypes[rate["type"]] {
		t.Errorf("rate.type must be per-hour or flat-fee, got %v", rate["type"])
	}
	if rate["currency"] != "EUR" {
		t.Errorf("rate.currency: want EUR, got %v", rate["currency"])
	}
	if amt, _ := rate["amount"].(float64); amt <= 0 {
		t.Errorf("rate.amount must be > 0, got %v", rate["amount"])
	}
	if op["id"] == "" {
		t.Error("id must not be empty")
	}
}

// ---- TS-05-14: Error Response Format ----

func TestErrorResponseFormat(t *testing.T) {
	srv := newTestServer(t)
	defer srv.Close()

	resp, err := http.Get(srv.URL + "/operators")
	if err != nil {
		t.Fatal(err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusBadRequest {
		t.Fatalf("status: want 400, got %d", resp.StatusCode)
	}

	var body map[string]string
	if err := json.NewDecoder(resp.Body).Decode(&body); err != nil {
		t.Fatalf("decode: %v", err)
	}
	if body["error"] == "" {
		t.Error("error response must have non-empty 'error' field")
	}
}

// ---- TS-05-E1: Missing lat/lon Parameters ----

func TestMissingLatLon(t *testing.T) {
	srv := newTestServer(t)
	defer srv.Close()

	urls := []string{
		"/operators",
		"/operators?lat=48.137",
		"/operators?lon=11.575",
	}
	for _, u := range urls {
		resp, err := http.Get(srv.URL + u)
		if err != nil {
			t.Errorf("%s: %v", u, err)
			continue
		}
		defer resp.Body.Close()

		if resp.StatusCode != http.StatusBadRequest {
			t.Errorf("%s: status want 400, got %d", u, resp.StatusCode)
		}

		var body map[string]string
		if err := json.NewDecoder(resp.Body).Decode(&body); err != nil {
			t.Errorf("%s: decode error: %v", u, err)
			continue
		}
		want := "lat and lon query parameters are required"
		if body["error"] != want {
			t.Errorf("%s: error msg: want %q, got %q", u, want, body["error"])
		}
	}
}

// ---- TS-05-E2: Invalid Coordinate Range ----

func TestInvalidCoordinateRange(t *testing.T) {
	srv := newTestServer(t)
	defer srv.Close()

	cases := [][2]string{
		{"91.0", "11.575"},
		{"-91.0", "11.575"},
		{"48.137", "181.0"},
		{"48.137", "-181.0"},
	}
	for _, c := range cases {
		url := fmt.Sprintf("/operators?lat=%s&lon=%s", c[0], c[1])
		resp, err := http.Get(srv.URL + url)
		if err != nil {
			t.Errorf("%s: %v", url, err)
			continue
		}
		defer resp.Body.Close()

		if resp.StatusCode != http.StatusBadRequest {
			t.Errorf("%s: status want 400, got %d", url, resp.StatusCode)
		}

		var body map[string]string
		if err := json.NewDecoder(resp.Body).Decode(&body); err != nil {
			t.Errorf("%s: decode error: %v", url, err)
			continue
		}
		if body["error"] != "invalid coordinates" {
			t.Errorf("%s: error msg: want %q, got %q", url, "invalid coordinates", body["error"])
		}
	}
}

// ---- TS-05-E3: Non-Numeric Coordinates ----

func TestNonNumericCoordinates(t *testing.T) {
	srv := newTestServer(t)
	defer srv.Close()

	cases := [][2]string{
		{"abc", "11.575"},
		{"48.137", "xyz"},
	}
	for _, c := range cases {
		url := fmt.Sprintf("/operators?lat=%s&lon=%s", c[0], c[1])
		resp, err := http.Get(srv.URL + url)
		if err != nil {
			t.Errorf("%s: %v", url, err)
			continue
		}
		defer resp.Body.Close()

		if resp.StatusCode != http.StatusBadRequest {
			t.Errorf("%s: status want 400, got %d", url, resp.StatusCode)
		}

		var body map[string]string
		if err := json.NewDecoder(resp.Body).Decode(&body); err != nil {
			t.Errorf("%s: decode error: %v", url, err)
			continue
		}
		if body["error"] != "invalid coordinates" {
			t.Errorf("%s: error msg: want %q, got %q", url, "invalid coordinates", body["error"])
		}
	}
}

// ---- TS-05-E4: Unknown Operator ID ----

func TestUnknownOperatorID(t *testing.T) {
	srv := newTestServer(t)
	defer srv.Close()

	resp, err := http.Get(srv.URL + "/operators/nonexistent-operator/adapter")
	if err != nil {
		t.Fatal(err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusNotFound {
		t.Fatalf("status: want 404, got %d", resp.StatusCode)
	}

	var body map[string]string
	if err := json.NewDecoder(resp.Body).Decode(&body); err != nil {
		t.Fatalf("decode: %v", err)
	}
	if body["error"] != "operator not found" {
		t.Errorf("error msg: want %q, got %q", "operator not found", body["error"])
	}
}

// ---- TS-05-P4: Coordinate Validation (property test) ----

func TestPropertyCoordinateValidation(t *testing.T) {
	srv := newTestServer(t)
	defer srv.Close()

	cases := []struct {
		lat    float64
		lon    float64
		expect400 bool
	}{
		{91.0, 11.0, true},
		{-91.0, 11.0, true},
		{48.0, 181.0, true},
		{48.0, -181.0, true},
		{90.0, 180.0, false},  // boundary values — valid
		{-90.0, -180.0, false},
		{0.0, 0.0, false},
	}

	for _, c := range cases {
		url := fmt.Sprintf("/operators?lat=%f&lon=%f", c.lat, c.lon)
		resp, err := http.Get(srv.URL + url)
		if err != nil {
			t.Errorf("%s: %v", url, err)
			continue
		}
		resp.Body.Close()

		if c.expect400 && resp.StatusCode != http.StatusBadRequest {
			t.Errorf("lat=%f lon=%f: want 400, got %d", c.lat, c.lon, resp.StatusCode)
		}
		if !c.expect400 && resp.StatusCode == http.StatusBadRequest {
			t.Errorf("lat=%f lon=%f: want not-400, got 400", c.lat, c.lon)
		}
	}
}
