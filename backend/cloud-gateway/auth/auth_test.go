package auth

import (
	"encoding/json"
	"fmt"
	"math/rand"
	"net/http"
	"net/http/httptest"
	"testing"

	"parking-fee-service/backend/cloud-gateway/model"
)

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

// dummyHandler is a simple handler that returns 200 OK when reached.
var dummyHandler = http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusOK)
	w.Write([]byte(`{"ok":true}`))
})

// TS-06-8: Bearer Token Validation
func TestBearerTokenValidation(t *testing.T) {
	cfg := testConfig()
	wrapped := Middleware(cfg)(dummyHandler)

	req := httptest.NewRequest("GET", "/vehicles/VIN12345/commands/x", nil)
	req.Header.Set("Authorization", "Bearer demo-token-001")
	rec := httptest.NewRecorder()
	wrapped.ServeHTTP(rec, req)

	if rec.Code == 401 {
		t.Error("middleware returned 401 for valid token")
	}
	if rec.Code == 403 {
		t.Error("middleware returned 403 for matching VIN")
	}
}

// TS-06-9: VIN Authorization Check
func TestVINAuthorizationCheck(t *testing.T) {
	cfg := testConfig()
	wrapped := Middleware(cfg)(dummyHandler)

	req := httptest.NewRequest("POST", "/vehicles/VIN99999/commands", nil)
	req.Header.Set("Authorization", "Bearer demo-token-001")
	rec := httptest.NewRecorder()
	wrapped.ServeHTTP(rec, req)

	if rec.Code != 403 {
		t.Errorf("status = %d, want 403", rec.Code)
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
func TestMissingAuthorizationHeader(t *testing.T) {
	cfg := testConfig()
	wrapped := Middleware(cfg)(dummyHandler)

	endpoints := []struct {
		method string
		path   string
	}{
		{"POST", "/vehicles/VIN12345/commands"},
		{"GET", "/vehicles/VIN12345/commands/x"},
	}

	for _, ep := range endpoints {
		t.Run(ep.method+" "+ep.path, func(t *testing.T) {
			req := httptest.NewRequest(ep.method, ep.path, nil)
			// No Authorization header set
			rec := httptest.NewRecorder()
			wrapped.ServeHTTP(rec, req)

			if rec.Code != 401 {
				t.Errorf("status = %d, want 401", rec.Code)
			}
			var body map[string]string
			if err := json.NewDecoder(rec.Body).Decode(&body); err != nil {
				t.Fatalf("failed to decode body: %v", err)
			}
			if body["error"] != "unauthorized" {
				t.Errorf("error = %q, want %q", body["error"], "unauthorized")
			}
		})
	}
}

// TS-06-E5: Invalid Token
func TestInvalidToken(t *testing.T) {
	cfg := testConfig()
	wrapped := Middleware(cfg)(dummyHandler)

	req := httptest.NewRequest("POST", "/vehicles/VIN12345/commands", nil)
	req.Header.Set("Authorization", "Bearer invalid-token-999")
	rec := httptest.NewRecorder()
	wrapped.ServeHTTP(rec, req)

	if rec.Code != 401 {
		t.Errorf("status = %d, want 401", rec.Code)
	}
	var body map[string]string
	if err := json.NewDecoder(rec.Body).Decode(&body); err != nil {
		t.Fatalf("failed to decode body: %v", err)
	}
	if body["error"] != "unauthorized" {
		t.Errorf("error = %q, want %q", body["error"], "unauthorized")
	}
}

// TS-06-P1: Property - Token-VIN Isolation
func TestPropertyTokenVINIsolation(t *testing.T) {
	cfg := testConfig()
	wrapped := Middleware(cfg)(dummyHandler)

	allVINs := []string{"VIN12345", "VIN67890", "VIN00000", "VINAAAAA", "VINZZZZZ"}

	for _, tm := range cfg.Tokens {
		for _, otherVIN := range allVINs {
			if otherVIN == tm.VIN {
				continue
			}
			t.Run(fmt.Sprintf("token=%s_vin=%s", tm.Token, otherVIN), func(t *testing.T) {
				req := httptest.NewRequest("POST", "/vehicles/"+otherVIN+"/commands", nil)
				req.Header.Set("Authorization", "Bearer "+tm.Token)
				rec := httptest.NewRecorder()
				wrapped.ServeHTTP(rec, req)

				if rec.Code != 403 {
					t.Errorf("status = %d, want 403 for token %s accessing VIN %s", rec.Code, tm.Token, otherVIN)
				}
			})
		}
	}
}

// TS-06-P4: Property - Authentication Gate
func TestPropertyAuthenticationGate(t *testing.T) {
	cfg := testConfig()
	wrapped := Middleware(cfg)(dummyHandler)

	// Generate random invalid tokens
	rng := rand.New(rand.NewSource(42))
	validTokens := make(map[string]bool)
	for _, tm := range cfg.Tokens {
		validTokens[tm.Token] = true
	}

	for i := 0; i < 50; i++ {
		token := fmt.Sprintf("random-token-%d-%d", i, rng.Int())
		if validTokens[token] {
			continue // extremely unlikely, but skip
		}
		t.Run(fmt.Sprintf("token=%s", token), func(t *testing.T) {
			req := httptest.NewRequest("GET", "/vehicles/VIN12345/commands/x", nil)
			req.Header.Set("Authorization", "Bearer "+token)
			rec := httptest.NewRecorder()
			wrapped.ServeHTTP(rec, req)

			if rec.Code != 401 {
				t.Errorf("status = %d, want 401 for invalid token %q", rec.Code, token)
			}
		})
	}
}
