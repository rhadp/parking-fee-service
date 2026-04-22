//go:build integration

package main

import (
	"encoding/json"
	"fmt"
	"net"
	"net/http"
	"strings"
	"testing"
	"time"

	"github.com/nats-io/nats.go"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/auth"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/config"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/handler"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/natsclient"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/store"
)

// setupSmokeServer creates a fully wired HTTP server and NATS client for smoke tests.
// Returns the base URL, the NATSClient, the Store, and a cleanup function.
func setupSmokeServer(t *testing.T, timeoutSeconds int) (string, *natsclient.NATSClient, *store.Store, func()) {
	t.Helper()

	nc, err := natsclient.Connect("nats://localhost:4222", 3)
	if err != nil {
		t.Fatalf("failed to connect to NATS: %v", err)
	}

	s := store.NewStore()

	if err := nc.SubscribeResponses(s); err != nil {
		t.Fatalf("failed to subscribe to responses: %v", err)
	}
	if err := nc.SubscribeTelemetry(); err != nil {
		t.Fatalf("failed to subscribe to telemetry: %v", err)
	}

	cfg := &config.Config{
		Tokens: []config.TokenMapping{
			{Token: "demo-token-001", VIN: "VIN12345"},
			{Token: "demo-token-002", VIN: "VIN67890"},
		},
	}
	timeout := time.Duration(timeoutSeconds) * time.Second

	mux := http.NewServeMux()
	authMw := auth.Middleware(cfg)
	mux.Handle("POST /vehicles/{vin}/commands",
		authMw(handler.NewSubmitCommandHandler(nc, s, timeout)))
	mux.Handle("GET /vehicles/{vin}/commands/{command_id}",
		authMw(handler.NewGetCommandStatusHandler(s)))
	mux.Handle("GET /health", handler.HealthHandler())

	// Use a random available port.
	listener, err := net.Listen("tcp", ":0")
	if err != nil {
		t.Fatalf("failed to listen: %v", err)
	}
	srv := &http.Server{Handler: mux}
	go srv.Serve(listener) //nolint:errcheck

	baseURL := fmt.Sprintf("http://localhost:%d", listener.Addr().(*net.TCPAddr).Port)
	cleanup := func() {
		srv.Close()
		nc.Drain()
	}

	return baseURL, nc, s, cleanup
}

// TS-06-SMOKE-1: End-to-End Command Flow
// Full flow: submit command via REST, receive on NATS, publish response on NATS,
// query status via REST.
func TestEndToEndCommandFlow(t *testing.T) {
	baseURL, _, _, cleanup := setupSmokeServer(t, 30)
	defer cleanup()

	// Connect a raw NATS client for the "vehicle" side.
	rawNC, err := nats.Connect("nats://localhost:4222")
	if err != nil {
		t.Fatalf("failed to connect raw NATS client: %v", err)
	}
	defer rawNC.Close()

	// 1. Subscribe to commands on NATS (simulating vehicle).
	sub, err := rawNC.SubscribeSync("vehicles.VIN12345.commands")
	if err != nil {
		t.Fatalf("failed to subscribe: %v", err)
	}
	defer sub.Unsubscribe()

	// 2. Submit command via REST.
	body := `{"command_id":"smoke-001","type":"lock","doors":["driver"]}`
	req, err := http.NewRequest("POST", baseURL+"/vehicles/VIN12345/commands",
		strings.NewReader(body))
	if err != nil {
		t.Fatal(err)
	}
	req.Header.Set("Authorization", "Bearer demo-token-001")
	req.Header.Set("Content-Type", "application/json")

	postResp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatal(err)
	}
	defer postResp.Body.Close()

	if postResp.StatusCode != http.StatusAccepted {
		t.Fatalf("expected 202, got %d", postResp.StatusCode)
	}

	// 3. Receive command on NATS subscriber.
	msg, err := sub.NextMsg(2 * time.Second)
	if err != nil {
		t.Fatalf("did not receive NATS message: %v", err)
	}

	var cmd model.Command
	if err := json.Unmarshal(msg.Data, &cmd); err != nil {
		t.Fatalf("failed to unmarshal NATS command: %v", err)
	}
	if cmd.CommandID != "smoke-001" {
		t.Errorf("expected command_id 'smoke-001', got '%s'", cmd.CommandID)
	}

	// Verify Authorization header on NATS message.
	authHeader := msg.Header.Get("Authorization")
	if authHeader != "Bearer demo-token-001" {
		t.Errorf("expected Authorization header 'Bearer demo-token-001', got '%s'", authHeader)
	}

	// 4. Publish response to NATS (simulating vehicle response).
	respData, _ := json.Marshal(model.CommandResponse{
		CommandID: "smoke-001",
		Status:    "success",
	})
	if err := rawNC.Publish("vehicles.VIN12345.command_responses", respData); err != nil {
		t.Fatalf("failed to publish response: %v", err)
	}
	rawNC.Flush()

	// Allow processing time.
	time.Sleep(200 * time.Millisecond)

	// 5. Query status via REST.
	getReq, err := http.NewRequest("GET", baseURL+"/vehicles/VIN12345/commands/smoke-001", nil)
	if err != nil {
		t.Fatal(err)
	}
	getReq.Header.Set("Authorization", "Bearer demo-token-001")

	getResp, err := http.DefaultClient.Do(getReq)
	if err != nil {
		t.Fatal(err)
	}
	defer getResp.Body.Close()

	if getResp.StatusCode != http.StatusOK {
		t.Fatalf("expected 200, got %d", getResp.StatusCode)
	}

	var result model.CommandResponse
	if err := json.NewDecoder(getResp.Body).Decode(&result); err != nil {
		t.Fatalf("failed to decode status response: %v", err)
	}
	if result.CommandID != "smoke-001" {
		t.Errorf("expected command_id 'smoke-001', got '%s'", result.CommandID)
	}
	if result.Status != "success" {
		t.Errorf("expected status 'success', got '%s'", result.Status)
	}
}

// TS-06-SMOKE-2: Command Timeout End-to-End
// Submit a command, do not send a response, verify timeout status.
func TestCommandTimeoutEndToEnd(t *testing.T) {
	// Use a 1-second timeout for fast test.
	baseURL, _, _, cleanup := setupSmokeServer(t, 1)
	defer cleanup()

	// 1. Submit command via REST.
	body := `{"command_id":"smoke-002","type":"unlock","doors":["driver"]}`
	req, err := http.NewRequest("POST", baseURL+"/vehicles/VIN12345/commands",
		strings.NewReader(body))
	if err != nil {
		t.Fatal(err)
	}
	req.Header.Set("Authorization", "Bearer demo-token-001")
	req.Header.Set("Content-Type", "application/json")

	postResp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatal(err)
	}
	defer postResp.Body.Close()

	if postResp.StatusCode != http.StatusAccepted {
		t.Fatalf("expected 202, got %d", postResp.StatusCode)
	}

	// 2. Wait past the 1-second timeout.
	time.Sleep(2 * time.Second)

	// 3. Query status via REST.
	getReq, err := http.NewRequest("GET", baseURL+"/vehicles/VIN12345/commands/smoke-002", nil)
	if err != nil {
		t.Fatal(err)
	}
	getReq.Header.Set("Authorization", "Bearer demo-token-001")

	getResp, err := http.DefaultClient.Do(getReq)
	if err != nil {
		t.Fatal(err)
	}
	defer getResp.Body.Close()

	if getResp.StatusCode != http.StatusOK {
		t.Fatalf("expected 200, got %d", getResp.StatusCode)
	}

	var result model.CommandResponse
	if err := json.NewDecoder(getResp.Body).Decode(&result); err != nil {
		t.Fatalf("failed to decode status response: %v", err)
	}
	if result.Status != "timeout" {
		t.Errorf("expected status 'timeout', got '%s'", result.Status)
	}
}
