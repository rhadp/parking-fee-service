//go:build integration

package main

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"

	"parking-fee-service/backend/cloud-gateway/auth"
	"parking-fee-service/backend/cloud-gateway/handler"
	"parking-fee-service/backend/cloud-gateway/model"
	"parking-fee-service/backend/cloud-gateway/natsclient"
	"parking-fee-service/backend/cloud-gateway/store"

	"github.com/nats-io/nats.go"
)

func smokeConfig() *model.Config {
	return &model.Config{
		Port:                  8081,
		NatsURL:               "nats://localhost:4222",
		CommandTimeoutSeconds: 1, // short timeout for smoke tests
		Tokens: []model.TokenMapping{
			{Token: "demo-token-001", VIN: "VIN12345"},
		},
	}
}

func setupSmokeServer(t *testing.T, cfg *model.Config) (*httptest.Server, *natsclient.NATSClient, *store.Store) {
	t.Helper()

	nc, err := natsclient.Connect(cfg.NatsURL, 3)
	if err != nil {
		t.Fatalf("failed to connect to NATS: %v", err)
	}

	s := store.NewStore()
	if err := nc.SubscribeResponses(s); err != nil {
		t.Fatalf("SubscribeResponses error: %v", err)
	}
	if err := nc.SubscribeTelemetry(); err != nil {
		t.Fatalf("SubscribeTelemetry error: %v", err)
	}

	mux := http.NewServeMux()
	authMw := auth.Middleware(cfg)
	timeout := time.Duration(cfg.CommandTimeoutSeconds) * time.Second

	mux.Handle("POST /vehicles/{vin}/commands",
		authMw(handler.NewSubmitCommandHandler(nc, s, timeout)))
	mux.Handle("GET /vehicles/{vin}/commands/{command_id}",
		authMw(handler.NewGetCommandStatusHandler(s)))
	mux.Handle("GET /health", handler.HealthHandler())

	srv := httptest.NewServer(mux)
	t.Cleanup(func() {
		srv.Close()
		nc.Drain()
	})

	return srv, nc, s
}

// TS-06-SMOKE-1: End-to-End Command Flow
func TestEndToEndCommandFlow(t *testing.T) {
	cfg := smokeConfig()
	srv, _, _ := setupSmokeServer(t, cfg)

	// 1. Subscribe to commands on raw NATS
	rawConn, err := nats.Connect(cfg.NatsURL)
	if err != nil {
		t.Fatalf("failed to connect raw NATS: %v", err)
	}
	defer rawConn.Close()

	sub, err := rawConn.SubscribeSync("vehicles.VIN12345.commands")
	if err != nil {
		t.Fatalf("failed to subscribe: %v", err)
	}

	// 2. Submit command via REST
	body := `{"command_id":"smoke-001","type":"lock","doors":["driver"]}`
	req, _ := http.NewRequest("POST", srv.URL+"/vehicles/VIN12345/commands", strings.NewReader(body))
	req.Header.Set("Authorization", "Bearer demo-token-001")
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatalf("POST error: %v", err)
	}
	defer resp.Body.Close()
	if resp.StatusCode != 202 {
		t.Errorf("POST status = %d, want 202", resp.StatusCode)
	}

	// 3. Receive on NATS
	msg, err := sub.NextMsg(2 * time.Second)
	if err != nil {
		t.Fatalf("did not receive NATS message: %v", err)
	}
	var cmd model.Command
	if err := json.Unmarshal(msg.Data, &cmd); err != nil {
		t.Fatalf("failed to unmarshal command: %v", err)
	}
	if cmd.CommandID != "smoke-001" {
		t.Errorf("command_id = %q, want %q", cmd.CommandID, "smoke-001")
	}
	if msg.Header.Get("Authorization") != "Bearer demo-token-001" {
		t.Errorf("NATS Authorization = %q, want %q", msg.Header.Get("Authorization"), "Bearer demo-token-001")
	}

	// 4. Publish response via NATS
	cmdResp := model.CommandResponse{CommandID: "smoke-001", Status: "success"}
	respData, _ := json.Marshal(cmdResp)
	if err := rawConn.Publish("vehicles.VIN12345.command_responses", respData); err != nil {
		t.Fatalf("failed to publish response: %v", err)
	}
	rawConn.Flush()
	time.Sleep(200 * time.Millisecond)

	// 5. Query status via REST
	getReq, _ := http.NewRequest("GET", srv.URL+"/vehicles/VIN12345/commands/smoke-001", nil)
	getReq.Header.Set("Authorization", "Bearer demo-token-001")
	getResp, err := http.DefaultClient.Do(getReq)
	if err != nil {
		t.Fatalf("GET error: %v", err)
	}
	defer getResp.Body.Close()
	if getResp.StatusCode != 200 {
		t.Errorf("GET status = %d, want 200", getResp.StatusCode)
	}

	var result model.CommandResponse
	if err := json.NewDecoder(getResp.Body).Decode(&result); err != nil {
		t.Fatalf("failed to decode GET response: %v", err)
	}
	if result.CommandID != "smoke-001" {
		t.Errorf("command_id = %q, want %q", result.CommandID, "smoke-001")
	}
	if result.Status != "success" {
		t.Errorf("status = %q, want %q", result.Status, "success")
	}
}

// TS-06-SMOKE-2: Command Timeout End-to-End
func TestCommandTimeoutEndToEnd(t *testing.T) {
	cfg := smokeConfig()
	cfg.CommandTimeoutSeconds = 1 // 1 second timeout for fast test
	srv, _, _ := setupSmokeServer(t, cfg)

	// 1. Submit command via REST
	body := `{"command_id":"smoke-002","type":"unlock","doors":["driver"]}`
	req, _ := http.NewRequest("POST", srv.URL+"/vehicles/VIN12345/commands", strings.NewReader(body))
	req.Header.Set("Authorization", "Bearer demo-token-001")
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatalf("POST error: %v", err)
	}
	defer resp.Body.Close()
	if resp.StatusCode != 202 {
		t.Errorf("POST status = %d, want 202", resp.StatusCode)
	}

	// 2. Wait past timeout
	time.Sleep(2 * time.Second)

	// 3. Query status
	getReq, _ := http.NewRequest("GET", srv.URL+"/vehicles/VIN12345/commands/smoke-002", nil)
	getReq.Header.Set("Authorization", "Bearer demo-token-001")
	getResp, err := http.DefaultClient.Do(getReq)
	if err != nil {
		t.Fatalf("GET error: %v", err)
	}
	defer getResp.Body.Close()
	if getResp.StatusCode != 200 {
		t.Errorf("GET status = %d, want 200", getResp.StatusCode)
	}

	var result model.CommandResponse
	if err := json.NewDecoder(getResp.Body).Decode(&result); err != nil {
		t.Fatalf("failed to decode: %v", err)
	}
	if result.Status != "timeout" {
		t.Errorf("status = %q, want %q", result.Status, "timeout")
	}
}

// Compile-time check: NATSClient must satisfy handler.NATSPublisher.
var _ handler.NATSPublisher = (*natsclient.NATSClient)(nil)
