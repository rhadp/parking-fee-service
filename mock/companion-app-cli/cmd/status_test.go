package cmd

import (
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

// TS-09-E3: status with missing --vin produces error.
func TestStatus_MissingFlags(t *testing.T) {
	err := RunStatus([]string{}, "http://localhost:8081", "")
	if err == nil {
		t.Fatal("expected error when --vin is missing")
	}
	errMsg := err.Error()
	if !strings.Contains(errMsg, "vin") && !strings.Contains(errMsg, "required") {
		t.Errorf("error should mention missing flag, got: %v", err)
	}
}

// TS-09-P6: status calls correct endpoint with bearer token.
func TestStatus_CorrectEndpoint(t *testing.T) {
	var receivedMethod string
	var receivedPath string
	var receivedAuth string

	ts := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		receivedMethod = r.Method
		receivedPath = r.URL.Path
		receivedAuth = r.Header.Get("Authorization")
		w.Header().Set("Content-Type", "application/json")
		w.Write([]byte(`{"vin": "VIN12345", "locked": true}`))
	}))
	defer ts.Close()

	err := RunStatus([]string{"--vin=VIN12345"}, ts.URL, "test-token-123")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if receivedMethod != http.MethodGet {
		t.Errorf("expected GET, got %s", receivedMethod)
	}

	if receivedPath != "/vehicles/VIN12345/status" {
		t.Errorf("expected path /vehicles/VIN12345/status, got %q", receivedPath)
	}

	if receivedAuth != "Bearer test-token-123" {
		t.Errorf("expected Authorization 'Bearer test-token-123', got %q", receivedAuth)
	}
}
