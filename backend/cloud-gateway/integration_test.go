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

// setupIntegrationEnv creates a full integration test environment with embedded NATS,
// wired stores, subscriptions, and an httptest server.
// Returns the server, NATS connection for simulating vehicle-side messages,
// the command store, telemetry store, and a cleanup function.
func setupIntegrationEnv(t *testing.T) (
	*httptest.Server,
	*nats.Conn,
	*CommandStore,
	*TelemetryStore,
	func(),
) {
	t.Helper()

	ns, natsURL := startEmbeddedNATS(t)

	client, err := NewNATSClient(natsURL)
	if err != nil {
		ns.Shutdown()
		t.Fatalf("failed to create NATS client: %v", err)
	}

	tokenStore := NewTokenStore(demoTokens())
	commandStore := NewCommandStore()
	telemetryStore := NewTelemetryStore()
	knownVINs := demoKnownVINs()

	// Subscribe to command responses and telemetry for all known VINs
	for vin := range knownVINs {
		v := vin
		if err := client.SubscribeCommandResponses(v, func(resp NATSCommandResponse) {
			commandStore.UpdateCommandStatus(resp.CommandID, resp.Status, resp.Reason)
		}); err != nil {
			client.Close()
			ns.Shutdown()
			t.Fatalf("failed to subscribe to command responses for %s: %v", v, err)
		}

		if err := client.SubscribeTelemetry(v, func(data TelemetryData) {
			telemetryStore.StoreTelemetry(data.VIN, data)
		}); err != nil {
			client.Close()
			ns.Shutdown()
			t.Fatalf("failed to subscribe to telemetry for %s: %v", v, err)
		}
	}

	router := NewRouter(tokenStore, commandStore, client, knownVINs)
	server := httptest.NewServer(router)

	// Vehicle-side NATS connection for publishing responses
	vehicleNC, err := nats.Connect(natsURL)
	if err != nil {
		server.Close()
		client.Close()
		ns.Shutdown()
		t.Fatalf("failed to create vehicle NATS connection: %v", err)
	}

	cleanup := func() {
		vehicleNC.Close()
		server.Close()
		client.Close()
		ns.Shutdown()
	}

	return server, vehicleNC, commandStore, telemetryStore, cleanup
}

// TS-06-1, TS-06-4: End-to-end command flow test
// Submits a command via REST, verifies NATS publish, simulates NATS response,
// and queries status via REST.
func TestEndToEndCommandFlow(t *testing.T) {
	server, vehicleNC, _, _, cleanup := setupIntegrationEnv(t)
	defer cleanup()

	// Subscribe to capture the command on the vehicle side
	cmdCh := make(chan *nats.Msg, 1)
	sub, err := vehicleNC.Subscribe("vehicles.VIN12345.commands", func(msg *nats.Msg) {
		cmdCh <- msg
	})
	if err != nil {
		t.Fatalf("failed to subscribe to commands: %v", err)
	}
	defer sub.Unsubscribe()
	vehicleNC.Flush()

	// Step 1: Submit a command via REST
	cmdBody := `{"command_id":"e2e-cmd-001","type":"lock","doors":["driver","passenger"]}`
	resp, err := http.Post(
		server.URL+"/vehicles/VIN12345/commands",
		"application/json",
		bytes.NewBufferString(cmdBody),
	)
	if err != nil {
		t.Fatalf("failed to POST command: %v", err)
	}
	// Need to add auth - use a custom request
	resp.Body.Close()

	// Use proper authenticated request
	req, _ := http.NewRequest(http.MethodPost, server.URL+"/vehicles/VIN12345/commands",
		bytes.NewBufferString(cmdBody))
	req.Header.Set("Authorization", "Bearer companion-token-vehicle-1")
	req.Header.Set("Content-Type", "application/json")

	httpClient := &http.Client{}
	resp, err = httpClient.Do(req)
	if err != nil {
		t.Fatalf("failed to POST command: %v", err)
	}
	defer resp.Body.Close()

	// Verify HTTP 202 response
	if resp.StatusCode != http.StatusAccepted {
		t.Fatalf("expected 202 Accepted, got %d", resp.StatusCode)
	}

	var cmdResp CommandStatus
	if err := json.NewDecoder(resp.Body).Decode(&cmdResp); err != nil {
		t.Fatalf("failed to decode command response: %v", err)
	}
	if cmdResp.CommandID != "e2e-cmd-001" {
		t.Errorf("expected command_id 'e2e-cmd-001', got %q", cmdResp.CommandID)
	}
	if cmdResp.Status != "pending" {
		t.Errorf("expected status 'pending', got %q", cmdResp.Status)
	}

	// Step 2: Verify the NATS message was published
	select {
	case msg := <-cmdCh:
		if msg.Subject != "vehicles.VIN12345.commands" {
			t.Errorf("expected NATS subject 'vehicles.VIN12345.commands', got %q", msg.Subject)
		}
		var natsCmd NATSCommand
		if err := json.Unmarshal(msg.Data, &natsCmd); err != nil {
			t.Fatalf("failed to unmarshal NATS command: %v", err)
		}
		if natsCmd.CommandID != "e2e-cmd-001" {
			t.Errorf("expected command_id 'e2e-cmd-001', got %q", natsCmd.CommandID)
		}
		if natsCmd.Action != "lock" {
			t.Errorf("expected action 'lock', got %q", natsCmd.Action)
		}
		if natsCmd.Source != "companion_app" {
			t.Errorf("expected source 'companion_app', got %q", natsCmd.Source)
		}
		if len(natsCmd.Doors) != 2 || natsCmd.Doors[0] != "driver" || natsCmd.Doors[1] != "passenger" {
			t.Errorf("expected doors [driver, passenger], got %v", natsCmd.Doors)
		}
	case <-time.After(2 * time.Second):
		t.Fatal("timed out waiting for NATS command message")
	}

	// Step 3: Simulate a command response from the vehicle
	natsResp, _ := json.Marshal(NATSCommandResponse{
		CommandID: "e2e-cmd-001",
		Status:    "success",
	})
	if err := vehicleNC.Publish("vehicles.VIN12345.command_responses", natsResp); err != nil {
		t.Fatalf("failed to publish command response: %v", err)
	}
	vehicleNC.Flush()
	time.Sleep(200 * time.Millisecond)

	// Step 4: Query command status via REST
	statusReq, _ := http.NewRequest(http.MethodGet,
		server.URL+"/vehicles/VIN12345/commands/e2e-cmd-001", nil)
	statusReq.Header.Set("Authorization", "Bearer companion-token-vehicle-1")

	statusResp, err := httpClient.Do(statusReq)
	if err != nil {
		t.Fatalf("failed to GET command status: %v", err)
	}
	defer statusResp.Body.Close()

	if statusResp.StatusCode != http.StatusOK {
		t.Fatalf("expected 200 OK, got %d", statusResp.StatusCode)
	}

	var finalStatus CommandStatus
	if err := json.NewDecoder(statusResp.Body).Decode(&finalStatus); err != nil {
		t.Fatalf("failed to decode status response: %v", err)
	}
	if finalStatus.CommandID != "e2e-cmd-001" {
		t.Errorf("expected command_id 'e2e-cmd-001', got %q", finalStatus.CommandID)
	}
	if finalStatus.Status != "success" {
		t.Errorf("expected status 'success', got %q", finalStatus.Status)
	}
}

// TS-06-3: Multi-vehicle routing test
// Verifies commands for different VINs reach the correct NATS subjects and
// responses route back correctly.
func TestMultiVehicleRouting(t *testing.T) {
	server, vehicleNC, _, _, cleanup := setupIntegrationEnv(t)
	defer cleanup()

	// Subscribe to commands for both VINs
	cmdCh1 := make(chan *nats.Msg, 5)
	cmdCh2 := make(chan *nats.Msg, 5)

	sub1, err := vehicleNC.Subscribe("vehicles.VIN12345.commands", func(msg *nats.Msg) {
		cmdCh1 <- msg
	})
	if err != nil {
		t.Fatalf("failed to subscribe to VIN12345 commands: %v", err)
	}
	defer sub1.Unsubscribe()

	sub2, err := vehicleNC.Subscribe("vehicles.VIN67890.commands", func(msg *nats.Msg) {
		cmdCh2 <- msg
	})
	if err != nil {
		t.Fatalf("failed to subscribe to VIN67890 commands: %v", err)
	}
	defer sub2.Unsubscribe()
	vehicleNC.Flush()

	httpClient := &http.Client{}

	// Submit command for VIN12345
	req1, _ := http.NewRequest(http.MethodPost, server.URL+"/vehicles/VIN12345/commands",
		bytes.NewBufferString(`{"command_id":"multi-cmd-1","type":"lock","doors":["driver"]}`))
	req1.Header.Set("Authorization", "Bearer companion-token-vehicle-1")
	req1.Header.Set("Content-Type", "application/json")
	resp1, err := httpClient.Do(req1)
	if err != nil {
		t.Fatalf("failed to POST command for VIN12345: %v", err)
	}
	resp1.Body.Close()
	if resp1.StatusCode != http.StatusAccepted {
		t.Fatalf("expected 202 for VIN12345, got %d", resp1.StatusCode)
	}

	// Submit command for VIN67890
	req2, _ := http.NewRequest(http.MethodPost, server.URL+"/vehicles/VIN67890/commands",
		bytes.NewBufferString(`{"command_id":"multi-cmd-2","type":"unlock","doors":["passenger"]}`))
	req2.Header.Set("Authorization", "Bearer companion-token-vehicle-2")
	req2.Header.Set("Content-Type", "application/json")
	resp2, err := httpClient.Do(req2)
	if err != nil {
		t.Fatalf("failed to POST command for VIN67890: %v", err)
	}
	resp2.Body.Close()
	if resp2.StatusCode != http.StatusAccepted {
		t.Fatalf("expected 202 for VIN67890, got %d", resp2.StatusCode)
	}

	// Verify VIN12345 received only its command
	select {
	case msg := <-cmdCh1:
		var cmd NATSCommand
		json.Unmarshal(msg.Data, &cmd)
		if cmd.CommandID != "multi-cmd-1" {
			t.Errorf("VIN12345 expected command_id 'multi-cmd-1', got %q", cmd.CommandID)
		}
		if cmd.Action != "lock" {
			t.Errorf("VIN12345 expected action 'lock', got %q", cmd.Action)
		}
	case <-time.After(2 * time.Second):
		t.Fatal("timed out waiting for VIN12345 command")
	}

	// Verify VIN67890 received only its command
	select {
	case msg := <-cmdCh2:
		var cmd NATSCommand
		json.Unmarshal(msg.Data, &cmd)
		if cmd.CommandID != "multi-cmd-2" {
			t.Errorf("VIN67890 expected command_id 'multi-cmd-2', got %q", cmd.CommandID)
		}
		if cmd.Action != "unlock" {
			t.Errorf("VIN67890 expected action 'unlock', got %q", cmd.Action)
		}
	case <-time.After(2 * time.Second):
		t.Fatal("timed out waiting for VIN67890 command")
	}

	// Verify no cross-contamination
	select {
	case msg := <-cmdCh1:
		t.Errorf("unexpected extra message on VIN12345: %s", string(msg.Data))
	case <-time.After(200 * time.Millisecond):
		// expected
	}
	select {
	case msg := <-cmdCh2:
		t.Errorf("unexpected extra message on VIN67890: %s", string(msg.Data))
	case <-time.After(200 * time.Millisecond):
		// expected
	}

	// Simulate responses from both vehicles
	resp1Data, _ := json.Marshal(NATSCommandResponse{
		CommandID: "multi-cmd-1",
		Status:    "success",
	})
	resp2Data, _ := json.Marshal(NATSCommandResponse{
		CommandID: "multi-cmd-2",
		Status:    "failed",
		Reason:    "door ajar",
	})
	vehicleNC.Publish("vehicles.VIN12345.command_responses", resp1Data)
	vehicleNC.Publish("vehicles.VIN67890.command_responses", resp2Data)
	vehicleNC.Flush()
	time.Sleep(200 * time.Millisecond)

	// Query status for both commands
	statusReq1, _ := http.NewRequest(http.MethodGet,
		server.URL+"/vehicles/VIN12345/commands/multi-cmd-1", nil)
	statusReq1.Header.Set("Authorization", "Bearer companion-token-vehicle-1")
	statusResp1, err := httpClient.Do(statusReq1)
	if err != nil {
		t.Fatalf("failed to GET status for multi-cmd-1: %v", err)
	}
	defer statusResp1.Body.Close()

	var status1 CommandStatus
	json.NewDecoder(statusResp1.Body).Decode(&status1)
	if status1.Status != "success" {
		t.Errorf("expected status 'success' for multi-cmd-1, got %q", status1.Status)
	}

	statusReq2, _ := http.NewRequest(http.MethodGet,
		server.URL+"/vehicles/VIN67890/commands/multi-cmd-2", nil)
	statusReq2.Header.Set("Authorization", "Bearer companion-token-vehicle-2")
	statusResp2, err := httpClient.Do(statusReq2)
	if err != nil {
		t.Fatalf("failed to GET status for multi-cmd-2: %v", err)
	}
	defer statusResp2.Body.Close()

	var status2 CommandStatus
	json.NewDecoder(statusResp2.Body).Decode(&status2)
	if status2.Status != "failed" {
		t.Errorf("expected status 'failed' for multi-cmd-2, got %q", status2.Status)
	}
	if status2.Reason != "door ajar" {
		t.Errorf("expected reason 'door ajar' for multi-cmd-2, got %q", status2.Reason)
	}
}

// TS-06-E8: NATS disconnect recovery test
// Verifies 503 on NATS failure and recovery on reconnect.
func TestNATSDisconnectRecovery(t *testing.T) {
	ns, natsURL := startEmbeddedNATS(t)

	client, err := NewNATSClient(natsURL)
	if err != nil {
		ns.Shutdown()
		t.Fatalf("failed to create NATS client: %v", err)
	}

	tokenStore := NewTokenStore(demoTokens())
	commandStore := NewCommandStore()
	knownVINs := demoKnownVINs()
	router := NewRouter(tokenStore, commandStore, client, knownVINs)
	server := httptest.NewServer(router)
	defer server.Close()

	httpClient := &http.Client{}

	// Verify service works initially
	req1, _ := http.NewRequest(http.MethodPost, server.URL+"/vehicles/VIN12345/commands",
		bytes.NewBufferString(`{"command_id":"disc-cmd-1","type":"lock","doors":["driver"]}`))
	req1.Header.Set("Authorization", "Bearer companion-token-vehicle-1")
	req1.Header.Set("Content-Type", "application/json")
	resp1, err := httpClient.Do(req1)
	if err != nil {
		t.Fatalf("failed to POST initial command: %v", err)
	}
	resp1.Body.Close()
	if resp1.StatusCode != http.StatusAccepted {
		t.Fatalf("expected 202 initially, got %d", resp1.StatusCode)
	}

	// Shut down NATS server
	ns.Shutdown()
	time.Sleep(200 * time.Millisecond)

	// Verify 503 when NATS is down
	req2, _ := http.NewRequest(http.MethodPost, server.URL+"/vehicles/VIN12345/commands",
		bytes.NewBufferString(`{"command_id":"disc-cmd-2","type":"lock","doors":["driver"]}`))
	req2.Header.Set("Authorization", "Bearer companion-token-vehicle-1")
	req2.Header.Set("Content-Type", "application/json")
	resp2, err := httpClient.Do(req2)
	if err != nil {
		t.Fatalf("failed to POST command after disconnect: %v", err)
	}
	defer resp2.Body.Close()

	if resp2.StatusCode != http.StatusServiceUnavailable {
		t.Errorf("expected 503 when NATS is down, got %d", resp2.StatusCode)
	}

	var errResp ErrorResponse
	json.NewDecoder(resp2.Body).Decode(&errResp)
	if errResp.Error == "" {
		t.Error("expected non-empty error message about messaging service unavailable")
	}

	client.Close()
}

// TestConcurrentCommandSubmission verifies no race conditions when submitting
// multiple commands concurrently.
func TestConcurrentCommandSubmission(t *testing.T) {
	server, vehicleNC, _, _, cleanup := setupIntegrationEnv(t)
	defer cleanup()

	// Subscribe to capture all commands
	cmdCh := make(chan *nats.Msg, 50)
	sub, err := vehicleNC.Subscribe("vehicles.VIN12345.commands", func(msg *nats.Msg) {
		cmdCh <- msg
	})
	if err != nil {
		t.Fatalf("failed to subscribe: %v", err)
	}
	defer sub.Unsubscribe()
	vehicleNC.Flush()

	httpClient := &http.Client{}
	const numCommands = 20

	var wg sync.WaitGroup
	errors := make(chan error, numCommands)
	statuses := make(chan int, numCommands)

	for i := 0; i < numCommands; i++ {
		wg.Add(1)
		go func(idx int) {
			defer wg.Done()

			body, _ := json.Marshal(CommandRequest{
				CommandID: "concurrent-cmd-" + string(rune('A'+idx)),
				Type:      "lock",
				Doors:     []string{"driver"},
			})

			req, _ := http.NewRequest(http.MethodPost,
				server.URL+"/vehicles/VIN12345/commands",
				bytes.NewBuffer(body))
			req.Header.Set("Authorization", "Bearer companion-token-vehicle-1")
			req.Header.Set("Content-Type", "application/json")

			resp, err := httpClient.Do(req)
			if err != nil {
				errors <- err
				return
			}
			resp.Body.Close()
			statuses <- resp.StatusCode
		}(i)
	}

	wg.Wait()
	close(errors)
	close(statuses)

	// Check for HTTP errors
	for err := range errors {
		t.Errorf("concurrent request error: %v", err)
	}

	// All requests should succeed with 202
	for status := range statuses {
		if status != http.StatusAccepted {
			t.Errorf("expected 202 for concurrent request, got %d", status)
		}
	}

	// Verify we received all messages on NATS
	received := 0
	timeout := time.After(3 * time.Second)
	for received < numCommands {
		select {
		case <-cmdCh:
			received++
		case <-timeout:
			t.Errorf("timed out: received %d/%d NATS messages", received, numCommands)
			return
		}
	}

	if received != numCommands {
		t.Errorf("expected %d NATS messages, received %d", numCommands, received)
	}
}
