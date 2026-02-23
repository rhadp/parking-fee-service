package cloud_connectivity_test

import (
	"encoding/json"
	"fmt"
	"os"
	"strings"
	"testing"
	"time"
)

// TestE2E_CLI_LockCommand verifies the full end-to-end flow using the CLI:
// CLI lock -> CLOUD_GATEWAY REST -> MQTT -> simulated subscriber ->
// MQTT response -> REST response -> CLI output.
// Requirements: 03-REQ-6.1, 03-REQ-4.1, 03-REQ-4.6
func TestE2E_CLI_LockCommand(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping e2e test in short mode")
	}
	skipIfNoMosquitto(t)

	root := repoRoot(t)
	port := freePort(t)

	buildCloudGateway(t, root)

	// Build the CLI binary
	cliBuildResult := execCommand(t, root, "mock/companion-app-cli", "go", "build", "-o", "cli-e2e-test", ".")
	if cliBuildResult.ExitCode != 0 {
		t.Fatalf("failed to build companion-app-cli: %s", cliBuildResult.Stderr)
	}
	cliBinPath := fmt.Sprintf("%s/mock/companion-app-cli/cli-e2e-test", root)
	t.Cleanup(func() { os.Remove(cliBinPath) })

	// Start the CLOUD_GATEWAY with Mosquitto
	gwEnv := append(os.Environ(),
		fmt.Sprintf("PORT=%d", port),
		"MQTT_BROKER_URL=tcp://localhost:1883",
		"AUTH_TOKEN=demo-token",
		"COMMAND_TIMEOUT=15s",
	)
	gwCmd, _, _ := startProcessWithOutput(t, root, "backend/cloud-gateway", gwEnv, "./cloud-gateway-test")
	t.Cleanup(func() {
		gwCmd.Process.Kill()
		gwCmd.Wait()
	})

	if !waitForPort(t, port, 5*time.Second) {
		t.Fatal("cloud-gateway did not start in time")
	}

	// Start a simulated vehicle responder that auto-responds to commands
	startSimulatedResponder(t, root, "VIN_E2E")

	// Give the responder a moment to subscribe
	time.Sleep(500 * time.Millisecond)

	// Execute the CLI lock command
	gatewayURL := fmt.Sprintf("http://localhost:%d", port)
	result := execCommand(t, root, "mock/companion-app-cli",
		"./cli-e2e-test", "lock",
		"--vin", "VIN_E2E",
		"--token", "demo-token",
		"--gateway-url", gatewayURL)

	if result.ExitCode != 0 {
		t.Errorf("CLI lock command failed: exit code %d\nstdout: %s\nstderr: %s",
			result.ExitCode, result.Stdout, result.Stderr)
	}

	// The stdout should contain the response JSON
	if result.Stdout == "" {
		t.Error("expected stdout to contain response JSON, but it was empty")
	} else {
		var resp map[string]interface{}
		if err := json.Unmarshal([]byte(strings.TrimSpace(result.Stdout)), &resp); err != nil {
			t.Errorf("stdout is not valid JSON: %v; output: %q", err, result.Stdout)
		} else {
			if _, ok := resp["command_id"]; !ok {
				t.Error("response missing command_id field")
			}
			if resp["status"] != "success" {
				t.Errorf("expected status 'success', got %v", resp["status"])
			}
		}
	}
}

// TestE2E_CLI_StatusCommand verifies the full flow for the status command:
// Publish telemetry via MQTT -> CLI status -> CLOUD_GATEWAY REST -> cached data.
// Requirements: 03-REQ-6.1, 03-REQ-4.3
func TestE2E_CLI_StatusCommand(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping e2e test in short mode")
	}
	skipIfNoMosquitto(t)

	root := repoRoot(t)
	port := freePort(t)

	buildCloudGateway(t, root)

	// Build the CLI binary
	cliBuildResult := execCommand(t, root, "mock/companion-app-cli", "go", "build", "-o", "cli-e2e-test", ".")
	if cliBuildResult.ExitCode != 0 {
		t.Fatalf("failed to build companion-app-cli: %s", cliBuildResult.Stderr)
	}
	cliBinPath := fmt.Sprintf("%s/mock/companion-app-cli/cli-e2e-test", root)
	t.Cleanup(func() { os.Remove(cliBinPath) })

	// Start the CLOUD_GATEWAY with Mosquitto
	gwEnv := append(os.Environ(),
		fmt.Sprintf("PORT=%d", port),
		"MQTT_BROKER_URL=tcp://localhost:1883",
		"AUTH_TOKEN=demo-token",
	)
	gwCmd, _, _ := startProcessWithOutput(t, root, "backend/cloud-gateway", gwEnv, "./cloud-gateway-test")
	t.Cleanup(func() {
		gwCmd.Process.Kill()
		gwCmd.Wait()
	})

	if !waitForPort(t, port, 5*time.Second) {
		t.Fatal("cloud-gateway did not start in time")
	}

	// Publish telemetry via mosquitto_pub
	telemetry := `{"vin":"VIN_STATUS","locked":true,"timestamp":1708700000}`
	pubResult := execCommand(t, root, ".", "mosquitto_pub",
		"-h", "localhost", "-p", "1883",
		"-t", "vehicles/VIN_STATUS/telemetry",
		"-m", telemetry)
	if pubResult.ExitCode != 0 {
		t.Fatalf("mosquitto_pub failed: %s", pubResult.Stderr)
	}

	// Wait for telemetry to be processed
	time.Sleep(1 * time.Second)

	// Execute the CLI status command
	gatewayURL := fmt.Sprintf("http://localhost:%d", port)
	result := execCommand(t, root, "mock/companion-app-cli",
		"./cli-e2e-test", "status",
		"--vin", "VIN_STATUS",
		"--token", "demo-token",
		"--gateway-url", gatewayURL)

	if result.ExitCode != 0 {
		t.Errorf("CLI status command failed: exit code %d\nstdout: %s\nstderr: %s",
			result.ExitCode, result.Stdout, result.Stderr)
	}

	// The stdout should contain the telemetry data
	if result.Stdout == "" {
		t.Error("expected stdout to contain status JSON, but it was empty")
	} else {
		var resp map[string]interface{}
		if err := json.Unmarshal([]byte(strings.TrimSpace(result.Stdout)), &resp); err != nil {
			t.Errorf("stdout is not valid JSON: %v; output: %q", err, result.Stdout)
		} else {
			if resp["vin"] != "VIN_STATUS" {
				t.Errorf("expected vin 'VIN_STATUS', got %v", resp["vin"])
			}
			if resp["locked"] != true {
				t.Errorf("expected locked=true, got %v", resp["locked"])
			}
		}
	}
}

// TestE2E_CLI_UnlockCommand verifies the unlock command works end-to-end.
// Requirements: 03-REQ-6.1, 03-REQ-4.2
func TestE2E_CLI_UnlockCommand(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping e2e test in short mode")
	}
	skipIfNoMosquitto(t)

	root := repoRoot(t)
	port := freePort(t)

	buildCloudGateway(t, root)

	// Build the CLI binary
	cliBuildResult := execCommand(t, root, "mock/companion-app-cli", "go", "build", "-o", "cli-e2e-test", ".")
	if cliBuildResult.ExitCode != 0 {
		t.Fatalf("failed to build companion-app-cli: %s", cliBuildResult.Stderr)
	}
	cliBinPath := fmt.Sprintf("%s/mock/companion-app-cli/cli-e2e-test", root)
	t.Cleanup(func() { os.Remove(cliBinPath) })

	// Start the CLOUD_GATEWAY with Mosquitto
	gwEnv := append(os.Environ(),
		fmt.Sprintf("PORT=%d", port),
		"MQTT_BROKER_URL=tcp://localhost:1883",
		"AUTH_TOKEN=demo-token",
		"COMMAND_TIMEOUT=15s",
	)
	gwCmd, _, _ := startProcessWithOutput(t, root, "backend/cloud-gateway", gwEnv, "./cloud-gateway-test")
	t.Cleanup(func() {
		gwCmd.Process.Kill()
		gwCmd.Wait()
	})

	if !waitForPort(t, port, 5*time.Second) {
		t.Fatal("cloud-gateway did not start in time")
	}

	// Start a simulated vehicle responder
	startSimulatedResponder(t, root, "VIN_UNLOCK")
	time.Sleep(500 * time.Millisecond)

	// Execute the CLI unlock command
	gatewayURL := fmt.Sprintf("http://localhost:%d", port)
	result := execCommand(t, root, "mock/companion-app-cli",
		"./cli-e2e-test", "unlock",
		"--vin", "VIN_UNLOCK",
		"--token", "demo-token",
		"--gateway-url", gatewayURL)

	if result.ExitCode != 0 {
		t.Errorf("CLI unlock command failed: exit code %d\nstdout: %s\nstderr: %s",
			result.ExitCode, result.Stdout, result.Stderr)
	}

	if result.Stdout == "" {
		t.Error("expected stdout to contain response JSON, but it was empty")
	} else {
		var resp map[string]interface{}
		if err := json.Unmarshal([]byte(strings.TrimSpace(result.Stdout)), &resp); err != nil {
			t.Errorf("stdout is not valid JSON: %v; output: %q", err, result.Stdout)
		} else {
			if _, ok := resp["command_id"]; !ok {
				t.Error("response missing command_id field")
			}
			if resp["status"] != "success" {
				t.Errorf("expected status 'success', got %v", resp["status"])
			}
		}
	}
}

// TestE2E_CLI_CommandCorrelation verifies command_id is preserved through the
// full CLI -> CLOUD_GATEWAY -> MQTT -> response -> CLI cycle.
// Requirements: 03-REQ-6.3
func TestE2E_CLI_CommandCorrelation(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping e2e test in short mode")
	}
	skipIfNoMosquitto(t)

	root := repoRoot(t)
	port := freePort(t)

	buildCloudGateway(t, root)

	// Start the CLOUD_GATEWAY with Mosquitto
	gwEnv := append(os.Environ(),
		fmt.Sprintf("PORT=%d", port),
		"MQTT_BROKER_URL=tcp://localhost:1883",
		"AUTH_TOKEN=demo-token",
		"COMMAND_TIMEOUT=15s",
	)
	gwCmd, _, _ := startProcessWithOutput(t, root, "backend/cloud-gateway", gwEnv, "./cloud-gateway-test")
	t.Cleanup(func() {
		gwCmd.Process.Kill()
		gwCmd.Wait()
	})

	if !waitForPort(t, port, 5*time.Second) {
		t.Fatal("cloud-gateway did not start in time")
	}

	// Start a simulated vehicle responder
	startSimulatedResponder(t, root, "VIN_CORR")
	time.Sleep(500 * time.Millisecond)

	// Send a command via HTTP (not CLI since CLI generates its own UUID)
	// to verify command_id correlation
	baseURL := fmt.Sprintf("http://localhost:%d", port)
	originalID := "e2e-corr-test-xyz-789"
	body := fmt.Sprintf(`{"command_id":"%s","type":"lock","doors":["driver"]}`, originalID)
	statusCode, respBody, err := httpPostJSONWithTimeout(t, baseURL+"/vehicles/VIN_CORR/commands",
		body, "demo-token", 20*time.Second)
	if err != nil {
		t.Fatalf("HTTP POST failed: %v", err)
	}

	if statusCode != 200 {
		t.Errorf("expected status 200, got %d; body: %s", statusCode, respBody)
	}

	var resp map[string]interface{}
	if err := json.Unmarshal([]byte(respBody), &resp); err != nil {
		t.Fatalf("response is not valid JSON: %v; body: %s", err, respBody)
	}

	if resp["command_id"] != originalID {
		t.Errorf("command_id mismatch: expected %q, got %v", originalID, resp["command_id"])
	}
}
