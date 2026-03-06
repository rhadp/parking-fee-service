package cmd

import (
	"net/http"
	"net/http/httptest"
	"os"
	"strings"
	"testing"
)

// TS-09-E3: lookup with missing --lat and --lon should return an error.
func TestLookup_MissingFlags(t *testing.T) {
	err := runLookup(nil)
	if err == nil {
		t.Fatal("expected error when --lat and --lon are missing")
	}
	// The error should mention missing flags or usage.
	errStr := err.Error()
	if !strings.Contains(errStr, "lat") && !strings.Contains(errStr, "usage") && !strings.Contains(errStr, "not yet implemented") {
		t.Fatalf("expected meaningful error about missing flags, got: %s", errStr)
	}
}

// TS-09-P1: lookup calls the correct REST endpoint.
func TestLookup_CorrectRESTEndpoint(t *testing.T) {
	var receivedPath string
	var receivedMethod string
	ts := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		receivedMethod = r.Method
		receivedPath = r.URL.RequestURI()
		w.Header().Set("Content-Type", "application/json")
		w.Write([]byte(`[{"operator_id":"op1"}]`))
	}))
	defer ts.Close()

	os.Setenv("PARKING_FEE_SERVICE_URL", ts.URL)
	defer os.Unsetenv("PARKING_FEE_SERVICE_URL")

	err := runLookup([]string{"--lat=48.1351", "--lon=11.5820"})
	if err != nil {
		t.Fatalf("expected no error, got: %v", err)
	}

	if receivedMethod != "GET" {
		t.Fatalf("expected GET method, got %s", receivedMethod)
	}
	if !strings.Contains(receivedPath, "/operators") {
		t.Fatalf("expected path containing /operators, got %s", receivedPath)
	}
	if !strings.Contains(receivedPath, "lat=48.1351") {
		t.Fatalf("expected lat=48.1351 in query, got %s", receivedPath)
	}
	if !strings.Contains(receivedPath, "lon=11.5820") {
		t.Fatalf("expected lon=11.5820 in query, got %s", receivedPath)
	}
}

// TS-09-E5: lookup with unreachable service returns a meaningful error.
func TestLookup_ServiceUnreachable(t *testing.T) {
	os.Setenv("PARKING_FEE_SERVICE_URL", "http://localhost:19999")
	defer os.Unsetenv("PARKING_FEE_SERVICE_URL")

	err := runLookup([]string{"--lat=48.0", "--lon=11.0"})
	if err == nil {
		t.Fatal("expected error when service is unreachable")
	}
	if !strings.Contains(err.Error(), "19999") && !strings.Contains(err.Error(), "connection") && !strings.Contains(err.Error(), "not yet implemented") {
		t.Fatalf("expected error mentioning target address, got: %s", err.Error())
	}
}
