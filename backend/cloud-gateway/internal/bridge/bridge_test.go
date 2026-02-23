package bridge

import (
	"encoding/json"
	"sync"
	"testing"
	"time"
)

// mockPublisher records published MQTT messages for test assertions.
type mockPublisher struct {
	mu       sync.Mutex
	messages []publishedMessage
	err      error // if set, Publish returns this error
}

type publishedMessage struct {
	Topic   string
	Payload []byte
}

func (m *mockPublisher) Publish(topic string, payload []byte) error {
	if m.err != nil {
		return m.err
	}
	m.mu.Lock()
	defer m.mu.Unlock()
	m.messages = append(m.messages, publishedMessage{Topic: topic, Payload: payload})
	return nil
}

func (m *mockPublisher) lastMessage() publishedMessage {
	m.mu.Lock()
	defer m.mu.Unlock()
	if len(m.messages) == 0 {
		return publishedMessage{}
	}
	return m.messages[len(m.messages)-1]
}

func (m *mockPublisher) count() int {
	m.mu.Lock()
	defer m.mu.Unlock()
	return len(m.messages)
}

// mockTelemetryStore records telemetry updates for test assertions.
type mockTelemetryStore struct {
	mu      sync.Mutex
	entries map[string]TelemetryData
}

func newMockTelemetryStore() *mockTelemetryStore {
	return &mockTelemetryStore{entries: make(map[string]TelemetryData)}
}

func (m *mockTelemetryStore) Update(vin string, data TelemetryData) {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.entries[vin] = data
}

func (m *mockTelemetryStore) get(vin string) (TelemetryData, bool) {
	m.mu.Lock()
	defer m.mu.Unlock()
	d, ok := m.entries[vin]
	return d, ok
}

// TS-03-11: Command ID preserved in MQTT message
// Requirement: 03-REQ-3.1
// Verifies the command_id from REST is preserved in the MQTT message.
func TestBridge_CommandIDPreserved(t *testing.T) {
	pub := &mockPublisher{}
	tracker := NewTracker(5 * time.Second)
	b := NewBridge(tracker, pub)

	ch, err := b.SendCommand("VIN12345", Command{
		CommandID: "preserve-me-123",
		Type:      "lock",
		Doors:     []string{"driver"},
	})
	if err != nil {
		t.Fatalf("SendCommand failed: %v", err)
	}

	msg := pub.lastMessage()
	var payload map[string]interface{}
	if err := json.Unmarshal(msg.Payload, &payload); err != nil {
		t.Fatalf("invalid JSON payload: %v", err)
	}

	if payload["command_id"] != "preserve-me-123" {
		t.Errorf("expected command_id 'preserve-me-123', got %v", payload["command_id"])
	}

	// Clean up pending command
	go func() {
		tracker.Resolve("preserve-me-123", CommandResponse{Status: "success"})
	}()
	<-ch
}

// TS-03-13: MQTT command message schema
// Requirement: 03-REQ-3.3
// Verifies the MQTT command message conforms to the expected JSON schema.
func TestBridge_CommandSchema(t *testing.T) {
	pub := &mockPublisher{}
	tracker := NewTracker(5 * time.Second)
	b := NewBridge(tracker, pub)

	ch, err := b.SendCommand("VIN12345", Command{
		CommandID: "schema-test",
		Type:      "lock",
		Doors:     []string{"driver"},
	})
	if err != nil {
		t.Fatalf("SendCommand failed: %v", err)
	}

	msg := pub.lastMessage()
	var payload map[string]interface{}
	if err := json.Unmarshal(msg.Payload, &payload); err != nil {
		t.Fatalf("invalid JSON payload: %v", err)
	}

	// Verify all required fields exist
	requiredFields := []string{"command_id", "action", "doors", "source"}
	for _, field := range requiredFields {
		if _, ok := payload[field]; !ok {
			t.Errorf("MQTT payload missing required field %q", field)
		}
	}

	// Verify field values
	if payload["action"] != "lock" {
		t.Errorf("expected action 'lock', got %v", payload["action"])
	}
	if payload["source"] != "companion_app" {
		t.Errorf("expected source 'companion_app', got %v", payload["source"])
	}

	doors, ok := payload["doors"].([]interface{})
	if !ok {
		t.Fatalf("expected doors to be an array, got %T", payload["doors"])
	}
	if len(doors) != 1 || doors[0] != "driver" {
		t.Errorf("expected doors ['driver'], got %v", doors)
	}

	// Clean up
	go func() {
		tracker.Resolve("schema-test", CommandResponse{Status: "success"})
	}()
	<-ch
}

// TestBridge_CommandSchemaUnlock verifies unlock commands produce correct MQTT
// action field.
func TestBridge_CommandSchemaUnlock(t *testing.T) {
	pub := &mockPublisher{}
	tracker := NewTracker(5 * time.Second)
	b := NewBridge(tracker, pub)

	ch, err := b.SendCommand("VIN12345", Command{
		CommandID: "unlock-test",
		Type:      "unlock",
		Doors:     []string{"driver", "passenger"},
	})
	if err != nil {
		t.Fatalf("SendCommand failed: %v", err)
	}

	msg := pub.lastMessage()
	var payload map[string]interface{}
	if err := json.Unmarshal(msg.Payload, &payload); err != nil {
		t.Fatalf("invalid JSON payload: %v", err)
	}

	if payload["action"] != "unlock" {
		t.Errorf("expected action 'unlock', got %v", payload["action"])
	}

	// Clean up
	go func() {
		tracker.Resolve("unlock-test", CommandResponse{Status: "success"})
	}()
	<-ch
}

// TS-03-14: MQTT response message schema validation
// Requirement: 03-REQ-3.4
// Verifies the bridge correctly parses MQTT response messages.
func TestBridge_ResponseSchema(t *testing.T) {
	pub := &mockPublisher{}
	tracker := NewTracker(5 * time.Second)
	b := NewBridge(tracker, pub)

	// Register a pending command
	ch := tracker.Register("resp-schema-test")

	// Handle a valid response
	b.HandleResponse([]byte(`{"command_id":"resp-schema-test","status":"success","reason":"","timestamp":123}`))

	select {
	case resp := <-ch:
		if resp.Status != "success" {
			t.Errorf("expected status 'success', got %q", resp.Status)
		}
		if resp.CommandID != "resp-schema-test" {
			t.Errorf("expected command_id 'resp-schema-test', got %q", resp.CommandID)
		}
		if resp.Timestamp != 123 {
			t.Errorf("expected timestamp 123, got %d", resp.Timestamp)
		}
	case <-time.After(2 * time.Second):
		t.Fatal("timed out waiting for response")
	}

	// Handle an invalid response (missing command_id): should not panic
	b.HandleResponse([]byte(`{"status":"success"}`))

	// Handle invalid JSON: should not panic
	b.HandleResponse([]byte(`not json`))
}

// TestBridge_ResponseSchemaFailed verifies the bridge handles "failed" status.
func TestBridge_ResponseSchemaFailed(t *testing.T) {
	pub := &mockPublisher{}
	tracker := NewTracker(5 * time.Second)
	b := NewBridge(tracker, pub)

	ch := tracker.Register("fail-test")

	b.HandleResponse([]byte(`{"command_id":"fail-test","status":"failed","reason":"door jammed","timestamp":456}`))

	select {
	case resp := <-ch:
		if resp.Status != "failed" {
			t.Errorf("expected status 'failed', got %q", resp.Status)
		}
		if resp.Reason != "door jammed" {
			t.Errorf("expected reason 'door jammed', got %q", resp.Reason)
		}
	case <-time.After(2 * time.Second):
		t.Fatal("timed out waiting for response")
	}
}

// TS-03-P1: Command ID Preservation (Property)
// Verifies command_id is preserved across multiple random IDs.
func TestBridge_CommandIDPreservation(t *testing.T) {
	vins := []string{"VIN-1", "VIN-2", "VIN-3", "VIN-4", "VIN-5"}
	ids := []string{
		"aaaa-1111", "bbbb-2222", "cccc-3333", "dddd-4444", "eeee-5555",
		"ffff-6666", "gggg-7777", "hhhh-8888", "iiii-9999", "jjjj-0000",
		"kkkk-1111", "llll-2222", "mmmm-3333", "nnnn-4444", "oooo-5555",
		"pppp-6666", "qqqq-7777", "rrrr-8888", "ssss-9999", "tttt-0000",
	}

	for i, id := range ids {
		t.Run(id, func(t *testing.T) {
			pub := &mockPublisher{}
			tracker := NewTracker(5 * time.Second)
			b := NewBridge(tracker, pub)

			vin := vins[i%len(vins)]
			ch, err := b.SendCommand(vin, Command{
				CommandID: id,
				Type:      "lock",
				Doors:     []string{"driver"},
			})
			if err != nil {
				t.Fatalf("SendCommand failed: %v", err)
			}

			msg := pub.lastMessage()
			var payload map[string]interface{}
			if err := json.Unmarshal(msg.Payload, &payload); err != nil {
				t.Fatalf("invalid JSON: %v", err)
			}

			if payload["command_id"] != id {
				t.Errorf("expected command_id %q, got %v", id, payload["command_id"])
			}

			// Clean up
			go func() {
				tracker.Resolve(id, CommandResponse{Status: "success"})
			}()
			<-ch
		})
	}
}

// TS-03-P4: Topic Routing Correctness (Property)
// Verifies commands are published to the correct MQTT topic for each VIN.
func TestBridge_TopicRouting(t *testing.T) {
	vins := []string{"VIN_A", "VIN_B", "VIN_XYZ123", "WBAPH5C55BA270000"}

	for _, vin := range vins {
		t.Run(vin, func(t *testing.T) {
			pub := &mockPublisher{}
			tracker := NewTracker(5 * time.Second)
			b := NewBridge(tracker, pub)

			ch, err := b.SendCommand(vin, Command{
				CommandID: "topic-routing-" + vin,
				Type:      "lock",
				Doors:     []string{"driver"},
			})
			if err != nil {
				t.Fatalf("SendCommand failed: %v", err)
			}

			expected := "vehicles/" + vin + "/commands"
			msg := pub.lastMessage()
			if msg.Topic != expected {
				t.Errorf("expected topic %q, got %q", expected, msg.Topic)
			}

			// Clean up
			go func() {
				tracker.Resolve("topic-routing-"+vin, CommandResponse{Status: "success"})
			}()
			<-ch
		})
	}
}

// TestBridge_SendCommand_PublishError verifies that SendCommand returns an
// error when the MQTT publish fails.
func TestBridge_SendCommand_PublishError(t *testing.T) {
	pub := &mockPublisher{err: errPublishFailed}
	tracker := NewTracker(5 * time.Second)
	b := NewBridge(tracker, pub)

	_, err := b.SendCommand("VIN12345", Command{
		CommandID: "will-fail",
		Type:      "lock",
		Doors:     []string{"driver"},
	})
	if err == nil {
		t.Error("expected SendCommand to return an error when publish fails")
	}
}

var errPublishFailed = &publishError{"MQTT client not connected"}

type publishError struct{ msg string }

func (e *publishError) Error() string { return e.msg }

// TestBridge_HandleTelemetry verifies telemetry updates are stored correctly.
func TestBridge_HandleTelemetry(t *testing.T) {
	store := newMockTelemetryStore()

	HandleTelemetry(store, "VIN12345", []byte(
		`{"vin":"VIN12345","locked":true,"timestamp":1708700000}`))

	data, ok := store.get("VIN12345")
	if !ok {
		t.Fatal("expected telemetry data for VIN12345")
	}
	if !data.Locked {
		t.Error("expected locked=true")
	}
	if data.Timestamp != 1708700000 {
		t.Errorf("expected timestamp 1708700000, got %d", data.Timestamp)
	}
	if data.VIN != "VIN12345" {
		t.Errorf("expected VIN 'VIN12345', got %q", data.VIN)
	}
}

// TestBridge_HandleTelemetry_InvalidJSON verifies invalid telemetry is logged
// and discarded (no panic).
func TestBridge_HandleTelemetry_InvalidJSON(t *testing.T) {
	store := newMockTelemetryStore()

	// Should not panic
	HandleTelemetry(store, "VIN12345", []byte("not json"))

	if _, ok := store.get("VIN12345"); ok {
		t.Error("expected no telemetry stored for invalid JSON")
	}
}

// TestBridge_HandleTelemetry_MultipleVINs verifies telemetry for different
// VINs is stored independently.
func TestBridge_HandleTelemetry_MultipleVINs(t *testing.T) {
	store := newMockTelemetryStore()

	HandleTelemetry(store, "VIN_A", []byte(
		`{"vin":"VIN_A","locked":true,"timestamp":100}`))
	HandleTelemetry(store, "VIN_B", []byte(
		`{"vin":"VIN_B","locked":false,"timestamp":200}`))

	dataA, ok := store.get("VIN_A")
	if !ok {
		t.Fatal("expected telemetry data for VIN_A")
	}
	if !dataA.Locked {
		t.Error("VIN_A: expected locked=true")
	}

	dataB, ok := store.get("VIN_B")
	if !ok {
		t.Fatal("expected telemetry data for VIN_B")
	}
	if dataB.Locked {
		t.Error("VIN_B: expected locked=false")
	}
}

// TestBridge_SendCommandAndResolve verifies the full send-and-resolve cycle.
func TestBridge_SendCommandAndResolve(t *testing.T) {
	pub := &mockPublisher{}
	tracker := NewTracker(5 * time.Second)
	b := NewBridge(tracker, pub)

	ch, err := b.SendCommand("VIN12345", Command{
		CommandID: "full-cycle",
		Type:      "unlock",
		Doors:     []string{"driver"},
	})
	if err != nil {
		t.Fatalf("SendCommand failed: %v", err)
	}

	// Verify command was published
	if pub.count() != 1 {
		t.Fatalf("expected 1 published message, got %d", pub.count())
	}

	// Simulate MQTT response
	b.HandleResponse([]byte(`{"command_id":"full-cycle","status":"success","reason":"","timestamp":999}`))

	select {
	case resp := <-ch:
		if resp.CommandID != "full-cycle" {
			t.Errorf("expected command_id 'full-cycle', got %q", resp.CommandID)
		}
		if resp.Status != "success" {
			t.Errorf("expected status 'success', got %q", resp.Status)
		}
	case <-time.After(2 * time.Second):
		t.Fatal("timed out waiting for response")
	}
}

// TestBridge_TrackerAccess verifies Tracker() returns the bridge's tracker.
func TestBridge_TrackerAccess(t *testing.T) {
	pub := &mockPublisher{}
	tracker := NewTracker(5 * time.Second)
	b := NewBridge(tracker, pub)

	if b.Tracker() != tracker {
		t.Error("Tracker() should return the same tracker instance")
	}
}
