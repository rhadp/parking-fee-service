//go:build integration

package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"syscall"
	"testing"
	"time"

	"github.com/nats-io/nats.go"
)

// natsIntegrationURL returns the NATS server URL for integration tests.
func natsIntegrationURL() string {
	if url := os.Getenv("NATS_URL"); url != "" {
		return url
	}
	return "nats://localhost:4222"
}

// buildBinary builds the cloud-gateway binary into a temporary directory.
func buildBinary(t *testing.T) string {
	t.Helper()
	tmpDir := t.TempDir()
	binPath := filepath.Join(tmpDir, "cloud-gateway")
	cmd := exec.Command("go", "build", "-o", binPath, ".")
	cmd.Dir = filepath.Dir(mustFindGoMod(t))
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("failed to build binary: %v\n%s", err, out)
	}
	return binPath
}

// mustFindGoMod walks up from the current directory to find go.mod.
func mustFindGoMod(t *testing.T) string {
	t.Helper()
	dir, err := os.Getwd()
	if err != nil {
		t.Fatalf("failed to get working directory: %v", err)
	}
	for {
		candidate := filepath.Join(dir, "go.mod")
		if _, err := os.Stat(candidate); err == nil {
			return candidate
		}
		parent := filepath.Dir(dir)
		if parent == dir {
			t.Fatal("could not find go.mod in any parent directory")
		}
		dir = parent
	}
}

// writeTestConfig writes a test configuration file to a temporary directory
// and returns the path.
func writeTestConfig(t *testing.T, port int, timeoutSeconds int) string {
	t.Helper()
	cfg := map[string]any{
		"port":                    port,
		"nats_url":               natsIntegrationURL(),
		"command_timeout_seconds": timeoutSeconds,
		"tokens": []map[string]string{
			{"token": "demo-token-001", "vin": "VIN12345"},
			{"token": "demo-token-002", "vin": "VIN67890"},
		},
	}

	data, err := json.Marshal(cfg)
	if err != nil {
		t.Fatalf("failed to marshal config: %v", err)
	}

	tmpDir := t.TempDir()
	cfgPath := filepath.Join(tmpDir, "config.json")
	if err := os.WriteFile(cfgPath, data, 0644); err != nil {
		t.Fatalf("failed to write config: %v", err)
	}
	return cfgPath
}

// startGateway starts the cloud-gateway binary and returns the process.
// It waits for the service to be ready by polling the health endpoint.
func startGateway(t *testing.T, binPath, cfgPath string, port int) *exec.Cmd {
	t.Helper()
	cmd := exec.Command(binPath)
	cmd.Env = append(os.Environ(), "CONFIG_PATH="+cfgPath)
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start gateway: %v", err)
	}

	// Wait for the health endpoint to respond
	healthURL := fmt.Sprintf("http://localhost:%d/health", port)
	deadline := time.Now().Add(10 * time.Second)
	for time.Now().Before(deadline) {
		resp, err := http.Get(healthURL)
		if err == nil {
			resp.Body.Close()
			if resp.StatusCode == 200 {
				return cmd
			}
		}
		time.Sleep(100 * time.Millisecond)
	}
	cmd.Process.Kill()
	t.Fatal("gateway did not become ready within timeout")
	return nil
}

// TS-06-15: Startup Logging
// Requirement: 06-REQ-8.1
// On startup, the service logs port, NATS URL, and token count.
func TestStartupLogging(t *testing.T) {
	binPath := buildBinary(t)
	port := 18081
	cfgPath := writeTestConfig(t, port, 30)

	cmd := exec.Command(binPath)
	cmd.Env = append(os.Environ(), "CONFIG_PATH="+cfgPath)

	var stderr bytes.Buffer
	var stdout bytes.Buffer
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start gateway: %v", err)
	}

	// Give the service time to start and log
	time.Sleep(2 * time.Second)

	// Send SIGTERM to stop the service
	cmd.Process.Signal(syscall.SIGTERM)
	cmd.Wait()

	output := stdout.String() + stderr.String()

	if !strings.Contains(output, fmt.Sprintf("%d", port)) {
		t.Errorf("startup logs do not contain port %d: %s", port, output)
	}
	if !strings.Contains(output, "nats://") {
		t.Errorf("startup logs do not contain NATS URL: %s", output)
	}
	if !strings.Contains(strings.ToLower(output), "token") {
		t.Errorf("startup logs do not contain token reference: %s", output)
	}
}

// TS-06-14: Graceful Shutdown
// Requirement: 06-REQ-8.2
// On SIGTERM, the service drains NATS and exits with code 0.
func TestGracefulShutdown(t *testing.T) {
	binPath := buildBinary(t)
	port := 18082
	cfgPath := writeTestConfig(t, port, 30)

	cmd := exec.Command(binPath)
	cmd.Env = append(os.Environ(), "CONFIG_PATH="+cfgPath)

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start gateway: %v", err)
	}

	// Wait for the service to start
	healthURL := fmt.Sprintf("http://localhost:%d/health", port)
	deadline := time.Now().Add(10 * time.Second)
	ready := false
	for time.Now().Before(deadline) {
		resp, err := http.Get(healthURL)
		if err == nil {
			resp.Body.Close()
			if resp.StatusCode == 200 {
				ready = true
				break
			}
		}
		time.Sleep(100 * time.Millisecond)
	}
	if !ready {
		cmd.Process.Kill()
		t.Fatal("gateway did not become ready")
	}

	// Send SIGTERM
	if err := cmd.Process.Signal(syscall.SIGTERM); err != nil {
		t.Fatalf("failed to send SIGTERM: %v", err)
	}

	// Wait for the process to exit
	err := cmd.Wait()
	if err != nil {
		// Check if it's an exit error with non-zero code
		if exitErr, ok := err.(*exec.ExitError); ok {
			t.Errorf("expected exit code 0, got %d", exitErr.ExitCode())
		} else {
			t.Fatalf("unexpected error waiting for process: %v", err)
		}
	}
	// If err == nil, exit code was 0, which is what we want
}

// TS-06-SMOKE-1: End-to-End Command Flow
// Full flow: submit command via REST, receive on NATS, publish response on
// NATS, query status via REST.
func TestEndToEndCommandFlow(t *testing.T) {
	binPath := buildBinary(t)
	port := 18083
	cfgPath := writeTestConfig(t, port, 30)

	cmd := startGateway(t, binPath, cfgPath, port)
	defer func() {
		cmd.Process.Signal(syscall.SIGTERM)
		cmd.Wait()
	}()

	baseURL := fmt.Sprintf("http://localhost:%d", port)

	// 1. Subscribe to vehicles.VIN12345.commands on NATS
	nc, err := nats.Connect(natsIntegrationURL())
	if err != nil {
		t.Fatalf("failed to connect to NATS: %v", err)
	}
	defer nc.Close()

	sub, err := nc.SubscribeSync("vehicles.VIN12345.commands")
	if err != nil {
		t.Fatalf("failed to subscribe: %v", err)
	}
	defer sub.Unsubscribe()

	// 2. POST /vehicles/VIN12345/commands
	cmdBody := `{"command_id":"smoke-001","type":"lock","doors":["driver"]}`
	req, _ := http.NewRequest("POST", baseURL+"/vehicles/VIN12345/commands",
		strings.NewReader(cmdBody))
	req.Header.Set("Authorization", "Bearer demo-token-001")
	req.Header.Set("Content-Type", "application/json")

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatalf("POST request failed: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != 202 {
		t.Errorf("POST status = %d, want 202", resp.StatusCode)
	}

	// 3. Receive command on NATS subscriber
	msg, err := sub.NextMsg(2 * time.Second)
	if err != nil {
		t.Fatalf("did not receive NATS message: %v", err)
	}

	var receivedCmd map[string]any
	if err := json.Unmarshal(msg.Data, &receivedCmd); err != nil {
		t.Fatalf("failed to decode NATS command: %v", err)
	}
	if receivedCmd["command_id"] != "smoke-001" {
		t.Errorf("NATS command_id = %v, want %q", receivedCmd["command_id"], "smoke-001")
	}

	authHeader := msg.Header.Get("Authorization")
	if authHeader != "Bearer demo-token-001" {
		t.Errorf("NATS Authorization header = %q, want %q", authHeader, "Bearer demo-token-001")
	}

	// 4. Publish response to vehicles.VIN12345.command_responses
	respPayload := `{"command_id":"smoke-001","status":"success"}`
	if err := nc.Publish("vehicles.VIN12345.command_responses", []byte(respPayload)); err != nil {
		t.Fatalf("failed to publish response: %v", err)
	}
	nc.Flush()
	time.Sleep(200 * time.Millisecond)

	// 5. GET /vehicles/VIN12345/commands/smoke-001
	getReq, _ := http.NewRequest("GET", baseURL+"/vehicles/VIN12345/commands/smoke-001", nil)
	getReq.Header.Set("Authorization", "Bearer demo-token-001")

	getResp, err := http.DefaultClient.Do(getReq)
	if err != nil {
		t.Fatalf("GET request failed: %v", err)
	}
	defer getResp.Body.Close()

	if getResp.StatusCode != 200 {
		t.Errorf("GET status = %d, want 200", getResp.StatusCode)
	}

	var result map[string]string
	if err := json.NewDecoder(getResp.Body).Decode(&result); err != nil {
		t.Fatalf("failed to decode GET response: %v", err)
	}
	if result["command_id"] != "smoke-001" {
		t.Errorf("command_id = %q, want %q", result["command_id"], "smoke-001")
	}
	if result["status"] != "success" {
		t.Errorf("status = %q, want %q", result["status"], "success")
	}
}

// TS-06-SMOKE-2: Command Timeout End-to-End
// Submit a command, do not send a response, verify timeout status.
func TestCommandTimeoutEndToEnd(t *testing.T) {
	binPath := buildBinary(t)
	port := 18084
	// Use a 1-second timeout for fast test
	cfgPath := writeTestConfig(t, port, 1)

	cmd := startGateway(t, binPath, cfgPath, port)
	defer func() {
		cmd.Process.Signal(syscall.SIGTERM)
		cmd.Wait()
	}()

	baseURL := fmt.Sprintf("http://localhost:%d", port)

	// 1. POST /vehicles/VIN12345/commands
	cmdBody := `{"command_id":"smoke-002","type":"unlock","doors":["driver"]}`
	req, _ := http.NewRequest("POST", baseURL+"/vehicles/VIN12345/commands",
		strings.NewReader(cmdBody))
	req.Header.Set("Authorization", "Bearer demo-token-001")
	req.Header.Set("Content-Type", "application/json")

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatalf("POST request failed: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != 202 {
		t.Errorf("POST status = %d, want 202", resp.StatusCode)
	}

	// 2. Wait past the 1s timeout
	time.Sleep(2 * time.Second)

	// 3. GET /vehicles/VIN12345/commands/smoke-002
	getReq, _ := http.NewRequest("GET", baseURL+"/vehicles/VIN12345/commands/smoke-002", nil)
	getReq.Header.Set("Authorization", "Bearer demo-token-001")

	getResp, err := http.DefaultClient.Do(getReq)
	if err != nil {
		t.Fatalf("GET request failed: %v", err)
	}
	defer getResp.Body.Close()

	if getResp.StatusCode != 200 {
		t.Errorf("GET status = %d, want 200", getResp.StatusCode)
	}

	var result map[string]string
	if err := json.NewDecoder(getResp.Body).Decode(&result); err != nil {
		t.Fatalf("failed to decode GET response: %v", err)
	}
	if result["status"] != "timeout" {
		t.Errorf("status = %q, want %q", result["status"], "timeout")
	}
}
