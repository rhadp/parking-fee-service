package main

import (
	"io"
	"net/http"
	"net/http/httptest"
	"testing"
)

func TestHealthzReturns200(t *testing.T) {
	srv := httptest.NewServer(newServeMux())
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

// stubRoutes lists all stub endpoints that must return HTTP 501.
var stubRoutes = []struct {
	method string
	path   string
}{
	{"POST", "/api/v1/vehicles/WVWZZZ1KZAW123456/lock"},
	{"POST", "/api/v1/vehicles/WVWZZZ1KZAW123456/unlock"},
	{"GET", "/api/v1/vehicles/WVWZZZ1KZAW123456/status"},
}

func TestStubRoutesReturn501(t *testing.T) {
	srv := httptest.NewServer(newServeMux())
	defer srv.Close()

	for _, rt := range stubRoutes {
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

			if resp.StatusCode != http.StatusNotImplemented {
				body, _ := io.ReadAll(resp.Body)
				t.Errorf("%s %s: got status %d, want %d; body: %s",
					rt.method, rt.path, resp.StatusCode, http.StatusNotImplemented, body)
			}

			ct := resp.Header.Get("Content-Type")
			if ct != "application/json" {
				t.Errorf("%s %s: Content-Type = %q, want %q",
					rt.method, rt.path, ct, "application/json")
			}
		})
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
