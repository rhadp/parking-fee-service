package cmd

import (
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

// TS-09-E3: lookup with missing --lat and --lon produces error.
func TestLookup_MissingFlags(t *testing.T) {
	err := RunLookup([]string{}, "http://localhost:8080")
	if err == nil {
		t.Fatal("expected error when --lat and --lon are missing")
	}
	errMsg := err.Error()
	if !strings.Contains(errMsg, "lat") && !strings.Contains(errMsg, "required") {
		t.Errorf("error should mention missing flag, got: %v", err)
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
		w.Write([]byte(`[{"operator_id": "op1"}]`))
	}))
	defer ts.Close()

	err := RunLookup([]string{"--lat=48.1351", "--lon=11.5820"}, ts.URL)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if receivedMethod != http.MethodGet {
		t.Errorf("expected GET, got %s", receivedMethod)
	}

	expectedPath := "/operators?lat=48.1351&lon=11.5820"
	if receivedPath != expectedPath {
		t.Errorf("expected path %q, got %q", expectedPath, receivedPath)
	}
}

// TS-09-E5: lookup with unreachable service produces error with URL.
func TestLookup_ServiceUnreachable(t *testing.T) {
	err := RunLookup([]string{"--lat=48.0", "--lon=11.0"}, "http://localhost:19999")
	if err == nil {
		t.Fatal("expected error when service is unreachable")
	}
	if !strings.Contains(err.Error(), "19999") && !strings.Contains(err.Error(), "localhost") {
		t.Errorf("error should include target address, got: %v", err)
	}
}
