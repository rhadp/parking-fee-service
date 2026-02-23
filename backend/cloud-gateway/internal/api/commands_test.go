package api

import (
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/internal/bridge"
)

// mockPublisher records published MQTT messages for test assertions.
type mockPublisher struct {
	topics   []string
	payloads [][]byte
	err      error
}

func (m *mockPublisher) Publish(topic string, payload []byte) error {
	m.topics = append(m.topics, topic)
	m.payloads = append(m.payloads, payload)
	return m.err
}

func TestCommandHandler_ValidLock(t *testing.T) {
	tracker := bridge.NewTracker(200 * time.Millisecond)
	pub := &mockPublisher{}
	handler := NewCommandHandler(tracker, pub)

	// Since the handler blocks waiting for response, we need to resolve
	// the command in a separate goroutine
	go func() {
		// Wait for the command to be registered
		time.Sleep(50 * time.Millisecond)
		tracker.Resolve("cmd-valid", bridge.CommandResponse{
			Status:    "success",
			Timestamp: 12345,
		})
	}()

	body := `{"command_id":"cmd-valid","type":"lock","doors":["driver"]}`
	req := httptest.NewRequest(http.MethodPost, "/vehicles/VIN12345/commands", strings.NewReader(body))
	req.SetPathValue("vin", "VIN12345")
	rec := httptest.NewRecorder()

	handler.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Errorf("expected 200, got %d; body: %s", rec.Code, rec.Body.String())
	}

	var resp map[string]interface{}
	json.NewDecoder(rec.Body).Decode(&resp)
	if resp["command_id"] != "cmd-valid" {
		t.Errorf("expected command_id 'cmd-valid', got %v", resp["command_id"])
	}
	if resp["status"] != "success" {
		t.Errorf("expected status 'success', got %v", resp["status"])
	}

	// Verify MQTT publish
	if len(pub.topics) != 1 {
		t.Fatalf("expected 1 MQTT publish, got %d", len(pub.topics))
	}
	if pub.topics[0] != "vehicles/VIN12345/commands" {
		t.Errorf("expected topic 'vehicles/VIN12345/commands', got %q", pub.topics[0])
	}

	var mqttMsg map[string]interface{}
	json.Unmarshal(pub.payloads[0], &mqttMsg)
	if mqttMsg["action"] != "lock" {
		t.Errorf("expected action 'lock', got %v", mqttMsg["action"])
	}
	if mqttMsg["source"] != "companion_app" {
		t.Errorf("expected source 'companion_app', got %v", mqttMsg["source"])
	}
}

func TestCommandHandler_ValidUnlock(t *testing.T) {
	tracker := bridge.NewTracker(200 * time.Millisecond)
	pub := &mockPublisher{}
	handler := NewCommandHandler(tracker, pub)

	go func() {
		time.Sleep(50 * time.Millisecond)
		tracker.Resolve("cmd-unlock", bridge.CommandResponse{Status: "success"})
	}()

	body := `{"command_id":"cmd-unlock","type":"unlock","doors":["driver"]}`
	req := httptest.NewRequest(http.MethodPost, "/vehicles/VIN12345/commands", strings.NewReader(body))
	req.SetPathValue("vin", "VIN12345")
	rec := httptest.NewRecorder()

	handler.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Errorf("expected 200, got %d", rec.Code)
	}

	var mqttMsg map[string]interface{}
	json.Unmarshal(pub.payloads[0], &mqttMsg)
	if mqttMsg["action"] != "unlock" {
		t.Errorf("expected action 'unlock', got %v", mqttMsg["action"])
	}
}

func TestCommandHandler_Timeout(t *testing.T) {
	tracker := bridge.NewTracker(100 * time.Millisecond)
	pub := &mockPublisher{}
	handler := NewCommandHandler(tracker, pub)

	body := `{"command_id":"timeout-cmd","type":"lock","doors":["driver"]}`
	req := httptest.NewRequest(http.MethodPost, "/vehicles/VIN12345/commands", strings.NewReader(body))
	req.SetPathValue("vin", "VIN12345")
	rec := httptest.NewRecorder()

	handler.ServeHTTP(rec, req)

	if rec.Code != http.StatusGatewayTimeout {
		t.Errorf("expected 504, got %d; body: %s", rec.Code, rec.Body.String())
	}

	var resp map[string]interface{}
	json.NewDecoder(rec.Body).Decode(&resp)
	if resp["command_id"] != "timeout-cmd" {
		t.Errorf("expected command_id 'timeout-cmd', got %v", resp["command_id"])
	}
	if resp["status"] != "timeout" {
		t.Errorf("expected status 'timeout', got %v", resp["status"])
	}
}

func TestCommandHandler_MissingCommandID(t *testing.T) {
	tracker := bridge.NewTracker(time.Second)
	pub := &mockPublisher{}
	handler := NewCommandHandler(tracker, pub)

	body := `{"type":"lock","doors":["driver"]}`
	req := httptest.NewRequest(http.MethodPost, "/vehicles/VIN12345/commands", strings.NewReader(body))
	req.SetPathValue("vin", "VIN12345")
	rec := httptest.NewRecorder()

	handler.ServeHTTP(rec, req)

	if rec.Code != http.StatusBadRequest {
		t.Errorf("expected 400, got %d", rec.Code)
	}

	var resp map[string]interface{}
	json.NewDecoder(rec.Body).Decode(&resp)
	if _, ok := resp["error"]; !ok {
		t.Error("expected 'error' field in response")
	}
}

func TestCommandHandler_MissingType(t *testing.T) {
	tracker := bridge.NewTracker(time.Second)
	pub := &mockPublisher{}
	handler := NewCommandHandler(tracker, pub)

	body := `{"command_id":"x","doors":["driver"]}`
	req := httptest.NewRequest(http.MethodPost, "/vehicles/VIN12345/commands", strings.NewReader(body))
	req.SetPathValue("vin", "VIN12345")
	rec := httptest.NewRecorder()

	handler.ServeHTTP(rec, req)

	if rec.Code != http.StatusBadRequest {
		t.Errorf("expected 400, got %d", rec.Code)
	}
}

func TestCommandHandler_InvalidType(t *testing.T) {
	tracker := bridge.NewTracker(time.Second)
	pub := &mockPublisher{}
	handler := NewCommandHandler(tracker, pub)

	body := `{"command_id":"x","type":"open","doors":["driver"]}`
	req := httptest.NewRequest(http.MethodPost, "/vehicles/VIN12345/commands", strings.NewReader(body))
	req.SetPathValue("vin", "VIN12345")
	rec := httptest.NewRecorder()

	handler.ServeHTTP(rec, req)

	if rec.Code != http.StatusBadRequest {
		t.Errorf("expected 400, got %d", rec.Code)
	}

	var resp map[string]interface{}
	json.NewDecoder(rec.Body).Decode(&resp)
	errMsg, ok := resp["error"].(string)
	if !ok {
		t.Fatal("expected 'error' string in response")
	}
	if errMsg != "type must be 'lock' or 'unlock'" {
		t.Errorf("unexpected error message: %q", errMsg)
	}
}

func TestCommandHandler_MissingDoors(t *testing.T) {
	tracker := bridge.NewTracker(time.Second)
	pub := &mockPublisher{}
	handler := NewCommandHandler(tracker, pub)

	body := `{"command_id":"x","type":"lock"}`
	req := httptest.NewRequest(http.MethodPost, "/vehicles/VIN12345/commands", strings.NewReader(body))
	req.SetPathValue("vin", "VIN12345")
	rec := httptest.NewRecorder()

	handler.ServeHTTP(rec, req)

	if rec.Code != http.StatusBadRequest {
		t.Errorf("expected 400, got %d", rec.Code)
	}
}

func TestCommandHandler_InvalidJSON(t *testing.T) {
	tracker := bridge.NewTracker(time.Second)
	pub := &mockPublisher{}
	handler := NewCommandHandler(tracker, pub)

	invalidBodies := []string{
		"not json at all",
		"{malformed",
		"",
	}

	for _, body := range invalidBodies {
		req := httptest.NewRequest(http.MethodPost, "/vehicles/VIN12345/commands", strings.NewReader(body))
		req.SetPathValue("vin", "VIN12345")
		rec := httptest.NewRecorder()

		handler.ServeHTTP(rec, req)

		if rec.Code != http.StatusBadRequest {
			t.Errorf("body %q: expected 400, got %d", body, rec.Code)
		}
	}
}

func TestCommandHandler_DegradedMode(t *testing.T) {
	tracker := bridge.NewTracker(time.Second)
	pub := &mockPublisher{err: fmt.Errorf("MQTT client not connected")}
	handler := NewCommandHandler(tracker, pub)

	body := `{"command_id":"degraded-cmd","type":"lock","doors":["driver"]}`
	req := httptest.NewRequest(http.MethodPost, "/vehicles/VIN12345/commands", strings.NewReader(body))
	req.SetPathValue("vin", "VIN12345")
	rec := httptest.NewRecorder()

	handler.ServeHTTP(rec, req)

	// In degraded mode (MQTT unreachable), should return 202 Accepted
	if rec.Code != http.StatusAccepted {
		t.Errorf("expected 202 in degraded mode, got %d; body: %s", rec.Code, rec.Body.String())
	}

	var resp map[string]interface{}
	json.NewDecoder(rec.Body).Decode(&resp)
	if resp["command_id"] != "degraded-cmd" {
		t.Errorf("expected command_id 'degraded-cmd', got %v", resp["command_id"])
	}
	if resp["status"] != "pending" {
		t.Errorf("expected status 'pending', got %v", resp["status"])
	}
}

func TestCommandHandler_MQTTPublishPayload(t *testing.T) {
	tracker := bridge.NewTracker(200 * time.Millisecond)
	pub := &mockPublisher{}
	handler := NewCommandHandler(tracker, pub)

	go func() {
		time.Sleep(50 * time.Millisecond)
		tracker.Resolve("schema-test", bridge.CommandResponse{Status: "success"})
	}()

	body := `{"command_id":"schema-test","type":"lock","doors":["driver"]}`
	req := httptest.NewRequest(http.MethodPost, "/vehicles/VIN12345/commands", strings.NewReader(body))
	req.SetPathValue("vin", "VIN12345")
	rec := httptest.NewRecorder()

	handler.ServeHTTP(rec, req)

	if len(pub.payloads) != 1 {
		t.Fatal("expected 1 published message")
	}

	var mqttMsg map[string]interface{}
	if err := json.Unmarshal(pub.payloads[0], &mqttMsg); err != nil {
		t.Fatalf("MQTT payload is not valid JSON: %v", err)
	}

	// Verify schema: command_id, action, doors, source
	if _, ok := mqttMsg["command_id"]; !ok {
		t.Error("MQTT payload missing 'command_id'")
	}
	if _, ok := mqttMsg["action"]; !ok {
		t.Error("MQTT payload missing 'action'")
	}
	if _, ok := mqttMsg["doors"]; !ok {
		t.Error("MQTT payload missing 'doors'")
	}
	if _, ok := mqttMsg["source"]; !ok {
		t.Error("MQTT payload missing 'source'")
	}

	if mqttMsg["command_id"] != "schema-test" {
		t.Errorf("expected command_id 'schema-test', got %v", mqttMsg["command_id"])
	}
	if mqttMsg["action"] != "lock" {
		t.Errorf("expected action 'lock', got %v", mqttMsg["action"])
	}
	if mqttMsg["source"] != "companion_app" {
		t.Errorf("expected source 'companion_app', got %v", mqttMsg["source"])
	}
}
