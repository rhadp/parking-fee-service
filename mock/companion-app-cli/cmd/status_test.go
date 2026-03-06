package cmd

import (
	"net/http"
	"net/http/httptest"
	"os"
	"testing"
)

// TS-09-E3: status with missing --vin should return an error.
func TestStatus_MissingFlags(t *testing.T) {
	err := runStatus(nil)
	if err == nil {
		t.Fatal("expected error when --vin is missing")
	}
}

// TS-09-P6: status calls the correct endpoint.
func TestStatus_CorrectEndpoint(t *testing.T) {
	var receivedMethod string
	var receivedPath string
	var receivedAuth string

	ts := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		receivedMethod = r.Method
		receivedPath = r.URL.Path
		receivedAuth = r.Header.Get("Authorization")
		w.Header().Set("Content-Type", "application/json")
		w.Write([]byte(`{"vin":"VIN12345","locked":true}`))
	}))
	defer ts.Close()

	os.Setenv("CLOUD_GATEWAY_URL", ts.URL)
	defer os.Unsetenv("CLOUD_GATEWAY_URL")
	os.Setenv("BEARER_TOKEN", "test-token")
	defer os.Unsetenv("BEARER_TOKEN")

	err := runStatus([]string{"--vin=VIN12345"})
	if err != nil {
		t.Fatalf("expected no error, got: %v", err)
	}

	if receivedMethod != "GET" {
		t.Fatalf("expected GET method, got %s", receivedMethod)
	}
	if receivedPath != "/vehicles/VIN12345/status" {
		t.Fatalf("expected path /vehicles/VIN12345/status, got %s", receivedPath)
	}
	if receivedAuth != "Bearer test-token" {
		t.Fatalf("expected Authorization 'Bearer test-token', got '%s'", receivedAuth)
	}
}

// TS-09-E5: Service unreachable returns a meaningful error.
func TestServiceUnreachable_REST(t *testing.T) {
	os.Setenv("CLOUD_GATEWAY_URL", "http://localhost:19999")
	defer os.Unsetenv("CLOUD_GATEWAY_URL")

	err := runStatus([]string{"--vin=VIN12345"})
	if err == nil {
		t.Fatal("expected error when service is unreachable")
	}
}
