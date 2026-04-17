package handler_test

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/sdv-demo/parking-fee-service/backend/parking-fee-service/config"
	"github.com/sdv-demo/parking-fee-service/backend/parking-fee-service/handler"
	"github.com/sdv-demo/parking-fee-service/backend/parking-fee-service/store"
)

// newTestServer creates an httptest.Server wired with the default Munich demo
// config. It registers all three handler routes.
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

// getJSON performs a GET request against the server and decodes the JSON body.
func getJSON(t *testing.T, server *httptest.Server, path string, out any) *http.Response {
	t.Helper()
	resp, err := http.Get(server.URL + path)
	if err != nil {
		t.Fatalf("GET %s: %v", path, err)
	}
	defer resp.Body.Close()
	body, err := io.ReadAll(resp.Body)
	if err != nil {
		t.Fatalf("reading body of GET %s: %v", path, err)
	}
	if out != nil {
		if err := json.Unmarshal(body, out); err != nil {
			t.Fatalf("decoding JSON from GET %s: %v\nbody: %s", path, err, body)
		}
	}
	return resp
}

// TS-05-1: GET /operators?lat=&lon= returns operators whose zones contain the
// coordinates.
func TestOperatorLookup(t *testing.T) {
	t.Helper()
	srv := newTestServer(t)
	defer srv.Close()

	var body []map[string]any
	resp := getJSON(t, srv, "/operators?lat=48.1375&lon=11.5600", &body)

	if resp.StatusCode != http.StatusOK {
		t.Fatalf("GET /operators?lat=48.1375&lon=11.5600: want 200, got %d", resp.StatusCode)
	}
	if len(body) < 1 {
		t.Fatalf("want at least 1 operator in response, got %d", len(body))
	}

	// Verify the first matching operator is from munich-central.
	found := false
	for _, op := range body {
		if op["zone_id"] == "munich-central" {
			found = true
			if op["id"] != "parkhaus-munich" {
				t.Errorf("operator in munich-central: want id='parkhaus-munich', got %v", op["id"])
			}
			break
		}
	}
	if !found {
		t.Errorf("want operator with zone_id='munich-central' in response, got %v", body)
	}
}

// TS-05-5: When no operators match, the service returns an empty JSON array
// with HTTP 200.
func TestEmptyArrayNoMatches(t *testing.T) {
	t.Helper()
	srv := newTestServer(t)
	defer srv.Close()

	var body []any
	resp := getJSON(t, srv, "/operators?lat=0.0&lon=0.0", &body)

	if resp.StatusCode != http.StatusOK {
		t.Fatalf("GET /operators?lat=0.0&lon=0.0: want 200, got %d", resp.StatusCode)
	}
	if len(body) != 0 {
		t.Errorf("want empty array for coordinates far from any zone, got %v", body)
	}
}

// TS-05-6: GET /operators/{id}/adapter returns adapter metadata with
// image_ref, checksum_sha256, and version.
func TestAdapterMetadataRetrieval(t *testing.T) {
	t.Helper()
	srv := newTestServer(t)
	defer srv.Close()

	var body map[string]any
	resp := getJSON(t, srv, "/operators/parkhaus-munich/adapter", &body)

	if resp.StatusCode != http.StatusOK {
		t.Fatalf("GET /operators/parkhaus-munich/adapter: want 200, got %d", resp.StatusCode)
	}

	for _, field := range []string{"image_ref", "checksum_sha256", "version"} {
		val, ok := body[field]
		if !ok || val == "" {
			t.Errorf("adapter metadata: field %q missing or empty in %v", field, body)
		}
	}
}

// TS-05-7: Successful adapter metadata retrieval returns HTTP 200.
func TestAdapterMetadataHTTP200(t *testing.T) {
	t.Helper()
	srv := newTestServer(t)
	defer srv.Close()

	resp, err := http.Get(srv.URL + "/operators/parkhaus-munich/adapter")
	if err != nil {
		t.Fatalf("GET /operators/parkhaus-munich/adapter: %v", err)
	}
	resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		t.Fatalf("want HTTP 200, got %d", resp.StatusCode)
	}
}

// TS-05-8: GET /health returns HTTP 200 with {"status":"ok"}.
func TestHealthCheck(t *testing.T) {
	t.Helper()
	srv := newTestServer(t)
	defer srv.Close()

	var body map[string]string
	resp := getJSON(t, srv, "/health", &body)

	if resp.StatusCode != http.StatusOK {
		t.Fatalf("GET /health: want 200, got %d", resp.StatusCode)
	}
	if body["status"] != "ok" {
		t.Errorf("GET /health body: want {\"status\":\"ok\"}, got %v", body)
	}
}

// TS-05-12: All responses set Content-Type: application/json.
func TestContentTypeHeader(t *testing.T) {
	t.Helper()
	srv := newTestServer(t)
	defer srv.Close()

	endpoints := []string{
		"/health",
		"/operators?lat=48.1375&lon=11.5600",
		"/operators/parkhaus-munich/adapter",
		// Also verify error responses carry the header.
		"/operators",
		"/operators?lat=999&lon=999",
		"/operators/nonexistent-operator/adapter",
	}

	for _, path := range endpoints {
		t.Run(path, func(t *testing.T) {
			resp, err := http.Get(srv.URL + path)
			if err != nil {
				t.Fatalf("GET %s: %v", path, err)
			}
			resp.Body.Close()
			ct := resp.Header.Get("Content-Type")
			if ct != "application/json" {
				t.Errorf("GET %s Content-Type: want 'application/json', got %q", path, ct)
			}
		})
	}
}

// TS-05-13: Operator lookup response includes id, name, zone_id, and rate
// fields. The adapter field must NOT be present.
func TestOperatorResponseFields(t *testing.T) {
	t.Helper()
	srv := newTestServer(t)
	defer srv.Close()

	var body []map[string]any
	resp := getJSON(t, srv, "/operators?lat=48.1375&lon=11.5600", &body)

	if resp.StatusCode != http.StatusOK {
		t.Fatalf("GET /operators?lat=48.1375&lon=11.5600: want 200, got %d", resp.StatusCode)
	}
	if len(body) == 0 {
		t.Fatal("want at least 1 operator in response")
	}

	op := body[0]
	for _, field := range []string{"id", "name", "zone_id"} {
		if val, ok := op[field]; !ok || val == "" {
			t.Errorf("operator response field %q missing or empty: %v", field, op)
		}
	}

	// Verify rate sub-object.
	rate, ok := op["rate"].(map[string]any)
	if !ok || rate == nil {
		t.Fatalf("operator response: 'rate' field missing or wrong type: %v", op)
	}
	if rate["type"] != "per-hour" && rate["type"] != "flat-fee" {
		t.Errorf("rate.type: want 'per-hour' or 'flat-fee', got %v", rate["type"])
	}
	if amount, _ := rate["amount"].(float64); amount <= 0 {
		t.Errorf("rate.amount: want > 0, got %v", rate["amount"])
	}
	if rate["currency"] != "EUR" {
		t.Errorf("rate.currency: want 'EUR', got %v", rate["currency"])
	}

	// The 'adapter' field must NOT be in the operator lookup response.
	if _, hasAdapter := op["adapter"]; hasAdapter {
		t.Error("operator lookup response must NOT include 'adapter' field")
	}
}

// TS-05-14: Error responses use {"error":"<message>"} format.
func TestErrorResponseFormat(t *testing.T) {
	t.Helper()
	srv := newTestServer(t)
	defer srv.Close()

	var body map[string]string
	resp := getJSON(t, srv, "/operators", &body)

	if resp.StatusCode != http.StatusBadRequest {
		t.Fatalf("GET /operators (no params): want 400, got %d", resp.StatusCode)
	}
	if body["error"] == "" {
		t.Errorf("error response: want non-empty 'error' field, got %v", body)
	}
}

// TS-05-E1: Missing lat or lon query parameters return HTTP 400 with specific
// error message.
func TestMissingLatLon(t *testing.T) {
	t.Helper()
	srv := newTestServer(t)
	defer srv.Close()

	wantMsg := "lat and lon query parameters are required"
	urls := []string{
		"/operators",
		"/operators?lat=48.137",
		"/operators?lon=11.575",
	}

	for _, path := range urls {
		t.Run(path, func(t *testing.T) {
			var body map[string]string
			resp := getJSON(t, srv, path, &body)

			if resp.StatusCode != http.StatusBadRequest {
				t.Errorf("GET %s: want 400, got %d", path, resp.StatusCode)
			}
			if body["error"] != wantMsg {
				t.Errorf("GET %s error: want %q, got %q", path, wantMsg, body["error"])
			}
		})
	}
}

// TS-05-E2: Coordinates outside valid ranges return HTTP 400.
func TestInvalidCoordinateRange(t *testing.T) {
	t.Helper()
	srv := newTestServer(t)
	defer srv.Close()

	wantMsg := "invalid coordinates"
	cases := []struct{ lat, lon string }{
		{"91.0", "11.575"},
		{"-91.0", "11.575"},
		{"48.137", "181.0"},
		{"48.137", "-181.0"},
	}

	for _, c := range cases {
		path := fmt.Sprintf("/operators?lat=%s&lon=%s", c.lat, c.lon)
		t.Run(path, func(t *testing.T) {
			var body map[string]string
			resp := getJSON(t, srv, path, &body)

			if resp.StatusCode != http.StatusBadRequest {
				t.Errorf("GET %s: want 400, got %d", path, resp.StatusCode)
			}
			if body["error"] != wantMsg {
				t.Errorf("GET %s error: want %q, got %q", path, wantMsg, body["error"])
			}
		})
	}
}

// TS-05-E3: Non-numeric lat or lon values return HTTP 400.
func TestNonNumericCoordinates(t *testing.T) {
	t.Helper()
	srv := newTestServer(t)
	defer srv.Close()

	wantMsg := "invalid coordinates"
	cases := []struct{ lat, lon string }{
		{"abc", "11.575"},
		{"48.137", "xyz"},
	}

	for _, c := range cases {
		path := fmt.Sprintf("/operators?lat=%s&lon=%s", c.lat, c.lon)
		t.Run(path, func(t *testing.T) {
			var body map[string]string
			resp := getJSON(t, srv, path, &body)

			if resp.StatusCode != http.StatusBadRequest {
				t.Errorf("GET %s: want 400, got %d", path, resp.StatusCode)
			}
			if body["error"] != wantMsg {
				t.Errorf("GET %s error: want %q, got %q", path, wantMsg, body["error"])
			}
		})
	}
}

// TS-05-E4: Unknown operator ID returns HTTP 404 with {"error":"operator not found"}.
func TestUnknownOperatorID(t *testing.T) {
	t.Helper()
	srv := newTestServer(t)
	defer srv.Close()

	var body map[string]string
	resp := getJSON(t, srv, "/operators/nonexistent-operator/adapter", &body)

	if resp.StatusCode != http.StatusNotFound {
		t.Fatalf("GET /operators/nonexistent-operator/adapter: want 404, got %d", resp.StatusCode)
	}
	if body["error"] != "operator not found" {
		t.Errorf("error message: want 'operator not found', got %q", body["error"])
	}
}

// TS-05-P4: Property — for any latitude outside [-90, 90] or longitude
// outside [-180, 180], the handler returns HTTP 400.
func TestPropertyCoordinateValidation(t *testing.T) {
	t.Helper()
	srv := newTestServer(t)
	defer srv.Close()

	outOfRange := []struct{ lat, lon float64 }{
		{91.0, 0.0},
		{-91.0, 0.0},
		{0.0, 181.0},
		{0.0, -181.0},
		{90.001, 0.0},
		{-90.001, 0.0},
		{0.0, 180.001},
		{0.0, -180.001},
		{200.0, 200.0},
		{-200.0, -200.0},
	}

	for _, c := range outOfRange {
		path := fmt.Sprintf("/operators?lat=%g&lon=%g", c.lat, c.lon)
		t.Run(path, func(t *testing.T) {
			resp, err := http.Get(srv.URL + path)
			if err != nil {
				t.Fatalf("GET %s: %v", path, err)
			}
			resp.Body.Close()
			if resp.StatusCode != http.StatusBadRequest {
				t.Errorf("GET %s: lat=%g, lon=%g: want 400, got %d",
					path, c.lat, c.lon, resp.StatusCode)
			}
		})
	}
}
