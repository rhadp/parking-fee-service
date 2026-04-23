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

	// Verify request body contains lock command with doors field.
	var body map[string]interface{}
	if err := json.Unmarshal(receivedBody, &body); err != nil {
		t.Errorf("failed to parse request body: %v", err)
	} else {
		if body["type"] != "lock" {
			t.Errorf("expected body type 'lock', got: %v", body["type"])
		}
		// 09-REQ-7.1: body must include doors: ["driver"].
		doors, ok := body["doors"].([]interface{})
		if !ok {
			t.Errorf("expected 'doors' array in body, got: %v", body["doors"])
		} else if len(doors) != 1 || doors[0] != "driver" {
			t.Errorf("expected doors=[\"driver\"], got: %v", doors)
		}
	}
}

// ---------------------------------------------------------------------------
// TS-09-12: Companion App CLI Unlock
// Requirement: 09-REQ-7.2
// ---------------------------------------------------------------------------

func TestUnlockCommand(t *testing.T) {
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

	if receivedMethod != "POST" {
		t.Errorf("expected POST request, got: %s", receivedMethod)
	}

	if receivedPath != "/vehicles/VIN001/commands" {
		t.Errorf("expected path '/vehicles/VIN001/commands', got: %s", receivedPath)
	}

	if receivedAuth != "Bearer test-token" {
		t.Errorf("expected Authorization 'Bearer test-token', got: %s", receivedAuth)
	}

	// Verify request body contains unlock command with doors field.
	var body map[string]interface{}
	if err := json.Unmarshal(receivedBody, &body); err != nil {
		t.Errorf("failed to parse request body: %v", err)
	} else {
		if body["type"] != "unlock" {
			t.Errorf("expected body type 'unlock', got: %v", body["type"])
		}
		// 09-REQ-7.2: body must include doors: ["driver"].
		doors, ok := body["doors"].([]interface{})
		if !ok {
			t.Errorf("expected 'doors' array in body, got: %v", body["doors"])
		} else if len(doors) != 1 || doors[0] != "driver" {
			t.Errorf("expected doors=[\"driver\"], got: %v", doors)
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
// TS-09-P6: Bearer Token Enforcement (property test)
// Requirement: 09-REQ-7.4, 09-REQ-7.E2
// Property: For any token string, the Authorization header equals
// "Bearer <token>". When no token is provided, exit code is 1.
// ---------------------------------------------------------------------------

func TestBearerTokenProperty(t *testing.T) {
	binary := buildBinary(t, "companion-app-cli")

	// Token propagation via --token flag with diverse token strings.
	flagTokens := []string{
		"simple-token",
		"eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.payload.signature", // JWT-like
		"token-with-special-chars_123!@#",
		"a",                    // minimal
		"x-" + strings.Repeat("A", 200), // long token
		"unicode-töken",   // non-ASCII
		"spaces are allowed",   // spaces in token
	}

	for _, token := range flagTokens {
		t.Run("flag/"+token[:min(len(token), 30)], func(t *testing.T) {
			var receivedAuth string
			srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
				receivedAuth = r.Header.Get("Authorization")
				w.Header().Set("Content-Type", "application/json")
				json.NewEncoder(w).Encode(map[string]string{"command_id": "x"})
			}))
			defer srv.Close()

			_, stderr, exitCode := runBinary(t, binary,
				"lock",
				"--vin=VIN001",
				"--token="+token,
				"--gateway-addr="+srv.URL,
			)

			if exitCode != 0 {
				t.Fatalf("expected exit code 0 with --token flag, got %d\nstderr: %s", exitCode, stderr)
			}

			expected := "Bearer " + token
			if receivedAuth != expected {
				t.Errorf("expected Authorization %q, got: %q", expected, receivedAuth)
			}
		})
	}

	// Token propagation via CLOUD_GATEWAY_TOKEN env var.
	envTokens := []string{
		"env-token-123",
		"another-env-token-with-dashes",
		"ENV_TOKEN_UPPER",
	}

	for _, token := range envTokens {
		t.Run("env/"+token, func(t *testing.T) {
			var receivedAuth string
			srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
				receivedAuth = r.Header.Get("Authorization")
				w.Header().Set("Content-Type", "application/json")
				json.NewEncoder(w).Encode(map[string]string{"command_id": "x"})
			}))
			defer srv.Close()

			_, stderr, exitCode := runBinaryWithEnv(t, binary,
				[]string{"CLOUD_GATEWAY_TOKEN=" + token},
				"lock",
				"--vin=VIN001",
				"--gateway-addr="+srv.URL,
			)

			if exitCode != 0 {
				t.Fatalf("expected exit code 0 with env token, got %d\nstderr: %s", exitCode, stderr)
			}

			expected := "Bearer " + token
			if receivedAuth != expected {
				t.Errorf("expected Authorization %q, got: %q", expected, receivedAuth)
			}
		})
	}

	// Token absent: verify failure.
	t.Run("absent", func(t *testing.T) {
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
	})

	// Flag takes precedence over env var.
	t.Run("flag-precedence", func(t *testing.T) {
		var receivedAuth string
		srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			receivedAuth = r.Header.Get("Authorization")
			w.Header().Set("Content-Type", "application/json")
			json.NewEncoder(w).Encode(map[string]string{"command_id": "x"})
		}))
		defer srv.Close()

		_, stderr, exitCode := runBinaryWithEnv(t, binary,
			[]string{"CLOUD_GATEWAY_TOKEN=env-token"},
			"lock",
			"--vin=VIN001",
			"--token=flag-token",
			"--gateway-addr="+srv.URL,
		)

		if exitCode != 0 {
			t.Fatalf("expected exit code 0, got %d\nstderr: %s", exitCode, stderr)
		}

		if receivedAuth != "Bearer flag-token" {
			t.Errorf("expected flag token to take precedence, got: %s", receivedAuth)
		}
	})
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
