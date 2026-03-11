package main

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"
	"time"

	natsserver "github.com/nats-io/nats-server/v2/server"
	"github.com/nats-io/nats.go"
)

// startEmbeddedNATS starts an embedded NATS server on a random port for testing.
func startEmbeddedNATS(t *testing.T) (*natsserver.Server, string) {
	t.Helper()
	opts := &natsserver.Options{
		Host: "127.0.0.1",
		Port: -1, // random port
	}
	ns, err := natsserver.NewServer(opts)
	if err != nil {
		t.Fatalf("failed to create embedded NATS server: %v", err)
	}
	ns.Start()
	if !ns.ReadyForConnections(5 * time.Second) {
		t.Fatal("NATS server not ready")
	}
	url := ns.ClientURL()
	return ns, url
}

// getNATSAddr returns the NATS client URL from an embedded NATS server.
func getNATSAddr(ns *natsserver.Server) string {
	return ns.ClientURL()
}

// TS-06-1: Command Submission via REST (integration with NATS)
func TestCommandSubmission(t *testing.T) {
	ns, _ := startEmbeddedNATS(t)
	defer ns.Shutdown()
	natsURL := getNATSAddr(ns)

	client, err := NewNATSClient(natsURL)
	if err != nil {
		t.Fatalf("failed to create NATS client: %v", err)
	}
	defer client.Close()

	if !client.IsConnected() {
		t.Fatal("NATS client not connected")
	}

	// Subscribe to the command subject to capture published messages
	nc, err := nats.Connect(natsURL)
	if err != nil {
		t.Fatalf("failed to connect subscriber: %v", err)
	}
	defer nc.Close()

	msgCh := make(chan *nats.Msg, 1)
	sub, err := nc.Subscribe("vehicles.VIN12345.commands", func(msg *nats.Msg) {
		msgCh <- msg
	})
	if err != nil {
		t.Fatalf("failed to subscribe: %v", err)
	}
	defer sub.Unsubscribe()
	nc.Flush()

	// Create router with real NATS client
	tokenStore := NewTokenStore(demoTokens())
	commandStore := NewCommandStore()
	knownVINs := demoKnownVINs()
	router := NewRouter(tokenStore, commandStore, client, knownVINs)

	body := `{"command_id":"cmd-001","type":"lock","doors":["driver"]}`
	req := httptest.NewRequest(http.MethodPost, "/vehicles/VIN12345/commands", bytes.NewBufferString(body))
	req.Header.Set("Authorization", "Bearer companion-token-vehicle-1")
	req.Header.Set("Content-Type", "application/json")
	rec := httptest.NewRecorder()
	router.ServeHTTP(rec, req)

	// Verify HTTP response
	if rec.Code != http.StatusAccepted {
		t.Errorf("expected status 202, got %d (body: %s)", rec.Code, rec.Body.String())
	}

	var resp CommandStatus
	if err := json.NewDecoder(rec.Body).Decode(&resp); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}
	if resp.CommandID != "cmd-001" {
		t.Errorf("expected command_id 'cmd-001', got %q", resp.CommandID)
	}
	if resp.Status != "pending" {
		t.Errorf("expected status 'pending', got %q", resp.Status)
	}

	// Verify NATS message
	select {
	case msg := <-msgCh:
		if msg.Subject != "vehicles.VIN12345.commands" {
			t.Errorf("expected subject 'vehicles.VIN12345.commands', got %q", msg.Subject)
		}
		var natsCmd NATSCommand
		if err := json.Unmarshal(msg.Data, &natsCmd); err != nil {
			t.Fatalf("failed to unmarshal NATS message: %v", err)
		}
		if natsCmd.Action != "lock" {
			t.Errorf("expected action 'lock', got %q", natsCmd.Action)
		}
		if natsCmd.Source != "companion_app" {
			t.Errorf("expected source 'companion_app', got %q", natsCmd.Source)
		}
		if natsCmd.CommandID != "cmd-001" {
			t.Errorf("expected command_id 'cmd-001', got %q", natsCmd.CommandID)
		}
	case <-time.After(2 * time.Second):
		t.Error("timed out waiting for NATS message")
	}
}

// TS-06-3: NATS Command Relay Publishes to Correct Subject
func TestNATSCommandRelay(t *testing.T) {
	ns, _ := startEmbeddedNATS(t)
	defer ns.Shutdown()
	natsURL := getNATSAddr(ns)

	client, err := NewNATSClient(natsURL)
	if err != nil {
		t.Fatalf("failed to create NATS client: %v", err)
	}
	defer client.Close()

	// Subscribe to both VIN subjects
	nc, err := nats.Connect(natsURL)
	if err != nil {
		t.Fatalf("failed to connect subscriber: %v", err)
	}
	defer nc.Close()

	msgCh1 := make(chan *nats.Msg, 5)
	msgCh2 := make(chan *nats.Msg, 5)

	sub1, err := nc.Subscribe("vehicles.VIN12345.commands", func(msg *nats.Msg) {
		msgCh1 <- msg
	})
	if err != nil {
		t.Fatalf("failed to subscribe to VIN12345: %v", err)
	}
	defer sub1.Unsubscribe()

	sub2, err := nc.Subscribe("vehicles.VIN67890.commands", func(msg *nats.Msg) {
		msgCh2 <- msg
	})
	if err != nil {
		t.Fatalf("failed to subscribe to VIN67890: %v", err)
	}
	defer sub2.Unsubscribe()
	nc.Flush()

	tokenStore := NewTokenStore(demoTokens())
	commandStore := NewCommandStore()
	knownVINs := demoKnownVINs()
	router := NewRouter(tokenStore, commandStore, client, knownVINs)

	// Submit command for VIN12345
	body1 := `{"command_id":"cmd-r1","type":"lock","doors":["driver"]}`
	req1 := httptest.NewRequest(http.MethodPost, "/vehicles/VIN12345/commands", bytes.NewBufferString(body1))
	req1.Header.Set("Authorization", "Bearer companion-token-vehicle-1")
	req1.Header.Set("Content-Type", "application/json")
	rec1 := httptest.NewRecorder()
	router.ServeHTTP(rec1, req1)

	// Submit command for VIN67890
	body2 := `{"command_id":"cmd-r2","type":"unlock","doors":["passenger"]}`
	req2 := httptest.NewRequest(http.MethodPost, "/vehicles/VIN67890/commands", bytes.NewBufferString(body2))
	req2.Header.Set("Authorization", "Bearer companion-token-vehicle-2")
	req2.Header.Set("Content-Type", "application/json")
	rec2 := httptest.NewRecorder()
	router.ServeHTTP(rec2, req2)

	// Verify VIN12345 message
	select {
	case msg := <-msgCh1:
		if msg.Subject != "vehicles.VIN12345.commands" {
			t.Errorf("expected subject 'vehicles.VIN12345.commands', got %q", msg.Subject)
		}
	case <-time.After(2 * time.Second):
		t.Error("timed out waiting for VIN12345 NATS message")
	}

	// Verify VIN67890 message
	select {
	case msg := <-msgCh2:
		if msg.Subject != "vehicles.VIN67890.commands" {
			t.Errorf("expected subject 'vehicles.VIN67890.commands', got %q", msg.Subject)
		}
	case <-time.After(2 * time.Second):
		t.Error("timed out waiting for VIN67890 NATS message")
	}

	// Verify no cross-contamination
	select {
	case msg := <-msgCh1:
		t.Errorf("unexpected extra message on VIN12345 subject: %s", string(msg.Data))
	case <-time.After(100 * time.Millisecond):
		// expected: no extra messages
	}

	select {
	case msg := <-msgCh2:
		t.Errorf("unexpected extra message on VIN67890 subject: %s", string(msg.Data))
	case <-time.After(100 * time.Millisecond):
		// expected: no extra messages
	}
}

// TS-06-4: NATS Response Subscription
func TestNATSResponseSubscription(t *testing.T) {
	ns, _ := startEmbeddedNATS(t)
	defer ns.Shutdown()
	natsURL := getNATSAddr(ns)

	client, err := NewNATSClient(natsURL)
	if err != nil {
		t.Fatalf("failed to create NATS client: %v", err)
	}
	defer client.Close()

	commandStore := NewCommandStore()
	commandStore.StoreCommand("cmd-003", "pending")

	// Subscribe to command responses and wire to command store
	err = client.SubscribeCommandResponses("VIN12345", func(resp NATSCommandResponse) {
		commandStore.UpdateCommandStatus(resp.CommandID, resp.Status, resp.Reason)
	})
	if err != nil {
		t.Fatalf("failed to subscribe to responses: %v", err)
	}

	// Publish a response from the "vehicle" side
	nc, err := nats.Connect(natsURL)
	if err != nil {
		t.Fatalf("failed to connect publisher: %v", err)
	}
	defer nc.Close()

	respData, _ := json.Marshal(NATSCommandResponse{
		CommandID: "cmd-003",
		Status:    "success",
	})
	err = nc.Publish("vehicles.VIN12345.command_responses", respData)
	if err != nil {
		t.Fatalf("failed to publish response: %v", err)
	}
	nc.Flush()

	// Wait for processing
	time.Sleep(200 * time.Millisecond)

	// Verify command status was updated
	status, found := commandStore.GetCommandStatus("cmd-003")
	if !found {
		t.Fatal("command not found in store")
	}
	if status.Status != "success" {
		t.Errorf("expected status 'success', got %q", status.Status)
	}
}

// TS-06-6: Telemetry Reception
func TestTelemetryReception(t *testing.T) {
	ns, _ := startEmbeddedNATS(t)
	defer ns.Shutdown()
	natsURL := getNATSAddr(ns)

	client, err := NewNATSClient(natsURL)
	if err != nil {
		t.Fatalf("failed to create NATS client: %v", err)
	}
	defer client.Close()

	telemetryStore := NewTelemetryStore()

	err = client.SubscribeTelemetry("VIN12345", func(data TelemetryData) {
		telemetryStore.StoreTelemetry(data.VIN, data)
	})
	if err != nil {
		t.Fatalf("failed to subscribe to telemetry: %v", err)
	}

	// Publish telemetry
	nc, err := nats.Connect(natsURL)
	if err != nil {
		t.Fatalf("failed to connect publisher: %v", err)
	}
	defer nc.Close()

	telemetry := TelemetryData{
		VIN:           "VIN12345",
		DoorLocked:    true,
		Latitude:      48.1351,
		Longitude:     11.582,
		ParkingActive: false,
		Timestamp:     1709654400,
	}
	data, _ := json.Marshal(telemetry)
	err = nc.Publish("vehicles.VIN12345.telemetry", data)
	if err != nil {
		t.Fatalf("failed to publish telemetry: %v", err)
	}
	nc.Flush()

	time.Sleep(200 * time.Millisecond)

	stored, found := telemetryStore.GetTelemetry("VIN12345")
	if !found {
		t.Fatal("telemetry not found in store")
	}
	if !stored.DoorLocked {
		t.Error("expected DoorLocked to be true")
	}
	if stored.Latitude != 48.1351 {
		t.Errorf("expected Latitude 48.1351, got %f", stored.Latitude)
	}
}

// TS-06-E8: NATS Unavailable Returns 503
func TestNATSUnavailable(t *testing.T) {
	// Create a NATS client with an invalid URL (no server running)
	client, err := NewNATSClient("nats://127.0.0.1:14222")
	if err != nil {
		// If the client fails to connect, that's also acceptable behavior.
		// We test that the handler returns 503 in this case.
		t.Logf("NATS client connection failed as expected: %v", err)
	}

	tokenStore := NewTokenStore(demoTokens())
	commandStore := NewCommandStore()
	knownVINs := demoKnownVINs()
	router := NewRouter(tokenStore, commandStore, client, knownVINs)

	body := `{"command_id":"cmd-e8","type":"lock","doors":["driver"]}`
	req := httptest.NewRequest(http.MethodPost, "/vehicles/VIN12345/commands", bytes.NewBufferString(body))
	req.Header.Set("Authorization", "Bearer companion-token-vehicle-1")
	req.Header.Set("Content-Type", "application/json")
	rec := httptest.NewRecorder()
	router.ServeHTTP(rec, req)

	if rec.Code != http.StatusServiceUnavailable {
		t.Errorf("expected status 503 when NATS unavailable, got %d (body: %s)", rec.Code, rec.Body.String())
	}

	var errResp ErrorResponse
	if err := json.NewDecoder(rec.Body).Decode(&errResp); err != nil {
		t.Fatalf("failed to decode error response: %v", err)
	}
	if errResp.Error == "" {
		t.Error("expected non-empty error message about messaging service unavailable")
	}
}

// TS-06-E10: Invalid Telemetry JSON Is Discarded
func TestInvalidTelemetryJSON(t *testing.T) {
	ns, _ := startEmbeddedNATS(t)
	defer ns.Shutdown()
	natsURL := getNATSAddr(ns)

	client, err := NewNATSClient(natsURL)
	if err != nil {
		t.Fatalf("failed to create NATS client: %v", err)
	}
	defer client.Close()

	telemetryStore := NewTelemetryStore()

	err = client.SubscribeTelemetry("VIN12345", func(data TelemetryData) {
		telemetryStore.StoreTelemetry(data.VIN, data)
	})
	if err != nil {
		t.Fatalf("failed to subscribe to telemetry: %v", err)
	}

	nc, err := nats.Connect(natsURL)
	if err != nil {
		t.Fatalf("failed to connect publisher: %v", err)
	}
	defer nc.Close()

	// Publish invalid JSON
	err = nc.Publish("vehicles.VIN12345.telemetry", []byte("not valid json{"))
	if err != nil {
		t.Fatalf("failed to publish invalid telemetry: %v", err)
	}
	nc.Flush()
	time.Sleep(200 * time.Millisecond)

	// Verify no data stored from invalid message
	stored, found := telemetryStore.GetTelemetry("VIN12345")
	if found && stored != nil {
		t.Error("expected no telemetry stored from invalid JSON")
	}

	// Now publish valid telemetry to verify the system still works
	validTelemetry := TelemetryData{
		VIN:           "VIN12345",
		DoorLocked:    true,
		Latitude:      48.1351,
		Longitude:     11.582,
		ParkingActive: false,
		Timestamp:     1709654400,
	}
	validData, _ := json.Marshal(validTelemetry)
	err = nc.Publish("vehicles.VIN12345.telemetry", validData)
	if err != nil {
		t.Fatalf("failed to publish valid telemetry: %v", err)
	}
	nc.Flush()
	time.Sleep(200 * time.Millisecond)

	stored, found = telemetryStore.GetTelemetry("VIN12345")
	if !found || stored == nil {
		t.Error("expected valid telemetry to be stored after invalid message")
	}
}
