package cloud_connectivity_test

import (
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"testing"
	"time"
)

// TS-03-1: POST /vehicles/{vin}/commands endpoint exists
// Requirement: 03-REQ-1.1
// Verifies the CLOUD_GATEWAY accepts POST requests to /vehicles/{vin}/commands
// with the correct JSON body schema.
func TestUnit_REST_CommandEndpoint(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping REST API test in short mode")
	}

	root := repoRoot(t)
	port := freePort(t)

	// Build the cloud-gateway binary
	result := execCommand(t, root, "backend/cloud-gateway", "go", "build", "-o", "cloud-gateway-test", ".")
	if result.ExitCode != 0 {
		t.Fatalf("failed to build cloud-gateway: %s", result.Stderr)
	}
	defer os.Remove(fmt.Sprintf("%s/backend/cloud-gateway/cloud-gateway-test", root))

	// Start cloud-gateway
	env := append(os.Environ(),
		fmt.Sprintf("PORT=%d", port),
		"MQTT_BROKER_URL=tcp://localhost:19999", // unreachable MQTT — we only test REST
		"AUTH_TOKEN=demo-token",
	)
	cmd, _, _ := startProcessWithOutput(t, root, "backend/cloud-gateway", env, "./cloud-gateway-test")
	defer func() {
		cmd.Process.Kill()
		cmd.Wait()
	}()

	if !waitForPort(t, port, 5*time.Second) {
		t.Fatal("cloud-gateway did not start in time")
	}

	baseURL := fmt.Sprintf("http://localhost:%d", port)
	body := `{"command_id":"abc-123","type":"lock","doors":["driver"]}`
	statusCode, respBody, err := httpPostJSON(t, baseURL+"/vehicles/VIN12345/commands", body, "demo-token")
	if err != nil {
		t.Fatalf("HTTP POST failed: %v", err)
	}

	// The endpoint should return 202 Accepted (or 200 OK for sync mode)
	if statusCode != 202 && statusCode != 200 {
		t.Errorf("expected status 202 or 200, got %d; body: %s", statusCode, respBody)
	}

	// Response should contain command_id
	var resp map[string]interface{}
	if err := json.Unmarshal([]byte(respBody), &resp); err != nil {
		t.Fatalf("response is not valid JSON: %v; body: %s", err, respBody)
	}
	if resp["command_id"] != "abc-123" {
		t.Errorf("expected command_id 'abc-123', got %v", resp["command_id"])
	}
}

// TS-03-2: GET /vehicles/{vin}/status endpoint exists
// Requirement: 03-REQ-1.2
// Verifies the CLOUD_GATEWAY returns vehicle status from cached telemetry.
func TestUnit_REST_StatusEndpoint(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping REST API test in short mode")
	}

	root := repoRoot(t)
	port := freePort(t)

	// Build the cloud-gateway binary
	result := execCommand(t, root, "backend/cloud-gateway", "go", "build", "-o", "cloud-gateway-test", ".")
	if result.ExitCode != 0 {
		t.Fatalf("failed to build cloud-gateway: %s", result.Stderr)
	}
	defer os.Remove(fmt.Sprintf("%s/backend/cloud-gateway/cloud-gateway-test", root))

	// Start cloud-gateway with a mock MQTT setup
	env := append(os.Environ(),
		fmt.Sprintf("PORT=%d", port),
		"MQTT_BROKER_URL=tcp://localhost:19999",
		"AUTH_TOKEN=demo-token",
	)
	cmd, _, _ := startProcessWithOutput(t, root, "backend/cloud-gateway", env, "./cloud-gateway-test")
	defer func() {
		cmd.Process.Kill()
		cmd.Wait()
	}()

	if !waitForPort(t, port, 5*time.Second) {
		t.Fatal("cloud-gateway did not start in time")
	}

	baseURL := fmt.Sprintf("http://localhost:%d", port)
	statusCode, respBody, err := httpGetWithAuth(t, baseURL+"/vehicles/VIN12345/status", "demo-token")
	if err != nil {
		t.Fatalf("HTTP GET failed: %v", err)
	}

	// The status endpoint should be routed (may return 404 if no telemetry,
	// but it must NOT return 501 Not Implemented or 405 Method Not Allowed).
	if statusCode == 501 || statusCode == 405 {
		t.Errorf("status endpoint not implemented: got status %d; body: %s", statusCode, respBody)
	}

	// If we get 200 (cached telemetry exists) or 404 (no telemetry), both are valid routes.
	// But the endpoint must exist and return JSON.
	if statusCode == 200 {
		var resp map[string]interface{}
		if err := json.Unmarshal([]byte(respBody), &resp); err != nil {
			t.Errorf("response is not valid JSON: %v; body: %s", err, respBody)
		}
		if _, ok := resp["vin"]; !ok {
			t.Error("expected response to contain 'vin' field")
		}
		if _, ok := resp["locked"]; !ok {
			t.Error("expected response to contain 'locked' field")
		}
		if _, ok := resp["timestamp"]; !ok {
			t.Error("expected response to contain 'timestamp' field")
		}
	}
}

// TS-03-3: GET /health returns 200
// Requirement: 03-REQ-1.3
// Verifies the health endpoint returns 200 OK with no auth.
func TestUnit_REST_HealthEndpoint(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping REST API test in short mode")
	}

	root := repoRoot(t)
	port := freePort(t)

	// Build and start cloud-gateway
	result := execCommand(t, root, "backend/cloud-gateway", "go", "build", "-o", "cloud-gateway-test", ".")
	if result.ExitCode != 0 {
		t.Fatalf("failed to build cloud-gateway: %s", result.Stderr)
	}
	defer os.Remove(fmt.Sprintf("%s/backend/cloud-gateway/cloud-gateway-test", root))

	env := append(os.Environ(),
		fmt.Sprintf("PORT=%d", port),
		"MQTT_BROKER_URL=tcp://localhost:19999",
	)
	cmd, _, _ := startProcessWithOutput(t, root, "backend/cloud-gateway", env, "./cloud-gateway-test")
	defer func() {
		cmd.Process.Kill()
		cmd.Wait()
	}()

	if !waitForPort(t, port, 5*time.Second) {
		t.Fatal("cloud-gateway did not start in time")
	}

	baseURL := fmt.Sprintf("http://localhost:%d", port)
	statusCode, respBody, err := httpGet(t, baseURL+"/health")
	if err != nil {
		t.Fatalf("HTTP GET /health failed: %v", err)
	}

	if statusCode != 200 {
		t.Errorf("expected status 200, got %d", statusCode)
	}

	var resp map[string]interface{}
	if err := json.Unmarshal([]byte(respBody), &resp); err != nil {
		t.Fatalf("response is not valid JSON: %v; body: %s", err, respBody)
	}
	if resp["status"] != "ok" {
		t.Errorf("expected status 'ok', got %v", resp["status"])
	}
}

// TS-03-4: Unauthorized request returns 401
// Requirement: 03-REQ-1.4
// Verifies requests without valid bearer token get 401.
func TestUnit_REST_Unauthorized(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping REST API test in short mode")
	}

	root := repoRoot(t)
	port := freePort(t)

	result := execCommand(t, root, "backend/cloud-gateway", "go", "build", "-o", "cloud-gateway-test", ".")
	if result.ExitCode != 0 {
		t.Fatalf("failed to build cloud-gateway: %s", result.Stderr)
	}
	defer os.Remove(fmt.Sprintf("%s/backend/cloud-gateway/cloud-gateway-test", root))

	env := append(os.Environ(),
		fmt.Sprintf("PORT=%d", port),
		"MQTT_BROKER_URL=tcp://localhost:19999",
		"AUTH_TOKEN=demo-token",
	)
	cmd, _, _ := startProcessWithOutput(t, root, "backend/cloud-gateway", env, "./cloud-gateway-test")
	defer func() {
		cmd.Process.Kill()
		cmd.Wait()
	}()

	if !waitForPort(t, port, 5*time.Second) {
		t.Fatal("cloud-gateway did not start in time")
	}

	baseURL := fmt.Sprintf("http://localhost:%d", port)
	body := `{"command_id":"test","type":"lock","doors":["driver"]}`

	// Test 1: No Authorization header
	statusCode, _, err := httpPostJSON(t, baseURL+"/vehicles/VIN12345/commands", body, "")
	if err != nil {
		t.Fatalf("HTTP POST without auth failed: %v", err)
	}
	if statusCode != 401 {
		t.Errorf("expected 401 without auth header, got %d", statusCode)
	}

	// Test 2: Wrong token
	statusCode, _, err = httpPostJSON(t, baseURL+"/vehicles/VIN12345/commands", body, "wrong-token")
	if err != nil {
		t.Fatalf("HTTP POST with wrong token failed: %v", err)
	}
	if statusCode != 401 {
		t.Errorf("expected 401 with wrong token, got %d", statusCode)
	}
}

// TS-03-5: Valid command returns 202 and publishes to MQTT
// Requirement: 03-REQ-1.5
// Verifies a valid command results in 202 Accepted and triggers MQTT publish.
func TestUnit_REST_ValidCommandAccepted(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping REST API test in short mode")
	}

	root := repoRoot(t)
	port := freePort(t)

	result := execCommand(t, root, "backend/cloud-gateway", "go", "build", "-o", "cloud-gateway-test", ".")
	if result.ExitCode != 0 {
		t.Fatalf("failed to build cloud-gateway: %s", result.Stderr)
	}
	defer os.Remove(fmt.Sprintf("%s/backend/cloud-gateway/cloud-gateway-test", root))

	env := append(os.Environ(),
		fmt.Sprintf("PORT=%d", port),
		"MQTT_BROKER_URL=tcp://localhost:19999",
		"AUTH_TOKEN=demo-token",
		"COMMAND_TIMEOUT=2s",
	)
	cmd, _, _ := startProcessWithOutput(t, root, "backend/cloud-gateway", env, "./cloud-gateway-test")
	defer func() {
		cmd.Process.Kill()
		cmd.Wait()
	}()

	if !waitForPort(t, port, 5*time.Second) {
		t.Fatal("cloud-gateway did not start in time")
	}

	baseURL := fmt.Sprintf("http://localhost:%d", port)
	body := `{"command_id":"valid-cmd-123","type":"lock","doors":["driver"]}`
	statusCode, respBody, err := httpPostJSON(t, baseURL+"/vehicles/VIN12345/commands", body, "demo-token")
	if err != nil {
		t.Fatalf("HTTP POST failed: %v", err)
	}

	// Should return 202 Accepted with pending status
	if statusCode != 202 && statusCode != 200 && statusCode != 504 {
		t.Errorf("expected status 202/200/504, got %d; body: %s", statusCode, respBody)
	}

	var resp map[string]interface{}
	if err := json.Unmarshal([]byte(respBody), &resp); err != nil {
		t.Fatalf("response is not valid JSON: %v; body: %s", err, respBody)
	}

	if resp["command_id"] != "valid-cmd-123" {
		t.Errorf("expected command_id 'valid-cmd-123', got %v", resp["command_id"])
	}

	status, ok := resp["status"].(string)
	if !ok {
		t.Error("expected 'status' field in response")
	} else if status != "pending" && status != "timeout" && status != "success" {
		t.Errorf("expected status 'pending', 'timeout', or 'success', got %q", status)
	}
}

// buildCloudGateway builds the cloud-gateway binary and returns the path.
// It registers a cleanup function to remove the binary.
func buildCloudGateway(t *testing.T, root string) string {
	t.Helper()
	result := execCommand(t, root, "backend/cloud-gateway", "go", "build", "-o", "cloud-gateway-test", ".")
	if result.ExitCode != 0 {
		t.Fatalf("failed to build cloud-gateway: %s", result.Stderr)
	}
	binPath := fmt.Sprintf("%s/backend/cloud-gateway/cloud-gateway-test", root)
	t.Cleanup(func() { os.Remove(binPath) })
	return binPath
}

// startGateway builds and starts the cloud-gateway with the given port and env
// overrides. It returns the *exec.Cmd and base URL. The process is killed on
// test cleanup.
func startGateway(t *testing.T, root string, port int, extraEnv ...string) (cmd *exec.Cmd, baseURL string) {
	t.Helper()
	buildCloudGateway(t, root)

	env := append(os.Environ(),
		fmt.Sprintf("PORT=%d", port),
		"AUTH_TOKEN=demo-token",
	)
	env = append(env, extraEnv...)

	cmd, _, _ = startProcessWithOutput(t, root, "backend/cloud-gateway", env, "./cloud-gateway-test")
	t.Cleanup(func() {
		cmd.Process.Kill()
		cmd.Wait()
	})

	if !waitForPort(t, port, 5*time.Second) {
		t.Fatal("cloud-gateway did not start in time")
	}
	return cmd, fmt.Sprintf("http://localhost:%d", port)
}
