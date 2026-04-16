// Integration tests for companion-app-cli.
//
// RED PHASE: the stub binary always exits 0 without making HTTP calls, so
// every assertion about output content, auth headers, and error exit codes
// will FAIL until task group 4 implements the real CLI.
package integration

import (
	"os"
	"os/exec"
	"strings"
	"testing"
)

const companionPkg = "github.com/sdv-demo/mock/companion-app-cli"

// ── TS-09-11: companion-app-cli lock ──────────────────────────────────────

// TestLockCommand verifies that `companion-app-cli lock --vin=VIN001`
// sends POST /vehicles/VIN001/commands with body {type:"lock",doors:["driver"]}
// and includes an Authorization: Bearer <token> header.
func TestLockCommand(t *testing.T) {
	ms := startMockHTTPServer(t, 200, `{"command_id":"cmd-1","status":"pending"}`)
	binary := buildBinary(t, companionPkg)

	cmd := exec.Command(binary,
		"lock",
		"--vin=VIN001",
		"--token=test-token",
		"--gateway-addr="+ms.URL,
	)
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("expected exit 0, got: %v\noutput: %s", err, out)
	}
	if !strings.Contains(string(out), "cmd-1") {
		t.Fatalf("expected 'cmd-1' in stdout, got: %s", out)
	}

	req := ms.lastRequest()
	if req == nil {
		t.Fatal("no HTTP request captured by mock server")
	}
	if req.Method != "POST" {
		t.Fatalf("expected POST, got %s", req.Method)
	}
	if !strings.HasSuffix(req.URL, "/vehicles/VIN001/commands") {
		t.Fatalf("expected URL ending in /vehicles/VIN001/commands, got: %s", req.URL)
	}
	if auth := req.Header.Get("Authorization"); auth != "Bearer test-token" {
		t.Fatalf("expected Authorization: Bearer test-token, got: %q", auth)
	}
	if req.BodyMap["type"] != "lock" {
		t.Fatalf("expected body.type=lock, got: %v", req.BodyMap["type"])
	}
}

// ── TS-09-12: companion-app-cli unlock ────────────────────────────────────

// TestUnlockCommand verifies that `companion-app-cli unlock` sends a POST with
// body {type:"unlock", doors:["driver"]}.
func TestUnlockCommand(t *testing.T) {
	ms := startMockHTTPServer(t, 200, `{"command_id":"cmd-2","status":"pending"}`)
	binary := buildBinary(t, companionPkg)

	cmd := exec.Command(binary,
		"unlock",
		"--vin=VIN001",
		"--token=test-token",
		"--gateway-addr="+ms.URL,
	)
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("expected exit 0, got: %v\noutput: %s", err, out)
	}

	req := ms.lastRequest()
	if req == nil {
		t.Fatal("no HTTP request captured by mock server")
	}
	if req.BodyMap["type"] != "unlock" {
		t.Fatalf("expected body.type=unlock, got: %v", req.BodyMap["type"])
	}
}

// ── TS-09-13: companion-app-cli status ────────────────────────────────────

// TestStatusCommand verifies that `companion-app-cli status` sends
// GET /vehicles/{vin}/commands/{command_id}.
func TestStatusCommand(t *testing.T) {
	ms := startMockHTTPServer(t, 200, `{"command_id":"cmd-1","status":"success"}`)
	binary := buildBinary(t, companionPkg)

	cmd := exec.Command(binary,
		"status",
		"--vin=VIN001",
		"--command-id=cmd-1",
		"--token=test-token",
		"--gateway-addr="+ms.URL,
	)
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("expected exit 0, got: %v\noutput: %s", err, out)
	}
	if !strings.Contains(string(out), "success") {
		t.Fatalf("expected 'success' in stdout, got: %s", out)
	}

	req := ms.lastRequest()
	if req == nil {
		t.Fatal("no HTTP request captured by mock server")
	}
	if req.Method != "GET" {
		t.Fatalf("expected GET, got %s", req.Method)
	}
	if !strings.Contains(req.URL, "/vehicles/VIN001/commands/cmd-1") {
		t.Fatalf("expected URL containing /vehicles/VIN001/commands/cmd-1, got: %s", req.URL)
	}
}

// ── TS-09-E5: companion-app-cli missing token ─────────────────────────────

// TestMissingToken verifies that companion-app-cli exits 1 when no bearer
// token is provided via --token flag or CLOUD_GATEWAY_TOKEN env var.
func TestMissingToken(t *testing.T) {
	binary := buildBinary(t, companionPkg)

	cmd := exec.Command(binary, "lock", "--vin=VIN001")
	cmd.Env = filterEnv(os.Environ(), "CLOUD_GATEWAY_TOKEN")
	out, err := cmd.CombinedOutput()
	if err == nil {
		t.Fatalf("expected exit 1 for missing token, got 0\noutput: %s", out)
	}
	if !strings.Contains(strings.ToLower(string(out)), "token") {
		t.Fatalf("expected 'token' in error output, got: %s", out)
	}
}

// ── TS-09-E6: companion-app-cli missing VIN ───────────────────────────────

// TestMissingVIN verifies that companion-app-cli exits 1 when --vin is absent.
func TestMissingVIN(t *testing.T) {
	binary := buildBinary(t, companionPkg)

	cmd := exec.Command(binary, "lock", "--token=test-token")
	out, err := cmd.CombinedOutput()
	if err == nil {
		t.Fatalf("expected exit 1 for missing --vin, got 0\noutput: %s", out)
	}
	if len(out) == 0 {
		t.Fatal("expected error message on stderr for missing --vin")
	}
}

// ── TS-09-P6: bearer token enforcement ────────────────────────────────────

// TestBearerTokenEnforcement verifies that the Authorization header carries
// the value from CLOUD_GATEWAY_TOKEN env var when --token flag is absent.
func TestBearerTokenEnvVar(t *testing.T) {
	ms := startMockHTTPServer(t, 200, `{"command_id":"cmd-3","status":"pending"}`)
	binary := buildBinary(t, companionPkg)

	cmd := exec.Command(binary, "lock", "--vin=VIN001", "--gateway-addr="+ms.URL)
	cmd.Env = append(filterEnv(os.Environ(), "CLOUD_GATEWAY_TOKEN"),
		"CLOUD_GATEWAY_TOKEN=env-bearer-token",
	)
	if _, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("expected exit 0, got: %v", err)
	}

	req := ms.lastRequest()
	if req == nil {
		t.Fatal("no HTTP request captured")
	}
	if auth := req.Header.Get("Authorization"); auth != "Bearer env-bearer-token" {
		t.Fatalf("expected Authorization: Bearer env-bearer-token, got: %q", auth)
	}
}

// filterEnv returns a copy of env with any variables matching key removed.
func filterEnv(env []string, key string) []string {
	prefix := key + "="
	out := env[:0:len(env)]
	for _, e := range env {
		if !strings.HasPrefix(e, prefix) {
			out = append(out, e)
		}
	}
	return out
}
