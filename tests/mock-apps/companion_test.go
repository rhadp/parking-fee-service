package mockappstests

import (
	"os"
	"strings"
	"testing"
)

// companionBin builds and returns the companion-app-cli binary path.
// Cached per test run.
var companionBinCache string

func companionBin(t *testing.T) string {
	t.Helper()
	if companionBinCache == "" {
		companionBinCache = buildBinary(t, "companion-app-cli")
	}
	return companionBinCache
}

// TS-09-11: companion-app-cli lock sends POST /vehicles/{vin}/commands
// with {"type":"lock","doors":["driver"]} and Authorization: Bearer <token>.
func TestLockCommand(t *testing.T) {
	mock := newMockHTTPServer(t, 200, `{"command_id":"cmd-1","status":"pending"}`)
	binary := companionBin(t)

	stdout, stderr, code := runBinary(t, binary,
		"lock",
		"--vin=VIN001",
		"--token=test-token",
		"--gateway-addr="+mock.URL(),
	)

	if code != 0 {
		t.Errorf("expected exit 0, got %d (stderr=%q)", code, stderr)
	}

	// Verify auth header
	authHeader := mock.Header("Authorization")
	if authHeader != "Bearer test-token" {
		t.Errorf("expected Authorization=Bearer test-token, got %q", authHeader)
	}

	// Verify request path
	if !strings.Contains(mock.lastPath, "/vehicles/VIN001/commands") {
		t.Errorf("expected path /vehicles/VIN001/commands, got %q", mock.lastPath)
	}

	// Verify request body
	body := mock.decodeLastBody(t)
	if typ, _ := body["type"].(string); typ != "lock" {
		t.Errorf("expected type=lock, got %q", typ)
	}

	// Verify response printed to stdout
	if !strings.Contains(stdout, "cmd-1") {
		t.Errorf("expected cmd-1 in stdout, got %q", stdout)
	}
}

// TS-09-12: companion-app-cli unlock sends POST /vehicles/{vin}/commands
// with {"type":"unlock","doors":["driver"]}.
func TestUnlockCommand(t *testing.T) {
	mock := newMockHTTPServer(t, 200, `{"command_id":"cmd-2","status":"pending"}`)
	binary := companionBin(t)

	stdout, stderr, code := runBinary(t, binary,
		"unlock",
		"--vin=VIN001",
		"--token=test-token",
		"--gateway-addr="+mock.URL(),
	)

	if code != 0 {
		t.Errorf("expected exit 0, got %d (stderr=%q)", code, stderr)
	}

	// Verify request body type=unlock
	if len(mock.lastBody) == 0 {
		t.Error("expected request body, got empty")
	} else {
		body := mock.decodeLastBody(t)
		if typ, _ := body["type"].(string); typ != "unlock" {
			t.Errorf("expected type=unlock, got %q", typ)
		}
	}

	if !strings.Contains(stdout, "cmd-2") {
		t.Errorf("expected cmd-2 in stdout, got %q", stdout)
	}
}

// TS-09-13: companion-app-cli status sends GET /vehicles/{vin}/commands/{command_id}.
func TestStatusCommand(t *testing.T) {
	mock := newMockHTTPServer(t, 200, `{"command_id":"cmd-1","status":"success"}`)
	binary := companionBin(t)

	stdout, stderr, code := runBinary(t, binary,
		"status",
		"--vin=VIN001",
		"--command-id=cmd-1",
		"--token=test-token",
		"--gateway-addr="+mock.URL(),
	)

	if code != 0 {
		t.Errorf("expected exit 0, got %d (stderr=%q)", code, stderr)
	}

	if !strings.Contains(mock.lastPath, "/vehicles/VIN001/commands/cmd-1") {
		t.Errorf("expected path /vehicles/VIN001/commands/cmd-1, got %q", mock.lastPath)
	}

	if !strings.Contains(stdout, "success") {
		t.Errorf("expected success in stdout, got %q", stdout)
	}
}

// TS-09-E5: companion-app-cli exits 1 when no bearer token is provided.
func TestMissingToken(t *testing.T) {
	binary := companionBin(t)
	// Ensure env var is not set
	t.Setenv("CLOUD_GATEWAY_TOKEN", "")

	_, stderr, code := runBinary(t, binary,
		"lock",
		"--vin=VIN001",
		// no --token flag, CLOUD_GATEWAY_TOKEN not set
	)

	if code != 1 {
		t.Errorf("expected exit 1 when token is missing, got %d", code)
	}
	if !strings.Contains(strings.ToLower(stderr), "token") {
		t.Errorf("expected token-related error in stderr, got %q", stderr)
	}
}

// TS-09-E6: companion-app-cli exits 1 when --vin is missing.
func TestMissingVIN(t *testing.T) {
	binary := companionBin(t)

	_, stderr, code := runBinary(t, binary,
		"lock",
		"--token=test-token",
		// no --vin flag
	)

	if code != 1 {
		t.Errorf("expected exit 1 when --vin is missing, got %d", code)
	}
	if len(stderr) == 0 {
		t.Error("expected error message in stderr")
	}
}

// TS-09-E5 / 09-REQ-7.E3: companion-app-cli exits 1 when CLOUD_GATEWAY returns non-2xx.
func TestCompanionHTTPError(t *testing.T) {
	mock := newMockHTTPServer(t, 500, `{"error":"internal server error"}`)
	binary := companionBin(t)

	_, stderr, code := runBinary(t, binary,
		"lock",
		"--vin=VIN001",
		"--token=test-token",
		"--gateway-addr="+mock.URL(),
	)

	if code != 1 {
		t.Errorf("expected exit 1 on non-2xx HTTP response, got %d", code)
	}
	if !strings.Contains(stderr, "500") {
		t.Errorf("expected HTTP 500 status in stderr, got %q", stderr)
	}
}

// TS-09-P6: Bearer token enforcement property.
// Companion-app-cli includes token from env var in Authorization header.
func TestBearerTokenEnvVar(t *testing.T) {
	mock := newMockHTTPServer(t, 200, `{"command_id":"cmd-env","status":"pending"}`)
	binary := companionBin(t)

	// Use token from environment variable
	os.Setenv("CLOUD_GATEWAY_TOKEN", "env-token-123")
	defer os.Unsetenv("CLOUD_GATEWAY_TOKEN")

	_, _, code := runBinary(t, binary,
		"lock",
		"--vin=VIN001",
		"--gateway-addr="+mock.URL(),
		// no --token flag; should use env var
	)

	if code != 0 {
		t.Errorf("expected exit 0 when token from env var, got %d", code)
	}

	authHeader := mock.Header("Authorization")
	if authHeader != "Bearer env-token-123" {
		t.Errorf("expected Authorization=Bearer env-token-123, got %q", authHeader)
	}
}
