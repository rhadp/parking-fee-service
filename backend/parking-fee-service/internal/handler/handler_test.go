package handler

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/internal/model"
	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/internal/store"
)

// setupTestServer creates an HTTP handler backed by a default store and
// standard test configuration.
func setupTestServer(t *testing.T) http.Handler {
	t.Helper()
	s := store.NewDefaultStore()
	tokens := []string{"demo-token-1"}
	fuzziness := float64(100)
	return NewRouter(s, tokens, fuzziness)
}

// setupTestServerWithTokens creates an HTTP handler with custom auth tokens.
func setupTestServerWithTokens(t *testing.T, tokens []string) http.Handler {
	t.Helper()
	s := store.NewDefaultStore()
	fuzziness := float64(100)
	return NewRouter(s, tokens, fuzziness)
}

// setupTestServerWithStore creates an HTTP handler with a custom store.
func setupTestServerWithStore(t *testing.T, s *store.Store) http.Handler {
	t.Helper()
	tokens := []string{"demo-token-1"}
	fuzziness := float64(100)
	return NewRouter(s, tokens, fuzziness)
}

// operatorLookupResponse is the JSON structure returned by GET /operators.
type operatorLookupResponse struct {
	Operators []operatorResult `json:"operators"`
}

// operatorResult represents a single operator in the lookup response.
type operatorResult struct {
	OperatorID string `json:"operator_id"`
	Name       string `json:"name"`
	Zone       struct {
		ZoneID  string         `json:"zone_id"`
		Name    string         `json:"name"`
		Polygon []model.Point  `json:"polygon"`
	} `json:"zone"`
	Rate struct {
		AmountPerHour float64 `json:"amount_per_hour"`
		Currency      string  `json:"currency"`
	} `json:"rate"`
}

// adapterResponse is the JSON structure returned by GET /operators/{id}/adapter.
type adapterResponse struct {
	OperatorID     string `json:"operator_id"`
	ImageRef       string `json:"image_ref"`
	ChecksumSHA256 string `json:"checksum_sha256"`
	Version        string `json:"version"`
}

// --- TS-05-1: Operator lookup returns matching operators ---

func TestOperatorLookup_InsideZone(t *testing.T) {
	srv := setupTestServer(t)

	req := httptest.NewRequest("GET", "/operators?lat=48.1351&lon=11.5750", nil)
	req.Header.Set("Authorization", "Bearer demo-token-1")
	rec := httptest.NewRecorder()
	srv.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Fatalf("expected status 200, got %d: %s", rec.Code, rec.Body.String())
	}

	var body operatorLookupResponse
	if err := json.Unmarshal(rec.Body.Bytes(), &body); err != nil {
		t.Fatalf("failed to parse response JSON: %v", err)
	}

	if len(body.Operators) < 1 {
		t.Fatalf("expected at least 1 operator, got %d", len(body.Operators))
	}

	found := false
	for _, op := range body.Operators {
		if op.OperatorID == "op-munich-01" {
			found = true
			break
		}
	}
	if !found {
		t.Error("expected op-munich-01 in results")
	}
}

// --- TS-05-2: Operator lookup response includes required fields ---

func TestOperatorLookup_ResponseFields(t *testing.T) {
	srv := setupTestServer(t)

	req := httptest.NewRequest("GET", "/operators?lat=48.1351&lon=11.5750", nil)
	req.Header.Set("Authorization", "Bearer demo-token-1")
	rec := httptest.NewRecorder()
	srv.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Fatalf("expected status 200, got %d: %s", rec.Code, rec.Body.String())
	}

	var body operatorLookupResponse
	if err := json.Unmarshal(rec.Body.Bytes(), &body); err != nil {
		t.Fatalf("failed to parse response JSON: %v", err)
	}

	if len(body.Operators) < 1 {
		t.Fatal("expected at least 1 operator to check fields")
	}

	for _, op := range body.Operators {
		if op.OperatorID == "" {
			t.Error("operator_id is empty")
		}
		if op.Name == "" {
			t.Error("name is empty")
		}
		if op.Zone.ZoneID == "" {
			t.Error("zone.zone_id is empty")
		}
		if op.Zone.Name == "" {
			t.Error("zone.name is empty")
		}
		if op.Rate.AmountPerHour <= 0 {
			t.Errorf("rate.amount_per_hour should be > 0, got %f", op.Rate.AmountPerHour)
		}
		if op.Rate.Currency == "" {
			t.Error("rate.currency is empty")
		}
	}
}

// --- TS-05-3: Operator lookup returns multiple matching operators ---

func TestOperatorLookup_MultipleMatches(t *testing.T) {
	// Create a store with two operators whose zones overlap at a specific point
	overlappingOps := &store.Store{}
	_ = overlappingOps // Store will be populated once NewStoreFromOperators is available

	// For now, we create operators with overlapping zones and use the handler
	srv := setupTestServer(t) // default store has 2 operators; we need overlapping zones

	// Use a custom setup: create two operators whose zones both cover the same point
	ops := []model.Operator{
		{
			ID:   "op-overlap-1",
			Name: "Overlap Operator 1",
			Zone: model.Zone{
				ID:      "zone-overlap-1",
				Name:    "Overlap Zone 1",
				Polygon: []model.Point{
					{Lat: 48.14, Lon: 11.56},
					{Lat: 48.14, Lon: 11.59},
					{Lat: 48.13, Lon: 11.59},
					{Lat: 48.13, Lon: 11.56},
				},
			},
			Rate:    model.Rate{AmountPerHour: 2.50, Currency: "EUR"},
			Adapter: model.Adapter{ImageRef: "img1", ChecksumSHA256: "sha256:aaa", Version: "1.0"},
		},
		{
			ID:   "op-overlap-2",
			Name: "Overlap Operator 2",
			Zone: model.Zone{
				ID:      "zone-overlap-2",
				Name:    "Overlap Zone 2",
				Polygon: []model.Point{
					{Lat: 48.15, Lon: 11.55},
					{Lat: 48.15, Lon: 11.60},
					{Lat: 48.12, Lon: 11.60},
					{Lat: 48.12, Lon: 11.55},
				},
			},
			Rate:    model.Rate{AmountPerHour: 3.00, Currency: "EUR"},
			Adapter: model.Adapter{ImageRef: "img2", ChecksumSHA256: "sha256:bbb", Version: "2.0"},
		},
	}

	overlapStore := store.NewStoreFromOperators(ops)
	overlapSrv := NewRouter(overlapStore, []string{"demo-token-1"}, 100)

	// Point inside both zones
	req := httptest.NewRequest("GET", "/operators?lat=48.135&lon=11.575", nil)
	req.Header.Set("Authorization", "Bearer demo-token-1")
	rec := httptest.NewRecorder()
	overlapSrv.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Fatalf("expected status 200, got %d: %s", rec.Code, rec.Body.String())
	}

	var body operatorLookupResponse
	if err := json.Unmarshal(rec.Body.Bytes(), &body); err != nil {
		t.Fatalf("failed to parse response JSON: %v", err)
	}

	if len(body.Operators) < 2 {
		t.Errorf("expected at least 2 matching operators, got %d", len(body.Operators))
	}

	_ = srv // suppress unused
}

// --- TS-05-4: Operator lookup response content type ---

func TestOperatorLookup_ContentType(t *testing.T) {
	srv := setupTestServer(t)

	req := httptest.NewRequest("GET", "/operators?lat=48.1351&lon=11.5750", nil)
	req.Header.Set("Authorization", "Bearer demo-token-1")
	rec := httptest.NewRecorder()
	srv.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Fatalf("expected status 200, got %d", rec.Code)
	}

	ct := rec.Header().Get("Content-Type")
	if !strings.Contains(ct, "application/json") {
		t.Errorf("expected Content-Type application/json, got %q", ct)
	}
}

// --- TS-05-12: Adapter metadata retrieval ---

func TestAdapterMetadata_ValidOperator(t *testing.T) {
	srv := setupTestServer(t)

	req := httptest.NewRequest("GET", "/operators/op-munich-01/adapter", nil)
	req.Header.Set("Authorization", "Bearer demo-token-1")
	rec := httptest.NewRecorder()
	srv.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Fatalf("expected status 200, got %d: %s", rec.Code, rec.Body.String())
	}

	var body adapterResponse
	if err := json.Unmarshal(rec.Body.Bytes(), &body); err != nil {
		t.Fatalf("failed to parse response JSON: %v", err)
	}

	if body.ImageRef == "" {
		t.Error("image_ref is empty")
	}
	if body.ChecksumSHA256 == "" {
		t.Error("checksum_sha256 is empty")
	}
	if body.Version == "" {
		t.Error("version is empty")
	}
}

// --- TS-05-13: Adapter metadata response fields ---

func TestAdapterMetadata_AllFields(t *testing.T) {
	srv := setupTestServer(t)

	req := httptest.NewRequest("GET", "/operators/op-munich-01/adapter", nil)
	req.Header.Set("Authorization", "Bearer demo-token-1")
	rec := httptest.NewRecorder()
	srv.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Fatalf("expected status 200, got %d: %s", rec.Code, rec.Body.String())
	}

	var resp map[string]interface{}
	if err := json.Unmarshal(rec.Body.Bytes(), &resp); err != nil {
		t.Fatalf("failed to parse response JSON: %v", err)
	}

	for _, field := range []string{"image_ref", "checksum_sha256", "version"} {
		if resp[field] == nil {
			t.Errorf("expected field %q in response, got nil", field)
		}
	}
}

// --- TS-05-14: Adapter metadata content type ---

func TestAdapterMetadata_ContentType(t *testing.T) {
	srv := setupTestServer(t)

	req := httptest.NewRequest("GET", "/operators/op-munich-01/adapter", nil)
	req.Header.Set("Authorization", "Bearer demo-token-1")
	rec := httptest.NewRecorder()
	srv.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Fatalf("expected status 200, got %d", rec.Code)
	}

	ct := rec.Header().Get("Content-Type")
	if !strings.Contains(ct, "application/json") {
		t.Errorf("expected Content-Type application/json, got %q", ct)
	}
}

// --- TS-05-15: Health check returns 200 OK ---

func TestHealthCheck(t *testing.T) {
	srv := setupTestServer(t)

	req := httptest.NewRequest("GET", "/health", nil)
	rec := httptest.NewRecorder()
	srv.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Fatalf("expected status 200, got %d", rec.Code)
	}

	body := rec.Body.String()
	if !strings.Contains(body, `"status"`) {
		t.Error("expected response to contain \"status\"")
	}
	if !strings.Contains(body, `"ok"`) {
		t.Error("expected response to contain \"ok\"")
	}
}

// --- TS-05-16: Health check does not require auth ---

func TestHealthCheck_NoAuth(t *testing.T) {
	srv := setupTestServer(t)

	req := httptest.NewRequest("GET", "/health", nil)
	// No Authorization header
	rec := httptest.NewRecorder()
	srv.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Errorf("expected status 200 for health without auth, got %d", rec.Code)
	}
}

// --- TS-05-21: Bearer token required on /operators ---

func TestAuth_OperatorsWithToken(t *testing.T) {
	srv := setupTestServer(t)

	req := httptest.NewRequest("GET", "/operators?lat=48.135&lon=11.575", nil)
	req.Header.Set("Authorization", "Bearer demo-token-1")
	rec := httptest.NewRecorder()
	srv.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Errorf("expected status 200 with valid token, got %d", rec.Code)
	}
}

// --- TS-05-22: Token validated against configured set ---

func TestAuth_TokenValidation(t *testing.T) {
	srv := setupTestServerWithTokens(t, []string{"demo-token-1", "demo-token-2"})

	// Valid token
	req1 := httptest.NewRequest("GET", "/operators?lat=48.135&lon=11.575", nil)
	req1.Header.Set("Authorization", "Bearer demo-token-1")
	rec1 := httptest.NewRecorder()
	srv.ServeHTTP(rec1, req1)
	if rec1.Code != http.StatusOK {
		t.Errorf("expected 200 with valid token, got %d", rec1.Code)
	}

	// Invalid token
	req2 := httptest.NewRequest("GET", "/operators?lat=48.135&lon=11.575", nil)
	req2.Header.Set("Authorization", "Bearer invalid-token")
	rec2 := httptest.NewRecorder()
	srv.ServeHTTP(rec2, req2)
	if rec2.Code != http.StatusUnauthorized {
		t.Errorf("expected 401 with invalid token, got %d", rec2.Code)
	}
}

// --- TS-05-23: Auth tokens configurable via environment ---
// (This is tested in config_test.go — TestConfig_AuthTokensEnvVar)

// --- TS-05-E1: No operators match location ---

func TestEdge_NoMatchingOperators(t *testing.T) {
	srv := setupTestServer(t)

	// Gulf of Guinea — no zones nearby
	req := httptest.NewRequest("GET", "/operators?lat=0.0&lon=0.0", nil)
	req.Header.Set("Authorization", "Bearer demo-token-1")
	rec := httptest.NewRecorder()
	srv.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Fatalf("expected status 200 for empty results, got %d", rec.Code)
	}

	var body operatorLookupResponse
	if err := json.Unmarshal(rec.Body.Bytes(), &body); err != nil {
		t.Fatalf("failed to parse response JSON: %v", err)
	}

	if len(body.Operators) != 0 {
		t.Errorf("expected 0 operators for remote location, got %d", len(body.Operators))
	}
}

// --- TS-05-E2: Missing lat parameter ---

func TestEdge_MissingLatParam(t *testing.T) {
	srv := setupTestServer(t)

	req := httptest.NewRequest("GET", "/operators?lon=11.575", nil)
	req.Header.Set("Authorization", "Bearer demo-token-1")
	rec := httptest.NewRecorder()
	srv.ServeHTTP(rec, req)

	if rec.Code != http.StatusBadRequest {
		t.Errorf("expected status 400, got %d", rec.Code)
	}

	body := rec.Body.String()
	if !strings.Contains(body, "lat") {
		t.Error("expected error message to mention 'lat'")
	}
}

// --- TS-05-E3: Missing lon parameter ---

func TestEdge_MissingLonParam(t *testing.T) {
	srv := setupTestServer(t)

	req := httptest.NewRequest("GET", "/operators?lat=48.135", nil)
	req.Header.Set("Authorization", "Bearer demo-token-1")
	rec := httptest.NewRecorder()
	srv.ServeHTTP(rec, req)

	if rec.Code != http.StatusBadRequest {
		t.Errorf("expected status 400, got %d", rec.Code)
	}

	body := rec.Body.String()
	if !strings.Contains(body, "lon") {
		t.Error("expected error message to mention 'lon'")
	}
}

// --- TS-05-E4: Invalid lat value (non-numeric) ---

func TestEdge_InvalidLatValue(t *testing.T) {
	srv := setupTestServer(t)

	req := httptest.NewRequest("GET", "/operators?lat=abc&lon=11.575", nil)
	req.Header.Set("Authorization", "Bearer demo-token-1")
	rec := httptest.NewRecorder()
	srv.ServeHTTP(rec, req)

	if rec.Code != http.StatusBadRequest {
		t.Errorf("expected status 400, got %d", rec.Code)
	}

	body := rec.Body.String()
	if !strings.Contains(body, "lat") {
		t.Error("expected error message to mention 'lat'")
	}
}

// --- TS-05-E5: Invalid lon value (non-numeric) ---

func TestEdge_InvalidLonValue(t *testing.T) {
	srv := setupTestServer(t)

	req := httptest.NewRequest("GET", "/operators?lat=48.135&lon=xyz", nil)
	req.Header.Set("Authorization", "Bearer demo-token-1")
	rec := httptest.NewRecorder()
	srv.ServeHTTP(rec, req)

	if rec.Code != http.StatusBadRequest {
		t.Errorf("expected status 400, got %d", rec.Code)
	}

	body := rec.Body.String()
	if !strings.Contains(body, "lon") {
		t.Error("expected error message to mention 'lon'")
	}
}

// --- TS-05-E6: Latitude out of range ---

func TestEdge_LatOutOfRange(t *testing.T) {
	srv := setupTestServer(t)

	req := httptest.NewRequest("GET", "/operators?lat=91.0&lon=11.575", nil)
	req.Header.Set("Authorization", "Bearer demo-token-1")
	rec := httptest.NewRecorder()
	srv.ServeHTTP(rec, req)

	if rec.Code != http.StatusBadRequest {
		t.Errorf("expected status 400, got %d", rec.Code)
	}

	body := rec.Body.String()
	if !strings.Contains(body, "lat") {
		t.Error("expected error message to mention 'lat'")
	}
}

// --- TS-05-E7: Longitude out of range ---

func TestEdge_LonOutOfRange(t *testing.T) {
	srv := setupTestServer(t)

	req := httptest.NewRequest("GET", "/operators?lat=48.135&lon=200.0", nil)
	req.Header.Set("Authorization", "Bearer demo-token-1")
	rec := httptest.NewRecorder()
	srv.ServeHTTP(rec, req)

	if rec.Code != http.StatusBadRequest {
		t.Errorf("expected status 400, got %d", rec.Code)
	}

	body := rec.Body.String()
	if !strings.Contains(body, "lon") {
		t.Error("expected error message to mention 'lon'")
	}
}

// --- TS-05-E10: Unknown operator ID returns 404 ---

func TestEdge_UnknownOperator(t *testing.T) {
	srv := setupTestServer(t)

	req := httptest.NewRequest("GET", "/operators/op-nonexistent/adapter", nil)
	req.Header.Set("Authorization", "Bearer demo-token-1")
	rec := httptest.NewRecorder()
	srv.ServeHTTP(rec, req)

	if rec.Code != http.StatusNotFound {
		t.Errorf("expected status 404, got %d", rec.Code)
	}

	body := rec.Body.String()
	if !strings.Contains(body, "op-nonexistent") {
		t.Error("expected error message to contain the unknown operator ID")
	}
}

// --- TS-05-E13: Missing Authorization header ---

func TestEdge_MissingAuthHeader(t *testing.T) {
	srv := setupTestServer(t)

	req := httptest.NewRequest("GET", "/operators?lat=48.135&lon=11.575", nil)
	// No Authorization header
	rec := httptest.NewRecorder()
	srv.ServeHTTP(rec, req)

	if rec.Code != http.StatusUnauthorized {
		t.Errorf("expected status 401, got %d", rec.Code)
	}

	body := rec.Body.String()
	if !strings.Contains(body, "missing authorization header") {
		t.Errorf("expected error 'missing authorization header', got %q", body)
	}
}

// --- TS-05-E14: Invalid bearer token ---

func TestEdge_InvalidToken(t *testing.T) {
	srv := setupTestServer(t)

	req := httptest.NewRequest("GET", "/operators?lat=48.135&lon=11.575", nil)
	req.Header.Set("Authorization", "Bearer wrong-token")
	rec := httptest.NewRecorder()
	srv.ServeHTTP(rec, req)

	if rec.Code != http.StatusUnauthorized {
		t.Errorf("expected status 401, got %d", rec.Code)
	}

	body := rec.Body.String()
	if !strings.Contains(body, "invalid token") {
		t.Errorf("expected error 'invalid token', got %q", body)
	}
}

// --- TS-05-E15: Invalid authorization scheme ---

func TestEdge_InvalidAuthScheme(t *testing.T) {
	srv := setupTestServer(t)

	req := httptest.NewRequest("GET", "/operators?lat=48.135&lon=11.575", nil)
	req.Header.Set("Authorization", "Basic dXNlcjpwYXNz")
	rec := httptest.NewRecorder()
	srv.ServeHTTP(rec, req)

	if rec.Code != http.StatusUnauthorized {
		t.Errorf("expected status 401, got %d", rec.Code)
	}

	body := rec.Body.String()
	if !strings.Contains(body, "invalid authorization scheme") {
		t.Errorf("expected error 'invalid authorization scheme', got %q", body)
	}
}

// --- TS-05-P5: Adapter Metadata Consistency ---

func TestProperty_AdapterMetadataConsistency(t *testing.T) {
	s := store.NewDefaultStore()
	if s == nil {
		t.Fatal("NewDefaultStore() returned nil")
	}

	for _, op := range s.ListOperators() {
		if op.Adapter.ImageRef == "" {
			t.Errorf("operator %s: image_ref is empty", op.ID)
		}
		if op.Adapter.ChecksumSHA256 == "" {
			t.Errorf("operator %s: checksum_sha256 is empty", op.ID)
		}
		if op.Adapter.Version == "" {
			t.Errorf("operator %s: version is empty", op.ID)
		}
	}
}

// --- TS-05-P6: Authentication Enforcement ---

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

		if rec.Code != http.StatusUnauthorized {
			t.Errorf("endpoint %s without auth: expected 401, got %d", ep, rec.Code)
		}
		if strings.Contains(rec.Body.String(), "op-munich-01") && ep != "/operators/op-munich-01/adapter" {
			t.Errorf("endpoint %s without auth: response should not contain operator data", ep)
		}

		// Wrong token
		req2 := httptest.NewRequest("GET", ep, nil)
		req2.Header.Set("Authorization", "Bearer wrong")
		rec2 := httptest.NewRecorder()
		srv.ServeHTTP(rec2, req2)

		if rec2.Code != http.StatusUnauthorized {
			t.Errorf("endpoint %s with wrong token: expected 401, got %d", ep, rec2.Code)
		}
	}
}

// --- TS-05-P7: Health Endpoint Availability ---

func TestProperty_HealthAlwaysAvailable(t *testing.T) {
	// With default store
	srv1 := setupTestServer(t)
	req1 := httptest.NewRequest("GET", "/health", nil)
	rec1 := httptest.NewRecorder()
	srv1.ServeHTTP(rec1, req1)
	if rec1.Code != http.StatusOK {
		t.Errorf("health with default store: expected 200, got %d", rec1.Code)
	}

	// With empty store
	srv2 := setupTestServerWithStore(t, store.NewEmptyStore())
	req2 := httptest.NewRequest("GET", "/health", nil)
	rec2 := httptest.NewRecorder()
	srv2.ServeHTTP(rec2, req2)
	if rec2.Code != http.StatusOK {
		t.Errorf("health with empty store: expected 200, got %d", rec2.Code)
	}
}
