//go:build integration

package main

import (
	"bufio"
	"encoding/json"
	"fmt"
	"net/http"
	"os"
	"os/exec"
	"strings"
	"syscall"
	"testing"
	"time"

	"github.com/nats-io/nats.go"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
)

// startAndWaitForReady builds, starts the service binary with the given config,
// and blocks until the "ready" log line appears on stderr. It returns the
// running *exec.Cmd. The caller must stop the process (e.g. SIGTERM + Wait).
func startAndWaitForReady(t *testing.T, binPath, configPath string) *exec.Cmd {
	t.Helper()

	cmd := exec.Command(binPath)
	cmd.Env = append(os.Environ(), "CONFIG_PATH="+configPath)

	stderrPipe, err := cmd.StderrPipe()
	if err != nil {
		t.Fatalf("failed to create stderr pipe: %v", err)
	}

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start service: %v", err)
	}

	readyCh := make(chan struct{}, 1)
	go func() {
		scanner := bufio.NewScanner(stderrPipe)
		for scanner.Scan() {
			if strings.Contains(scanner.Text(), "ready") {
				select {
				case readyCh <- struct{}{}:
				default:
				}
			}
		}
	}()

	select {
	case <-readyCh:
		// Service is ready.
	case <-time.After(15 * time.Second):
		cmd.Process.Kill()
		cmd.Wait()
		t.Fatal("timed out waiting for service to become ready")
	}

	return cmd
}

// stopService sends SIGTERM and waits for the process to exit.
func stopService(t *testing.T, cmd *exec.Cmd) {
	t.Helper()
	cmd.Process.Signal(syscall.SIGTERM)
	done := make(chan error, 1)
	go func() { done <- cmd.Wait() }()
	select {
	case <-done:
	case <-time.After(10 * time.Second):
		cmd.Process.Kill()
	}
}

// ---------------------------------------------------------------------------
// TS-06-SMOKE-1: End-to-End Command Flow
// Description: Full flow: submit command via REST, receive on NATS, publish
// response on NATS, query status via REST.
// ---------------------------------------------------------------------------

func TestEndToEndCommandFlow(t *testing.T) {
	if !natsAvailable(t, "localhost:4222") {
		t.Skip("NATS not available at localhost:4222, skipping end-to-end smoke test")
	}

	binPath := buildBinary(t)
	port := getFreePort(t)
	configPath := writeTestConfig(t, port, "nats://localhost:4222", 30)

	cmd := startAndWaitForReady(t, binPath, configPath)
	defer stopService(t, cmd)

	baseURL := fmt.Sprintf("http://localhost:%d", port)

	// 1. Subscribe to vehicles.VIN12345.commands on NATS.
	nc, err := nats.Connect("nats://localhost:4222")
	if err != nil {
		t.Fatalf("failed to connect to NATS: %v", err)
	}
	defer nc.Close()

	sub, err := nc.SubscribeSync("vehicles.VIN12345.commands")
	if err != nil {
		t.Fatalf("failed to subscribe: %v", err)
	}
	defer sub.Unsubscribe()

	// 2. POST /vehicles/VIN12345/commands with command payload.
	body := `{"command_id":"smoke-001","type":"lock","doors":["driver"]}`
	postReq, _ := http.NewRequest("POST", baseURL+"/vehicles/VIN12345/commands",
		strings.NewReader(body))
	postReq.Header.Set("Authorization", "Bearer demo-token-001")
	postReq.Header.Set("Content-Type", "application/json")

	postResp, err := http.DefaultClient.Do(postReq)
	if err != nil {
		t.Fatalf("POST request failed: %v", err)
	}
	defer postResp.Body.Close()

	if postResp.StatusCode != 202 {
		t.Fatalf("expected POST status 202, got %d", postResp.StatusCode)
	}

	// 3. Receive the command on NATS subscriber and verify.
	msg, err := sub.NextMsg(2 * time.Second)
	if err != nil {
		t.Fatalf("did not receive NATS message within 2s: %v", err)
	}

	var receivedCmd model.Command
	if err := json.Unmarshal(msg.Data, &receivedCmd); err != nil {
		t.Fatalf("failed to unmarshal NATS command payload: %v", err)
	}
	if receivedCmd.CommandID != "smoke-001" {
		t.Errorf("expected command_id 'smoke-001', got %q", receivedCmd.CommandID)
	}

	// Verify Authorization header propagated to NATS message.
	authHeader := msg.Header.Get("Authorization")
	if authHeader != "Bearer demo-token-001" {
		t.Errorf("expected NATS Authorization header 'Bearer demo-token-001', got %q", authHeader)
	}

	// 4. Publish a success response to vehicles.VIN12345.command_responses.
	respJSON := `{"command_id":"smoke-001","status":"success"}`
	if err := nc.Publish("vehicles.VIN12345.command_responses", []byte(respJSON)); err != nil {
		t.Fatalf("failed to publish command response: %v", err)
	}
	nc.Flush()
	time.Sleep(200 * time.Millisecond) // allow the service to process the subscription

	// 5. GET /vehicles/VIN12345/commands/smoke-001 and verify success.
	getReq, _ := http.NewRequest("GET", baseURL+"/vehicles/VIN12345/commands/smoke-001", nil)
	getReq.Header.Set("Authorization", "Bearer demo-token-001")

	getResp, err := http.DefaultClient.Do(getReq)
	if err != nil {
		t.Fatalf("GET request failed: %v", err)
	}
	defer getResp.Body.Close()

	if getResp.StatusCode != 200 {
		t.Fatalf("expected GET status 200, got %d", getResp.StatusCode)
	}

	var result model.CommandResponse
	if err := json.NewDecoder(getResp.Body).Decode(&result); err != nil {
		t.Fatalf("failed to decode GET response: %v", err)
	}
	if result.CommandID != "smoke-001" {
		t.Errorf("expected command_id 'smoke-001', got %q", result.CommandID)
	}
	if result.Status != "success" {
		t.Errorf("expected status 'success', got %q", result.Status)
	}
}

// ---------------------------------------------------------------------------
// TS-06-SMOKE-2: Command Timeout End-to-End
// Description: Submit a command, do not send a response, verify timeout status.
// ---------------------------------------------------------------------------

func TestCommandTimeoutEndToEnd(t *testing.T) {
	if !natsAvailable(t, "localhost:4222") {
		t.Skip("NATS not available at localhost:4222, skipping timeout smoke test")
	}

	binPath := buildBinary(t)
	port := getFreePort(t)
	// Use a 1-second timeout for fast test execution.
	configPath := writeTestConfig(t, port, "nats://localhost:4222", 1)

	cmd := startAndWaitForReady(t, binPath, configPath)
	defer stopService(t, cmd)

	baseURL := fmt.Sprintf("http://localhost:%d", port)

	// 1. POST /vehicles/VIN12345/commands with a command that will time out.
	body := `{"command_id":"smoke-002","type":"unlock","doors":["driver"]}`
	postReq, _ := http.NewRequest("POST", baseURL+"/vehicles/VIN12345/commands",
		strings.NewReader(body))
	postReq.Header.Set("Authorization", "Bearer demo-token-001")
	postReq.Header.Set("Content-Type", "application/json")

	postResp, err := http.DefaultClient.Do(postReq)
	if err != nil {
		t.Fatalf("POST request failed: %v", err)
	}
	defer postResp.Body.Close()

	if postResp.StatusCode != 202 {
		t.Fatalf("expected POST status 202, got %d", postResp.StatusCode)
	}

	// 2. Wait past the 1-second timeout (no NATS response sent).
	time.Sleep(2 * time.Second)

	// 3. GET /vehicles/VIN12345/commands/smoke-002 and verify timeout status.
	getReq, _ := http.NewRequest("GET", baseURL+"/vehicles/VIN12345/commands/smoke-002", nil)
	getReq.Header.Set("Authorization", "Bearer demo-token-001")

	getResp, err := http.DefaultClient.Do(getReq)
	if err != nil {
		t.Fatalf("GET request failed: %v", err)
	}
	defer getResp.Body.Close()

	if getResp.StatusCode != 200 {
		t.Fatalf("expected GET status 200, got %d", getResp.StatusCode)
	}

	var result model.CommandResponse
	if err := json.NewDecoder(getResp.Body).Decode(&result); err != nil {
		t.Fatalf("failed to decode GET response: %v", err)
	}
	if result.CommandID != "smoke-002" {
		t.Errorf("expected command_id 'smoke-002', got %q", result.CommandID)
	}
	if result.Status != "timeout" {
		t.Errorf("expected status 'timeout', got %q", result.Status)
	}
}
