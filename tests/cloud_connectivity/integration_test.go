package cloud_connectivity_test

import (
	"encoding/json"
	"fmt"
	"os"
	"strings"
	"sync"
	"testing"
	"time"
)

// TS-03-22: Multi-vehicle concurrent commands
// Requirement: 03-REQ-5.1
// Verifies concurrent commands for different VINs are routed to the correct
// MQTT topics.
func TestIntegration_MultiVehicleRouting(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping integration test in short mode")
	}
	skipIfNoMosquitto(t)

	root := repoRoot(t)
	port := freePort(t)

	buildCloudGateway(t, root)

	env := append(os.Environ(),
		fmt.Sprintf("PORT=%d", port),
		"MQTT_BROKER_URL=tcp://localhost:1883",
		"AUTH_TOKEN=demo-token",
		"COMMAND_TIMEOUT=10s",
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

	// Subscribe to both VINs' command topics
	resultA := make(chan string, 1)
	resultB := make(chan string, 1)

	subCmdA := startMosquittoSub(t, root, "vehicles/VIN_A/commands", resultA)
	subCmdB := startMosquittoSub(t, root, "vehicles/VIN_B/commands", resultB)
	defer func() {
		subCmdA.Process.Kill()
		subCmdA.Wait()
		subCmdB.Process.Kill()
		subCmdB.Wait()
	}()

	time.Sleep(500 * time.Millisecond) // let subscribers connect

	// Send concurrent commands for two different VINs
	bodyA := `{"command_id":"cmdA","type":"lock","doors":["driver"]}`
	bodyB := `{"command_id":"cmdB","type":"unlock","doors":["driver"]}`

	var wg sync.WaitGroup
	wg.Add(2)
	go func() {
		defer wg.Done()
		httpPostJSONWithTimeout(t, baseURL+"/vehicles/VIN_A/commands", bodyA, "demo-token", 15*time.Second)
	}()
	go func() {
		defer wg.Done()
		httpPostJSONWithTimeout(t, baseURL+"/vehicles/VIN_B/commands", bodyB, "demo-token", 15*time.Second)
	}()

	// Verify messages arrive on the correct topics
	timeout := time.After(10 * time.Second)

	var gotA, gotB bool
	for !gotA || !gotB {
		select {
		case msg := <-resultA:
			var payload map[string]interface{}
			if err := json.Unmarshal([]byte(msg), &payload); err != nil {
				t.Errorf("VIN_A message is not valid JSON: %v", err)
			} else {
				if payload["command_id"] != "cmdA" {
					t.Errorf("VIN_A: expected command_id 'cmdA', got %v", payload["command_id"])
				}
				if payload["action"] != "lock" {
					t.Errorf("VIN_A: expected action 'lock', got %v", payload["action"])
				}
			}
			gotA = true
		case msg := <-resultB:
			var payload map[string]interface{}
			if err := json.Unmarshal([]byte(msg), &payload); err != nil {
				t.Errorf("VIN_B message is not valid JSON: %v", err)
			} else {
				if payload["command_id"] != "cmdB" {
					t.Errorf("VIN_B: expected command_id 'cmdB', got %v", payload["command_id"])
				}
				if payload["action"] != "unlock" {
					t.Errorf("VIN_B: expected action 'unlock', got %v", payload["action"])
				}
			}
			gotB = true
		case <-timeout:
			if !gotA {
				t.Error("timed out waiting for message on vehicles/VIN_A/commands")
			}
			if !gotB {
				t.Error("timed out waiting for message on vehicles/VIN_B/commands")
			}
			goto done
		}
	}
done:
	wg.Wait()
}

// TS-03-23: Multi-vehicle response isolation
// Requirement: 03-REQ-5.2
// Verifies command response tracking is isolated per vehicle.
func TestUnit_MultiVehicle_ResponseIsolation(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping multi-vehicle test in short mode")
	}

	root := repoRoot(t)

	result := execCommand(t, root, "backend/cloud-gateway",
		"go", "test", "-v", "-count=1", "-run", "TestTracker_Isolation", "./internal/bridge/...")
	if result.ExitCode != 0 {
		t.Errorf("multi-vehicle response isolation tests failed:\nstdout: %s\nstderr: %s",
			result.Stdout, result.Stderr)
	}
}

// TS-03-24: End-to-end integration test
// Requirement: 03-REQ-6.1
// Full end-to-end test: HTTP POST -> MQTT pub -> simulated subscriber ->
// MQTT response -> HTTP response.
func TestIntegration_EndToEnd(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping integration test in short mode")
	}
	skipIfNoMosquitto(t)

	root := repoRoot(t)
	port := freePort(t)

	buildCloudGateway(t, root)

	env := append(os.Environ(),
		fmt.Sprintf("PORT=%d", port),
		"MQTT_BROKER_URL=tcp://localhost:1883",
		"AUTH_TOKEN=demo-token",
		"COMMAND_TIMEOUT=15s",
	)
	cmd, _, _ := startProcessWithOutput(t, root, "backend/cloud-gateway", env, "./cloud-gateway-test")
	t.Cleanup(func() {
		cmd.Process.Kill()
		cmd.Wait()
	})

	if !waitForPort(t, port, 5*time.Second) {
		t.Fatal("cloud-gateway did not start in time")
	}

	// Start a simulated vehicle responder that echoes commands as success responses
	startSimulatedResponder(t, root, "VIN12345")

	// Send a command via HTTP (simulating the CLI)
	baseURL := fmt.Sprintf("http://localhost:%d", port)
	body := `{"command_id":"e2e-test-001","type":"lock","doors":["driver"]}`
	statusCode, respBody, err := httpPostJSONWithTimeout(t, baseURL+"/vehicles/VIN12345/commands",
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

	if resp["command_id"] != "e2e-test-001" {
		t.Errorf("expected command_id 'e2e-test-001', got %v", resp["command_id"])
	}
	if resp["status"] != "success" {
		t.Errorf("expected status 'success', got %v", resp["status"])
	}
}

// TS-03-25: Integration test runs with go test
// Requirement: 03-REQ-6.2
// Verifies the integration test is executable via `go test`.
func TestIntegration_RunsWithGoTest(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping integration test in short mode")
	}

	root := repoRoot(t)

	// Run the cloud-gateway integration tests via go test
	// They should either pass (if Mosquitto is running) or skip (if not).
	result := execCommand(t, root, "backend/cloud-gateway",
		"go", "test", "-v", "-count=1", "-run", "TestIntegration", "./...")
	if result.ExitCode != 0 {
		// If tests failed, check if it's because integration tests don't exist yet
		if strings.Contains(result.Stderr, "no test files") ||
			strings.Contains(result.Stderr, "no Go files") ||
			strings.Contains(result.Stdout, "no test files") {
			t.Error("integration tests do not exist yet in backend/cloud-gateway")
		} else {
			t.Errorf("integration tests failed:\nstdout: %s\nstderr: %s",
				result.Stdout, result.Stderr)
		}
	}

	// Tests should either PASS or SKIP — never FAIL
	if strings.Contains(result.Stdout, "FAIL") && !strings.Contains(result.Stdout, "SKIP") {
		t.Errorf("integration tests should either pass or skip, but they failed:\n%s", result.Stdout)
	}
}

// TS-03-26: Integration test verifies command correlation
// Requirement: 03-REQ-6.3
// Verifies the integration test checks command_id correlation end-to-end.
func TestIntegration_CommandCorrelation(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping integration test in short mode")
	}
	skipIfNoMosquitto(t)

	root := repoRoot(t)
	port := freePort(t)

	buildCloudGateway(t, root)

	env := append(os.Environ(),
		fmt.Sprintf("PORT=%d", port),
		"MQTT_BROKER_URL=tcp://localhost:1883",
		"AUTH_TOKEN=demo-token",
		"COMMAND_TIMEOUT=15s",
	)
	cmd, _, _ := startProcessWithOutput(t, root, "backend/cloud-gateway", env, "./cloud-gateway-test")
	t.Cleanup(func() {
		cmd.Process.Kill()
		cmd.Wait()
	})

	if !waitForPort(t, port, 5*time.Second) {
		t.Fatal("cloud-gateway did not start in time")
	}

	startSimulatedResponder(t, root, "VIN12345")

	// Send a command with a specific command_id and verify it comes back
	originalID := "correlation-test-abc-123-xyz"
	baseURL := fmt.Sprintf("http://localhost:%d", port)
	body := fmt.Sprintf(`{"command_id":"%s","type":"unlock","doors":["driver"]}`, originalID)
	statusCode, respBody, err := httpPostJSONWithTimeout(t, baseURL+"/vehicles/VIN12345/commands",
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

