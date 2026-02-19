package main

import (
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/zones"
)

func TestHealthzReturns200(t *testing.T) {
	store := zones.LoadSeedData()
	srv := httptest.NewServer(newServeMux(store))
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

func TestNewServeMux_RoutesRegistered(t *testing.T) {
	store := zones.LoadSeedData()
	srv := httptest.NewServer(newServeMux(store))
	defer srv.Close()

	// Verify that the zone API routes return non-404 responses.
	routes := []struct {
		method string
		path   string
	}{
		{"GET", "/healthz"},
		{"GET", "/api/v1/zones?lat=48.137&lon=11.575"},
		{"GET", "/api/v1/zones/zone-marienplatz"},
		{"GET", "/api/v1/zones/zone-marienplatz/adapter"},
	}

	for _, rt := range routes {
		t.Run(rt.method+" "+rt.path, func(t *testing.T) {
			req, err := http.NewRequest(rt.method, srv.URL+rt.path, nil)
			if err != nil {
				t.Fatalf("creating request: %v", err)
			}

			resp, err := http.DefaultClient.Do(req)
			if err != nil {
				t.Fatalf("%s %s: %v", rt.method, rt.path, err)
			}
			defer resp.Body.Close()

			if resp.StatusCode == http.StatusNotFound {
				t.Errorf("%s %s: got 404, route not registered", rt.method, rt.path)
			}

			ct := resp.Header.Get("Content-Type")
			if ct != "application/json" {
				t.Errorf("%s %s: Content-Type = %q, want %q",
					rt.method, rt.path, ct, "application/json")
			}
		})
	}
}
