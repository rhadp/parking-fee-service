package mockapps_test

import (
	"encoding/json"
	"path/filepath"
	"strings"
	"testing"
)

// companionBinary builds and returns the path to the companion-app-cli binary.
func companionBinary(t *testing.T) string {
	t.Helper()
	root := findRepoRoot(t)
	moduleDir := filepath.Join(root, "mock", "companion-app-cli")
	return buildGoBinary(t, moduleDir, "companion-app-cli")
}

// ---------------------------------------------------------------------------
// TS-09-11: Companion App CLI Lock
// Requirement: 09-REQ-7.1, 09-REQ-7.4, 09-REQ-7.5
// ---------------------------------------------------------------------------

func TestLockCommand(t *testing.T) {
	mock := newMockHTTPServer(t, 200, map[string]any{
		"command_id": "cmd-1",
		"status":     "pending",
	})

	bin := companionBinary(t)
	env := baseEnv()

	stdout, stderr, exitCode := runBinary(t, bin, []string{
		"lock",
		"--vin=VIN001",
		"--token=test-token",
		"--gateway-addr=" + mock.URL(),
	}, env)

	if exitCode != 0 {
		t.Fatalf("expected exit 0, got %d\nstderr: %s", exitCode, stderr)
	}

	if !strings.Contains(stdout, "cmd-1") {
		t.Errorf("expected stdout to contain 'cmd-1', got: %s", stdout)
	}

	// Verify the HTTP request
	reqs := mock.getRequests()
	if len(reqs) != 1 {
		t.Fatalf("expected 1 request, got %d", len(reqs))
	}

	req := reqs[0]
	if req.Method != "POST" {
		t.Errorf("expected POST, got %s", req.Method)
	}
	if req.Path != "/vehicles/VIN001/commands" {
		t.Errorf("expected path /vehicles/VIN001/commands, got %s", req.Path)
	}

	// Verify Authorization header
	authHeader := req.Headers.Get("Authorization")
	if authHeader != "Bearer test-token" {
		t.Errorf("expected Authorization 'Bearer test-token', got %q", authHeader)
	}

	// Verify request body
	var body map[string]any
	if err := json.Unmarshal([]byte(req.Body), &body); err != nil {
		t.Fatalf("failed to parse request body: %v", err)
	}
	if body["type"] != "lock" {
		t.Errorf("expected body type 'lock', got %v", body["type"])
	}
}

// ---------------------------------------------------------------------------
// TS-09-12: Companion App CLI Unlock
// Requirement: 09-REQ-7.2
// ---------------------------------------------------------------------------

func TestUnlockCommand(t *testing.T) {
	mock := newMockHTTPServer(t, 200, map[string]any{
		"command_id": "cmd-2",
		"status":     "pending",
	})

	bin := companionBinary(t)
	env := baseEnv()

	stdout, _, exitCode := runBinary(t, bin, []string{
		"unlock",
		"--vin=VIN001",
		"--token=test-token",
		"--gateway-addr=" + mock.URL(),
	}, env)

	if exitCode != 0 {
		t.Fatalf("expected exit 0, got %d", exitCode)
	}

	if !strings.Contains(stdout, "cmd-2") {
		t.Errorf("expected stdout to contain 'cmd-2', got: %s", stdout)
	}

	// Verify the request body contains "unlock"
	reqs := mock.getRequests()
	if len(reqs) != 1 {
		t.Fatalf("expected 1 request, got %d", len(reqs))
	}

	var body map[string]any
	if err := json.Unmarshal([]byte(reqs[0].Body), &body); err != nil {
		t.Fatalf("failed to parse request body: %v", err)
	}
	if body["type"] != "unlock" {
		t.Errorf("expected body type 'unlock', got %v", body["type"])
	}
}

// ---------------------------------------------------------------------------
// TS-09-13: Companion App CLI Status
// Requirement: 09-REQ-7.3
// ---------------------------------------------------------------------------

func TestStatusCommand(t *testing.T) {
	mock := newMockHTTPServer(t, 200, map[string]any{
		"command_id": "cmd-1",
		"status":     "success",
	})

	bin := companionBinary(t)
	env := baseEnv()

	stdout, _, exitCode := runBinary(t, bin, []string{
		"status",
		"--vin=VIN001",
		"--command-id=cmd-1",
		"--token=test-token",
		"--gateway-addr=" + mock.URL(),
	}, env)

	if exitCode != 0 {
		t.Fatalf("expected exit 0, got %d", exitCode)
	}

	if !strings.Contains(stdout, "success") {
		t.Errorf("expected stdout to contain 'success', got: %s", stdout)
	}

	// Verify GET request path
	reqs := mock.getRequests()
	if len(reqs) != 1 {
		t.Fatalf("expected 1 request, got %d", len(reqs))
	}
	if reqs[0].Method != "GET" {
		t.Errorf("expected GET, got %s", reqs[0].Method)
	}
	if reqs[0].Path != "/vehicles/VIN001/commands/cmd-1" {
		t.Errorf("expected path /vehicles/VIN001/commands/cmd-1, got %s", reqs[0].Path)
	}
}

// ---------------------------------------------------------------------------
// TS-09-E5: Companion App Missing Token
// Requirement: 09-REQ-7.E2
// ---------------------------------------------------------------------------

func TestMissingToken(t *testing.T) {
	bin := companionBinary(t)

	// Use clean environment without CLOUD_GATEWAY_TOKEN
	env := baseEnv()

	_, stderr, exitCode := runBinary(t, bin, []string{
		"lock",
		"--vin=VIN001",
	}, env)

	if exitCode != 1 {
		t.Errorf("expected exit 1, got %d", exitCode)
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
	bin := companionBinary(t)
	env := baseEnv()

	_, stderr, exitCode := runBinary(t, bin, []string{
		"lock",
		"--token=test-token",
	}, env)

	if exitCode != 1 {
		t.Errorf("expected exit 1, got %d", exitCode)
	}

	if len(stderr) == 0 {
		t.Error("expected stderr to contain error message")
	}
}

// ---------------------------------------------------------------------------
// TS-09-P6: Bearer Token Enforcement (property test)
// Property 6 from design.md
// Requirement: 09-REQ-7.4, 09-REQ-7.E2
// ---------------------------------------------------------------------------

func TestBearerTokenProperty(t *testing.T) {
	// Test that different tokens are correctly propagated to the Authorization header
	tokens := []string{
		"token-abc",
		"my-secret-123",
		"Bearer-already",
		"x",
		"a-very-long-token-value-1234567890",
		"special!@#chars",
		"unicode-token-éè",
		"token with spaces",
		"",
		"null",
		"123456",
		"eyJhbGciOiJIUzI1NiJ9.test",
	}

	bin := companionBinary(t)

	for _, token := range tokens {
		if token == "" {
			// Empty token is a special case - skip for positive tests
			continue
		}

		t.Run("token_"+token, func(t *testing.T) {
			mock := newMockHTTPServer(t, 200, map[string]any{
				"command_id": "x",
				"status":     "pending",
			})

			env := baseEnv()

			_, _, exitCode := runBinary(t, bin, []string{
				"lock",
				"--vin=VIN001",
				"--token=" + token,
				"--gateway-addr=" + mock.URL(),
			}, env)

			if exitCode != 0 {
				t.Errorf("token=%q: expected exit 0, got %d", token, exitCode)
				return
			}

			reqs := mock.getRequests()
			if len(reqs) == 0 {
				t.Errorf("token=%q: no requests received", token)
				return
			}

			authHeader := reqs[0].Headers.Get("Authorization")
			expected := "Bearer " + token
			if authHeader != expected {
				t.Errorf("token=%q: expected Authorization %q, got %q",
					token, expected, authHeader)
			}
		})
	}

	// Verify failure when no token is provided
	t.Run("no_token_fails", func(t *testing.T) {
		env := baseEnv()
		_, stderr, exitCode := runBinary(t, bin, []string{
			"lock",
			"--vin=VIN001",
		}, env)

		if exitCode != 1 {
			t.Errorf("no-token: expected exit 1, got %d", exitCode)
		}
		if !strings.Contains(strings.ToLower(stderr), "token") {
			t.Errorf("no-token: expected stderr to mention 'token', got: %s", stderr)
		}
	})
}

// ---------------------------------------------------------------------------
// Companion App Non-2xx Response (covers 09-REQ-7.E3)
// Note: The test_spec coverage matrix incorrectly maps 09-REQ-7.E3 to
// TS-09-E11 (which tests parking-app-cli). This test covers the companion-app
// non-2xx path that was missing from the coverage matrix.
// ---------------------------------------------------------------------------

func TestCompanionAppHTTPError(t *testing.T) {
	mock := newMockHTTPServer(t, 500, map[string]any{
		"error": "internal server error",
	})

	bin := companionBinary(t)
	env := baseEnv()

	_, stderr, exitCode := runBinary(t, bin, []string{
		"lock",
		"--vin=VIN001",
		"--token=test-token",
		"--gateway-addr=" + mock.URL(),
	}, env)

	if exitCode != 1 {
		t.Errorf("expected exit 1 on HTTP 500, got %d", exitCode)
	}

	if !strings.Contains(stderr, "500") {
		t.Errorf("expected stderr to contain '500', got: %s", stderr)
	}
}
