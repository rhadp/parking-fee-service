package cmd

import (
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

// TS-09-E3: adapter-info with missing --operator-id produces error.
func TestAdapterInfo_MissingFlags(t *testing.T) {
	err := RunAdapterInfo([]string{}, "http://localhost:8080")
	if err == nil {
		t.Fatal("expected error when --operator-id is missing")
	}
	errMsg := err.Error()
	if !strings.Contains(errMsg, "operator") && !strings.Contains(errMsg, "required") {
		t.Errorf("error should mention missing flag, got: %v", err)
	}
}

// TS-09-P2: adapter-info calls the correct REST endpoint.
func TestAdapterInfo_CorrectRESTEndpoint(t *testing.T) {
	var receivedPath string
	var receivedMethod string
	ts := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		receivedMethod = r.Method
		receivedPath = r.URL.Path
		w.Header().Set("Content-Type", "application/json")
		w.Write([]byte(`{"image_ref": "registry/adapter:v1", "checksum_sha256": "abc123", "version": "1.0"}`))
	}))
	defer ts.Close()

	err := RunAdapterInfo([]string{"--operator-id=op1"}, ts.URL)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if receivedMethod != http.MethodGet {
		t.Errorf("expected GET, got %s", receivedMethod)
	}

	expectedPath := "/operators/op1/adapter"
	if receivedPath != expectedPath {
		t.Errorf("expected path %q, got %q", expectedPath, receivedPath)
	}
}
