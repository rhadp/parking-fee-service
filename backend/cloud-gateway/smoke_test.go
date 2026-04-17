//go:build integration

package main

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"os/exec"
	"strings"
	"testing"
	"time"

	nats "github.com/nats-io/nats.go"

	"parking-fee-service/backend/cloud-gateway/auth"
	"parking-fee-service/backend/cloud-gateway/handler"
	"parking-fee-service/backend/cloud-gateway/model"
	"parking-fee-service/backend/cloud-gateway/natsclient"
	"parking-fee-service/backend/cloud-gateway/store"
)

// smokeTestConfig returns a model.Config suitable for integration smoke tests.
func smokeTestConfig(timeoutSeconds int) *model.Config {
	return &model.Config{
		Port:                  8081,
		NatsURL:               "nats://localhost:4222",
		CommandTimeoutSeconds: timeoutSeconds,
		Tokens: []model.TokenMapping{
			{Token: "demo-token-001", VIN: "VIN12345"},
		},
	}
}

// skipIfNATSUnavailableSmoke skips the test if NATS is not available on localhost:4222.
func skipIfNATSUnavailableSmoke(t *testing.T) {
	t.Helper()
	if out, err := exec.Command("nc", "-z", "localhost", "4222").CombinedOutput(); err != nil {
		t.Skipf("NATS server not available on localhost:4222 (%v %s); skipping smoke test", err, out)
	}
}

// buildSmokeServer creates an httptest.Server with the full handler stack, wired to the
// provided natsclient and store. The server is registered under a Cleanup hook.
func buildSmokeServer(t *testing.T, nc *natsclient.NATSClient, s *store.Store, cfg *model.Config, timeout time.Duration) *httptest.Server {
	t.Helper()
	mux := http.NewServeMux()
	authMW := auth.Middleware(cfg)

	mux.HandleFunc("GET /health", handler.HealthHandler())
	mux.Handle("POST /vehicles/{vin}/commands",
		authMW(handler.NewSubmitCommandHandler(nc, s, timeout)))
	mux.Handle("GET /vehicles/{vin}/commands/{command_id}",
		authMW(handler.NewGetCommandStatusHandler(s)))

	srv := httptest.NewServer(mux)
	t.Cleanup(srv.Close)
	return srv
}

// TestEndToEndCommandFlow tests the full REST -> NATS -> response -> status query flow.
// Test Spec: TS-06-SMOKE-1
// Requirements: 06-REQ-1.1, 06-REQ-1.2, 06-REQ-2.1, 06-REQ-5.2
func TestEndToEndCommandFlow(t *testing.T) {
	skipIfNATSUnavailableSmoke(t)

	cfg := smokeTestConfig(30)
	timeout := 30 * time.Second

	// Connect to NATS for the server-side (gateway) client.
	nc, err := natsclient.Connect(cfg.NatsURL, 3)
	if err != nil {
		t.Skipf("NATS connect failed: %v", err)
	}
	t.Cleanup(func() { _ = nc.Drain() })

	s := store.NewStore()
	if err := nc.SubscribeResponses(s); err != nil {
		t.Fatalf("SubscribeResponses failed: %v", err)
	}

	// Raw NATS connection for the test subscriber (acting as CLOUD_GATEWAY_CLIENT).
	rawConn, err := nats.Connect(cfg.NatsURL)
	if err != nil {
		t.Fatalf("raw NATS connect failed: %v", err)
	}
	t.Cleanup(rawConn.Close)

	// Step 1: Subscribe to vehicles.VIN12345.commands to capture published commands.
	cmdCh := make(chan *nats.Msg, 1)
	sub, err := rawConn.ChanSubscribe("vehicles.VIN12345.commands", cmdCh)
	if err != nil {
		t.Fatalf("ChanSubscribe failed: %v", err)
	}
	t.Cleanup(func() { _ = sub.Unsubscribe() })
	// Flush to ensure the subscription is registered at the NATS server before
	// submitting the command, to avoid a race where the published message is missed.
	if err := rawConn.Flush(); err != nil {
		t.Fatalf("Flush after subscribe failed: %v", err)
	}

	// Build and start the full HTTP test server.
	srv := buildSmokeServer(t, nc, s, cfg, timeout)

	// Step 2: Submit command via REST.
	cmdBody := `{"command_id":"smoke-001","type":"lock","doors":["driver"]}`
	req, err := http.NewRequest(http.MethodPost, srv.URL+"/vehicles/VIN12345/commands", strings.NewReader(cmdBody))
	if err != nil {
		t.Fatalf("failed to build POST request: %v", err)
	}
	req.Header.Set("Authorization", "Bearer demo-token-001")
	req.Header.Set("Content-Type", "application/json")

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatalf("POST command failed: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusAccepted {
		t.Errorf("step 2: expected 202 Accepted, got %d", resp.StatusCode)
	}
	var postBody map[string]interface{}
	if err := json.NewDecoder(resp.Body).Decode(&postBody); err != nil {
		t.Fatalf("step 2: failed to decode POST response: %v", err)
	}
	if postBody["command_id"] != "smoke-001" {
		t.Errorf("step 2: expected command_id smoke-001 in POST response, got %v", postBody["command_id"])
	}

	// Step 3: Receive the command on the NATS subscriber; verify Authorization header.
	select {
	case msg := <-cmdCh:
		var cmd map[string]interface{}
		if err := json.Unmarshal(msg.Data, &cmd); err != nil {
			t.Fatalf("step 3: failed to unmarshal NATS command: %v", err)
		}
		if cmd["command_id"] != "smoke-001" {
			t.Errorf("step 3: NATS command_id: expected smoke-001, got %v", cmd["command_id"])
		}
		authHeader := msg.Header.Get("Authorization")
		if authHeader != "Bearer demo-token-001" {
			t.Errorf("step 3: NATS Authorization header: expected 'Bearer demo-token-001', got %q", authHeader)
		}
	case <-time.After(2 * time.Second):
		t.Fatal("step 3: timed out waiting for NATS command message")
	}

	// Step 4: Publish a success response to command_responses (simulating CLOUD_GATEWAY_CLIENT).
	responseData, err := json.Marshal(map[string]string{
		"command_id": "smoke-001",
		"status":     "success",
	})
	if err != nil {
		t.Fatalf("step 4: failed to marshal response: %v", err)
	}
	if err := rawConn.Publish("vehicles.VIN12345.command_responses", responseData); err != nil {
		t.Fatalf("step 4: failed to publish NATS response: %v", err)
	}
	// Allow time for the gateway subscription to process the response.
	time.Sleep(200 * time.Millisecond)

	// Step 5: Query status via REST.
	req2, err := http.NewRequest(http.MethodGet, srv.URL+"/vehicles/VIN12345/commands/smoke-001", nil)
	if err != nil {
		t.Fatalf("step 5: failed to build GET request: %v", err)
	}
	req2.Header.Set("Authorization", "Bearer demo-token-001")

	resp2, err := http.DefaultClient.Do(req2)
	if err != nil {
		t.Fatalf("step 5: GET command status failed: %v", err)
	}
	defer resp2.Body.Close()

	if resp2.StatusCode != http.StatusOK {
		t.Errorf("step 5: expected 200 OK, got %d", resp2.StatusCode)
	}
	var getBody map[string]interface{}
	if err := json.NewDecoder(resp2.Body).Decode(&getBody); err != nil {
		t.Fatalf("step 5: failed to decode GET response: %v", err)
	}
	if getBody["command_id"] != "smoke-001" {
		t.Errorf("step 5: expected command_id smoke-001, got %v", getBody["command_id"])
	}
	if getBody["status"] != "success" {
		t.Errorf("step 5: expected status success, got %v", getBody["status"])
	}
}

// TestCommandTimeoutEndToEnd tests that a command with no response receives "timeout" status.
// Test Spec: TS-06-SMOKE-2
// Requirements: 06-REQ-1.3, 06-REQ-2.1
func TestCommandTimeoutEndToEnd(t *testing.T) {
	skipIfNATSUnavailableSmoke(t)

	// Use 1-second timeout so the test completes quickly.
	cfg := smokeTestConfig(1)
	timeout := 1 * time.Second

	nc, err := natsclient.Connect(cfg.NatsURL, 3)
	if err != nil {
		t.Skipf("NATS connect failed: %v", err)
	}
	t.Cleanup(func() { _ = nc.Drain() })

	s := store.NewStore()
	if err := nc.SubscribeResponses(s); err != nil {
		t.Fatalf("SubscribeResponses failed: %v", err)
	}

	srv := buildSmokeServer(t, nc, s, cfg, timeout)

	// Step 1: Submit command via REST.
	cmdBody := `{"command_id":"smoke-002","type":"unlock","doors":["driver"]}`
	req, err := http.NewRequest(http.MethodPost, srv.URL+"/vehicles/VIN12345/commands", strings.NewReader(cmdBody))
	if err != nil {
		t.Fatalf("failed to build POST request: %v", err)
	}
	req.Header.Set("Authorization", "Bearer demo-token-001")
	req.Header.Set("Content-Type", "application/json")

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatalf("POST command failed: %v", err)
	}
	resp.Body.Close()

	if resp.StatusCode != http.StatusAccepted {
		t.Errorf("step 1: expected 202 Accepted, got %d", resp.StatusCode)
	}

	// Step 2: Wait past the timeout (timeout = 1s, wait 2s to be safe).
	time.Sleep(2 * time.Second)

	// Step 3: Query status — should be "timeout".
	req2, err := http.NewRequest(http.MethodGet, srv.URL+"/vehicles/VIN12345/commands/smoke-002", nil)
	if err != nil {
		t.Fatalf("failed to build GET request: %v", err)
	}
	req2.Header.Set("Authorization", "Bearer demo-token-001")

	resp2, err := http.DefaultClient.Do(req2)
	if err != nil {
		t.Fatalf("GET command status failed: %v", err)
	}
	defer resp2.Body.Close()

	if resp2.StatusCode != http.StatusOK {
		t.Errorf("step 3: expected 200 OK, got %d", resp2.StatusCode)
	}
	var getBody map[string]interface{}
	if err := json.NewDecoder(resp2.Body).Decode(&getBody); err != nil {
		t.Fatalf("step 3: failed to decode GET response: %v", err)
	}
	if getBody["command_id"] != "smoke-002" {
		t.Errorf("step 3: expected command_id smoke-002, got %v", getBody["command_id"])
	}
	if getBody["status"] != "timeout" {
		t.Errorf("step 3: expected status timeout, got %v", getBody["status"])
	}
}
