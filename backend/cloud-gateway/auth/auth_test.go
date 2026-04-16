package auth_test

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"parking-fee-service/backend/cloud-gateway/auth"
	"parking-fee-service/backend/cloud-gateway/model"
)

// makeTestConfig creates a test configuration with two token-VIN pairs.
func makeTestConfig() *model.Config {
	return &model.Config{
		Tokens: []model.TokenMapping{
			{Token: "demo-token-001", VIN: "VIN12345"},
			{Token: "demo-token-002", VIN: "VIN67890"},
		},
	}
}

// okHandler is a simple handler that always returns 200.
var okHandler = http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusOK)
})

// decodeError decodes {"error":"..."} from body.
func decodeError(body *strings.Reader) string {
	var m map[string]string
	_ = json.NewDecoder(body).Decode(&m)
	return m["error"]
}

// TestBearerTokenValidation verifies that a valid token paired with the correct VIN
// is allowed through the middleware.
// Test Spec: TS-06-8
// Requirements: 06-REQ-3.1
func TestBearerTokenValidation(t *testing.T) {
	cfg := makeTestConfig()
	handler := auth.Middleware(cfg)(okHandler)

	req := httptest.NewRequest(http.MethodGet, "/vehicles/VIN12345/commands/x", nil)
	req.Header.Set("Authorization", "Bearer demo-token-001")
	rec := httptest.NewRecorder()
	handler.ServeHTTP(rec, req)

	if rec.Code == http.StatusUnauthorized {
		t.Errorf("valid token should not return 401, got 401")
	}
	if rec.Code == http.StatusForbidden {
		t.Errorf("valid token with matching VIN should not return 403, got 403")
	}
}

// TestVINAuthorizationCheck verifies that a valid token used against a different VIN
// returns HTTP 403.
// Test Spec: TS-06-9
// Requirements: 06-REQ-3.2
func TestVINAuthorizationCheck(t *testing.T) {
	cfg := makeTestConfig()
	handler := auth.Middleware(cfg)(okHandler)

	req := httptest.NewRequest(http.MethodPost, "/vehicles/VIN99999/commands", nil)
	req.Header.Set("Authorization", "Bearer demo-token-001")
	rec := httptest.NewRecorder()
	handler.ServeHTTP(rec, req)

	if rec.Code != http.StatusForbidden {
		t.Errorf("expected 403 for VIN mismatch, got %d", rec.Code)
	}
	body := strings.NewReader(rec.Body.String())
	errMsg := decodeError(body)
	if errMsg != "forbidden" {
		t.Errorf(`expected error "forbidden", got %q`, errMsg)
	}
}

// TestMissingAuthorizationHeader verifies that requests without an Authorization header
// return HTTP 401.
// Test Spec: TS-06-E4
// Requirements: 06-REQ-3.E1
func TestMissingAuthorizationHeader(t *testing.T) {
	cfg := makeTestConfig()
	handler := auth.Middleware(cfg)(okHandler)

	for _, method := range []string{http.MethodPost, http.MethodGet} {
		req := httptest.NewRequest(method, "/vehicles/VIN12345/commands", nil)
		// No Authorization header set
		rec := httptest.NewRecorder()
		handler.ServeHTTP(rec, req)

		if rec.Code != http.StatusUnauthorized {
			t.Errorf("%s: expected 401 for missing auth header, got %d", method, rec.Code)
		}
		body := strings.NewReader(rec.Body.String())
		errMsg := decodeError(body)
		if errMsg != "unauthorized" {
			t.Errorf(`%s: expected error "unauthorized", got %q`, method, errMsg)
		}
	}
}

// TestInvalidToken verifies that an unrecognized bearer token returns HTTP 401.
// Test Spec: TS-06-E5
// Requirements: 06-REQ-3.E1
func TestInvalidToken(t *testing.T) {
	cfg := makeTestConfig()
	handler := auth.Middleware(cfg)(okHandler)

	req := httptest.NewRequest(http.MethodPost, "/vehicles/VIN12345/commands", nil)
	req.Header.Set("Authorization", "Bearer invalid-token-999")
	rec := httptest.NewRecorder()
	handler.ServeHTTP(rec, req)

	if rec.Code != http.StatusUnauthorized {
		t.Errorf("expected 401 for invalid token, got %d", rec.Code)
	}
	body := strings.NewReader(rec.Body.String())
	errMsg := decodeError(body)
	if errMsg != "unauthorized" {
		t.Errorf(`expected error "unauthorized", got %q`, errMsg)
	}
}

// TestPropertyTokenVINIsolation verifies that for any valid token mapped to VIN V,
// requests to a different VIN always return 403.
// Test Spec: TS-06-P1
// Property: Property 1 from design.md
// Requirements: 06-REQ-3.2
func TestPropertyTokenVINIsolation(t *testing.T) {
	cfg := makeTestConfig()
	handler := auth.Middleware(cfg)(okHandler)

	// All VINs in the config
	allVINs := []string{"VIN12345", "VIN67890", "VIN99999"}

	for _, tm := range cfg.Tokens {
		for _, otherVIN := range allVINs {
			if otherVIN == tm.VIN {
				continue // skip matching VIN
			}
			path := "/vehicles/" + otherVIN + "/commands"
			req := httptest.NewRequest(http.MethodPost, path, nil)
			req.Header.Set("Authorization", "Bearer "+tm.Token)
			rec := httptest.NewRecorder()
			handler.ServeHTTP(rec, req)

			if rec.Code != http.StatusForbidden {
				t.Errorf("token %q on VIN %q: expected 403, got %d", tm.Token, otherVIN, rec.Code)
			}
		}
	}
}

// TestPropertyAuthenticationGate verifies that requests without a valid token
// always return 401.
// Test Spec: TS-06-P4
// Property: Property 4 from design.md
// Requirements: 06-REQ-3.E1
func TestPropertyAuthenticationGate(t *testing.T) {
	cfg := makeTestConfig()
	handler := auth.Middleware(cfg)(okHandler)

	// List of clearly invalid tokens
	invalidTokens := []string{
		"",
		"bad-token",
		"not-a-token",
		"Bearer demo-token-001", // accidentally double-Bearer
		"demo-token-003",        // not in config
		"DEMO-TOKEN-001",        // wrong case
	}

	for _, token := range invalidTokens {
		req := httptest.NewRequest(http.MethodGet, "/vehicles/VIN12345/commands/x", nil)
		if token != "" {
			req.Header.Set("Authorization", "Bearer "+token)
		}
		rec := httptest.NewRecorder()
		handler.ServeHTTP(rec, req)

		if rec.Code != http.StatusUnauthorized {
			t.Errorf("token %q: expected 401, got %d", token, rec.Code)
		}
	}
}
