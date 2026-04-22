package mockapps_test

import (
	"encoding/json"
	"io"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

// ---------------------------------------------------------------------------
// TS-09-11: Companion App CLI Lock
// Requirement: 09-REQ-7.1, 09-REQ-7.4, 09-REQ-7.5
// ---------------------------------------------------------------------------

func TestLockCommand(t *testing.T) {
	var receivedBody []byte
	var receivedAuth string
	var receivedPath string
	var receivedMethod string

	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		receivedAuth = r.Header.Get("Authorization")
		receivedPath = r.URL.Path
		receivedMethod = r.Method
		receivedBody, _ = io.ReadAll(r.Body)
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(map[string]string{
			"command_id": "cmd-1",
			"status":     "pending",
		})
	}))
	defer srv.Close()

	binary := buildBinary(t, "companion-app-cli")
	stdout, stderr, exitCode := runBinary(t, binary,
		"lock",
		"--vin=VIN001",
		"--token=test-token",
		"--gateway-addr="+srv.URL,
	)

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d\nstderr: %s", exitCode, stderr)
	}

	if !strings.Contains(stdout, "cmd-1") {
		t.Errorf("expected stdout to contain 'cmd-1', got: %s", stdout)
	}

	if receivedMethod != "POST" {
		t.Errorf("expected POST request, got: %s", receivedMethod)
	}

	if receivedPath != "/vehicles/VIN001/commands" {
		t.Errorf("expected path '/vehicles/VIN001/commands', got: %s", receivedPath)
	}

	if receivedAuth != "Bearer test-token" {
		t.Errorf("expected Authorization 'Bearer test-token', got: %s", receivedAuth)
	}

	// Verify request body contains lock command
	var body map[string]interface{}
	if err := json.Unmarshal(receivedBody, &body); err != nil {
		t.Errorf("failed to parse request body: %v", err)
	} else {
		if body["type"] != "lock" {
			t.Errorf("expected body type 'lock', got: %v", body["type"])
		}
	}
}

// ---------------------------------------------------------------------------
// TS-09-12: Companion App CLI Unlock
// Requirement: 09-REQ-7.2
// ---------------------------------------------------------------------------

func TestUnlockCommand(t *testing.T) {
	var receivedBody []byte

	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		receivedBody, _ = io.ReadAll(r.Body)
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(map[string]string{
			"command_id": "cmd-2",
			"status":     "pending",
		})
	}))
	defer srv.Close()

	binary := buildBinary(t, "companion-app-cli")
	stdout, stderr, exitCode := runBinary(t, binary,
		"unlock",
		"--vin=VIN001",
		"--token=test-token",
		"--gateway-addr="+srv.URL,
	)

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d\nstderr: %s", exitCode, stderr)
	}

	if !strings.Contains(stdout, "cmd-2") {
		t.Errorf("expected stdout to contain 'cmd-2', got: %s", stdout)
	}

	// Verify request body contains unlock command
	var body map[string]interface{}
	if err := json.Unmarshal(receivedBody, &body); err != nil {
		t.Errorf("failed to parse request body: %v", err)
	} else {
		if body["type"] != "unlock" {
			t.Errorf("expected body type 'unlock', got: %v", body["type"])
		}
	}
}

// ---------------------------------------------------------------------------
// TS-09-13: Companion App CLI Status
// Requirement: 09-REQ-7.3
// ---------------------------------------------------------------------------

func TestStatusCommand(t *testing.T) {
	var receivedPath string
	var receivedMethod string

	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		receivedPath = r.URL.Path
		receivedMethod = r.Method
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(map[string]string{
			"command_id": "cmd-1",
			"status":     "success",
		})
	}))
	defer srv.Close()

	binary := buildBinary(t, "companion-app-cli")
	stdout, stderr, exitCode := runBinary(t, binary,
		"status",
		"--vin=VIN001",
		"--command-id=cmd-1",
		"--token=test-token",
		"--gateway-addr="+srv.URL,
	)

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d\nstderr: %s", exitCode, stderr)
	}

	if !strings.Contains(stdout, "success") {
		t.Errorf("expected stdout to contain 'success', got: %s", stdout)
	}

	if receivedMethod != "GET" {
		t.Errorf("expected GET request, got: %s", receivedMethod)
	}

	if receivedPath != "/vehicles/VIN001/commands/cmd-1" {
		t.Errorf("expected path '/vehicles/VIN001/commands/cmd-1', got: %s", receivedPath)
	}
}

// ---------------------------------------------------------------------------
// TS-09-E5: Companion App Missing Token
// Requirement: 09-REQ-7.E2
// ---------------------------------------------------------------------------

func TestMissingToken(t *testing.T) {
	binary := buildBinary(t, "companion-app-cli")

	// Run without --token and without CLOUD_GATEWAY_TOKEN env var.
	// Filter out the env var if it happens to be set in the test environment.
	_, stderr, exitCode := runBinaryWithEnv(t, binary,
		[]string{"CLOUD_GATEWAY_TOKEN="},
		"lock", "--vin=VIN001",
	)

	if exitCode != 1 {
		t.Fatalf("expected exit code 1 when no token provided, got %d", exitCode)
	}

	if !strings.Contains(strings.ToLower(stderr), "token") {
		t.Errorf("expected stderr to mention 'token', got: %s", stderr)
	}
}

// ---------------------------------------------------------------------------
// TS-09-E6: Companion App Missing VIN
// Requirement: 09-REQ-7.E1
// ---------------------------------------------------------------------------

func TestMissingVIN(t *testing.T) {
	binary := buildBinary(t, "companion-app-cli")

	_, stderr, exitCode := runBinary(t, binary,
		"lock", "--token=test-token",
	)

	if exitCode != 1 {
		t.Fatalf("expected exit code 1 when --vin is missing, got %d", exitCode)
	}

	if len(stderr) == 0 {
		t.Error("expected error message on stderr when --vin is missing")
	}
}

// ---------------------------------------------------------------------------
// TS-09-P6: Bearer Token Enforcement (env var)
// Requirement: 09-REQ-7.4, 09-REQ-7.E2
// ---------------------------------------------------------------------------

func TestBearerTokenEnvVar(t *testing.T) {
	var receivedAuth string

	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		receivedAuth = r.Header.Get("Authorization")
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(map[string]string{"command_id": "x"})
	}))
	defer srv.Close()

	binary := buildBinary(t, "companion-app-cli")

	// Token provided via environment variable (no --token flag).
	_, stderr, exitCode := runBinaryWithEnv(t, binary,
		[]string{"CLOUD_GATEWAY_TOKEN=env-token-123"},
		"lock",
		"--vin=VIN001",
		"--gateway-addr="+srv.URL,
	)

	if exitCode != 0 {
		t.Fatalf("expected exit code 0 with env token, got %d\nstderr: %s", exitCode, stderr)
	}

	if receivedAuth != "Bearer env-token-123" {
		t.Errorf("expected Authorization 'Bearer env-token-123', got: %s", receivedAuth)
	}
}

// ---------------------------------------------------------------------------
// Companion App HTTP Error (09-REQ-7.E3)
// Addresses Skeptic finding: no test covers companion-app-cli non-2xx response.
// ---------------------------------------------------------------------------

func TestCompanionAppHTTPError(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusInternalServerError)
		w.Write([]byte("internal error"))
	}))
	defer srv.Close()

	binary := buildBinary(t, "companion-app-cli")
	_, stderr, exitCode := runBinary(t, binary,
		"lock",
		"--vin=VIN001",
		"--token=test-token",
		"--gateway-addr="+srv.URL,
	)

	if exitCode != 1 {
		t.Fatalf("expected exit code 1 on non-2xx response, got %d", exitCode)
	}

	if !strings.Contains(stderr, "500") {
		t.Errorf("expected stderr to contain '500', got: %s", stderr)
	}
}
