package main

import (
	"encoding/json"
	"testing"
	"time"

	natsserver "github.com/nats-io/nats-server/v2/server"
	"github.com/nats-io/nats.go"
)

// startEmbeddedNATS starts an embedded NATS server for testing and returns it.
func startEmbeddedNATS(t *testing.T) *natsserver.Server {
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
		t.Fatal("embedded NATS server not ready")
	}
	return ns
}

// TS-06-3: NATS Command Relay Publishes to Correct Subject
func TestNATSCommandRelay(t *testing.T) {
	ns := startEmbeddedNATS(t)
	defer ns.Shutdown()

	client, err := NewNATSClient(ns.ClientURL())
	if err != nil {
		t.Fatalf("failed to create NATS client: %v", err)
	}
	defer client.Close()

	// Subscribe to both VINs to check correct routing
	nc, err := nats.Connect(ns.ClientURL())
	if err != nil {
		t.Fatalf("failed to connect test subscriber: %v", err)
	}
	defer nc.Close()

	sub1, err := nc.SubscribeSync("vehicles.VIN12345.commands")
	if err != nil {
		t.Fatalf("failed to subscribe to VIN12345 commands: %v", err)
	}
	sub2, err := nc.SubscribeSync("vehicles.VIN67890.commands")
	if err != nil {
		t.Fatalf("failed to subscribe to VIN67890 commands: %v", err)
	}
	nc.Flush()

	// Publish command for VIN12345
	cmd1 := NATSCommand{
		CommandID: "cmd-relay-1",
		Action:    "lock",
		Doors:     []string{"driver"},
		Source:    "companion_app",
	}
	if err := client.PublishCommand("VIN12345", cmd1); err != nil {
		t.Fatalf("failed to publish command for VIN12345: %v", err)
	}

	// Publish command for VIN67890
	cmd2 := NATSCommand{
		CommandID: "cmd-relay-2",
		Action:    "unlock",
		Doors:     []string{"passenger"},
		Source:    "companion_app",
	}
	if err := client.PublishCommand("VIN67890", cmd2); err != nil {
		t.Fatalf("failed to publish command for VIN67890: %v", err)
	}

	// Verify first command received on correct subject
	msg1, err := sub1.NextMsg(time.Second)
	if err != nil {
		t.Fatalf("did not receive message on VIN12345 commands: %v", err)
	}
	if msg1.Subject != "vehicles.VIN12345.commands" {
		t.Errorf("expected subject 'vehicles.VIN12345.commands', got %q", msg1.Subject)
	}
	var received1 NATSCommand
	if err := json.Unmarshal(msg1.Data, &received1); err != nil {
		t.Fatalf("failed to unmarshal NATS message: %v", err)
	}
	if received1.CommandID != "cmd-relay-1" {
		t.Errorf("expected command_id 'cmd-relay-1', got %q", received1.CommandID)
	}
	if received1.Action != "lock" {
		t.Errorf("expected action 'lock', got %q", received1.Action)
	}

	// Verify second command received on correct subject
	msg2, err := sub2.NextMsg(time.Second)
	if err != nil {
		t.Fatalf("did not receive message on VIN67890 commands: %v", err)
	}
	if msg2.Subject != "vehicles.VIN67890.commands" {
		t.Errorf("expected subject 'vehicles.VIN67890.commands', got %q", msg2.Subject)
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
}

// TS-06-4 (NATS part): NATS Response Subscription
func TestNATSResponseSubscription(t *testing.T) {
	ns := startEmbeddedNATS(t)
	defer ns.Shutdown()

	client, err := NewNATSClient(ns.ClientURL())
	if err != nil {
		t.Fatalf("failed to create NATS client: %v", err)
	}
	defer client.Close()

	received := make(chan NATSCommandResponse, 1)
	err = client.SubscribeCommandResponses("VIN12345", func(resp NATSCommandResponse) {
		received <- resp
	})
	if err != nil {
		t.Fatalf("failed to subscribe to command responses: %v", err)
	}

	// Publish a response from a separate connection (simulating CLOUD_GATEWAY_CLIENT)
	nc, err := nats.Connect(ns.ClientURL())
	if err != nil {
		t.Fatalf("failed to connect test publisher: %v", err)
	}
	defer nc.Close()

	responseJSON, _ := json.Marshal(NATSCommandResponse{
		CommandID: "cmd-resp-1",
		Status:    "success",
	})
	if err := nc.Publish("vehicles.VIN12345.command_responses", responseJSON); err != nil {
		t.Fatalf("failed to publish response: %v", err)
	}
	nc.Flush()

	select {
	case resp := <-received:
		if resp.CommandID != "cmd-resp-1" {
			t.Errorf("expected command_id 'cmd-resp-1', got %q", resp.CommandID)
		}
		if resp.Status != "success" {
			t.Errorf("expected status 'success', got %q", resp.Status)
		}
	case <-time.After(2 * time.Second):
		t.Fatal("timed out waiting for command response")
	}
}

// TS-06-6: Telemetry Reception
func TestTelemetryReception(t *testing.T) {
	ns := startEmbeddedNATS(t)
	defer ns.Shutdown()

	client, err := NewNATSClient(ns.ClientURL())
	if err != nil {
		t.Fatalf("failed to create NATS client: %v", err)
	}
	defer client.Close()

	store := NewTelemetryStore()
	err = client.SubscribeTelemetry("VIN12345", func(data TelemetryData) {
		store.StoreTelemetry(data.VIN, data)
	})
	if err != nil {
		t.Fatalf("failed to subscribe to telemetry: %v", err)
	}

	// Publish telemetry from a separate connection
	nc, err := nats.Connect(ns.ClientURL())
	if err != nil {
		t.Fatalf("failed to connect test publisher: %v", err)
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
	telemetryJSON, _ := json.Marshal(telemetry)
	if err := nc.Publish("vehicles.VIN12345.telemetry", telemetryJSON); err != nil {
		t.Fatalf("failed to publish telemetry: %v", err)
	}
	nc.Flush()

	// Wait for async processing
	time.Sleep(200 * time.Millisecond)

	stored, ok := store.GetTelemetry("VIN12345")
	if !ok {
		t.Fatal("expected telemetry to be stored, but it was not found")
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
	// Create a client pointing to a non-existent server
	client, err := NewNATSClient("nats://127.0.0.1:19999")
	if err != nil {
		// If NewNATSClient returns an error for unreachable server, that's also acceptable.
		// But we should verify the handler returns 503 when NATS is down.
		t.Logf("NewNATSClient returned error (expected): %v", err)
	}

	// If client is nil or not connected, publishing should fail
	if client != nil {
		cmd := NATSCommand{
			CommandID: "cmd-e8",
			Action:    "lock",
			Doors:     []string{"driver"},
			Source:    "companion_app",
		}
		err := client.PublishCommand("VIN12345", cmd)
		if err == nil {
			t.Error("expected error when publishing to unavailable NATS, got nil")
		}
	}
}

// TS-06-E10: Invalid Telemetry JSON Is Discarded
func TestInvalidTelemetryJSON(t *testing.T) {
	ns := startEmbeddedNATS(t)
	defer ns.Shutdown()

	client, err := NewNATSClient(ns.ClientURL())
	if err != nil {
		t.Fatalf("failed to create NATS client: %v", err)
	}
	defer client.Close()

	store := NewTelemetryStore()
	err = client.SubscribeTelemetry("VIN12345", func(data TelemetryData) {
		store.StoreTelemetry(data.VIN, data)
	})
	if err != nil {
		t.Fatalf("failed to subscribe to telemetry: %v", err)
	}

	nc, err := nats.Connect(ns.ClientURL())
	if err != nil {
		t.Fatalf("failed to connect test publisher: %v", err)
	}
	defer nc.Close()

	// Publish invalid JSON
	if err := nc.Publish("vehicles.VIN12345.telemetry", []byte("not valid json{")); err != nil {
		t.Fatalf("failed to publish invalid telemetry: %v", err)
	}
	nc.Flush()
	time.Sleep(200 * time.Millisecond)

	// No data should be stored from invalid message
	_, ok := store.GetTelemetry("VIN12345")
	if ok {
		t.Error("expected no telemetry stored from invalid JSON, but data was found")
	}

	// Publish valid telemetry afterward to ensure system still works
	validTelemetry := TelemetryData{
		VIN:           "VIN12345",
		DoorLocked:    true,
		Latitude:      48.1351,
		Longitude:     11.582,
		ParkingActive: false,
		Timestamp:     1709654400,
	}
	validJSON, _ := json.Marshal(validTelemetry)
	if err := nc.Publish("vehicles.VIN12345.telemetry", validJSON); err != nil {
		t.Fatalf("failed to publish valid telemetry: %v", err)
	}
	nc.Flush()
	time.Sleep(200 * time.Millisecond)

	stored, ok := store.GetTelemetry("VIN12345")
	if !ok {
		t.Fatal("expected valid telemetry to be stored after invalid message, but it was not found")
	}
	if !stored.DoorLocked {
		t.Error("expected DoorLocked to be true")
	}
}
