package auth_test

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/auth"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/config"
)

// testConfig returns a Config for tests.
func testConfig() *config.Config {
	return &config.Config{
		Tokens: []config.TokenMapping{
			{Token: "demo-token-001", VIN: "VIN12345"},
			{Token: "demo-token-002", VIN: "VIN67890"},
		},
	}
}

// testHandler is a simple handler that returns 200 OK.
var testHandler = http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusOK)
})

// newMux creates a ServeMux with the middleware applied, matching both command routes.
func newMux(cfg *config.Config) *http.ServeMux {
	mux := http.NewServeMux()
	mw := auth.Middleware(cfg)
	mux.Handle("POST /vehicles/{vin}/commands", mw(testHandler))
	mux.Handle("GET /vehicles/{vin}/commands/{command_id}", mw(testHandler))
	return mux
}

// TestBearerTokenValidation verifies a valid token passes through middleware (TS-06-8).
func TestBearerTokenValidation(t *testing.T) {
	cfg := testConfig()
	mux := newMux(cfg)
	req := httptest.NewRequest("GET", "/vehicles/VIN12345/commands/x", nil)
	req.Header.Set("Authorization", "Bearer demo-token-001")
	rec := httptest.NewRecorder()
	mux.ServeHTTP(rec, req)
	if rec.Code == http.StatusUnauthorized {
		t.Errorf("BearerTokenValidation: valid token returned 401")
	}
	if rec.Code == http.StatusForbidden {
		t.Errorf("BearerTokenValidation: valid token returned 403")
	}
}

// TestVINAuthorizationCheck verifies wrong VIN returns 403 (TS-06-9).
func TestVINAuthorizationCheck(t *testing.T) {
	cfg := testConfig()
	mux := newMux(cfg)
	req := httptest.NewRequest("POST", "/vehicles/VIN99999/commands", nil)
	req.Header.Set("Authorization", "Bearer demo-token-001")
	rec := httptest.NewRecorder()
	mux.ServeHTTP(rec, req)
	if rec.Code != http.StatusForbidden {
		t.Errorf("VINAuthorizationCheck: got %d, want 403", rec.Code)
	}
	var body map[string]string
	if err := json.NewDecoder(rec.Body).Decode(&body); err != nil {
		t.Fatalf("failed to decode response body: %v", err)
	}
	if body["error"] != "forbidden" {
		t.Errorf("VINAuthorizationCheck: body.error = %q, want %q", body["error"], "forbidden")
	}
}

// TestMissingAuthorizationHeader verifies missing header returns 401 (TS-06-E4).
func TestMissingAuthorizationHeader(t *testing.T) {
	cfg := testConfig()
	mux := newMux(cfg)
	for _, method := range []string{"POST", "GET"} {
		url := "/vehicles/VIN12345/commands"
		if method == "GET" {
			url = "/vehicles/VIN12345/commands/x"
		}
		req := httptest.NewRequest(method, url, nil)
		rec := httptest.NewRecorder()
		mux.ServeHTTP(rec, req)
		if rec.Code != http.StatusUnauthorized {
			t.Errorf("MissingAuthorizationHeader [%s]: got %d, want 401", method, rec.Code)
		}
		var body map[string]string
		if err := json.NewDecoder(rec.Body).Decode(&body); err != nil {
			t.Fatalf("failed to decode response body: %v", err)
		}
		if body["error"] != "unauthorized" {
			t.Errorf("MissingAuthorizationHeader [%s]: body.error = %q, want %q", method, body["error"], "unauthorized")
		}
	}
}

// TestInvalidToken verifies an unrecognized token returns 401 (TS-06-E5).
func TestInvalidToken(t *testing.T) {
	cfg := testConfig()
	mux := newMux(cfg)
	req := httptest.NewRequest("POST", "/vehicles/VIN12345/commands", nil)
	req.Header.Set("Authorization", "Bearer invalid-token-999")
	rec := httptest.NewRecorder()
	mux.ServeHTTP(rec, req)
	if rec.Code != http.StatusUnauthorized {
		t.Errorf("InvalidToken: got %d, want 401", rec.Code)
	}
	var body map[string]string
	if err := json.NewDecoder(rec.Body).Decode(&body); err != nil {
		t.Fatalf("failed to decode response body: %v", err)
	}
	if body["error"] != "unauthorized" {
		t.Errorf("InvalidToken: body.error = %q, want %q", body["error"], "unauthorized")
	}
}

// TestPropertyTokenVINIsolation verifies all token-VIN pairs are isolated (TS-06-P1).
func TestPropertyTokenVINIsolation(t *testing.T) {
	cfg := testConfig()
	otherVINs := []string{"VIN00001", "VIN99999", "WRONG-VIN"}
	for _, tm := range cfg.Tokens {
		for _, otherVIN := range otherVINs {
			if otherVIN == tm.VIN {
				continue
			}
			mux := newMux(cfg)
			req := httptest.NewRequest("POST", "/vehicles/"+otherVIN+"/commands", nil)
			req.Header.Set("Authorization", "Bearer "+tm.Token)
			rec := httptest.NewRecorder()
			mux.ServeHTTP(rec, req)
			if rec.Code != http.StatusForbidden {
				t.Errorf("TokenVINIsolation [token=%s otherVIN=%s]: got %d, want 403", tm.Token, otherVIN, rec.Code)
			}
		}
	}
}

// TestPropertyAuthenticationGate verifies random invalid tokens return 401 (TS-06-P4).
func TestPropertyAuthenticationGate(t *testing.T) {
	cfg := testConfig()
	invalidTokens := []string{"", "bad", "invalid-token-999", "Bearer ", "token-without-bearer"}
	validTokens := map[string]bool{
		"demo-token-001": true,
		"demo-token-002": true,
	}
	mux := newMux(cfg)
	for _, token := range invalidTokens {
		if validTokens[token] {
			continue
		}
		req := httptest.NewRequest("GET", "/vehicles/VIN12345/commands/x", nil)
		if token != "" {
			req.Header.Set("Authorization", "Bearer "+token)
		}
		rec := httptest.NewRecorder()
		mux.ServeHTTP(rec, req)
		if rec.Code != http.StatusUnauthorized {
			t.Errorf("AuthenticationGate [token=%q]: got %d, want 401", token, rec.Code)
		}
	}
}
