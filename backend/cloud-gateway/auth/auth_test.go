package auth_test

import (
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/auth"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/config"
)

func testConfig() *config.Config {
	return &config.Config{
		Tokens: []config.TokenMapping{
			{Token: "demo-token-001", VIN: "VIN12345"},
			{Token: "demo-token-002", VIN: "VIN67890"},
		},
	}
}

// testHandler is a simple handler that verifies the middleware set context values
// and responds with 200.
func testHandler(t *testing.T) http.HandlerFunc {
	t.Helper()
	return func(w http.ResponseWriter, r *http.Request) {
		token, ok := r.Context().Value(auth.TokenContextKey).(string)
		if !ok || token == "" {
			t.Error("auth middleware did not set token in context")
		}
		w.WriteHeader(http.StatusOK)
	}
}

// TS-06-8: Bearer Token Validation
// Requirement: 06-REQ-3.1
func TestBearerTokenValidation(t *testing.T) {
	cfg := testConfig()
	wrapped := auth.Middleware(cfg)(testHandler(t))

	req := httptest.NewRequest("GET", "/vehicles/VIN12345/commands/x", nil)
	req.Header.Set("Authorization", "Bearer demo-token-001")
	req.SetPathValue("vin", "VIN12345")
	rec := httptest.NewRecorder()

	wrapped.ServeHTTP(rec, req)

	if rec.Code == http.StatusUnauthorized {
		t.Error("expected request to not be rejected as unauthorized")
	}
	if rec.Code == http.StatusForbidden {
		t.Error("expected request to not be rejected as forbidden")
	}
}

// TS-06-9: VIN Authorization Check
// Requirement: 06-REQ-3.2
func TestVINAuthorizationCheck(t *testing.T) {
	cfg := testConfig()
	dummyHandler := http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		t.Error("handler should not be called when VIN does not match")
		w.WriteHeader(http.StatusOK)
	})
	wrapped := auth.Middleware(cfg)(dummyHandler)

	req := httptest.NewRequest("POST", "/vehicles/VIN99999/commands", nil)
	req.Header.Set("Authorization", "Bearer demo-token-001")
	req.SetPathValue("vin", "VIN99999")
	rec := httptest.NewRecorder()

	wrapped.ServeHTTP(rec, req)

	if rec.Code != http.StatusForbidden {
		t.Errorf("expected 403, got %d", rec.Code)
	}

	var body map[string]string
	if err := json.NewDecoder(rec.Body).Decode(&body); err != nil {
		t.Fatalf("failed to decode response body: %v", err)
	}
	if body["error"] != "forbidden" {
		t.Errorf("expected error 'forbidden', got '%s'", body["error"])
	}
}

// TS-06-E4: Missing Authorization Header
// Requirement: 06-REQ-3.E1
func TestMissingAuthorizationHeader(t *testing.T) {
	cfg := testConfig()
	dummyHandler := http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		t.Error("handler should not be called without authorization")
		w.WriteHeader(http.StatusOK)
	})
	wrapped := auth.Middleware(cfg)(dummyHandler)

	methods := []string{"POST", "GET"}
	for _, method := range methods {
		t.Run(method, func(t *testing.T) {
			req := httptest.NewRequest(method, "/vehicles/VIN12345/commands", nil)
			req.SetPathValue("vin", "VIN12345")
			// No Authorization header.
			rec := httptest.NewRecorder()

			wrapped.ServeHTTP(rec, req)

			if rec.Code != http.StatusUnauthorized {
				t.Errorf("expected 401, got %d", rec.Code)
			}

			var body map[string]string
			if err := json.NewDecoder(rec.Body).Decode(&body); err != nil {
				t.Fatalf("failed to decode response body: %v", err)
			}
			if body["error"] != "unauthorized" {
				t.Errorf("expected error 'unauthorized', got '%s'", body["error"])
			}
		})
	}
}

// TS-06-E5: Invalid Token
// Requirement: 06-REQ-3.E1
func TestInvalidToken(t *testing.T) {
	cfg := testConfig()
	dummyHandler := http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		t.Error("handler should not be called with invalid token")
		w.WriteHeader(http.StatusOK)
	})
	wrapped := auth.Middleware(cfg)(dummyHandler)

	req := httptest.NewRequest("POST", "/vehicles/VIN12345/commands", nil)
	req.Header.Set("Authorization", "Bearer invalid-token-999")
	req.SetPathValue("vin", "VIN12345")
	rec := httptest.NewRecorder()

	wrapped.ServeHTTP(rec, req)

	if rec.Code != http.StatusUnauthorized {
		t.Errorf("expected 401, got %d", rec.Code)
	}

	var body map[string]string
	if err := json.NewDecoder(rec.Body).Decode(&body); err != nil {
		t.Fatalf("failed to decode response body: %v", err)
	}
	if body["error"] != "unauthorized" {
		t.Errorf("expected error 'unauthorized', got '%s'", body["error"])
	}
}

// TS-06-P1: Token-VIN Isolation (Property Test)
// Property 1: For any valid token mapped to VIN V, requests to a different VIN W return 403.
// Validates: 06-REQ-3.2
func TestPropertyTokenVINIsolation(t *testing.T) {
	cfg := testConfig()
	dummyHandler := http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusOK)
	})
	wrapped := auth.Middleware(cfg)(dummyHandler)

	allVINs := []string{"VIN12345", "VIN67890", "VIN11111", "VIN22222", "VIN33333"}

	for _, tm := range cfg.Tokens {
		for _, otherVIN := range allVINs {
			if otherVIN == tm.VIN {
				continue // skip matching VIN
			}
			t.Run(fmt.Sprintf("token_%s_vin_%s", tm.Token, otherVIN), func(t *testing.T) {
				req := httptest.NewRequest("POST", "/vehicles/"+otherVIN+"/commands", nil)
				req.Header.Set("Authorization", "Bearer "+tm.Token)
				req.SetPathValue("vin", otherVIN)
				rec := httptest.NewRecorder()

				wrapped.ServeHTTP(rec, req)

				if rec.Code != http.StatusForbidden {
					t.Errorf("expected 403 for token %s with VIN %s, got %d",
						tm.Token, otherVIN, rec.Code)
				}
			})
		}
	}
}

// TS-06-P4: Authentication Gate (Property Test)
// Property 4: For any request without a valid token, the middleware returns 401.
// Validates: 06-REQ-3.E1
func TestPropertyAuthenticationGate(t *testing.T) {
	cfg := testConfig()
	dummyHandler := http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusOK)
	})
	wrapped := auth.Middleware(cfg)(dummyHandler)

	invalidTokens := []string{
		"invalid-1",
		"invalid-2",
		"random-token",
		"not-a-real-token",
		"demo-token-003",
		"DEMO-TOKEN-001", // case-sensitive mismatch
	}

	for _, token := range invalidTokens {
		t.Run("token_"+token, func(t *testing.T) {
			req := httptest.NewRequest("GET", "/vehicles/VIN12345/commands/x", nil)
			req.Header.Set("Authorization", "Bearer "+token)
			req.SetPathValue("vin", "VIN12345")
			rec := httptest.NewRecorder()

			wrapped.ServeHTTP(rec, req)

			if rec.Code != http.StatusUnauthorized {
				t.Errorf("expected 401 for invalid token %q, got %d", token, rec.Code)
			}
		})
	}
}
