package auth_test

import (
	"fmt"
	"net/http/httptest"
	"testing"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/auth"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
)

// TS-06-P1: Token-VIN Isolation
// Property 1 from design.md
// Validates: 06-REQ-3.2
// For any valid token mapped to VIN V, requests to a different VIN W always return 403.
func TestPropertyTokenVINIsolation(t *testing.T) {
	cfg := &model.Config{
		Tokens: []model.TokenMapping{
			{Token: "token-A", VIN: "VIN-A"},
			{Token: "token-B", VIN: "VIN-B"},
			{Token: "token-C", VIN: "VIN-C"},
		},
	}

	allVINs := []string{"VIN-A", "VIN-B", "VIN-C", "VIN-D", "VIN-E"}

	handler := auth.Middleware(cfg)(dummyHandler)

	for _, tm := range cfg.Tokens {
		for _, otherVIN := range allVINs {
			if otherVIN == tm.VIN {
				continue
			}
			t.Run(fmt.Sprintf("%s_to_%s", tm.Token, otherVIN), func(t *testing.T) {
				req := httptest.NewRequest("POST",
					"/vehicles/"+otherVIN+"/commands", nil)
				req.Header.Set("Authorization", "Bearer "+tm.Token)
				rec := httptest.NewRecorder()
				handler.ServeHTTP(rec, req)

				if rec.Code != 403 {
					t.Errorf("token %q to VIN %q: got %d, want 403",
						tm.Token, otherVIN, rec.Code)
				}
			})
		}
	}
}

// TS-06-P4: Authentication Gate
// Property 4 from design.md
// Validates: 06-REQ-3.E1
// For any request without a valid token, the middleware returns 401.
func TestPropertyAuthenticationGate(t *testing.T) {
	cfg := &model.Config{
		Tokens: []model.TokenMapping{
			{Token: "valid-token-1", VIN: "VIN12345"},
		},
	}

	invalidTokens := []string{
		"",
		"random-string",
		"invalid-token-999",
		"valid-token-1-extra",
		"VALID-TOKEN-1", // case-sensitive
		"another-fake-token",
		"admin",
		"root",
	}

	handler := auth.Middleware(cfg)(dummyHandler)

	for _, token := range invalidTokens {
		name := token
		if name == "" {
			name = "empty"
		}
		t.Run(name, func(t *testing.T) {
			req := httptest.NewRequest("GET", "/vehicles/VIN12345/commands/x", nil)
			if token != "" {
				req.Header.Set("Authorization", "Bearer "+token)
			}
			rec := httptest.NewRecorder()
			handler.ServeHTTP(rec, req)

			if rec.Code != 401 {
				t.Errorf("token %q: got %d, want 401", token, rec.Code)
			}
		})
	}
}
