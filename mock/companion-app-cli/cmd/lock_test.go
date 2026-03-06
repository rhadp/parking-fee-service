package cmd

import (
	"encoding/json"
	"io"
	"net/http"
	"net/http/httptest"
	"os"
	"testing"
)

// TS-09-E3: lock with missing --vin should return an error.
func TestLock_MissingFlags(t *testing.T) {
	err := runLock(nil)
	if err == nil {
		t.Fatal("expected error when --vin is missing")
	}
}

// TS-09-P4: lock sends correct command payload.
func TestLock_CorrectPayload(t *testing.T) {
	var receivedMethod string
	var receivedPath string
	var receivedBody map[string]interface{}
	var receivedAuth string

	ts := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		receivedMethod = r.Method
		receivedPath = r.URL.Path
		receivedAuth = r.Header.Get("Authorization")
		body, _ := io.ReadAll(r.Body)
		json.Unmarshal(body, &receivedBody)
		w.Header().Set("Content-Type", "application/json")
		w.Write([]byte(`{"status":"accepted"}`))
	}))
	defer ts.Close()

	os.Setenv("CLOUD_GATEWAY_URL", ts.URL)
	defer os.Unsetenv("CLOUD_GATEWAY_URL")
	os.Setenv("BEARER_TOKEN", "test-token-123")
	defer os.Unsetenv("BEARER_TOKEN")

	err := runLock([]string{"--vin=VIN12345"})
	if err != nil {
		t.Fatalf("expected no error, got: %v", err)
	}

	if receivedMethod != "POST" {
		t.Fatalf("expected POST method, got %s", receivedMethod)
	}
	if receivedPath != "/vehicles/VIN12345/commands" {
		t.Fatalf("expected path /vehicles/VIN12345/commands, got %s", receivedPath)
	}
	if receivedAuth != "Bearer test-token-123" {
		t.Fatalf("expected Authorization 'Bearer test-token-123', got '%s'", receivedAuth)
	}

	cmdType, ok := receivedBody["type"].(string)
	if !ok || cmdType != "lock" {
		t.Fatalf("expected type 'lock', got %v", receivedBody["type"])
	}

	cmdID, ok := receivedBody["command_id"].(string)
	if !ok || cmdID == "" {
		t.Fatal("expected non-empty command_id UUID")
	}
}

// TS-09-P4: Bearer token is included in requests.
func TestBearerToken_IncludedInRequests(t *testing.T) {
	var receivedAuth string
	ts := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		receivedAuth = r.Header.Get("Authorization")
		w.Header().Set("Content-Type", "application/json")
		w.Write([]byte(`{"status":"accepted"}`))
	}))
	defer ts.Close()

	os.Setenv("CLOUD_GATEWAY_URL", ts.URL)
	defer os.Unsetenv("CLOUD_GATEWAY_URL")
	os.Setenv("BEARER_TOKEN", "my-secret-token")
	defer os.Unsetenv("BEARER_TOKEN")

	err := runLock([]string{"--vin=VIN99999"})
	if err != nil {
		t.Fatalf("expected no error, got: %v", err)
	}

	if receivedAuth != "Bearer my-secret-token" {
		t.Fatalf("expected Authorization 'Bearer my-secret-token', got '%s'", receivedAuth)
	}
}
