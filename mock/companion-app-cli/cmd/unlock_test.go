package cmd

import (
	"encoding/json"
	"io"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

// TS-09-E3: unlock with missing --vin produces error.
func TestUnlock_MissingFlags(t *testing.T) {
	err := RunUnlock([]string{}, "http://localhost:8081", "")
	if err == nil {
		t.Fatal("expected error when --vin is missing")
	}
	errMsg := err.Error()
	if !strings.Contains(errMsg, "vin") && !strings.Contains(errMsg, "required") {
		t.Errorf("error should mention missing flag, got: %v", err)
	}
}

// TS-09-P5: unlock sends correct payload.
func TestUnlock_CorrectPayload(t *testing.T) {
	var receivedBody map[string]interface{}

	ts := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		body, _ := io.ReadAll(r.Body)
		json.Unmarshal(body, &receivedBody)
		w.Header().Set("Content-Type", "application/json")
		w.Write([]byte(`{"status": "accepted"}`))
	}))
	defer ts.Close()

	err := RunUnlock([]string{"--vin=VIN12345"}, ts.URL, "test-token-123")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if receivedBody == nil {
		t.Fatal("expected non-nil body")
	}

	cmdType, ok := receivedBody["type"].(string)
	if !ok || cmdType != "unlock" {
		t.Errorf("expected type 'unlock', got %v", receivedBody["type"])
	}

	cmdID, ok := receivedBody["command_id"].(string)
	if !ok || cmdID == "" {
		t.Error("expected non-empty command_id (UUID)")
	}
}
