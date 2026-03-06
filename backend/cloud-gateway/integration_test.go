package main

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"sync"
	"testing"
	"time"

	"github.com/nats-io/nats.go"
)

// setupIntegrationServer creates a fully-wired HTTP server with embedded NATS
// for integration testing. Returns the httptest server, NATS client, embedded
// NATS test connection, command store, telemetry store, and a cleanup function.
func setupIntegrationServer(t *testing.T) (
	*httptest.Server,
	*NATSClient,
	*nats.Conn,
	*CommandStore,
	*TelemetryStore,
	func(),
) {
	t.Helper()

	// Start embedded NATS
	ns := startEmbeddedNATS(t)

	// Create stores
	commandStore := NewCommandStore()
	telemetryStore := NewTelemetryStore()

	// Create token store with demo tokens
	tokens := map[string]string{
		"companion-token-vehicle-1": "VIN12345",
		"companion-token-vehicle-2": "VIN67890",
	}
	tokenStore := NewTokenStore(tokens)

	knownVINs := map[string]bool{
		"VIN12345": true,
		"VIN67890": true,
	}

	// Create NATS client
	natsClient, err := NewNATSClient(ns.ClientURL())
	if err != nil {
		ns.Shutdown()
		t.Fatalf("failed to create NATS client: %v", err)
	}

	// Subscribe to command responses for known VINs
	for vin := range knownVINs {
		v := vin // capture loop variable
		if err := natsClient.SubscribeCommandResponses(v, func(resp NATSCommandResponse) {
			commandStore.UpdateCommandStatus(resp.CommandID, resp.Status, resp.Reason)
		}); err != nil {
			natsClient.Close()
			ns.Shutdown()
			t.Fatalf("failed to subscribe to command responses for %s: %v", v, err)
		}

		if err := natsClient.SubscribeTelemetry(v, func(data TelemetryData) {
			telemetryStore.StoreTelemetry(v, data)
		}); err != nil {
			natsClient.Close()
			ns.Shutdown()
			t.Fatalf("failed to subscribe to telemetry for %s: %v", v, err)
		}
	}

	// Build HTTP handler
	mux := http.NewServeMux()
	mux.HandleFunc("GET /health", HandleHealth)
	mux.Handle("POST /vehicles/{vin}/commands",
		AuthMiddleware(tokenStore, knownVINs)(HandleCommandSubmit(commandStore, natsClient, knownVINs)))
	mux.Handle("GET /vehicles/{vin}/commands/{command_id}",
		AuthMiddleware(tokenStore, knownVINs)(HandleCommandStatus(commandStore, knownVINs)))
	mux.HandleFunc("/", NotFoundHandler())

	handler := recoveryMiddleware(mux)
	ts := httptest.NewServer(handler)

	// Create a separate NATS connection for test assertions
	testNC, err := nats.Connect(ns.ClientURL())
	if err != nil {
		ts.Close()
		natsClient.Close()
		ns.Shutdown()
		t.Fatalf("failed to create test NATS connection: %v", err)
	}

	cleanup := func() {
		testNC.Close()
		ts.Close()
		natsClient.Close()
		ns.Shutdown()
	}

	return ts, natsClient, testNC, commandStore, telemetryStore, cleanup
}

// TS-06-1, TS-06-4: End-to-end command flow
// Submit command via REST, verify NATS publish, simulate NATS response, query status via REST.
func TestEndToEndCommandFlow(t *testing.T) {
	ts, _, testNC, _, _, cleanup := setupIntegrationServer(t)
	defer cleanup()

	// Subscribe to the command subject to capture the published NATS message
	cmdSub, err := testNC.SubscribeSync("vehicles.VIN12345.commands")
	if err != nil {
		t.Fatalf("failed to subscribe to commands: %v", err)
	}
	if err := testNC.Flush(); err != nil {
		t.Fatalf("failed to flush: %v", err)
	}

	// Step 1: Submit a command via REST
	cmdBody := `{"command_id":"e2e-cmd-001","type":"lock","doors":["driver","passenger"]}`
	req, err := http.NewRequest("POST", ts.URL+"/vehicles/VIN12345/commands", bytes.NewBufferString(cmdBody))
	if err != nil {
		t.Fatalf("failed to create request: %v", err)
	}
	req.Header.Set("Authorization", "Bearer companion-token-vehicle-1")
	req.Header.Set("Content-Type", "application/json")

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatalf("request failed: %v", err)
	}
	defer resp.Body.Close()

	// Verify HTTP 202
	if resp.StatusCode != http.StatusAccepted {
		t.Fatalf("expected status 202, got %d", resp.StatusCode)
	}

	var submitResult CommandStatus
	if err := json.NewDecoder(resp.Body).Decode(&submitResult); err != nil {
		t.Fatalf("failed to decode submit response: %v", err)
	}
	if submitResult.CommandID != "e2e-cmd-001" {
		t.Errorf("expected command_id 'e2e-cmd-001', got %q", submitResult.CommandID)
	}
	if submitResult.Status != "pending" {
		t.Errorf("expected status 'pending', got %q", submitResult.Status)
	}

	// Step 2: Verify the command was published to the correct NATS subject
	msg, err := cmdSub.NextMsg(2 * time.Second)
	if err != nil {
		t.Fatalf("did not receive NATS command message: %v", err)
	}
	if msg.Subject != "vehicles.VIN12345.commands" {
		t.Errorf("expected NATS subject 'vehicles.VIN12345.commands', got %q", msg.Subject)
	}

	var natsCmd NATSCommand
	if err := json.Unmarshal(msg.Data, &natsCmd); err != nil {
		t.Fatalf("failed to unmarshal NATS command: %v", err)
	}
	if natsCmd.CommandID != "e2e-cmd-001" {
		t.Errorf("NATS command_id: expected 'e2e-cmd-001', got %q", natsCmd.CommandID)
	}
	if natsCmd.Action != "lock" {
		t.Errorf("NATS action: expected 'lock', got %q", natsCmd.Action)
	}
	if len(natsCmd.Doors) != 2 || natsCmd.Doors[0] != "driver" || natsCmd.Doors[1] != "passenger" {
		t.Errorf("NATS doors: expected [driver, passenger], got %v", natsCmd.Doors)
	}
	if natsCmd.Source != "companion_app" {
		t.Errorf("NATS source: expected 'companion_app', got %q", natsCmd.Source)
	}

	// Step 3: Simulate command response from vehicle via NATS
	cmdResp := NATSCommandResponse{
		CommandID: "e2e-cmd-001",
		Status:    "success",
	}
	respJSON, _ := json.Marshal(cmdResp)
	if err := testNC.Publish("vehicles.VIN12345.command_responses", respJSON); err != nil {
		t.Fatalf("failed to publish command response: %v", err)
	}
	if err := testNC.Flush(); err != nil {
		t.Fatalf("failed to flush: %v", err)
	}

	// Wait for async processing
	time.Sleep(200 * time.Millisecond)

	// Step 4: Query command status via REST
	statusReq, err := http.NewRequest("GET", ts.URL+"/vehicles/VIN12345/commands/e2e-cmd-001", nil)
	if err != nil {
		t.Fatalf("failed to create status request: %v", err)
	}
	statusReq.Header.Set("Authorization", "Bearer companion-token-vehicle-1")

	statusResp, err := http.DefaultClient.Do(statusReq)
	if err != nil {
		t.Fatalf("status request failed: %v", err)
	}
	defer statusResp.Body.Close()

	if statusResp.StatusCode != http.StatusOK {
		t.Fatalf("expected status 200, got %d", statusResp.StatusCode)
	}

	var statusResult CommandStatus
	if err := json.NewDecoder(statusResp.Body).Decode(&statusResult); err != nil {
		t.Fatalf("failed to decode status response: %v", err)
	}
	if statusResult.CommandID != "e2e-cmd-001" {
		t.Errorf("expected command_id 'e2e-cmd-001', got %q", statusResult.CommandID)
	}
	if statusResult.Status != "success" {
		t.Errorf("expected status 'success', got %q", statusResult.Status)
	}
}

// TS-06-3: Multi-vehicle routing
// Submit commands for two different VINs, verify each reaches the correct NATS subject
// and responses route back correctly.
func TestMultiVehicleRouting(t *testing.T) {
	ts, _, testNC, _, _, cleanup := setupIntegrationServer(t)
	defer cleanup()

	// Subscribe to commands for both VINs
	sub1, err := testNC.SubscribeSync("vehicles.VIN12345.commands")
	if err != nil {
		t.Fatalf("failed to subscribe to VIN12345 commands: %v", err)
	}
	sub2, err := testNC.SubscribeSync("vehicles.VIN67890.commands")
	if err != nil {
		t.Fatalf("failed to subscribe to VIN67890 commands: %v", err)
	}
	if err := testNC.Flush(); err != nil {
		t.Fatalf("failed to flush: %v", err)
	}

	// Submit command for VIN12345
	cmd1Body := `{"command_id":"multi-cmd-1","type":"lock","doors":["driver"]}`
	req1, _ := http.NewRequest("POST", ts.URL+"/vehicles/VIN12345/commands", bytes.NewBufferString(cmd1Body))
	req1.Header.Set("Authorization", "Bearer companion-token-vehicle-1")
	req1.Header.Set("Content-Type", "application/json")

	resp1, err := http.DefaultClient.Do(req1)
	if err != nil {
		t.Fatalf("request 1 failed: %v", err)
	}
	resp1.Body.Close()
	if resp1.StatusCode != http.StatusAccepted {
		t.Fatalf("expected status 202 for VIN12345, got %d", resp1.StatusCode)
	}

	// Submit command for VIN67890
	cmd2Body := `{"command_id":"multi-cmd-2","type":"unlock","doors":["passenger"]}`
	req2, _ := http.NewRequest("POST", ts.URL+"/vehicles/VIN67890/commands", bytes.NewBufferString(cmd2Body))
	req2.Header.Set("Authorization", "Bearer companion-token-vehicle-2")
	req2.Header.Set("Content-Type", "application/json")

	resp2, err := http.DefaultClient.Do(req2)
	if err != nil {
		t.Fatalf("request 2 failed: %v", err)
	}
	resp2.Body.Close()
	if resp2.StatusCode != http.StatusAccepted {
		t.Fatalf("expected status 202 for VIN67890, got %d", resp2.StatusCode)
	}

	// Verify VIN12345 received only its command
	msg1, err := sub1.NextMsg(2 * time.Second)
	if err != nil {
		t.Fatalf("did not receive message on VIN12345 commands: %v", err)
	}
	if msg1.Subject != "vehicles.VIN12345.commands" {
		t.Errorf("expected subject 'vehicles.VIN12345.commands', got %q", msg1.Subject)
	}
	var natsCmd1 NATSCommand
	json.Unmarshal(msg1.Data, &natsCmd1)
	if natsCmd1.CommandID != "multi-cmd-1" {
		t.Errorf("VIN12345 command_id: expected 'multi-cmd-1', got %q", natsCmd1.CommandID)
	}
	if natsCmd1.Action != "lock" {
		t.Errorf("VIN12345 action: expected 'lock', got %q", natsCmd1.Action)
	}

	// Verify VIN67890 received only its command
	msg2, err := sub2.NextMsg(2 * time.Second)
	if err != nil {
		t.Fatalf("did not receive message on VIN67890 commands: %v", err)
	}
	if msg2.Subject != "vehicles.VIN67890.commands" {
		t.Errorf("expected subject 'vehicles.VIN67890.commands', got %q", msg2.Subject)
	}
	var natsCmd2 NATSCommand
	json.Unmarshal(msg2.Data, &natsCmd2)
	if natsCmd2.CommandID != "multi-cmd-2" {
		t.Errorf("VIN67890 command_id: expected 'multi-cmd-2', got %q", natsCmd2.CommandID)
	}
	if natsCmd2.Action != "unlock" {
		t.Errorf("VIN67890 action: expected 'unlock', got %q", natsCmd2.Action)
	}

	// Verify no cross-contamination
	_, err = sub1.NextMsg(200 * time.Millisecond)
	if err == nil {
		t.Error("received unexpected extra message on VIN12345 commands")
	}
	_, err = sub2.NextMsg(200 * time.Millisecond)
	if err == nil {
		t.Error("received unexpected extra message on VIN67890 commands")
	}

	// Simulate responses from both vehicles and verify correct routing
	resp1JSON, _ := json.Marshal(NATSCommandResponse{CommandID: "multi-cmd-1", Status: "success"})
	resp2JSON, _ := json.Marshal(NATSCommandResponse{CommandID: "multi-cmd-2", Status: "failed", Reason: "door ajar"})

	testNC.Publish("vehicles.VIN12345.command_responses", resp1JSON)
	testNC.Publish("vehicles.VIN67890.command_responses", resp2JSON)
	testNC.Flush()

	time.Sleep(200 * time.Millisecond)

	// Query status of command 1
	statusReq1, _ := http.NewRequest("GET", ts.URL+"/vehicles/VIN12345/commands/multi-cmd-1", nil)
	statusReq1.Header.Set("Authorization", "Bearer companion-token-vehicle-1")
	statusResp1, err := http.DefaultClient.Do(statusReq1)
	if err != nil {
		t.Fatalf("status request 1 failed: %v", err)
	}
	defer statusResp1.Body.Close()
	if statusResp1.StatusCode != http.StatusOK {
		t.Fatalf("expected status 200 for cmd-1, got %d", statusResp1.StatusCode)
	}
	var result1 CommandStatus
	json.NewDecoder(statusResp1.Body).Decode(&result1)
	if result1.Status != "success" {
		t.Errorf("cmd-1 status: expected 'success', got %q", result1.Status)
	}

	// Query status of command 2
	statusReq2, _ := http.NewRequest("GET", ts.URL+"/vehicles/VIN67890/commands/multi-cmd-2", nil)
	statusReq2.Header.Set("Authorization", "Bearer companion-token-vehicle-2")
	statusResp2, err := http.DefaultClient.Do(statusReq2)
	if err != nil {
		t.Fatalf("status request 2 failed: %v", err)
	}
	defer statusResp2.Body.Close()
	if statusResp2.StatusCode != http.StatusOK {
		t.Fatalf("expected status 200 for cmd-2, got %d", statusResp2.StatusCode)
	}
	var result2 CommandStatus
	json.NewDecoder(statusResp2.Body).Decode(&result2)
	if result2.Status != "failed" {
		t.Errorf("cmd-2 status: expected 'failed', got %q", result2.Status)
	}
	if result2.Reason != "door ajar" {
		t.Errorf("cmd-2 reason: expected 'door ajar', got %q", result2.Reason)
	}
}

// TS-06-E8: NATS disconnect - verify 503 on NATS failure
func TestNATSDisconnectRecovery(t *testing.T) {
	// Start embedded NATS
	ns := startEmbeddedNATS(t)

	tokens := map[string]string{
		"companion-token-vehicle-1": "VIN12345",
	}
	tokenStore := NewTokenStore(tokens)
	commandStore := NewCommandStore()
	knownVINs := map[string]bool{"VIN12345": true}

	natsClient, err := NewNATSClient(ns.ClientURL())
	if err != nil {
		ns.Shutdown()
		t.Fatalf("failed to create NATS client: %v", err)
	}

	mux := http.NewServeMux()
	mux.HandleFunc("GET /health", HandleHealth)
	mux.Handle("POST /vehicles/{vin}/commands",
		AuthMiddleware(tokenStore, knownVINs)(HandleCommandSubmit(commandStore, natsClient, knownVINs)))
	mux.HandleFunc("/", NotFoundHandler())

	handler := recoveryMiddleware(mux)
	ts := httptest.NewServer(handler)
	defer ts.Close()

	// First: verify the command works when NATS is up
	cmdBody := `{"command_id":"disconnect-cmd-1","type":"lock","doors":["driver"]}`
	req1, _ := http.NewRequest("POST", ts.URL+"/vehicles/VIN12345/commands", bytes.NewBufferString(cmdBody))
	req1.Header.Set("Authorization", "Bearer companion-token-vehicle-1")
	req1.Header.Set("Content-Type", "application/json")

	resp1, err := http.DefaultClient.Do(req1)
	if err != nil {
		t.Fatalf("first request failed: %v", err)
	}
	resp1.Body.Close()
	if resp1.StatusCode != http.StatusAccepted {
		t.Fatalf("expected status 202 when NATS is up, got %d", resp1.StatusCode)
	}

	// Shut down NATS server
	ns.Shutdown()

	// Give the client time to notice the disconnect
	time.Sleep(200 * time.Millisecond)

	// Second: verify the command returns 503 when NATS is down
	cmdBody2 := `{"command_id":"disconnect-cmd-2","type":"lock","doors":["driver"]}`
	req2, _ := http.NewRequest("POST", ts.URL+"/vehicles/VIN12345/commands", bytes.NewBufferString(cmdBody2))
	req2.Header.Set("Authorization", "Bearer companion-token-vehicle-1")
	req2.Header.Set("Content-Type", "application/json")

	resp2, err := http.DefaultClient.Do(req2)
	if err != nil {
		t.Fatalf("second request failed: %v", err)
	}
	defer resp2.Body.Close()

	if resp2.StatusCode != http.StatusServiceUnavailable {
		t.Errorf("expected status 503 when NATS is down, got %d", resp2.StatusCode)
	}

	var errResult ErrorResponse
	if err := json.NewDecoder(resp2.Body).Decode(&errResult); err != nil {
		t.Fatalf("failed to decode error response: %v", err)
	}
	if errResult.Error == "" {
		t.Error("expected non-empty error message")
	}
}

// TestConcurrentCommandSubmission verifies no race conditions when submitting
// multiple commands concurrently.
func TestConcurrentCommandSubmission(t *testing.T) {
	ts, _, _, commandStore, _, cleanup := setupIntegrationServer(t)
	defer cleanup()

	const numCommands = 20
	var wg sync.WaitGroup
	errors := make(chan error, numCommands)

	for i := range numCommands {
		wg.Add(1)
		go func(idx int) {
			defer wg.Done()

			cmdID := "concurrent-cmd-" + string(rune('A'+idx%26)) + "-" + time.Now().Format("150405.000000")
			body := `{"command_id":"` + cmdID + `","type":"lock","doors":["driver"]}`

			// Alternate between VIN12345 and VIN67890
			var url, token string
			if idx%2 == 0 {
				url = ts.URL + "/vehicles/VIN12345/commands"
				token = "Bearer companion-token-vehicle-1"
			} else {
				url = ts.URL + "/vehicles/VIN67890/commands"
				token = "Bearer companion-token-vehicle-2"
			}

			req, err := http.NewRequest("POST", url, bytes.NewBufferString(body))
			if err != nil {
				errors <- err
				return
			}
			req.Header.Set("Authorization", token)
			req.Header.Set("Content-Type", "application/json")

			resp, err := http.DefaultClient.Do(req)
			if err != nil {
				errors <- err
				return
			}
			resp.Body.Close()

			if resp.StatusCode != http.StatusAccepted {
				errors <- &statusError{expected: 202, got: resp.StatusCode}
			}
		}(i)
	}

	wg.Wait()
	close(errors)

	for err := range errors {
		t.Errorf("concurrent command error: %v", err)
	}

	// Verify that the command store has commands (at least some should be stored)
	// We can't predict exact count due to unique IDs, but none should have failed
	// Just verify the store is functional after concurrent access
	testCmd := "verify-after-concurrent"
	commandStore.StoreCommand(testCmd, "pending")
	status, ok := commandStore.GetCommandStatus(testCmd)
	if !ok {
		t.Error("command store broken after concurrent access")
	}
	if status.Status != "pending" {
		t.Errorf("expected status 'pending', got %q", status.Status)
	}
}

// statusError implements the error interface for HTTP status mismatches.
type statusError struct {
	expected int
	got      int
}

func (e *statusError) Error() string {
	return "expected status " + http.StatusText(e.expected) + ", got " + http.StatusText(e.got)
}
