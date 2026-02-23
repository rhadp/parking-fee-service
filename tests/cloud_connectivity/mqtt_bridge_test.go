package cloud_connectivity_test

import (
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"testing"
	"time"
)

// TS-03-6: MQTT broker connection on startup
// Requirement: 03-REQ-2.1
// Verifies CLOUD_GATEWAY connects to MQTT broker on startup.
func TestIntegration_MQTT_Connect(t *testing.T) {
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
	)
	cmd, stdout, stderr := startProcessWithOutput(t, root, "backend/cloud-gateway", env, "./cloud-gateway-test")
	t.Cleanup(func() {
		cmd.Process.Kill()
		cmd.Wait()
	})

	if !waitForPort(t, port, 5*time.Second) {
		t.Fatalf("cloud-gateway did not start in time; stdout: %s; stderr: %s", stdout.String(), stderr.String())
	}

	// Verify health endpoint is responsive
	statusCode, _, err := httpGet(t, fmt.Sprintf("http://localhost:%d/health", port))
	if err != nil {
		t.Fatalf("health check failed: %v", err)
	}
	if statusCode != 200 {
		t.Errorf("expected health check 200, got %d", statusCode)
	}

	// The gateway should have logged an MQTT connection event.
	// We check for "connected" or "MQTT" in log output as a signal that the
	// MQTT client was initialized (not just the stub "not connected" message).
	combinedOutput := stdout.String() + stderr.String()
	if combinedOutput == "" {
		t.Error("expected some log output from cloud-gateway")
	}

	// The current stub only prints "MQTT client: not connected". After
	// implementation, it should print something indicating a successful connection.
	// This test will fail until the MQTT client is implemented.
	if !containsAny(combinedOutput, "mqtt connected", "MQTT connected", "connected to broker", "connected to mqtt") {
		t.Errorf("expected log output indicating MQTT connection, got: %s", combinedOutput)
	}
}

// TS-03-7: REST command publishes to correct MQTT topic
// Requirement: 03-REQ-2.2
// Verifies a REST command is published to the correct MQTT topic with the
// correct payload.
func TestIntegration_MQTT_PublishCommand(t *testing.T) {
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
		"COMMAND_TIMEOUT=5s",
	)
	cmd, _, _ := startProcessWithOutput(t, root, "backend/cloud-gateway", env, "./cloud-gateway-test")
	t.Cleanup(func() {
		cmd.Process.Kill()
		cmd.Wait()
	})

	if !waitForPort(t, port, 5*time.Second) {
		t.Fatal("cloud-gateway did not start in time")
	}

	// Subscribe to the MQTT topic using mosquitto_sub to capture the published command
	subResult := make(chan string, 1)
	subCmd := startMosquittoSub(t, root, "vehicles/VIN12345/commands", subResult)
	defer func() {
		subCmd.Process.Kill()
		subCmd.Wait()
	}()

	// Give the subscriber time to connect
	time.Sleep(500 * time.Millisecond)

	// Send a command via REST
	baseURL := fmt.Sprintf("http://localhost:%d", port)
	body := `{"command_id":"abc-123","type":"lock","doors":["driver"]}`
	_, _, err := httpPostJSONWithTimeout(t, baseURL+"/vehicles/VIN12345/commands", body, "demo-token", 10*time.Second)
	if err != nil {
		t.Logf("HTTP POST returned error (may be expected if command times out): %v", err)
	}

	// Wait for the MQTT message
	select {
	case msg := <-subResult:
		var payload map[string]interface{}
		if err := json.Unmarshal([]byte(msg), &payload); err != nil {
			t.Fatalf("MQTT message is not valid JSON: %v; msg: %s", err, msg)
		}
		if payload["command_id"] != "abc-123" {
			t.Errorf("expected command_id 'abc-123', got %v", payload["command_id"])
		}
		if payload["action"] != "lock" {
			t.Errorf("expected action 'lock', got %v", payload["action"])
		}
		if payload["source"] != "companion_app" {
			t.Errorf("expected source 'companion_app', got %v", payload["source"])
		}
	case <-time.After(10 * time.Second):
		t.Error("timed out waiting for MQTT message on vehicles/VIN12345/commands")
	}
}

// TS-03-8: Subscribe to command_responses topic
// Requirement: 03-REQ-2.3
// Verifies CLOUD_GATEWAY subscribes to command_responses and resolves pending
// commands.
func TestIntegration_MQTT_SubscribeResponses(t *testing.T) {
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

	// Start a simulated vehicle responder
	startSimulatedResponder(t, root, "VIN12345")

	// Send a command via REST
	baseURL := fmt.Sprintf("http://localhost:%d", port)
	body := `{"command_id":"resp-test-001","type":"unlock","doors":["driver"]}`
	statusCode, respBody, err := httpPostJSONWithTimeout(t, baseURL+"/vehicles/VIN12345/commands", body, "demo-token", 15*time.Second)
	if err != nil {
		t.Fatalf("HTTP POST failed: %v", err)
	}

	// The response should come back with status 200 and the command resolved
	if statusCode != 200 {
		t.Errorf("expected status 200 (resolved command), got %d; body: %s", statusCode, respBody)
	}

	var resp map[string]interface{}
	if err := json.Unmarshal([]byte(respBody), &resp); err != nil {
		t.Fatalf("response is not valid JSON: %v; body: %s", err, respBody)
	}
	if resp["status"] != "success" {
		t.Errorf("expected status 'success', got %v", resp["status"])
	}
}

// TS-03-9: Subscribe to telemetry topic
// Requirement: 03-REQ-2.4
// Verifies CLOUD_GATEWAY subscribes to telemetry and caches data for status
// endpoint.
func TestIntegration_MQTT_SubscribeTelemetry(t *testing.T) {
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
	)
	cmd, _, _ := startProcessWithOutput(t, root, "backend/cloud-gateway", env, "./cloud-gateway-test")
	t.Cleanup(func() {
		cmd.Process.Kill()
		cmd.Wait()
	})

	if !waitForPort(t, port, 5*time.Second) {
		t.Fatal("cloud-gateway did not start in time")
	}

	// Publish telemetry via mosquitto_pub
	telemetry := `{"vin":"VIN12345","locked":true,"timestamp":1708700000}`
	pubResult := execCommand(t, root, ".", "mosquitto_pub",
		"-h", "localhost", "-p", "1883",
		"-t", "vehicles/VIN12345/telemetry",
		"-m", telemetry)
	if pubResult.ExitCode != 0 {
		t.Fatalf("mosquitto_pub failed: %s", pubResult.Stderr)
	}

	// Wait for telemetry to be processed
	time.Sleep(1 * time.Second)

	// Query the status endpoint
	baseURL := fmt.Sprintf("http://localhost:%d", port)
	statusCode, respBody, err := httpGetWithAuth(t, baseURL+"/vehicles/VIN12345/status", "demo-token")
	if err != nil {
		t.Fatalf("HTTP GET /status failed: %v", err)
	}

	if statusCode != 200 {
		t.Errorf("expected status 200, got %d; body: %s", statusCode, respBody)
	}

	var resp map[string]interface{}
	if err := json.Unmarshal([]byte(respBody), &resp); err != nil {
		t.Fatalf("response is not valid JSON: %v; body: %s", err, respBody)
	}
	if resp["locked"] != true {
		t.Errorf("expected locked=true, got %v", resp["locked"])
	}
}

// TS-03-10: MQTT response resolves pending command
// Requirement: 03-REQ-2.5
// Verifies the Command Tracker correctly resolves a pending command when a
// matching MQTT response arrives.
func TestUnit_Bridge_ResolvesPending(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping bridge test in short mode")
	}

	root := repoRoot(t)

	// This test verifies the bridge/tracker module directly.
	// It runs the cloud-gateway unit tests that exercise the tracker.
	result := execCommand(t, root, "backend/cloud-gateway",
		"go", "test", "-v", "-count=1", "-run", "TestTracker_Resolve", "./internal/bridge/...")
	if result.ExitCode != 0 {
		t.Errorf("tracker resolve tests failed:\nstdout: %s\nstderr: %s", result.Stdout, result.Stderr)
	}
}

// TS-03-11: Command ID preserved in MQTT message
// Requirement: 03-REQ-3.1
// Verifies the command_id from REST is preserved in the MQTT message.
func TestUnit_Bridge_CommandIDPreserved(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping bridge test in short mode")
	}

	root := repoRoot(t)

	result := execCommand(t, root, "backend/cloud-gateway",
		"go", "test", "-v", "-count=1", "-run", "TestBridge_CommandIDPreserved", "./internal/bridge/...")
	if result.ExitCode != 0 {
		t.Errorf("command ID preservation tests failed:\nstdout: %s\nstderr: %s", result.Stdout, result.Stderr)
	}
}

// TS-03-12: Response matched by command_id
// Requirement: 03-REQ-3.2
// Verifies MQTT response is matched to the correct pending request using
// command_id.
func TestUnit_Bridge_ResponseMatchedByID(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping bridge test in short mode")
	}

	root := repoRoot(t)

	result := execCommand(t, root, "backend/cloud-gateway",
		"go", "test", "-v", "-count=1", "-run", "TestTracker_MatchByID", "./internal/bridge/...")
	if result.ExitCode != 0 {
		t.Errorf("response matching tests failed:\nstdout: %s\nstderr: %s", result.Stdout, result.Stderr)
	}
}

// TS-03-13: MQTT command message schema
// Requirement: 03-REQ-3.3
// Verifies the MQTT command message conforms to the expected JSON schema.
func TestUnit_Bridge_MQTTCommandSchema(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping bridge test in short mode")
	}

	root := repoRoot(t)

	result := execCommand(t, root, "backend/cloud-gateway",
		"go", "test", "-v", "-count=1", "-run", "TestBridge_CommandSchema", "./internal/bridge/...")
	if result.ExitCode != 0 {
		t.Errorf("MQTT command schema tests failed:\nstdout: %s\nstderr: %s", result.Stdout, result.Stderr)
	}
}

// TS-03-14: MQTT response message schema validation
// Requirement: 03-REQ-3.4
// Verifies the bridge correctly parses MQTT response messages conforming to
// the expected schema.
func TestUnit_Bridge_MQTTResponseSchema(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping bridge test in short mode")
	}

	root := repoRoot(t)

	result := execCommand(t, root, "backend/cloud-gateway",
		"go", "test", "-v", "-count=1", "-run", "TestBridge_ResponseSchema", "./internal/bridge/...")
	if result.ExitCode != 0 {
		t.Errorf("MQTT response schema tests failed:\nstdout: %s\nstderr: %s", result.Stdout, result.Stderr)
	}
}

// containsAny checks if s contains any of the given substrings (case-insensitive).
func containsAny(s string, subs ...string) bool {
	lower := toLower(s)
	for _, sub := range subs {
		if contains(lower, toLower(sub)) {
			return true
		}
	}
	return false
}

func toLower(s string) string {
	result := make([]byte, len(s))
	for i := 0; i < len(s); i++ {
		c := s[i]
		if c >= 'A' && c <= 'Z' {
			c += 'a' - 'A'
		}
		result[i] = c
	}
	return string(result)
}

func contains(s, sub string) bool {
	return len(sub) <= len(s) && (s == sub || len(sub) == 0 || findSubstring(s, sub))
}

func findSubstring(s, sub string) bool {
	for i := 0; i <= len(s)-len(sub); i++ {
		if s[i:i+len(sub)] == sub {
			return true
		}
	}
	return false
}

// startMosquittoSub starts a mosquitto_sub process that reads one message and
// sends it on the result channel.
func startMosquittoSub(t *testing.T, root, topic string, result chan<- string) *Cmd {
	t.Helper()

	// Use mosquitto_sub -C 1 to capture exactly one message
	cmd := newCmd("mosquitto_sub", "-h", "localhost", "-p", "1883", "-t", topic, "-C", "1")
	cmd.Dir = root

	go func() {
		out, err := cmd.Output()
		if err != nil {
			return
		}
		result <- string(out)
	}()

	return cmd
}

// startSimulatedResponder subscribes to commands on the given VIN's topic and
// auto-responds with success on the command_responses topic.
func startSimulatedResponder(t *testing.T, root, vin string) {
	t.Helper()

	// This uses mosquitto_sub and mosquitto_pub to simulate a vehicle responder.
	// Subscribe to commands, parse command_id, and publish response.
	go func() {
		// Subscribe to commands for this VIN
		subCmd := newCmd("mosquitto_sub", "-h", "localhost", "-p", "1883",
			"-t", fmt.Sprintf("vehicles/%s/commands", vin), "-C", "1")
		subCmd.Dir = root
		out, err := subCmd.Output()
		if err != nil {
			return
		}

		// Parse the command to extract command_id
		var cmd map[string]interface{}
		if err := json.Unmarshal(out, &cmd); err != nil {
			return
		}
		cmdID, _ := cmd["command_id"].(string)
		if cmdID == "" {
			return
		}

		// Publish a success response
		resp := fmt.Sprintf(`{"command_id":"%s","status":"success","reason":"","timestamp":%d}`,
			cmdID, time.Now().Unix())
		pubCmd := newCmd("mosquitto_pub", "-h", "localhost", "-p", "1883",
			"-t", fmt.Sprintf("vehicles/%s/command_responses", vin),
			"-m", resp)
		pubCmd.Dir = root
		pubCmd.Run()
	}()
}

// Cmd is a wrapper for exec.Cmd to avoid import conflicts in test helpers.
type Cmd = exec.Cmd

// newCmd creates a new exec.Cmd.
func newCmd(name string, args ...string) *exec.Cmd {
	return exec.Command(name, args...)
}
