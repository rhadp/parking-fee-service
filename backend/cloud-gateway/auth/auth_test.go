package auth_test

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/sdv-demo/parking-fee-service/backend/cloud-gateway/auth"
	"github.com/sdv-demo/parking-fee-service/backend/cloud-gateway/config"
)

// testConfig returns a Config with two demo tokens for testing.
func testConfig() *config.Config {
	return &config.Config{
		Port:                  8081,
		NatsURL:               "nats://localhost:4222",
		CommandTimeoutSeconds: 30,
		Tokens: []config.TokenMapping{
			{Token: "demo-token-001", VIN: "VIN12345"},
			{Token: "demo-token-002", VIN: "VIN67890"},
		},
	}
}

// okHandler returns HTTP 200 to confirm the request reached the downstream handler.
var okHandler = http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusOK)
})

// assertStatus fails the test if the recorded status code does not equal want.
func assertStatus(t *testing.T, rec *httptest.ResponseRecorder, want int) {
	t.Helper()
	if rec.Code != want {
		t.Errorf("status: want %d, got %d", want, rec.Code)
	}
}

// assertErrorBody fails the test if the response body is not {"error":"<wantError>"}.
func assertErrorBody(t *testing.T, rec *httptest.ResponseRecorder, wantError string) {
	t.Helper()
	var body map[string]string
	if err := json.Unmarshal(rec.Body.Bytes(), &body); err != nil {
		t.Fatalf("decode error response: %v (body=%s)", err, rec.Body)
	}
	if body["error"] != wantError {
		t.Errorf("error body: want %q, got %q", wantError, body["error"])
	}
}

// TestBearerTokenValidation verifies that a valid bearer token mapped to the correct
// VIN passes through the middleware without a 401 or 403.
// TS-06-8
func TestBearerTokenValidation(t *testing.T) {
	cfg := testConfig()
	handler := auth.Middleware(cfg)(okHandler)

	req := httptest.NewRequest(http.MethodGet, "/vehicles/VIN12345/commands/x", nil)
	req.Header.Set("Authorization", "Bearer demo-token-001")
	rec := httptest.NewRecorder()
	handler.ServeHTTP(rec, req)

	if rec.Code == http.StatusUnauthorized {
		t.Errorf("valid token: want not-401, got 401")
	}
	if rec.Code == http.StatusForbidden {
		t.Errorf("valid token for correct VIN: want not-403, got 403")
	}
}

// TestVINAuthorizationCheck verifies that a valid token used against a different VIN
// returns HTTP 403 with body {"error":"forbidden"}.
// TS-06-9
func TestVINAuthorizationCheck(t *testing.T) {
	cfg := testConfig()
	handler := auth.Middleware(cfg)(okHandler)

	// demo-token-001 is mapped to VIN12345, but we request VIN99999.
	req := httptest.NewRequest(http.MethodPost, "/vehicles/VIN99999/commands", nil)
	req.Header.Set("Authorization", "Bearer demo-token-001")
	rec := httptest.NewRecorder()
	handler.ServeHTTP(rec, req)

	assertStatus(t, rec, http.StatusForbidden)
	assertErrorBody(t, rec, "forbidden")
}

// TestMissingAuthorizationHeader verifies that requests without an Authorization header
// return HTTP 401 with body {"error":"unauthorized"}.
// TS-06-E4
func TestMissingAuthorizationHeader(t *testing.T) {
	cfg := testConfig()
	handler := auth.Middleware(cfg)(okHandler)

	for _, method := range []string{http.MethodPost, http.MethodGet} {
		t.Run(method, func(t *testing.T) {
			req := httptest.NewRequest(method, "/vehicles/VIN12345/commands", nil)
			// No Authorization header set.
			rec := httptest.NewRecorder()
			handler.ServeHTTP(rec, req)
			assertStatus(t, rec, http.StatusUnauthorized)
			assertErrorBody(t, rec, "unauthorized")
		})
	}
}

// TestInvalidToken verifies that an unrecognized bearer token returns HTTP 401
// with body {"error":"unauthorized"}.
// TS-06-E5
func TestInvalidToken(t *testing.T) {
	cfg := testConfig()
	handler := auth.Middleware(cfg)(okHandler)

	req := httptest.NewRequest(http.MethodPost, "/vehicles/VIN12345/commands", nil)
	req.Header.Set("Authorization", "Bearer invalid-token-999")
	rec := httptest.NewRecorder()
	handler.ServeHTTP(rec, req)

	assertStatus(t, rec, http.StatusUnauthorized)
	assertErrorBody(t, rec, "unauthorized")
}

// TestPropertyTokenVINIsolation verifies that for any valid token mapped to VIN V,
// a request using that token to a different VIN W always returns 403.
// TS-06-P1
func TestPropertyTokenVINIsolation(t *testing.T) {
	cfg := testConfig()
	handler := auth.Middleware(cfg)(okHandler)

	cases := []struct {
		token    string
		otherVIN string
	}{
		{"demo-token-001", "VIN67890"},
		{"demo-token-001", "VIN00000"},
		{"demo-token-002", "VIN12345"},
		{"demo-token-002", "VIN99999"},
	}

	for _, c := range cases {
		t.Run(c.token+"/"+c.otherVIN, func(t *testing.T) {
			req := httptest.NewRequest(http.MethodPost, "/vehicles/"+c.otherVIN+"/commands", nil)
			req.Header.Set("Authorization", "Bearer "+c.token)
			rec := httptest.NewRecorder()
			handler.ServeHTTP(rec, req)
			assertStatus(t, rec, http.StatusForbidden)
		})
	}
}

// TestPropertyAuthenticationGate verifies that for any request without a valid bearer
// token, the middleware returns 401.
// TS-06-P4
func TestPropertyAuthenticationGate(t *testing.T) {
	cfg := testConfig()
	handler := auth.Middleware(cfg)(okHandler)

	invalidTokens := []string{
		"",
		"invalid-token",
		"not-in-config",
		"demo-token-999",
		"Bearer-missing-actual-token",
	}

	for _, token := range invalidTokens {
		t.Run("token="+token, func(t *testing.T) {
			req := httptest.NewRequest(http.MethodGet, "/vehicles/VIN12345/commands/x", nil)
			if token != "" {
				req.Header.Set("Authorization", "Bearer "+token)
			}
			rec := httptest.NewRecorder()
			handler.ServeHTTP(rec, req)
			assertStatus(t, rec, http.StatusUnauthorized)
		})
	}
}
