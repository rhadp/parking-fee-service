package main

import (
	"bytes"
	"encoding/json"
	"io"
	"net/http"
	"net/http/httptest"
	"os"
	"strings"
	"testing"
)

// capturedRequest holds the details of an HTTP request captured by a test handler.
type capturedRequest struct {
	Method  string
	Path    string
	Headers http.Header
	Body    []byte
}

// newCapturingServer starts an httptest.Server that captures the request and
// returns a fixed 200 JSON response.
func newCapturingServer(t *testing.T, capture *capturedRequest) *httptest.Server {
	t.Helper()
	return httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		capture.Method = r.Method
		capture.Path = r.URL.Path
		capture.Headers = r.Header.Clone()
		capture.Body, _ = io.ReadAll(r.Body)
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		_, _ = w.Write([]byte(`{"status":"ok"}`))
	}))
}

// newErrorServer starts an httptest.Server that always returns the given HTTP status.
func newErrorServer(t *testing.T, statusCode int) *httptest.Server {
	t.Helper()
	return httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(statusCode)
		_, _ = w.Write([]byte(`{"error":"forbidden"}`))
	}))
}

// runCLI calls run() with the given args (no env side effects).
func runCLI(args []string) (stdout string, stderr string, code int) {
	var stdoutBuf, stderrBuf bytes.Buffer
	code = run(args, &stdoutBuf, &stderrBuf)
	return stdoutBuf.String(), stderrBuf.String(), code
}

// TS-09-22: COMPANION_APP Config Default
// Requirement: 09-REQ-5.2
func TestConfigDefault(t *testing.T) {
	os.Unsetenv("CLOUD_GATEWAY_URL")
	os.Unsetenv("CLOUD_GATEWAY_TOKEN")
	cfg := loadConfig(nil)
	if cfg.GatewayURL != "http://localhost:8081" {
		t.Errorf("GatewayURL = %q, want %q", cfg.GatewayURL, "http://localhost:8081")
	}
}

// TS-09-22: Gateway URL env override
func TestConfigEnvGatewayURL(t *testing.T) {
	os.Setenv("CLOUD_GATEWAY_URL", "http://mygateway:9999")
	defer os.Unsetenv("CLOUD_GATEWAY_URL")
	cfg := loadConfig(nil)
	if cfg.GatewayURL != "http://mygateway:9999" {
		t.Errorf("GatewayURL = %q, want %q", cfg.GatewayURL, "http://mygateway:9999")
	}
}

// TS-09-22: Flag overrides env
func TestConfigFlagOverridesEnv(t *testing.T) {
	os.Setenv("CLOUD_GATEWAY_URL", "http://envgateway:8081")
	defer os.Unsetenv("CLOUD_GATEWAY_URL")
	cfg := loadConfig([]string{"--gateway-url=http://flaggateway:7777"})
	if cfg.GatewayURL != "http://flaggateway:7777" {
		t.Errorf("GatewayURL = %q, want %q", cfg.GatewayURL, "http://flaggateway:7777")
	}
}

// TS-09-9: lock subcommand sends POST /vehicles/{vin}/commands
// Requirement: 09-REQ-3.1
func TestLockCommand(t *testing.T) {
	var cap capturedRequest
	srv := newCapturingServer(t, &cap)
	defer srv.Close()

	_, stderr, code := runCLI([]string{
		"lock",
		"--vin=VIN001",
		"--token=test-token",
		"--gateway-url=" + srv.URL,
	})
	if code != 0 {
		t.Fatalf("exit code = %d, want 0; stderr: %s", code, stderr)
	}
	if cap.Method != "POST" {
		t.Errorf("method = %q, want POST", cap.Method)
	}
	if cap.Path != "/vehicles/VIN001/commands" {
		t.Errorf("path = %q, want /vehicles/VIN001/commands", cap.Path)
	}
	if auth := cap.Headers.Get("Authorization"); auth != "Bearer test-token" {
		t.Errorf("Authorization = %q, want %q", auth, "Bearer test-token")
	}

	var body map[string]any
	if err := json.Unmarshal(cap.Body, &body); err != nil {
		t.Fatalf("decode body: %v", err)
	}
	if body["type"] != "lock" {
		t.Errorf("body.type = %q, want lock", body["type"])
	}
	if body["command_id"] == "" || body["command_id"] == nil {
		t.Error("body.command_id is empty, want a UUID")
	}
}

// TS-09-10: unlock subcommand sends POST with type=unlock
// Requirement: 09-REQ-3.2
func TestUnlockCommand(t *testing.T) {
	var cap capturedRequest
	srv := newCapturingServer(t, &cap)
	defer srv.Close()

	_, stderr, code := runCLI([]string{
		"unlock",
		"--vin=VIN001",
		"--token=test-token",
		"--gateway-url=" + srv.URL,
	})
	if code != 0 {
		t.Fatalf("exit code = %d, want 0; stderr: %s", code, stderr)
	}

	var body map[string]any
	if err := json.Unmarshal(cap.Body, &body); err != nil {
		t.Fatalf("decode body: %v", err)
	}
	if body["type"] != "unlock" {
		t.Errorf("body.type = %q, want unlock", body["type"])
	}
}

// TS-09-11: status subcommand sends GET /vehicles/{vin}/commands/{id}
// Requirement: 09-REQ-3.3
func TestStatusQuery(t *testing.T) {
	var cap capturedRequest
	srv := newCapturingServer(t, &cap)
	defer srv.Close()

	_, stderr, code := runCLI([]string{
		"status",
		"--vin=VIN001",
		"--command-id=cmd-123",
		"--token=test-token",
		"--gateway-url=" + srv.URL,
	})
	if code != 0 {
		t.Fatalf("exit code = %d, want 0; stderr: %s", code, stderr)
	}
	if cap.Method != "GET" {
		t.Errorf("method = %q, want GET", cap.Method)
	}
	if cap.Path != "/vehicles/VIN001/commands/cmd-123" {
		t.Errorf("path = %q, want /vehicles/VIN001/commands/cmd-123", cap.Path)
	}
}

// TS-09-E6: Missing bearer token → exit 1 with token mention in stderr
// Requirement: 09-REQ-3.E1
func TestMissingToken(t *testing.T) {
	os.Unsetenv("CLOUD_GATEWAY_TOKEN")
	_, stderr, code := runCLI([]string{"lock", "--vin=VIN001"})
	if code == 0 {
		t.Error("expected exit code != 0, got 0")
	}
	if !strings.Contains(strings.ToLower(stderr), "token") {
		t.Errorf("stderr does not mention 'token': %q", stderr)
	}
}

// TS-09-E7: CLOUD_GATEWAY unreachable → exit 1
// Requirement: 09-REQ-3.E2
func TestGatewayUnreachable(t *testing.T) {
	_, _, code := runCLI([]string{
		"lock",
		"--vin=VIN001",
		"--token=t",
		"--gateway-url=http://localhost:19999",
	})
	if code == 0 {
		t.Error("expected exit code != 0 when gateway unreachable, got 0")
	}
}

// TS-09-26: Connection error message includes the target address
// Requirement: 09-REQ-6.2
func TestConnectionErrorMessage(t *testing.T) {
	_, stderr, code := runCLI([]string{
		"lock",
		"--vin=VIN001",
		"--token=t",
		"--gateway-url=http://localhost:19999",
	})
	if code == 0 {
		t.Fatal("expected non-zero exit code")
	}
	if !strings.Contains(stderr, "19999") && !strings.Contains(strings.ToLower(stderr), "connect") {
		t.Errorf("stderr does not mention port or connection: %q", stderr)
	}
}

// TS-09-27: Upstream error response printed to stderr
// Requirement: 09-REQ-6.3
func TestUpstreamErrorResponse(t *testing.T) {
	srv := newErrorServer(t, http.StatusForbidden)
	defer srv.Close()

	_, stderr, code := runCLI([]string{
		"lock",
		"--vin=VIN001",
		"--token=t",
		"--gateway-url=" + srv.URL,
	})
	if code == 0 {
		t.Fatal("expected non-zero exit code on HTTP 403")
	}
	if len(stderr) == 0 {
		t.Error("expected non-empty stderr on upstream error, got empty")
	}
}

// TS-09-25: --help exits 0 with usage output
// Requirement: 09-REQ-6.1
func TestHelpFlag(t *testing.T) {
	var stdoutBuf, stderrBuf bytes.Buffer
	code := run([]string{"--help"}, &stdoutBuf, &stderrBuf)
	output := stdoutBuf.String() + stderrBuf.String()
	if code != 0 {
		t.Errorf("--help exit code = %d, want 0", code)
	}
	if len(output) == 0 {
		t.Error("--help produced no output")
	}
}

// TS-09-P5: Error Exit Code Consistency
// Property 5 — error conditions produce exit code 1 with stderr output.
func TestPropertyErrorExitCode(t *testing.T) {
	os.Unsetenv("CLOUD_GATEWAY_TOKEN")
	scenarios := []struct {
		name string
		args []string
	}{
		{"no_token", []string{"lock", "--vin=VIN001"}},
		{"unknown_subcmd", []string{"foobar", "--token=t"}},
		{"unreachable", []string{"lock", "--vin=VIN001", "--token=t", "--gateway-url=http://localhost:19999"}},
	}
	for _, sc := range scenarios {
		t.Run(sc.name, func(t *testing.T) {
			_, stderr, code := runCLI(sc.args)
			if code == 0 {
				t.Errorf("scenario %q: expected exit code != 0, got 0", sc.name)
			}
			if len(stderr) == 0 {
				t.Errorf("scenario %q: expected non-empty stderr", sc.name)
			}
		})
	}
}

// TS-09-P4: Configuration Defaults Property
// Property 4 — unset env vars use defaults.
func TestPropertyConfigDefaults(t *testing.T) {
	os.Unsetenv("CLOUD_GATEWAY_URL")
	os.Unsetenv("CLOUD_GATEWAY_TOKEN")
	cfg := loadConfig(nil)
	if cfg.GatewayURL != "http://localhost:8081" {
		t.Errorf("GatewayURL default = %q, want http://localhost:8081", cfg.GatewayURL)
	}

	// Override with env.
	os.Setenv("CLOUD_GATEWAY_URL", "http://custom:9999")
	defer os.Unsetenv("CLOUD_GATEWAY_URL")
	cfg2 := loadConfig(nil)
	if cfg2.GatewayURL != "http://custom:9999" {
		t.Errorf("GatewayURL with env = %q, want http://custom:9999", cfg2.GatewayURL)
	}
}
