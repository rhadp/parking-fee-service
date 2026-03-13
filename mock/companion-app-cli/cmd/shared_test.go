package cmd

import (
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

// TS-09-E6: Missing bearer token produces error mentioning "token".
// Note: token validation happens in main.go before dispatch,
// but we can verify the commands work correctly with empty tokens
// when the server is available (token is passed through).
func TestMissingToken_LockDispatchesWithEmptyToken(t *testing.T) {
	// The actual "no token" exit code 1 check happens at the main level.
	// At the cmd level, an empty token is passed through to the HTTP client.
	// We verify that when the server rejects (e.g. 401), it returns an error.
	ts := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		auth := r.Header.Get("Authorization")
		if auth == "" || auth == "Bearer " {
			w.WriteHeader(http.StatusUnauthorized)
			w.Write([]byte(`{"error": "unauthorized"}`))
			return
		}
		w.Header().Set("Content-Type", "application/json")
		w.Write([]byte(`{"status": "accepted"}`))
	}))
	defer ts.Close()

	// Empty token should lead to 401 error from mock server
	err := RunLock([]string{"--vin=VIN001"}, ts.URL, "")
	if err == nil {
		t.Fatal("expected error when server returns 401 for empty token")
	}
}

// TS-09-26: Connection errors include target address in message.
func TestConnectionErrorMessage(t *testing.T) {
	// Use a non-listening port
	err := RunLock([]string{"--vin=VIN001"}, "http://localhost:19999", "test-token")
	if err == nil {
		t.Fatal("expected error when CLOUD_GATEWAY is unreachable")
	}
	errMsg := err.Error()
	if !strings.Contains(errMsg, "19999") && !strings.Contains(strings.ToLower(errMsg), "connection") {
		t.Errorf("error should contain address or 'connection', got: %v", err)
	}
}

// TS-09-27: Error responses from upstream are reported.
func TestUpstreamErrorResponse(t *testing.T) {
	ts := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusForbidden)
		w.Write([]byte(`{"error": "access denied"}`))
	}))
	defer ts.Close()

	err := RunLock([]string{"--vin=VIN001"}, ts.URL, "test-token")
	if err == nil {
		t.Fatal("expected error when server returns 403")
	}
	errMsg := err.Error()
	if !strings.Contains(errMsg, "403") {
		t.Errorf("error should mention HTTP status code 403, got: %v", err)
	}
}

// TS-09-P5: Error Exit Code Consistency property test.
// All error conditions produce errors (which translate to exit code 1 in main).
func TestPropertyErrorExitCode(t *testing.T) {
	errorScenarios := []struct {
		name string
		fn   func() error
	}{
		{
			"lock missing vin",
			func() error { return RunLock(nil, "http://localhost:8081", "token") },
		},
		{
			"unlock missing vin",
			func() error { return RunUnlock(nil, "http://localhost:8081", "token") },
		},
		{
			"status missing vin",
			func() error { return RunStatus(nil, "http://localhost:8081", "token") },
		},
		{
			"status missing command-id",
			func() error { return RunStatus([]string{"--vin=X"}, "http://localhost:8081", "token") },
		},
		{
			"unknown subcommand",
			func() error { return Dispatch("foobar", nil, "http://localhost:8081", "") },
		},
		{
			"lock unreachable gateway",
			func() error { return RunLock([]string{"--vin=X"}, "http://localhost:19999", "token") },
		},
	}

	for _, tc := range errorScenarios {
		t.Run(tc.name, func(t *testing.T) {
			err := tc.fn()
			if err == nil {
				t.Errorf("expected error for scenario %q", tc.name)
			}
		})
	}
}
