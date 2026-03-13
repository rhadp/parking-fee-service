package cmd

import (
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

// TS-09-E6: status with missing --vin produces error.
func TestStatus_MissingVIN(t *testing.T) {
	err := RunStatus([]string{"--command-id=cmd-123"}, "http://localhost:8081", "token")
	if err == nil {
		t.Fatal("expected error when --vin is missing")
	}
	errMsg := err.Error()
	if !strings.Contains(errMsg, "vin") && !strings.Contains(errMsg, "required") {
		t.Errorf("error should mention missing flag, got: %v", err)
	}
}

// TS-09-E6: status with missing --command-id produces error.
func TestStatus_MissingCommandID(t *testing.T) {
	err := RunStatus([]string{"--vin=VIN001"}, "http://localhost:8081", "token")
	if err == nil {
		t.Fatal("expected error when --command-id is missing")
	}
	errMsg := err.Error()
	if !strings.Contains(errMsg, "command-id") && !strings.Contains(errMsg, "required") {
		t.Errorf("error should mention missing flag, got: %v", err)
	}
}

// TS-09-11: status calls GET /vehicles/{vin}/commands/{command_id} with bearer token.
func TestStatus_CorrectEndpoint(t *testing.T) {
	var receivedMethod string
	var receivedPath string
	var receivedAuth string

	ts := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		receivedMethod = r.Method
		receivedPath = r.URL.Path
		receivedAuth = r.Header.Get("Authorization")
		w.Header().Set("Content-Type", "application/json")
		w.Write([]byte(`{"command_id": "cmd-123", "status": "completed"}`))
	}))
	defer ts.Close()

	err := RunStatus([]string{"--vin=VIN001", "--command-id=cmd-123"}, ts.URL, "test-token-123")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if receivedMethod != http.MethodGet {
		t.Errorf("expected GET, got %s", receivedMethod)
	}

	if receivedPath != "/vehicles/VIN001/commands/cmd-123" {
		t.Errorf("expected path /vehicles/VIN001/commands/cmd-123, got %q", receivedPath)
	}

	if receivedAuth != "Bearer test-token-123" {
		t.Errorf("expected Authorization 'Bearer test-token-123', got %q", receivedAuth)
	}
}
