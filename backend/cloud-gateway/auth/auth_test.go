package auth_test

import (
	"encoding/json"
	"fmt"
	"math/rand"
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

// dummyHandler is a no-op handler that returns 200 when reached.
var dummyHandler = http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusOK)
})

// ---------------------------------------------------------------------------
// TS-06-8: Bearer Token Validation
// Requirement: 06-REQ-3.1
// ---------------------------------------------------------------------------

func TestBearerTokenValidation(t *testing.T) {
	cfg := testConfig()
	handler := auth.Middleware(cfg)(dummyHandler)

	req := httptest.NewRequest("GET", "/vehicles/VIN12345/commands/x", nil)
	req.Header.Set("Authorization", "Bearer demo-token-001")
	rec := httptest.NewRecorder()
	handler.ServeHTTP(rec, req)

	if rec.Code == 401 {
		t.Error("expected request to pass auth (not 401)")
	}
	if rec.Code == 403 {
		t.Error("expected request to pass auth (not 403)")
	}
}

// ---------------------------------------------------------------------------
// TS-06-9: VIN Authorization Check
// Requirement: 06-REQ-3.2
// ---------------------------------------------------------------------------

func TestVINAuthorizationCheck(t *testing.T) {
	cfg := testConfig()
	handler := auth.Middleware(cfg)(dummyHandler)

	req := httptest.NewRequest("POST", "/vehicles/VIN99999/commands", nil)
	req.Header.Set("Authorization", "Bearer demo-token-001")
	rec := httptest.NewRecorder()
	handler.ServeHTTP(rec, req)

	if rec.Code != 403 {
		t.Errorf("expected status 403, got %d", rec.Code)
	}

	var body map[string]string
	if err := json.NewDecoder(rec.Body).Decode(&body); err != nil {
		t.Fatalf("failed to decode response body: %v", err)
	}
	if body["error"] != "forbidden" {
		t.Errorf("expected error 'forbidden', got %q", body["error"])
	}
}

// ---------------------------------------------------------------------------
// TS-06-E4: Missing Authorization Header
// Requirement: 06-REQ-3.E1
// Note: GET path uses /vehicles/VIN12345/commands/x (not /commands)
//       per skeptic review finding about Go 1.22 ServeMux routing.
// ---------------------------------------------------------------------------

func TestMissingAuthorizationHeader(t *testing.T) {
	cfg := testConfig()
	handler := auth.Middleware(cfg)(dummyHandler)

	tests := []struct {
		method string
		path   string
	}{
		{"POST", "/vehicles/VIN12345/commands"},
		{"GET", "/vehicles/VIN12345/commands/x"},
	}

	for _, tt := range tests {
		t.Run(tt.method, func(t *testing.T) {
			req := httptest.NewRequest(tt.method, tt.path, nil)
			// No Authorization header
			rec := httptest.NewRecorder()
			handler.ServeHTTP(rec, req)

			if rec.Code != 401 {
				t.Errorf("expected status 401, got %d", rec.Code)
			}

			var body map[string]string
			if err := json.NewDecoder(rec.Body).Decode(&body); err != nil {
				t.Fatalf("failed to decode response body: %v", err)
			}
			if body["error"] != "unauthorized" {
				t.Errorf("expected error 'unauthorized', got %q", body["error"])
			}
		})
	}
}

// ---------------------------------------------------------------------------
// TS-06-E5: Invalid Token
// Requirement: 06-REQ-3.E1
// ---------------------------------------------------------------------------

func TestInvalidToken(t *testing.T) {
	cfg := testConfig()
	handler := auth.Middleware(cfg)(dummyHandler)

	req := httptest.NewRequest("POST", "/vehicles/VIN12345/commands", nil)
	req.Header.Set("Authorization", "Bearer invalid-token-999")
	rec := httptest.NewRecorder()
	handler.ServeHTTP(rec, req)

	if rec.Code != 401 {
		t.Errorf("expected status 401, got %d", rec.Code)
	}

	var body map[string]string
	if err := json.NewDecoder(rec.Body).Decode(&body); err != nil {
		t.Fatalf("failed to decode response body: %v", err)
	}
	if body["error"] != "unauthorized" {
		t.Errorf("expected error 'unauthorized', got %q", body["error"])
	}
}

// ---------------------------------------------------------------------------
// TS-06-P1: Token-VIN Isolation Property
// Property 1 from design.md
// Requirement: 06-REQ-3.2
// ---------------------------------------------------------------------------

func TestPropertyTokenVINIsolation(t *testing.T) {
	cfg := testConfig()
	handler := auth.Middleware(cfg)(dummyHandler)

	allVINs := []string{"VIN12345", "VIN67890", "VINAAA", "VINBBB", "VINCCC"}

	for _, tm := range cfg.Tokens {
		for _, otherVIN := range allVINs {
			if otherVIN == tm.VIN {
				continue
			}
			t.Run(fmt.Sprintf("token_%s_vin_%s", tm.Token, otherVIN), func(t *testing.T) {
				req := httptest.NewRequest("POST", "/vehicles/"+otherVIN+"/commands", nil)
				req.Header.Set("Authorization", "Bearer "+tm.Token)
				rec := httptest.NewRecorder()
				handler.ServeHTTP(rec, req)

				if rec.Code != 403 {
					t.Errorf("token %q to VIN %q: expected 403, got %d",
						tm.Token, otherVIN, rec.Code)
				}
			})
		}
	}
}

// ---------------------------------------------------------------------------
// TS-06-P4: Authentication Gate Property
// Property 4 from design.md
// Requirement: 06-REQ-3.E1
// ---------------------------------------------------------------------------

func TestPropertyAuthenticationGate(t *testing.T) {
	cfg := testConfig()
	handler := auth.Middleware(cfg)(dummyHandler)

	// Generate random invalid tokens
	rng := rand.New(rand.NewSource(42))
	validTokens := make(map[string]bool)
	for _, tm := range cfg.Tokens {
		validTokens[tm.Token] = true
	}

	for i := 0; i < 20; i++ {
		token := fmt.Sprintf("random-token-%d-%d", i, rng.Int())
		if validTokens[token] {
			continue
		}
		t.Run(fmt.Sprintf("invalid_token_%d", i), func(t *testing.T) {
			req := httptest.NewRequest("GET", "/vehicles/VIN12345/commands/x", nil)
			req.Header.Set("Authorization", "Bearer "+token)
			rec := httptest.NewRecorder()
			handler.ServeHTTP(rec, req)

			if rec.Code != 401 {
				t.Errorf("invalid token %q: expected 401, got %d", token, rec.Code)
			}
		})
	}
}
