package cmd

import (
	"encoding/json"
	"io"
	"net/http"
	"net/http/httptest"
	"os"
	"testing"
)

// TS-09-E3: unlock with missing --vin should return an error.
func TestUnlock_MissingFlags(t *testing.T) {
	err := runUnlock(nil)
	if err == nil {
		t.Fatal("expected error when --vin is missing")
	}
}

// TS-09-P5: unlock sends correct command payload.
func TestUnlock_CorrectPayload(t *testing.T) {
	var receivedBody map[string]interface{}

	ts := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		body, _ := io.ReadAll(r.Body)
		json.Unmarshal(body, &receivedBody)
		w.Header().Set("Content-Type", "application/json")
		w.Write([]byte(`{"status":"accepted"}`))
	}))
	defer ts.Close()

	os.Setenv("CLOUD_GATEWAY_URL", ts.URL)
	defer os.Unsetenv("CLOUD_GATEWAY_URL")
	os.Setenv("BEARER_TOKEN", "test-token")
	defer os.Unsetenv("BEARER_TOKEN")

	err := runUnlock([]string{"--vin=VIN12345"})
	if err != nil {
		t.Fatalf("expected no error, got: %v", err)
	}

	cmdType, ok := receivedBody["type"].(string)
	if !ok || cmdType != "unlock" {
		t.Fatalf("expected type 'unlock', got %v", receivedBody["type"])
	}
}
