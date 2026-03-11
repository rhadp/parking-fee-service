package cmd

import (
	"encoding/json"
	"io"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

// TS-09-E3: lock with missing --vin produces error.
func TestLock_MissingFlags(t *testing.T) {
	err := RunLock([]string{}, "http://localhost:8081", "")
	if err == nil {
		t.Fatal("expected error when --vin is missing")
	}
	errMsg := err.Error()
	if !strings.Contains(errMsg, "vin") && !strings.Contains(errMsg, "required") {
		t.Errorf("error should mention missing flag, got: %v", err)
	}
}

// TS-09-P4: lock sends correct payload with bearer token.
func TestLock_CorrectPayload(t *testing.T) {
	var receivedMethod string
	var receivedPath string
	var receivedAuth string
	var receivedContentType string
	var receivedBody map[string]interface{}

	ts := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		receivedMethod = r.Method
		receivedPath = r.URL.Path
		receivedAuth = r.Header.Get("Authorization")
		receivedContentType = r.Header.Get("Content-Type")
		body, _ := io.ReadAll(r.Body)
		json.Unmarshal(body, &receivedBody)
		w.Header().Set("Content-Type", "application/json")
		w.Write([]byte(`{"status": "accepted"}`))
	}))
	defer ts.Close()

	err := RunLock([]string{"--vin=VIN12345"}, ts.URL, "test-token-123")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if receivedMethod != http.MethodPost {
		t.Errorf("expected POST, got %s", receivedMethod)
	}

	if receivedPath != "/vehicles/VIN12345/commands" {
		t.Errorf("expected path /vehicles/VIN12345/commands, got %q", receivedPath)
	}

	if receivedAuth != "Bearer test-token-123" {
		t.Errorf("expected Authorization 'Bearer test-token-123', got %q", receivedAuth)
	}

	if receivedContentType != "application/json" {
		t.Errorf("expected Content-Type application/json, got %q", receivedContentType)
	}

	if receivedBody == nil {
		t.Fatal("expected non-nil body")
	}

	cmdType, ok := receivedBody["type"].(string)
	if !ok || cmdType != "lock" {
		t.Errorf("expected type 'lock', got %v", receivedBody["type"])
	}

	cmdID, ok := receivedBody["command_id"].(string)
	if !ok || cmdID == "" {
		t.Error("expected non-empty command_id (UUID)")
	}
}

// TS-09-E5: lock with unreachable service produces error.
func TestLock_ServiceUnreachable(t *testing.T) {
	err := RunLock([]string{"--vin=VIN12345"}, "http://localhost:19999", "token")
	if err == nil {
		t.Fatal("expected error when service is unreachable")
	}
}
