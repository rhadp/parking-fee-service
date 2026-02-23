package cloud_connectivity_test

import (
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/httptest"
	"os"
	"strings"
	"testing"
)

// TS-03-15: CLI lock command sends correct REST request
// Requirement: 03-REQ-4.1
// Verifies the lock subcommand sends the correct POST request.
func TestUnit_CLI_LockCommand(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping CLI test in short mode")
	}

	root := repoRoot(t)

	var receivedMethod string
	var receivedPath string
	var receivedBody map[string]interface{}
	var receivedAuth string

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		receivedMethod = r.Method
		receivedPath = r.URL.Path
		receivedAuth = r.Header.Get("Authorization")

		var body map[string]interface{}
		if err := json.NewDecoder(r.Body).Decode(&body); err == nil {
			receivedBody = body
		}

		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		w.Write([]byte(`{"command_id":"test","status":"success"}`))
	}))
	defer server.Close()

	// Build and run the companion-app-cli with lock command
	buildResult := execCommand(t, root, "mock/companion-app-cli", "go", "build", "-o", "cli-test", ".")
	if buildResult.ExitCode != 0 {
		t.Fatalf("failed to build companion-app-cli: %s", buildResult.Stderr)
	}
	defer os.Remove(fmt.Sprintf("%s/mock/companion-app-cli/cli-test", root))

	result := execCommand(t, root, "mock/companion-app-cli",
		"./cli-test", "lock",
		"--vin", "VIN12345",
		"--token", "demo-token",
		"--gateway-url", server.URL)

	if result.ExitCode != 0 {
		t.Errorf("expected exit code 0, got %d; stderr: %s", result.ExitCode, result.Stderr)
	}

	if receivedMethod != "POST" {
		t.Errorf("expected POST method, got %s", receivedMethod)
	}

	if receivedPath != "/vehicles/VIN12345/commands" {
		t.Errorf("expected path /vehicles/VIN12345/commands, got %s", receivedPath)
	}

	if receivedAuth != "Bearer demo-token" {
		t.Errorf("expected Authorization 'Bearer demo-token', got %q", receivedAuth)
	}

	if receivedBody == nil {
		t.Fatal("no request body received")
	}

	if receivedBody["type"] != "lock" {
		t.Errorf("expected type 'lock', got %v", receivedBody["type"])
	}

	doors, ok := receivedBody["doors"].([]interface{})
	if !ok || len(doors) == 0 {
		t.Error("expected doors array with at least one entry")
	} else if doors[0] != "driver" {
		t.Errorf("expected first door 'driver', got %v", doors[0])
	}

	cmdID, ok := receivedBody["command_id"].(string)
	if !ok || cmdID == "" {
		t.Error("expected non-empty command_id in request body")
	}
}

// TS-03-16: CLI unlock command sends correct REST request
// Requirement: 03-REQ-4.2
// Verifies the unlock subcommand sends the correct POST request.
func TestUnit_CLI_UnlockCommand(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping CLI test in short mode")
	}

	root := repoRoot(t)

	var receivedBody map[string]interface{}

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		var body map[string]interface{}
		if err := json.NewDecoder(r.Body).Decode(&body); err == nil {
			receivedBody = body
		}
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		w.Write([]byte(`{"command_id":"test","status":"success"}`))
	}))
	defer server.Close()

	buildResult := execCommand(t, root, "mock/companion-app-cli", "go", "build", "-o", "cli-test", ".")
	if buildResult.ExitCode != 0 {
		t.Fatalf("failed to build companion-app-cli: %s", buildResult.Stderr)
	}
	defer os.Remove(fmt.Sprintf("%s/mock/companion-app-cli/cli-test", root))

	result := execCommand(t, root, "mock/companion-app-cli",
		"./cli-test", "unlock",
		"--vin", "VIN12345",
		"--token", "demo-token",
		"--gateway-url", server.URL)

	if result.ExitCode != 0 {
		t.Errorf("expected exit code 0, got %d; stderr: %s", result.ExitCode, result.Stderr)
	}

	if receivedBody == nil {
		t.Fatal("no request body received")
	}

	if receivedBody["type"] != "unlock" {
		t.Errorf("expected type 'unlock', got %v", receivedBody["type"])
	}
}

// TS-03-17: CLI status command sends GET request
// Requirement: 03-REQ-4.3
// Verifies the status subcommand sends a GET request and displays the response.
func TestUnit_CLI_StatusCommand(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping CLI test in short mode")
	}

	root := repoRoot(t)

	var receivedMethod string
	var receivedPath string

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		receivedMethod = r.Method
		receivedPath = r.URL.Path
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		w.Write([]byte(`{"vin":"VIN12345","locked":true,"timestamp":1234}`))
	}))
	defer server.Close()

	buildResult := execCommand(t, root, "mock/companion-app-cli", "go", "build", "-o", "cli-test", ".")
	if buildResult.ExitCode != 0 {
		t.Fatalf("failed to build companion-app-cli: %s", buildResult.Stderr)
	}
	defer os.Remove(fmt.Sprintf("%s/mock/companion-app-cli/cli-test", root))

	result := execCommand(t, root, "mock/companion-app-cli",
		"./cli-test", "status",
		"--vin", "VIN12345",
		"--token", "demo-token",
		"--gateway-url", server.URL)

	if result.ExitCode != 0 {
		t.Errorf("expected exit code 0, got %d; stderr: %s", result.ExitCode, result.Stderr)
	}

	if receivedMethod != "GET" {
		t.Errorf("expected GET method, got %s", receivedMethod)
	}

	if receivedPath != "/vehicles/VIN12345/status" {
		t.Errorf("expected path /vehicles/VIN12345/status, got %s", receivedPath)
	}

	// stdout should contain the response
	if !strings.Contains(result.Stdout, "VIN12345") {
		t.Errorf("expected stdout to contain 'VIN12345', got: %s", result.Stdout)
	}
	if !strings.Contains(result.Stdout, "locked") {
		t.Errorf("expected stdout to contain 'locked', got: %s", result.Stdout)
	}
}

// TS-03-18: CLI includes bearer token in Authorization header
// Requirement: 03-REQ-4.4
// Verifies all CLI commands include the bearer token.
func TestUnit_CLI_BearerToken(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping CLI test in short mode")
	}

	root := repoRoot(t)

	buildResult := execCommand(t, root, "mock/companion-app-cli", "go", "build", "-o", "cli-test", ".")
	if buildResult.ExitCode != 0 {
		t.Fatalf("failed to build companion-app-cli: %s", buildResult.Stderr)
	}
	defer os.Remove(fmt.Sprintf("%s/mock/companion-app-cli/cli-test", root))

	commands := []string{"lock", "unlock", "status"}

	for _, cmd := range commands {
		t.Run(cmd, func(t *testing.T) {
			var receivedAuth string

			server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
				receivedAuth = r.Header.Get("Authorization")
				w.Header().Set("Content-Type", "application/json")
				w.WriteHeader(http.StatusOK)
				if cmd == "status" {
					w.Write([]byte(`{"vin":"VIN12345","locked":true,"timestamp":1234}`))
				} else {
					w.Write([]byte(`{"command_id":"test","status":"success"}`))
				}
			}))
			defer server.Close()

			execCommand(t, root, "mock/companion-app-cli",
				"./cli-test", cmd,
				"--vin", "VIN12345",
				"--token", "my-secret-token",
				"--gateway-url", server.URL)

			if receivedAuth != "Bearer my-secret-token" {
				t.Errorf("[%s] expected Authorization 'Bearer my-secret-token', got %q", cmd, receivedAuth)
			}
		})
	}
}

// TS-03-19: CLI uses VIN from --vin flag in URL
// Requirement: 03-REQ-4.5
// Verifies the CLI uses the --vin flag value in the request URL.
func TestUnit_CLI_VINInURL(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping CLI test in short mode")
	}

	root := repoRoot(t)

	var receivedPath string

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		receivedPath = r.URL.Path
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		w.Write([]byte(`{"command_id":"test","status":"success"}`))
	}))
	defer server.Close()

	buildResult := execCommand(t, root, "mock/companion-app-cli", "go", "build", "-o", "cli-test", ".")
	if buildResult.ExitCode != 0 {
		t.Fatalf("failed to build companion-app-cli: %s", buildResult.Stderr)
	}
	defer os.Remove(fmt.Sprintf("%s/mock/companion-app-cli/cli-test", root))

	result := execCommand(t, root, "mock/companion-app-cli",
		"./cli-test", "lock",
		"--vin", "CUSTOM_VIN_999",
		"--token", "demo-token",
		"--gateway-url", server.URL)

	if result.ExitCode != 0 {
		t.Errorf("expected exit code 0, got %d; stderr: %s", result.ExitCode, result.Stderr)
	}

	if !strings.Contains(receivedPath, "CUSTOM_VIN_999") {
		t.Errorf("expected URL path to contain 'CUSTOM_VIN_999', got %s", receivedPath)
	}
}

// TS-03-20: CLI success output
// Requirement: 03-REQ-4.6
// Verifies CLI prints JSON response to stdout on success.
func TestUnit_CLI_SuccessOutput(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping CLI test in short mode")
	}

	root := repoRoot(t)

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		w.Write([]byte(`{"command_id":"abc","status":"success"}`))
	}))
	defer server.Close()

	buildResult := execCommand(t, root, "mock/companion-app-cli", "go", "build", "-o", "cli-test", ".")
	if buildResult.ExitCode != 0 {
		t.Fatalf("failed to build companion-app-cli: %s", buildResult.Stderr)
	}
	defer os.Remove(fmt.Sprintf("%s/mock/companion-app-cli/cli-test", root))

	result := execCommand(t, root, "mock/companion-app-cli",
		"./cli-test", "lock",
		"--vin", "VIN12345",
		"--token", "demo-token",
		"--gateway-url", server.URL)

	if result.ExitCode != 0 {
		t.Errorf("expected exit code 0, got %d; stderr: %s", result.ExitCode, result.Stderr)
	}

	if !strings.Contains(result.Stdout, "command_id") {
		t.Errorf("expected stdout to contain 'command_id', got: %s", result.Stdout)
	}
	if !strings.Contains(result.Stdout, "success") {
		t.Errorf("expected stdout to contain 'success', got: %s", result.Stdout)
	}
}

// TS-03-21: CLI error output
// Requirement: 03-REQ-4.7
// Verifies CLI prints error to stderr and exits non-zero on failure.
func TestUnit_CLI_ErrorOutput(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping CLI test in short mode")
	}

	root := repoRoot(t)

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusUnauthorized)
		w.Write([]byte(`{"error":"unauthorized"}`))
	}))
	defer server.Close()

	buildResult := execCommand(t, root, "mock/companion-app-cli", "go", "build", "-o", "cli-test", ".")
	if buildResult.ExitCode != 0 {
		t.Fatalf("failed to build companion-app-cli: %s", buildResult.Stderr)
	}
	defer os.Remove(fmt.Sprintf("%s/mock/companion-app-cli/cli-test", root))

	result := execCommand(t, root, "mock/companion-app-cli",
		"./cli-test", "lock",
		"--vin", "VIN12345",
		"--token", "wrong-token",
		"--gateway-url", server.URL)

	if result.ExitCode == 0 {
		t.Error("expected non-zero exit code on error response")
	}

	if len(result.Stderr) == 0 {
		t.Error("expected error message on stderr")
	}
}
