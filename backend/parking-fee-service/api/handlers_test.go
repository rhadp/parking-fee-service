package api

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/geo"
	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/zones"
)

// --- Test Helpers ---

// testStore creates a Store with the three seed zones for testing.
func testStore() *zones.Store {
	return zones.LoadSeedData()
}

// testServer creates an httptest.Server wired up with the API handlers and
// the seed-data store.
func testServer() *httptest.Server {
	store := testStore()
	mux := http.NewServeMux()
	h := NewHandler(store)
	h.RegisterRoutes(mux)
	return httptest.NewServer(mux)
}

// --- Health Check Tests ---

func TestHandleHealthz_Returns200(t *testing.T) {
	srv := testServer()
	defer srv.Close()

	resp, err := http.Get(srv.URL + "/healthz")
	if err != nil {
		t.Fatalf("GET /healthz: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		t.Errorf("GET /healthz: got status %d, want %d", resp.StatusCode, http.StatusOK)
	}

	ct := resp.Header.Get("Content-Type")
	if ct != "application/json" {
		t.Errorf("Content-Type = %q, want %q", ct, "application/json")
	}

	// Body should be an empty JSON object.
	var body map[string]any
	if err := json.NewDecoder(resp.Body).Decode(&body); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}
	if len(body) != 0 {
		t.Errorf("expected empty JSON object, got %v", body)
	}
}

// --- Zone Lookup Tests ---

func TestHandleZoneLookup_ValidCoords_InsideZone(t *testing.T) {
	srv := testServer()
	defer srv.Close()

	// Center of Marienplatz zone.
	resp, err := http.Get(srv.URL + "/api/v1/zones?lat=48.13675&lon=11.5755")
	if err != nil {
		t.Fatalf("GET /api/v1/zones: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		t.Errorf("status = %d, want %d", resp.StatusCode, http.StatusOK)
	}

	var matches []zones.ZoneMatch
	if err := json.NewDecoder(resp.Body).Decode(&matches); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}

	if len(matches) == 0 {
		t.Fatal("expected at least 1 match for coords inside Marienplatz zone")
	}

	found := false
	for _, m := range matches {
		if m.ZoneID == "zone-marienplatz" {
			found = true
			if m.DistanceMeters != 0 {
				t.Errorf("expected distance_meters=0 for inside match, got %f", m.DistanceMeters)
			}
			// Verify all required fields from 05-REQ-1.4.
			if m.Name == "" {
				t.Error("name is empty")
			}
			if m.OperatorName == "" {
				t.Error("operator_name is empty")
			}
			if m.RateType == "" {
				t.Error("rate_type is empty")
			}
			if m.RateAmount <= 0 {
				t.Error("rate_amount should be positive")
			}
			if m.Currency == "" {
				t.Error("currency is empty")
			}
		}
	}
	if !found {
		t.Error("expected zone-marienplatz in results")
	}
}

func TestHandleZoneLookup_NoMatch_ReturnsEmptyArray(t *testing.T) {
	srv := testServer()
	defer srv.Close()

	// Coords far from all zones (Berlin).
	resp, err := http.Get(srv.URL + "/api/v1/zones?lat=52.52&lon=13.405")
	if err != nil {
		t.Fatalf("GET /api/v1/zones: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		t.Errorf("status = %d, want %d for no-match", resp.StatusCode, http.StatusOK)
	}

	var matches []zones.ZoneMatch
	if err := json.NewDecoder(resp.Body).Decode(&matches); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}

	if len(matches) != 0 {
		t.Errorf("expected empty array for no-match, got %d matches", len(matches))
	}
}

func TestHandleZoneLookup_MissingLat_Returns400(t *testing.T) {
	srv := testServer()
	defer srv.Close()

	resp, err := http.Get(srv.URL + "/api/v1/zones?lon=11.575")
	if err != nil {
		t.Fatalf("GET /api/v1/zones: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusBadRequest {
		t.Errorf("status = %d, want %d for missing lat", resp.StatusCode, http.StatusBadRequest)
	}

	var errResp ErrorResponse
	if err := json.NewDecoder(resp.Body).Decode(&errResp); err != nil {
		t.Fatalf("failed to decode error response: %v", err)
	}

	if errResp.Code != "BAD_REQUEST" {
		t.Errorf("error code = %q, want %q", errResp.Code, "BAD_REQUEST")
	}
	if errResp.Error == "" {
		t.Error("expected non-empty error message")
	}
}

func TestHandleZoneLookup_MissingLon_Returns400(t *testing.T) {
	srv := testServer()
	defer srv.Close()

	resp, err := http.Get(srv.URL + "/api/v1/zones?lat=48.137")
	if err != nil {
		t.Fatalf("GET /api/v1/zones: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusBadRequest {
		t.Errorf("status = %d, want %d for missing lon", resp.StatusCode, http.StatusBadRequest)
	}

	var errResp ErrorResponse
	if err := json.NewDecoder(resp.Body).Decode(&errResp); err != nil {
		t.Fatalf("failed to decode error response: %v", err)
	}

	if errResp.Code != "BAD_REQUEST" {
		t.Errorf("error code = %q, want %q", errResp.Code, "BAD_REQUEST")
	}
}

func TestHandleZoneLookup_InvalidLat_Returns400(t *testing.T) {
	srv := testServer()
	defer srv.Close()

	resp, err := http.Get(srv.URL + "/api/v1/zones?lat=not-a-number&lon=11.575")
	if err != nil {
		t.Fatalf("GET /api/v1/zones: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusBadRequest {
		t.Errorf("status = %d, want %d for invalid lat", resp.StatusCode, http.StatusBadRequest)
	}
}

func TestHandleZoneLookup_InvalidLon_Returns400(t *testing.T) {
	srv := testServer()
	defer srv.Close()

	resp, err := http.Get(srv.URL + "/api/v1/zones?lat=48.137&lon=abc")
	if err != nil {
		t.Fatalf("GET /api/v1/zones: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusBadRequest {
		t.Errorf("status = %d, want %d for invalid lon", resp.StatusCode, http.StatusBadRequest)
	}
}

func TestHandleZoneLookup_MissingBothParams_Returns400(t *testing.T) {
	srv := testServer()
	defer srv.Close()

	resp, err := http.Get(srv.URL + "/api/v1/zones")
	if err != nil {
		t.Fatalf("GET /api/v1/zones: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusBadRequest {
		t.Errorf("status = %d, want %d for missing both params", resp.StatusCode, http.StatusBadRequest)
	}
}

func TestHandleZoneLookup_FuzzyMatch(t *testing.T) {
	srv := testServer()
	defer srv.Close()

	// Point ~100m north of Marienplatz zone (north edge at lat 48.1380).
	resp, err := http.Get(srv.URL + "/api/v1/zones?lat=48.1389&lon=11.5755")
	if err != nil {
		t.Fatalf("GET /api/v1/zones: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		t.Errorf("status = %d, want %d", resp.StatusCode, http.StatusOK)
	}

	var matches []zones.ZoneMatch
	if err := json.NewDecoder(resp.Body).Decode(&matches); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}

	found := false
	for _, m := range matches {
		if m.ZoneID == "zone-marienplatz" {
			found = true
			if m.DistanceMeters == 0 {
				t.Error("expected non-zero distance_meters for fuzzy match")
			}
			if m.DistanceMeters > 200 {
				t.Errorf("expected distance_meters <= 200, got %f", m.DistanceMeters)
			}
		}
	}
	if !found {
		t.Error("expected zone-marienplatz in fuzzy match results")
	}
}

func TestHandleZoneLookup_ResultsSortedByDistance(t *testing.T) {
	srv := testServer()
	defer srv.Close()

	// Point between Sendlinger Tor and Marienplatz zones — should fuzzy-match both.
	resp, err := http.Get(srv.URL + "/api/v1/zones?lat=48.1350&lon=11.5715")
	if err != nil {
		t.Fatalf("GET /api/v1/zones: %v", err)
	}
	defer resp.Body.Close()

	var matches []zones.ZoneMatch
	if err := json.NewDecoder(resp.Body).Decode(&matches); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}

	// Verify sort order (Property 5: Sort Order Invariant).
	for i := 1; i < len(matches); i++ {
		if matches[i].DistanceMeters < matches[i-1].DistanceMeters {
			t.Errorf("results not sorted by distance: [%d]=%f > [%d]=%f",
				i-1, matches[i-1].DistanceMeters, i, matches[i].DistanceMeters)
		}
	}
}

// --- Zone Details Tests ---

func TestHandleZoneDetails_KnownZone(t *testing.T) {
	srv := testServer()
	defer srv.Close()

	resp, err := http.Get(srv.URL + "/api/v1/zones/zone-marienplatz")
	if err != nil {
		t.Fatalf("GET /api/v1/zones/zone-marienplatz: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		t.Errorf("status = %d, want %d", resp.StatusCode, http.StatusOK)
	}

	ct := resp.Header.Get("Content-Type")
	if ct != "application/json" {
		t.Errorf("Content-Type = %q, want %q", ct, "application/json")
	}

	var detail ZoneDetailResponse
	if err := json.NewDecoder(resp.Body).Decode(&detail); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}

	// Verify 05-REQ-2.2 required fields.
	if detail.ZoneID != "zone-marienplatz" {
		t.Errorf("zone_id = %q, want %q", detail.ZoneID, "zone-marienplatz")
	}
	if detail.Name != "Marienplatz Central" {
		t.Errorf("name = %q, want %q", detail.Name, "Marienplatz Central")
	}
	if detail.OperatorName == "" {
		t.Error("operator_name is empty")
	}
	if detail.RateType == "" {
		t.Error("rate_type is empty")
	}
	if detail.RateAmount <= 0 {
		t.Error("rate_amount should be positive")
	}
	if detail.Currency == "" {
		t.Error("currency is empty")
	}

	// Verify polygon is present and has coordinates.
	var polygon []geo.LatLon
	if err := json.Unmarshal(detail.Polygon, &polygon); err != nil {
		t.Fatalf("failed to unmarshal polygon: %v", err)
	}
	if len(polygon) < 4 {
		t.Errorf("polygon has %d points, want >= 4", len(polygon))
	}
}

func TestHandleZoneDetails_UnknownZone_Returns404(t *testing.T) {
	srv := testServer()
	defer srv.Close()

	resp, err := http.Get(srv.URL + "/api/v1/zones/zone-nonexistent")
	if err != nil {
		t.Fatalf("GET /api/v1/zones/zone-nonexistent: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusNotFound {
		t.Errorf("status = %d, want %d", resp.StatusCode, http.StatusNotFound)
	}

	var errResp ErrorResponse
	if err := json.NewDecoder(resp.Body).Decode(&errResp); err != nil {
		t.Fatalf("failed to decode error response: %v", err)
	}

	if errResp.Code != "NOT_FOUND" {
		t.Errorf("error code = %q, want %q", errResp.Code, "NOT_FOUND")
	}
	if errResp.Error == "" {
		t.Error("expected non-empty error message")
	}
}

func TestHandleZoneDetails_AllSeedZones(t *testing.T) {
	srv := testServer()
	defer srv.Close()

	zoneIDs := []string{"zone-marienplatz", "zone-olympiapark", "zone-sendlinger-tor"}
	for _, id := range zoneIDs {
		t.Run(id, func(t *testing.T) {
			resp, err := http.Get(srv.URL + "/api/v1/zones/" + id)
			if err != nil {
				t.Fatalf("GET /api/v1/zones/%s: %v", id, err)
			}
			defer resp.Body.Close()

			if resp.StatusCode != http.StatusOK {
				t.Errorf("status = %d, want %d", resp.StatusCode, http.StatusOK)
			}

			var detail ZoneDetailResponse
			if err := json.NewDecoder(resp.Body).Decode(&detail); err != nil {
				t.Fatalf("failed to decode response: %v", err)
			}

			if detail.ZoneID != id {
				t.Errorf("zone_id = %q, want %q", detail.ZoneID, id)
			}
		})
	}
}

// --- Adapter Metadata Tests ---

func TestHandleAdapterMetadata_KnownZone(t *testing.T) {
	srv := testServer()
	defer srv.Close()

	resp, err := http.Get(srv.URL + "/api/v1/zones/zone-marienplatz/adapter")
	if err != nil {
		t.Fatalf("GET /api/v1/zones/zone-marienplatz/adapter: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		t.Errorf("status = %d, want %d", resp.StatusCode, http.StatusOK)
	}

	ct := resp.Header.Get("Content-Type")
	if ct != "application/json" {
		t.Errorf("Content-Type = %q, want %q", ct, "application/json")
	}

	var metadata zones.AdapterMetadata
	if err := json.NewDecoder(resp.Body).Decode(&metadata); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}

	// Verify 05-REQ-3.2 required fields.
	if metadata.ZoneID != "zone-marienplatz" {
		t.Errorf("zone_id = %q, want %q", metadata.ZoneID, "zone-marienplatz")
	}
	if metadata.ImageRef == "" {
		t.Error("image_ref is empty")
	}
	if metadata.Checksum == "" {
		t.Error("checksum is empty")
	}
}

func TestHandleAdapterMetadata_UnknownZone_Returns404(t *testing.T) {
	srv := testServer()
	defer srv.Close()

	resp, err := http.Get(srv.URL + "/api/v1/zones/zone-nonexistent/adapter")
	if err != nil {
		t.Fatalf("GET /api/v1/zones/zone-nonexistent/adapter: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusNotFound {
		t.Errorf("status = %d, want %d", resp.StatusCode, http.StatusNotFound)
	}

	var errResp ErrorResponse
	if err := json.NewDecoder(resp.Body).Decode(&errResp); err != nil {
		t.Fatalf("failed to decode error response: %v", err)
	}

	if errResp.Code != "NOT_FOUND" {
		t.Errorf("error code = %q, want %q", errResp.Code, "NOT_FOUND")
	}
}

// --- Property 4: Adapter Metadata Consistency ---

func TestProperty_AdapterMetadataConsistency(t *testing.T) {
	srv := testServer()
	defer srv.Close()

	store := testStore()

	zoneIDs := []string{"zone-marienplatz", "zone-olympiapark", "zone-sendlinger-tor"}
	for _, id := range zoneIDs {
		t.Run(id, func(t *testing.T) {
			// Get zone from store for ground truth.
			zone, ok := store.GetByID(id)
			if !ok {
				t.Fatalf("zone %q not found in store", id)
			}

			// Get adapter metadata via API.
			resp, err := http.Get(srv.URL + "/api/v1/zones/" + id + "/adapter")
			if err != nil {
				t.Fatalf("GET /api/v1/zones/%s/adapter: %v", id, err)
			}
			defer resp.Body.Close()

			var metadata zones.AdapterMetadata
			if err := json.NewDecoder(resp.Body).Decode(&metadata); err != nil {
				t.Fatalf("failed to decode response: %v", err)
			}

			// Property 4: image_ref and checksum must match store values exactly.
			if metadata.ImageRef != zone.AdapterImageRef {
				t.Errorf("image_ref = %q, want %q (from store)",
					metadata.ImageRef, zone.AdapterImageRef)
			}
			if metadata.Checksum != zone.AdapterChecksum {
				t.Errorf("checksum = %q, want %q (from store)",
					metadata.Checksum, zone.AdapterChecksum)
			}
			if metadata.ZoneID != zone.ZoneID {
				t.Errorf("zone_id = %q, want %q (from store)",
					metadata.ZoneID, zone.ZoneID)
			}
		})
	}
}

// --- Response Format Tests ---

func TestZoneLookup_ResponseIsJSONArray(t *testing.T) {
	srv := testServer()
	defer srv.Close()

	resp, err := http.Get(srv.URL + "/api/v1/zones?lat=48.137&lon=11.575")
	if err != nil {
		t.Fatalf("GET: %v", err)
	}
	defer resp.Body.Close()

	// Decode as raw JSON to verify it's an array.
	var raw json.RawMessage
	if err := json.NewDecoder(resp.Body).Decode(&raw); err != nil {
		t.Fatalf("failed to decode JSON: %v", err)
	}

	if len(raw) == 0 || raw[0] != '[' {
		t.Errorf("expected JSON array, got: %s", string(raw))
	}
}

func TestZoneLookup_EmptyResult_ReturnsEmptyArray(t *testing.T) {
	srv := testServer()
	defer srv.Close()

	// Coords far from all zones.
	resp, err := http.Get(srv.URL + "/api/v1/zones?lat=0.0&lon=0.0")
	if err != nil {
		t.Fatalf("GET: %v", err)
	}
	defer resp.Body.Close()

	var raw json.RawMessage
	if err := json.NewDecoder(resp.Body).Decode(&raw); err != nil {
		t.Fatalf("failed to decode JSON: %v", err)
	}

	// Must be `[]`, not `null`.
	trimmed := string(raw)
	if trimmed != "[]" {
		t.Errorf("expected empty JSON array '[]', got: %s", trimmed)
	}
}

// --- Content-Type Tests ---

func TestAllEndpoints_ReturnJSON(t *testing.T) {
	srv := testServer()
	defer srv.Close()

	endpoints := []string{
		"/healthz",
		"/api/v1/zones?lat=48.137&lon=11.575",
		"/api/v1/zones/zone-marienplatz",
		"/api/v1/zones/zone-marienplatz/adapter",
		"/api/v1/zones/zone-nonexistent",           // 404
		"/api/v1/zones/zone-nonexistent/adapter",    // 404
		"/api/v1/zones",                             // 400 (missing params)
	}

	for _, ep := range endpoints {
		t.Run(ep, func(t *testing.T) {
			resp, err := http.Get(srv.URL + ep)
			if err != nil {
				t.Fatalf("GET %s: %v", ep, err)
			}
			defer resp.Body.Close()

			ct := resp.Header.Get("Content-Type")
			if ct != "application/json" {
				t.Errorf("Content-Type = %q, want %q", ct, "application/json")
			}
		})
	}
}

// --- Logging Middleware Tests ---

func TestLoggingMiddleware_PassesThrough(t *testing.T) {
	// Verify that the logging middleware passes the request through.
	inner := http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusOK)
		w.Write([]byte("ok"))
	})

	handler := LoggingMiddleware(inner)
	srv := httptest.NewServer(handler)
	defer srv.Close()

	resp, err := http.Get(srv.URL + "/test")
	if err != nil {
		t.Fatalf("GET /test: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		t.Errorf("status = %d, want %d", resp.StatusCode, http.StatusOK)
	}
}
