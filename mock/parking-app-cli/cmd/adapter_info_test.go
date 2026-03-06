package cmd

import (
	"net/http"
	"net/http/httptest"
	"os"
	"testing"
)

// TS-09-E3: adapter-info with missing --operator-id should return an error.
func TestAdapterInfo_MissingFlags(t *testing.T) {
	err := runAdapterInfo(nil)
	if err == nil {
		t.Fatal("expected error when --operator-id is missing")
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
		w.Write([]byte(`{"image_ref":"registry/adapter:v1","checksum_sha256":"abc123","version":"1.0"}`))
	}))
	defer ts.Close()

	os.Setenv("PARKING_FEE_SERVICE_URL", ts.URL)
	defer os.Unsetenv("PARKING_FEE_SERVICE_URL")

	err := runAdapterInfo([]string{"--operator-id=op1"})
	if err != nil {
		t.Fatalf("expected no error, got: %v", err)
	}

	if receivedMethod != "GET" {
		t.Fatalf("expected GET method, got %s", receivedMethod)
	}
	if receivedPath != "/operators/op1/adapter" {
		t.Fatalf("expected path /operators/op1/adapter, got %s", receivedPath)
	}
}
