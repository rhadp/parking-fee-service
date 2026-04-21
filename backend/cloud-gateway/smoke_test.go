//go:build integration

// Package main smoke tests for cloud-gateway.
// These tests require a running NATS server on localhost:4222.
package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"os/exec"
	"syscall"
	"testing"
	"time"

	"github.com/nats-io/nats.go"
)

const (
	smokeNATSURL  = "nats://localhost:4222"
	smokeToken    = "demo-token-001"
	smokeVIN      = "VIN12345"
	smokeGWPort   = 18090
	smokeGWPort2  = 18091
)

// probeNATS returns true if a NATS server is reachable at smokeNATSURL.
func probeNATS(t *testing.T) bool {
	t.Helper()
	nc, err := nats.Connect(smokeNATSURL, nats.Timeout(2*time.Second))
	if err != nil {
		return false
	}
	nc.Close()
	return true
}

// startGateway builds the binary, writes a config, starts the gateway as a subprocess,
// and waits until its /health endpoint responds. Returns the process and a cleanup function.
func startGateway(t *testing.T, port int, natsURL string, timeoutSeconds int) (*exec.Cmd, string) {
	t.Helper()

	binPath := buildBinary(t)

	// Write config with the given port and command_timeout_seconds.
	cfgPath := writeTestConfigFull(t, port, natsURL, timeoutSeconds)

	cmd := exec.Command(binPath)
	cmd.Env = append(os.Environ(), "CONFIG_PATH="+cfgPath)
	cmd.Stdout = io.Discard
	cmd.Stderr = io.Discard

	if err := cmd.Start(); err != nil {
		t.Fatalf("start gateway: %v", err)
	}

	baseURL := fmt.Sprintf("http://localhost:%d", port)

	// Wait for health endpoint to be ready.
	deadline := time.Now().Add(10 * time.Second)
	for time.Now().Before(deadline) {
		resp, err := http.Get(baseURL + "/health")
		if err == nil && resp.StatusCode == http.StatusOK {
			resp.Body.Close()
			break
		}
		if resp != nil {
			resp.Body.Close()
		}
		time.Sleep(100 * time.Millisecond)
	}

	t.Cleanup(func() {
		_ = cmd.Process.Signal(syscall.SIGTERM)
		_ = cmd.Wait()
	})

	return cmd, baseURL
}

// writeTestConfigFull writes a temporary config.json with full control over all fields.
func writeTestConfigFull(t *testing.T, port int, natsURL string, timeoutSeconds int) string {
	t.Helper()
	type tokenMapping struct {
		Token string `json:"token"`
		VIN   string `json:"vin"`
	}
	type cfg struct {
		Port                  int            `json:"port"`
		NatsURL               string         `json:"nats_url"`
		CommandTimeoutSeconds int            `json:"command_timeout_seconds"`
		Tokens                []tokenMapping `json:"tokens"`
	}
	c := cfg{
		Port:                  port,
		NatsURL:               natsURL,
		CommandTimeoutSeconds: timeoutSeconds,
		Tokens: []tokenMapping{
			{Token: smokeToken, VIN: smokeVIN},
		},
	}
	data, err := json.Marshal(c)
	if err != nil {
		t.Fatalf("marshal config: %v", err)
	}
	f, err := os.CreateTemp("", "cloud-gateway-smoke-*.json")
	if err != nil {
		t.Fatalf("create temp config: %v", err)
	}
	defer f.Close()
	if _, err := f.Write(data); err != nil {
		t.Fatalf("write config: %v", err)
	}
	t.Cleanup(func() { os.Remove(f.Name()) })
	return f.Name()
}

// TestEndToEndCommandFlow verifies the full REST->NATS->response flow (TS-06-SMOKE-1).
// Steps:
//  1. Subscribe to vehicles.VIN12345.commands on NATS.
//  2. POST /vehicles/VIN12345/commands -> expect HTTP 202.
//  3. Receive command on NATS subscriber -> verify content and Authorization header.
//  4. Publish response to vehicles.VIN12345.command_responses.
//  5. GET /vehicles/VIN12345/commands/{command_id} -> expect HTTP 200 with success.
func TestEndToEndCommandFlow(t *testing.T) {
	if !probeNATS(t) {
		t.Skip("NATS server not available, skipping TestEndToEndCommandFlow")
	}

	_, baseURL := startGateway(t, smokeGWPort, smokeNATSURL, 30)

	// Connect to NATS to subscribe and publish.
	nc, err := nats.Connect(smokeNATSURL)
	if err != nil {
		t.Fatalf("connect to NATS: %v", err)
	}
	defer nc.Close()

	// 1. Subscribe to command subject.
	commandSubject := fmt.Sprintf("vehicles.%s.commands", smokeVIN)
	sub, err := nc.SubscribeSync(commandSubject)
	if err != nil {
		t.Fatalf("subscribe to %s: %v", commandSubject, err)
	}
	defer sub.Unsubscribe()

	// 2. POST command via REST.
	commandID := "smoke-001"
	reqBody, _ := json.Marshal(map[string]interface{}{
		"command_id": commandID,
		"type":       "lock",
		"doors":      []string{"driver"},
	})
	req, err := http.NewRequest("POST",
		fmt.Sprintf("%s/vehicles/%s/commands", baseURL, smokeVIN),
		bytes.NewReader(reqBody))
	if err != nil {
		t.Fatalf("create POST request: %v", err)
	}
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Authorization", "Bearer "+smokeToken)

	client := &http.Client{Timeout: 5 * time.Second}
	postResp, err := client.Do(req)
	if err != nil {
		t.Fatalf("POST /vehicles/.../commands: %v", err)
	}
	defer postResp.Body.Close()

	if postResp.StatusCode != http.StatusAccepted {
		body, _ := io.ReadAll(postResp.Body)
		t.Fatalf("POST: expected 202, got %d: %s", postResp.StatusCode, body)
	}

	var postBody map[string]interface{}
	if err := json.NewDecoder(postResp.Body).Decode(&postBody); err != nil {
		t.Fatalf("decode POST response: %v", err)
	}
	if postBody["command_id"] != commandID {
		t.Errorf("POST response command_id: got %v, want %q", postBody["command_id"], commandID)
	}
	if postBody["type"] != "lock" {
		t.Errorf("POST response type: got %v, want %q", postBody["type"], "lock")
	}

	// 3. Receive command on NATS.
	msg, err := sub.NextMsg(3 * time.Second)
	if err != nil {
		t.Fatalf("NextMsg on %s: %v", commandSubject, err)
	}

	var natsCmd map[string]interface{}
	if err := json.Unmarshal(msg.Data, &natsCmd); err != nil {
		t.Fatalf("decode NATS command: %v", err)
	}
	if natsCmd["command_id"] != commandID {
		t.Errorf("NATS command_id: got %v, want %q", natsCmd["command_id"], commandID)
	}

	wantAuth := "Bearer " + smokeToken
	if got := msg.Header.Get("Authorization"); got != wantAuth {
		t.Errorf("NATS Authorization header: got %q, want %q", got, wantAuth)
	}

	// 4. Publish response on NATS.
	responseSubject := fmt.Sprintf("vehicles.%s.command_responses", smokeVIN)
	respPayload, _ := json.Marshal(map[string]string{
		"command_id": commandID,
		"status":     "success",
	})
	if err := nc.Publish(responseSubject, respPayload); err != nil {
		t.Fatalf("publish response: %v", err)
	}

	// Wait briefly for the gateway to process the response.
	time.Sleep(200 * time.Millisecond)

	// 5. GET command status.
	getReq, _ := http.NewRequest("GET",
		fmt.Sprintf("%s/vehicles/%s/commands/%s", baseURL, smokeVIN, commandID),
		nil)
	getReq.Header.Set("Authorization", "Bearer "+smokeToken)

	getResp, err := client.Do(getReq)
	if err != nil {
		t.Fatalf("GET /vehicles/.../commands/%s: %v", commandID, err)
	}
	defer getResp.Body.Close()

	if getResp.StatusCode != http.StatusOK {
		body, _ := io.ReadAll(getResp.Body)
		t.Fatalf("GET: expected 200, got %d: %s", getResp.StatusCode, body)
	}

	var result map[string]interface{}
	if err := json.NewDecoder(getResp.Body).Decode(&result); err != nil {
		t.Fatalf("decode GET response: %v", err)
	}
	if result["command_id"] != commandID {
		t.Errorf("GET command_id: got %v, want %q", result["command_id"], commandID)
	}
	if result["status"] != "success" {
		t.Errorf("GET status: got %v, want %q", result["status"], "success")
	}
}

// TestCommandTimeoutEndToEnd submits a command with no vehicle response and verifies
// the command status becomes "timeout" after the configured duration (TS-06-SMOKE-2).
// Steps:
//  1. POST /vehicles/VIN12345/commands -> expect HTTP 202.
//  2. Wait 2 seconds (past the 1s timeout).
//  3. GET /vehicles/VIN12345/commands/{command_id} -> expect status "timeout".
func TestCommandTimeoutEndToEnd(t *testing.T) {
	if !probeNATS(t) {
		t.Skip("NATS server not available, skipping TestCommandTimeoutEndToEnd")
	}

	// Use 1-second command timeout for a fast test.
	_, baseURL := startGateway(t, smokeGWPort2, smokeNATSURL, 1)

	commandID := "smoke-002"
	reqBody, _ := json.Marshal(map[string]interface{}{
		"command_id": commandID,
		"type":       "unlock",
		"doors":      []string{"driver"},
	})
	req, err := http.NewRequest("POST",
		fmt.Sprintf("%s/vehicles/%s/commands", baseURL, smokeVIN),
		bytes.NewReader(reqBody))
	if err != nil {
		t.Fatalf("create POST request: %v", err)
	}
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Authorization", "Bearer "+smokeToken)

	client := &http.Client{Timeout: 5 * time.Second}
	postResp, err := client.Do(req)
	if err != nil {
		t.Fatalf("POST /vehicles/.../commands: %v", err)
	}
	defer postResp.Body.Close()

	if postResp.StatusCode != http.StatusAccepted {
		body, _ := io.ReadAll(postResp.Body)
		t.Fatalf("POST: expected 202, got %d: %s", postResp.StatusCode, body)
	}
	io.Copy(io.Discard, postResp.Body)

	// 2. Wait past the 1-second timeout.
	time.Sleep(2 * time.Second)

	// 3. GET command status — expect timeout.
	getReq, _ := http.NewRequest("GET",
		fmt.Sprintf("%s/vehicles/%s/commands/%s", baseURL, smokeVIN, commandID),
		nil)
	getReq.Header.Set("Authorization", "Bearer "+smokeToken)

	getResp, err := client.Do(getReq)
	if err != nil {
		t.Fatalf("GET /vehicles/.../commands/%s: %v", commandID, err)
	}
	defer getResp.Body.Close()

	if getResp.StatusCode != http.StatusOK {
		body, _ := io.ReadAll(getResp.Body)
		t.Fatalf("GET: expected 200, got %d: %s", getResp.StatusCode, body)
	}

	var result map[string]interface{}
	if err := json.NewDecoder(getResp.Body).Decode(&result); err != nil {
		t.Fatalf("decode GET response: %v", err)
	}
	if result["command_id"] != commandID {
		t.Errorf("GET command_id: got %v, want %q", result["command_id"], commandID)
	}
	if result["status"] != "timeout" {
		t.Errorf("GET status: got %v, want %q", result["status"], "timeout")
	}
}
