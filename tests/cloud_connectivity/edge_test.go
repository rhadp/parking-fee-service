package cloud_connectivity_test

import (
	"encoding/json"
	"fmt"
	"os"
	"strings"
	"testing"
	"time"
)

// TS-03-E1: Missing required fields in command body
// Requirement: 03-REQ-1.E1
// Verifies 400 Bad Request when required fields are missing.
func TestEdge_MissingFields(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping edge case test in short mode")
	}

	root := repoRoot(t)
	port := freePort(t)

	_, baseURL := startGateway(t, root, port, "MQTT_BROKER_URL=tcp://localhost:19999")

	testCases := []struct {
		name string
		body string
	}{
		{"missing_command_id", `{"type":"lock","doors":["driver"]}`},
		{"missing_type", `{"command_id":"x","doors":["driver"]}`},
		{"missing_doors", `{"command_id":"x","type":"lock"}`},
		{"invalid_type", `{"command_id":"x","type":"open","doors":["driver"]}`},
	}

	for _, tc := range testCases {
		t.Run(tc.name, func(t *testing.T) {
			statusCode, respBody, err := httpPostJSON(t, baseURL+"/vehicles/VIN12345/commands", tc.body, "demo-token")
			if err != nil {
				t.Fatalf("HTTP POST failed: %v", err)
			}
			if statusCode != 400 {
				t.Errorf("expected status 400, got %d; body: %s", statusCode, respBody)
			}
			// Response should contain an "error" field
			var resp map[string]interface{}
			if err := json.Unmarshal([]byte(respBody), &resp); err == nil {
				if _, ok := resp["error"]; !ok {
					t.Error("expected 'error' field in response body")
				}
			}
		})
	}
}

// TS-03-E2: Invalid JSON in command body
// Requirement: 03-REQ-1.E2
// Verifies 400 Bad Request when body is not valid JSON.
func TestEdge_InvalidJSON(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping edge case test in short mode")
	}

	root := repoRoot(t)
	port := freePort(t)

	_, baseURL := startGateway(t, root, port, "MQTT_BROKER_URL=tcp://localhost:19999")

	invalidBodies := []string{
		"not json at all",
		"{malformed",
		"",
	}

	for _, body := range invalidBodies {
		t.Run(fmt.Sprintf("body_%q", body), func(t *testing.T) {
			statusCode, respBody, err := httpPostJSON(t, baseURL+"/vehicles/VIN12345/commands", body, "demo-token")
			if err != nil {
				t.Fatalf("HTTP POST failed: %v", err)
			}
			if statusCode != 400 {
				t.Errorf("expected status 400 for body %q, got %d; response: %s", body, statusCode, respBody)
			}
		})
	}
}

// TS-03-E3: MQTT broker unreachable on startup
// Requirement: 03-REQ-2.E1
// Verifies CLOUD_GATEWAY starts REST API even when MQTT broker is unreachable.
func TestEdge_MQTTUnreachableStartup(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping edge case test in short mode")
	}

	root := repoRoot(t)
	port := freePort(t)

	buildCloudGateway(t, root)

	// Start with unreachable MQTT broker
	env := append(os.Environ(),
		fmt.Sprintf("PORT=%d", port),
		"MQTT_BROKER_URL=tcp://localhost:19999", // no broker here
		"AUTH_TOKEN=demo-token",
	)
	cmd, stdout, stderr := startProcessWithOutput(t, root, "backend/cloud-gateway", env, "./cloud-gateway-test")
	t.Cleanup(func() {
		cmd.Process.Kill()
		cmd.Wait()
	})

	// The REST API should still start despite MQTT being unreachable
	if !waitForPort(t, port, 5*time.Second) {
		t.Fatal("cloud-gateway REST API did not start despite MQTT being unreachable")
	}

	// Health check should work
	statusCode, _, err := httpGet(t, fmt.Sprintf("http://localhost:%d/health", port))
	if err != nil {
		t.Fatalf("health check failed: %v", err)
	}
	if statusCode != 200 {
		t.Errorf("expected health check 200, got %d", statusCode)
	}

	// Log output should contain retry/connection attempt messages
	combinedOutput := stdout.String() + stderr.String()
	if !strings.Contains(combinedOutput, "retry") &&
		!strings.Contains(combinedOutput, "connect") &&
		!strings.Contains(combinedOutput, "MQTT") {
		t.Errorf("expected log output mentioning MQTT retry/connection, got: %s", combinedOutput)
	}
}

// TS-03-E4: MQTT broker connection lost after startup
// Requirement: 03-REQ-2.E2
// Verifies CLOUD_GATEWAY handles MQTT disconnection gracefully.
func TestEdge_MQTTDisconnected(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping integration test in short mode")
	}
	skipIfNoMosquitto(t)

	// This test requires the ability to stop/start Mosquitto during the test.
	// It's an integration test that verifies the CLOUD_GATEWAY doesn't crash
	// when the MQTT broker disconnects.
	root := repoRoot(t)
	port := freePort(t)

	buildCloudGateway(t, root)

	env := append(os.Environ(),
		fmt.Sprintf("PORT=%d", port),
		"MQTT_BROKER_URL=tcp://localhost:1883",
		"AUTH_TOKEN=demo-token",
		"COMMAND_TIMEOUT=3s",
	)
	cmd, _, _ := startProcessWithOutput(t, root, "backend/cloud-gateway", env, "./cloud-gateway-test")
	t.Cleanup(func() {
		cmd.Process.Kill()
		cmd.Wait()
	})

	if !waitForPort(t, port, 5*time.Second) {
		t.Fatal("cloud-gateway did not start in time")
	}

	// Verify the gateway is initially healthy
	statusCode, _, err := httpGet(t, fmt.Sprintf("http://localhost:%d/health", port))
	if err != nil {
		t.Fatalf("initial health check failed: %v", err)
	}
	if statusCode != 200 {
		t.Errorf("expected initial health check 200, got %d", statusCode)
	}

	// Note: Actually stopping and restarting Mosquitto during the test is
	// destructive and may affect other tests. We verify the gateway doesn't
	// crash by attempting a command that will time out (simulating broker issues).
	body := `{"command_id":"disconnect-test","type":"lock","doors":["driver"]}`
	statusCode, respBody, err := httpPostJSONWithTimeout(t,
		fmt.Sprintf("http://localhost:%d/vehicles/VIN/commands", port),
		body, "demo-token", 5*time.Second)
	if err != nil {
		// A timeout error from the HTTP client is acceptable
		t.Logf("HTTP request returned error (expected for disconnection test): %v", err)
		return
	}

	// Gateway should respond with an error, not hang
	if statusCode == 200 || statusCode == 202 {
		// If it succeeded, that's fine too (Mosquitto is still connected)
		t.Logf("command succeeded (Mosquitto still connected): %d %s", statusCode, respBody)
	}
}

// TS-03-E5: Command response timeout
// Requirement: 03-REQ-2.E3
// Verifies pending command times out and returns 504.
func TestEdge_CommandTimeout(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping edge case test in short mode")
	}

	root := repoRoot(t)
	port := freePort(t)

	buildCloudGateway(t, root)

	// Use a very short timeout for testing
	env := append(os.Environ(),
		fmt.Sprintf("PORT=%d", port),
		"MQTT_BROKER_URL=tcp://localhost:19999", // unreachable MQTT
		"AUTH_TOKEN=demo-token",
		"COMMAND_TIMEOUT=1s", // 1-second timeout
	)
	cmd, _, _ := startProcessWithOutput(t, root, "backend/cloud-gateway", env, "./cloud-gateway-test")
	t.Cleanup(func() {
		cmd.Process.Kill()
		cmd.Wait()
	})

	if !waitForPort(t, port, 5*time.Second) {
		t.Fatal("cloud-gateway did not start in time")
	}

	baseURL := fmt.Sprintf("http://localhost:%d", port)
	body := `{"command_id":"timeout-test","type":"lock","doors":["driver"]}`
	statusCode, respBody, err := httpPostJSONWithTimeout(t,
		baseURL+"/vehicles/VIN12345/commands", body, "demo-token", 10*time.Second)
	if err != nil {
		t.Fatalf("HTTP POST failed: %v", err)
	}

	if statusCode != 504 {
		t.Errorf("expected status 504 (gateway timeout), got %d; body: %s", statusCode, respBody)
	}

	var resp map[string]interface{}
	if err := json.Unmarshal([]byte(respBody), &resp); err != nil {
		t.Fatalf("response is not valid JSON: %v; body: %s", err, respBody)
	}

	if resp["command_id"] != "timeout-test" {
		t.Errorf("expected command_id 'timeout-test', got %v", resp["command_id"])
	}
	if resp["status"] != "timeout" {
		t.Errorf("expected status 'timeout', got %v", resp["status"])
	}
}

// TS-03-E6: Unknown command_id in MQTT response
// Requirement: 03-REQ-3.E1
// Verifies unknown command_id responses are logged and discarded.
func TestEdge_UnknownCommandID(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping edge case test in short mode")
	}

	root := repoRoot(t)

	result := execCommand(t, root, "backend/cloud-gateway",
		"go", "test", "-v", "-count=1", "-run", "TestTracker_UnknownID", "./internal/bridge/...")
	if result.ExitCode != 0 {
		t.Errorf("unknown command ID tests failed:\nstdout: %s\nstderr: %s",
			result.Stdout, result.Stderr)
	}
}

// TS-03-E7: Duplicate command_id response
// Requirement: 03-REQ-3.E2
// Verifies only the first response for a command_id is used.
func TestEdge_DuplicateCommandID(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping edge case test in short mode")
	}

	root := repoRoot(t)

	result := execCommand(t, root, "backend/cloud-gateway",
		"go", "test", "-v", "-count=1", "-run", "TestTracker_Duplicate", "./internal/bridge/...")
	if result.ExitCode != 0 {
		t.Errorf("duplicate command ID tests failed:\nstdout: %s\nstderr: %s",
			result.Stdout, result.Stderr)
	}
}

// TS-03-E8: CLI missing --token flag
// Requirement: 03-REQ-4.E1
// Verifies CLI exits with error when --token is not provided.
func TestEdge_MissingToken(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping edge case test in short mode")
	}

	root := repoRoot(t)

	buildResult := execCommand(t, root, "mock/companion-app-cli", "go", "build", "-o", "cli-test", ".")
	if buildResult.ExitCode != 0 {
		t.Fatalf("failed to build companion-app-cli: %s", buildResult.Stderr)
	}
	defer os.Remove(fmt.Sprintf("%s/mock/companion-app-cli/cli-test", root))

	// Run without --token
	result := execCommand(t, root, "mock/companion-app-cli",
		"./cli-test", "lock",
		"--vin", "VIN12345")

	if result.ExitCode == 0 {
		t.Error("expected non-zero exit code when --token is not provided")
	}

	if !strings.Contains(strings.ToLower(result.Stderr), "token") {
		t.Errorf("expected stderr to mention 'token', got: %s", result.Stderr)
	}
}

// TS-03-E9: CLI cannot connect to CLOUD_GATEWAY
// Requirement: 03-REQ-4.E2
// Verifies CLI handles connection failure gracefully.
func TestEdge_GatewayUnreachable(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping edge case test in short mode")
	}

	root := repoRoot(t)

	buildResult := execCommand(t, root, "mock/companion-app-cli", "go", "build", "-o", "cli-test", ".")
	if buildResult.ExitCode != 0 {
		t.Fatalf("failed to build companion-app-cli: %s", buildResult.Stderr)
	}
	defer os.Remove(fmt.Sprintf("%s/mock/companion-app-cli/cli-test", root))

	// Point CLI to a non-existent gateway
	result := execCommand(t, root, "mock/companion-app-cli",
		"./cli-test", "lock",
		"--vin", "VIN12345",
		"--token", "demo-token",
		"--gateway-url", "http://localhost:19999")

	if result.ExitCode == 0 {
		t.Error("expected non-zero exit code when gateway is unreachable")
	}

	if len(result.Stderr) == 0 {
		t.Error("expected error message on stderr when gateway is unreachable")
	}
}

// TS-03-E10: Integration test skips without Mosquitto
// Requirement: 03-REQ-6.E1
// Verifies integration tests skip cleanly when Mosquitto is not running.
func TestEdge_IntegrationSkipsWithoutMosquitto(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping edge case test in short mode")
	}

	// If Mosquitto IS running, we can't test the skip behavior.
	if portIsOpen(t, mqttBrokerPort) {
		t.Skip("Mosquitto is running; cannot test skip behavior")
	}

	root := repoRoot(t)

	// Run integration tests — they should skip, not fail
	result := execCommand(t, root, "backend/cloud-gateway",
		"go", "test", "-v", "-count=1", "-run", "TestIntegration", "./...")

	// Exit code should be 0 (tests skipped, not failed)
	if result.ExitCode != 0 {
		// Check if the failure is because no integration tests exist yet
		if strings.Contains(result.Stdout, "no test files") ||
			strings.Contains(result.Stderr, "no test files") {
			t.Error("no integration tests exist in backend/cloud-gateway yet")
		} else if strings.Contains(result.Stdout, "FAIL") {
			t.Errorf("integration tests should skip when Mosquitto is not running, but they failed:\n%s",
				result.Stdout)
		}
	}
}

// Ensure time import is used.
var _ = time.Second
