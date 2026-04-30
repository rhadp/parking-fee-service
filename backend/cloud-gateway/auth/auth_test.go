package auth_test

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/auth"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
)

// testConfig returns a config with demo token-VIN mappings.
func testConfig() *model.Config {
	return &model.Config{
		Port:                  8081,
		NatsURL:               "nats://localhost:4222",
		CommandTimeoutSeconds: 30,
		Tokens: []model.TokenMapping{
			{Token: "demo-token-001", VIN: "VIN12345"},
			{Token: "demo-token-002", VIN: "VIN67890"},
		},
	}
}

// dummyHandler is a handler that returns 200 to indicate middleware passed.
var dummyHandler = http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusOK)
})

// TS-06-8: Bearer Token Validation
// Requirement: 06-REQ-3.1
// The auth middleware extracts and validates bearer tokens.
func TestBearerTokenValidation(t *testing.T) {
	cfg := testConfig()
	handler := auth.Middleware(cfg)(dummyHandler)

	req := httptest.NewRequest("GET", "/vehicles/VIN12345/commands/x", nil)
	req.Header.Set("Authorization", "Bearer demo-token-001")
	rec := httptest.NewRecorder()
	handler.ServeHTTP(rec, req)

	if rec.Code == 401 {
		t.Errorf("expected request to pass auth, got 401")
	}
	if rec.Code == 403 {
		t.Errorf("expected request to pass auth, got 403")
	}
}

// TS-06-9: VIN Authorization Check
// Requirement: 06-REQ-3.2
// A valid token used against a different VIN returns HTTP 403.
func TestVINAuthorizationCheck(t *testing.T) {
	cfg := testConfig()
	handler := auth.Middleware(cfg)(dummyHandler)

	req := httptest.NewRequest("POST", "/vehicles/VIN99999/commands", nil)
	req.Header.Set("Authorization", "Bearer demo-token-001")
	rec := httptest.NewRecorder()
	handler.ServeHTTP(rec, req)

	if rec.Code != 403 {
		t.Errorf("expected 403 for VIN mismatch, got %d", rec.Code)
	}

	var body map[string]string
	if err := json.NewDecoder(rec.Body).Decode(&body); err != nil {
		t.Fatalf("failed to decode response body: %v", err)
	}
	if body["error"] != "forbidden" {
		t.Errorf("error = %q, want %q", body["error"], "forbidden")
	}
}

// TS-06-E4: Missing Authorization Header
// Requirement: 06-REQ-3.E1
// Requests without Authorization header return HTTP 401.
func TestMissingAuthorizationHeader(t *testing.T) {
	cfg := testConfig()
	handler := auth.Middleware(cfg)(dummyHandler)

	methods := []string{"POST", "GET"}
	for _, method := range methods {
		t.Run(method, func(t *testing.T) {
			req := httptest.NewRequest(method, "/vehicles/VIN12345/commands", nil)
			// No Authorization header
			rec := httptest.NewRecorder()
			handler.ServeHTTP(rec, req)

			if rec.Code != 401 {
				t.Errorf("expected 401 for missing auth, got %d", rec.Code)
			}

			var body map[string]string
			if err := json.NewDecoder(rec.Body).Decode(&body); err != nil {
				t.Fatalf("failed to decode response body: %v", err)
			}
			if body["error"] != "unauthorized" {
				t.Errorf("error = %q, want %q", body["error"], "unauthorized")
			}
		})
	}
}

// TS-06-E5: Invalid Token
// Requirement: 06-REQ-3.E1
// Requests with an unrecognized token return HTTP 401.
func TestInvalidToken(t *testing.T) {
	cfg := testConfig()
	handler := auth.Middleware(cfg)(dummyHandler)

	req := httptest.NewRequest("POST", "/vehicles/VIN12345/commands", nil)
	req.Header.Set("Authorization", "Bearer invalid-token-999")
	rec := httptest.NewRecorder()
	handler.ServeHTTP(rec, req)

	if rec.Code != 401 {
		t.Errorf("expected 401 for invalid token, got %d", rec.Code)
	}

	var body map[string]string
	if err := json.NewDecoder(rec.Body).Decode(&body); err != nil {
		t.Fatalf("failed to decode response body: %v", err)
	}
	if body["error"] != "unauthorized" {
		t.Errorf("error = %q, want %q", body["error"], "unauthorized")
	}
}
