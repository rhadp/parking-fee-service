package main

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/state"
)

func TestHealthzReturns200(t *testing.T) {
	store := state.NewStore()
	srv := httptest.NewServer(newServeMux(store, nil))
	defer srv.Close()

	resp, err := http.Get(srv.URL + "/healthz")
	if err != nil {
		t.Fatalf("GET /healthz: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		t.Errorf("GET /healthz: got status %d, want %d", resp.StatusCode, http.StatusOK)
	}
}

func TestProtectedEndpointsRequireAuth(t *testing.T) {
	store := state.NewStore()
	store.RegisterVehicle("WVWZZZ1KZAW123456", "123456")

	srv := httptest.NewServer(newServeMux(store, nil))
	defer srv.Close()

	routes := []struct {
		method string
		path   string
	}{
		{"POST", "/api/v1/vehicles/WVWZZZ1KZAW123456/lock"},
		{"POST", "/api/v1/vehicles/WVWZZZ1KZAW123456/unlock"},
		{"GET", "/api/v1/vehicles/WVWZZZ1KZAW123456/status"},
	}

	for _, rt := range routes {
		t.Run(rt.method+" "+rt.path, func(t *testing.T) {
			req, err := http.NewRequest(rt.method, srv.URL+rt.path, nil)
			if err != nil {
				t.Fatalf("creating request: %v", err)
			}
			// No Authorization header — should get 401.
			resp, err := http.DefaultClient.Do(req)
			if err != nil {
				t.Fatalf("%s %s: %v", rt.method, rt.path, err)
			}
			defer resp.Body.Close()

			if resp.StatusCode != http.StatusUnauthorized {
				t.Errorf("%s %s: got status %d, want %d",
					rt.method, rt.path, resp.StatusCode, http.StatusUnauthorized)
			}
		})
	}
}

func TestProtectedEndpointsWorkWithAuth(t *testing.T) {
	store := state.NewStore()
	store.RegisterVehicle("WVWZZZ1KZAW123456", "123456")
	token, _ := store.PairVehicle("WVWZZZ1KZAW123456", "123456")

	srv := httptest.NewServer(newServeMux(store, nil))
	defer srv.Close()

	routes := []struct {
		method     string
		path       string
		wantStatus int
	}{
		{"POST", "/api/v1/vehicles/WVWZZZ1KZAW123456/lock", http.StatusAccepted},
		{"POST", "/api/v1/vehicles/WVWZZZ1KZAW123456/unlock", http.StatusAccepted},
		{"GET", "/api/v1/vehicles/WVWZZZ1KZAW123456/status", http.StatusOK},
	}

	for _, rt := range routes {
		t.Run(rt.method+" "+rt.path, func(t *testing.T) {
			req, err := http.NewRequest(rt.method, srv.URL+rt.path, nil)
			if err != nil {
				t.Fatalf("creating request: %v", err)
			}
			req.Header.Set("Authorization", "Bearer "+token)

			resp, err := http.DefaultClient.Do(req)
			if err != nil {
				t.Fatalf("%s %s: %v", rt.method, rt.path, err)
			}
			defer resp.Body.Close()

			if resp.StatusCode != rt.wantStatus {
				t.Errorf("%s %s: got status %d, want %d",
					rt.method, rt.path, resp.StatusCode, rt.wantStatus)
			}

			ct := resp.Header.Get("Content-Type")
			if ct != "application/json" {
				t.Errorf("%s %s: Content-Type = %q, want %q",
					rt.method, rt.path, ct, "application/json")
			}
		})
	}
}

func TestPairEndpoint(t *testing.T) {
	store := state.NewStore()
	store.RegisterVehicle("VIN1", "123456")

	srv := httptest.NewServer(newServeMux(store, nil))
	defer srv.Close()

	body := `{"vin":"VIN1","pin":"123456"}`
	resp, err := http.Post(srv.URL+"/api/v1/pair", "application/json", strings.NewReader(body))
	if err != nil {
		t.Fatalf("POST /api/v1/pair: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		t.Errorf("status = %d, want %d", resp.StatusCode, http.StatusOK)
	}

	var result struct {
		Token string `json:"token"`
		VIN   string `json:"vin"`
	}
	json.NewDecoder(resp.Body).Decode(&result)

	if result.Token == "" {
		t.Error("expected non-empty token")
	}
	if result.VIN != "VIN1" {
		t.Errorf("VIN = %q, want %q", result.VIN, "VIN1")
	}
}

func TestEnvOrDefault(t *testing.T) {
	// With no env var set, should return default.
	val := envOrDefault("TEST_NONEXISTENT_VAR_XYZ", "fallback")
	if val != "fallback" {
		t.Errorf("envOrDefault: got %q, want %q", val, "fallback")
	}

	// With env var set, should return env value.
	t.Setenv("TEST_ENV_VAR_ABC", "custom-value")
	val = envOrDefault("TEST_ENV_VAR_ABC", "fallback")
	if val != "custom-value" {
		t.Errorf("envOrDefault: got %q, want %q", val, "custom-value")
	}
}
