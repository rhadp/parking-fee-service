package api

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/state"
)

// setupPairedStore creates a store with a registered and paired vehicle,
// returning the store and the pairing token.
func setupPairedStore(t *testing.T) (*state.Store, string) {
	t.Helper()
	s := state.NewStore()
	s.RegisterVehicle("VIN1", "123456")
	token, err := s.PairVehicle("VIN1", "123456")
	if err != nil {
		t.Fatalf("PairVehicle: %v", err)
	}
	return s, token
}

// okHandler is a simple handler that writes 200 OK.
var okHandler = http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusOK)
	w.Write([]byte(`{"ok":true}`))
})

func TestAuthMiddleware_ValidToken(t *testing.T) {
	store, token := setupPairedStore(t)

	mux := http.NewServeMux()
	mux.Handle("GET /api/v1/vehicles/{vin}/status", AuthMiddleware(store, okHandler))

	srv := httptest.NewServer(mux)
	defer srv.Close()

	req, _ := http.NewRequest("GET", srv.URL+"/api/v1/vehicles/VIN1/status", nil)
	req.Header.Set("Authorization", "Bearer "+token)

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatalf("request: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		t.Errorf("status = %d, want %d", resp.StatusCode, http.StatusOK)
	}
}

func TestAuthMiddleware_MissingAuthHeader(t *testing.T) {
	store, _ := setupPairedStore(t)

	mux := http.NewServeMux()
	mux.Handle("GET /api/v1/vehicles/{vin}/status", AuthMiddleware(store, okHandler))

	srv := httptest.NewServer(mux)
	defer srv.Close()

	req, _ := http.NewRequest("GET", srv.URL+"/api/v1/vehicles/VIN1/status", nil)
	// No Authorization header.

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatalf("request: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusUnauthorized {
		t.Errorf("status = %d, want %d", resp.StatusCode, http.StatusUnauthorized)
	}

	var errResp ErrorResponse
	json.NewDecoder(resp.Body).Decode(&errResp)
	if errResp.Code != "UNAUTHORIZED" {
		t.Errorf("error code = %q, want %q", errResp.Code, "UNAUTHORIZED")
	}
}

func TestAuthMiddleware_InvalidScheme(t *testing.T) {
	store, token := setupPairedStore(t)

	mux := http.NewServeMux()
	mux.Handle("GET /api/v1/vehicles/{vin}/status", AuthMiddleware(store, okHandler))

	srv := httptest.NewServer(mux)
	defer srv.Close()

	req, _ := http.NewRequest("GET", srv.URL+"/api/v1/vehicles/VIN1/status", nil)
	req.Header.Set("Authorization", "Basic "+token)

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatalf("request: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusUnauthorized {
		t.Errorf("status = %d, want %d", resp.StatusCode, http.StatusUnauthorized)
	}
}

func TestAuthMiddleware_WrongToken(t *testing.T) {
	store, _ := setupPairedStore(t)

	mux := http.NewServeMux()
	mux.Handle("GET /api/v1/vehicles/{vin}/status", AuthMiddleware(store, okHandler))

	srv := httptest.NewServer(mux)
	defer srv.Close()

	req, _ := http.NewRequest("GET", srv.URL+"/api/v1/vehicles/VIN1/status", nil)
	req.Header.Set("Authorization", "Bearer wrong-token")

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatalf("request: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusUnauthorized {
		t.Errorf("status = %d, want %d", resp.StatusCode, http.StatusUnauthorized)
	}
}

func TestAuthMiddleware_TokenForDifferentVIN(t *testing.T) {
	store, token := setupPairedStore(t)
	// Register another vehicle.
	store.RegisterVehicle("VIN2", "654321")

	mux := http.NewServeMux()
	mux.Handle("GET /api/v1/vehicles/{vin}/status", AuthMiddleware(store, okHandler))

	srv := httptest.NewServer(mux)
	defer srv.Close()

	// Use token for VIN1 but access VIN2.
	req, _ := http.NewRequest("GET", srv.URL+"/api/v1/vehicles/VIN2/status", nil)
	req.Header.Set("Authorization", "Bearer "+token)

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatalf("request: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusUnauthorized {
		t.Errorf("status = %d, want %d", resp.StatusCode, http.StatusUnauthorized)
	}
}

func TestExtractBearerToken(t *testing.T) {
	tests := []struct {
		name   string
		header string
		want   string
	}{
		{"valid", "Bearer abc123", "abc123"},
		{"case insensitive", "bearer abc123", "abc123"},
		{"BEARER", "BEARER abc123", "abc123"},
		{"empty", "", ""},
		{"no scheme", "abc123", ""},
		{"basic scheme", "Basic abc123", ""},
		{"only bearer", "Bearer", ""},
		{"bearer with spaces", "Bearer  abc123 ", "abc123"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			r := httptest.NewRequest("GET", "/", nil)
			if tt.header != "" {
				r.Header.Set("Authorization", tt.header)
			}
			got := extractBearerToken(r)
			if got != tt.want {
				t.Errorf("extractBearerToken(%q) = %q, want %q", tt.header, got, tt.want)
			}
		})
	}
}
