package mock_apps

import (
	"encoding/json"
	"net/http"
	"strings"
	"testing"
)

// TS-09-11: companion-app-cli lock sends POST /vehicles/{vin}/commands with lock body and auth header.
func TestLockCommand(t *testing.T) {
	binary := findBinary(t, "companion-app-cli")

	serverURL, cap := startMockHTTPServerWithCapture(t,
		map[string]string{"command_id": "cmd-1", "status": "pending"}, 200)

	stdout, _, exitCode := runCmd(t, binary,
		[]string{"lock", "--vin=VIN001", "--token=test-token", "--gateway-addr=" + serverURL},
		nil,
	)

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d", exitCode)
	}
	if cap.Header.Get("Authorization") != "Bearer test-token" {
		t.Errorf("expected Authorization: Bearer test-token, got %q", cap.Header.Get("Authorization"))
	}
	var body map[string]any
	if err := json.Unmarshal(cap.Body, &body); err != nil {
		t.Fatalf("failed to parse request body: %v", err)
	}
	if body["type"] != "lock" {
		t.Errorf("expected request body type='lock', got %q", body["type"])
	}
	if !strings.Contains(stdout, "cmd-1") {
		t.Errorf("expected 'cmd-1' in stdout, got %q", stdout)
	}
}

// TS-09-12: companion-app-cli unlock sends POST with unlock body.
func TestUnlockCommand(t *testing.T) {
	binary := findBinary(t, "companion-app-cli")

	serverURL, cap := startMockHTTPServerWithCapture(t,
		map[string]string{"command_id": "cmd-2", "status": "pending"}, 200)

	_, _, exitCode := runCmd(t, binary,
		[]string{"unlock", "--vin=VIN001", "--token=test-token", "--gateway-addr=" + serverURL},
		nil,
	)

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d", exitCode)
	}
	var body map[string]any
	if err := json.Unmarshal(cap.Body, &body); err != nil {
		t.Fatalf("failed to parse request body: %v", err)
	}
	if body["type"] != "unlock" {
		t.Errorf("expected request body type='unlock', got %q", body["type"])
	}
}

// TS-09-13: companion-app-cli status sends GET /vehicles/{vin}/commands/{command_id}.
func TestStatusCommand(t *testing.T) {
	binary := findBinary(t, "companion-app-cli")

	serverURL, cap := startMockHTTPServerWithCapture(t,
		map[string]string{"command_id": "cmd-1", "status": "success"}, 200)

	stdout, _, exitCode := runCmd(t, binary,
		[]string{"status", "--vin=VIN001", "--command-id=cmd-1", "--token=test-token", "--gateway-addr=" + serverURL},
		nil,
	)

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d", exitCode)
	}
	if cap.Method != "GET" {
		t.Errorf("expected GET request, got %q", cap.Method)
	}
	if !strings.Contains(stdout, "success") {
		t.Errorf("expected 'success' in stdout, got %q", stdout)
	}
}

// TS-09-E5: companion-app-cli exits 1 when no bearer token is provided.
func TestMissingToken(t *testing.T) {
	binary := findBinary(t, "companion-app-cli")

	_, stderr, exitCode := runCmd(t, binary,
		[]string{"lock", "--vin=VIN001"},
		map[string]string{"CLOUD_GATEWAY_TOKEN": ""}, // unset env var
	)

	if exitCode != 1 {
		t.Errorf("expected exit code 1 when token is missing, got %d", exitCode)
	}
	if !strings.Contains(strings.ToLower(stderr), "token") {
		t.Errorf("expected 'token' mentioned in stderr, got %q", stderr)
	}
}

// TS-09-E6: companion-app-cli exits 1 when --vin is missing.
func TestMissingVIN(t *testing.T) {
	binary := findBinary(t, "companion-app-cli")

	_, stderr, exitCode := runCmd(t, binary,
		[]string{"lock", "--token=test-token"},
		nil,
	)

	if exitCode != 1 {
		t.Errorf("expected exit code 1 when --vin is missing, got %d", exitCode)
	}
	if !strings.Contains(strings.ToLower(stderr), "vin") {
		t.Errorf("expected 'vin' mentioned in stderr, got %q", stderr)
	}
}

// TS-09-P6: bearer token enforcement: token from env var is included in Authorization header.
func TestBearerTokenEnvVar(t *testing.T) {
	binary := findBinary(t, "companion-app-cli")

	serverURL, cap := startMockHTTPServerWithCapture(t,
		map[string]string{"command_id": "cmd-x", "status": "pending"}, 200)

	_, _, exitCode := runCmd(t, binary,
		[]string{"lock", "--vin=VIN001", "--gateway-addr=" + serverURL},
		map[string]string{"CLOUD_GATEWAY_TOKEN": "env-token"},
	)

	if exitCode != 0 {
		t.Fatalf("expected exit code 0 when token from env, got %d", exitCode)
	}
	if cap.Header.Get("Authorization") != "Bearer env-token" {
		t.Errorf("expected Authorization: Bearer env-token, got %q", cap.Header.Get("Authorization"))
	}
}

// TS-09-E (HTTP error): companion-app-cli exits 1 on non-2xx response from CLOUD_GATEWAY.
func TestHTTPError(t *testing.T) {
	binary := findBinary(t, "companion-app-cli")

	serverURL := startMockHTTPServer(t, http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		http.Error(w, "internal error", http.StatusInternalServerError)
	}))

	_, stderr, exitCode := runCmd(t, binary,
		[]string{"lock", "--vin=VIN001", "--token=test-token", "--gateway-addr=" + serverURL},
		nil,
	)

	if exitCode != 1 {
		t.Errorf("expected exit code 1 on HTTP 500, got %d", exitCode)
	}
	if !strings.Contains(stderr, "500") {
		t.Errorf("expected '500' in stderr, got %q", stderr)
	}
}
