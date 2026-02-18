package mqtt

import (
	"encoding/json"
	"testing"
	"time"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/messages"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/state"
)

// newTestClient creates a Client with only the store set (no real MQTT
// connection). This is sufficient for testing handlers that only use
// c.store.
func newTestClient(store *state.Store) *Client {
	return &Client{store: store}
}

// --- handleCommandResponse ---

func TestHandleCommandResponse_Success(t *testing.T) {
	store := state.NewStore()
	store.RegisterVehicle("VIN1", "123456")
	store.AddCommand("VIN1", "cmd-1", "lock")

	c := newTestClient(store)

	resp := messages.CommandResponse{
		CommandID: "cmd-1",
		Type:      messages.CommandTypeLock,
		Result:    messages.CommandResultSuccess,
		Timestamp: time.Now().Unix(),
	}
	payload, _ := json.Marshal(resp)

	c.handleCommandResponse("VIN1", payload)

	v := store.GetVehicle("VIN1")
	if v == nil {
		t.Fatal("vehicle not found")
	}

	cmd, ok := v.Commands["cmd-1"]
	if !ok {
		t.Fatal("command not found")
	}
	if cmd.Status != "success" {
		t.Errorf("command status = %q, want %q", cmd.Status, "success")
	}
	if cmd.Result != "SUCCESS" {
		t.Errorf("command result = %q, want %q", cmd.Result, "SUCCESS")
	}

	// Vehicle lock state should also be updated.
	if v.IsLocked == nil || !*v.IsLocked {
		t.Errorf("IsLocked = %v, want true", v.IsLocked)
	}
}

func TestHandleCommandResponse_UnlockSuccess(t *testing.T) {
	store := state.NewStore()
	store.RegisterVehicle("VIN1", "123456")
	store.AddCommand("VIN1", "cmd-2", "unlock")

	// Set initial locked state.
	locked := true
	store.UpdateState("VIN1", &locked, nil, nil, nil, nil, nil)

	c := newTestClient(store)

	resp := messages.CommandResponse{
		CommandID: "cmd-2",
		Type:      messages.CommandTypeUnlock,
		Result:    messages.CommandResultSuccess,
		Timestamp: time.Now().Unix(),
	}
	payload, _ := json.Marshal(resp)

	c.handleCommandResponse("VIN1", payload)

	v := store.GetVehicle("VIN1")
	if v == nil {
		t.Fatal("vehicle not found")
	}

	if v.IsLocked == nil || *v.IsLocked {
		t.Errorf("IsLocked = %v, want false", v.IsLocked)
	}
}

func TestHandleCommandResponse_Rejected(t *testing.T) {
	store := state.NewStore()
	store.RegisterVehicle("VIN1", "123456")
	store.AddCommand("VIN1", "cmd-1", "lock")

	c := newTestClient(store)

	resp := messages.CommandResponse{
		CommandID: "cmd-1",
		Type:      messages.CommandTypeLock,
		Result:    messages.CommandResultRejectedSpeed,
		Timestamp: time.Now().Unix(),
	}
	payload, _ := json.Marshal(resp)

	c.handleCommandResponse("VIN1", payload)

	v := store.GetVehicle("VIN1")
	cmd := v.Commands["cmd-1"]
	if cmd.Status != "rejected" {
		t.Errorf("command status = %q, want %q", cmd.Status, "rejected")
	}
	if cmd.Result != "REJECTED_SPEED" {
		t.Errorf("command result = %q, want %q", cmd.Result, "REJECTED_SPEED")
	}

	// Lock state should NOT be updated on rejection.
	if v.IsLocked != nil {
		t.Errorf("IsLocked should be nil on rejection, got %v", *v.IsLocked)
	}
}

func TestHandleCommandResponse_UnknownCommandID(t *testing.T) {
	// 03-REQ-2.E2: Unknown command_id should be logged and discarded.
	store := state.NewStore()
	store.RegisterVehicle("VIN1", "123456")
	// No commands added.

	c := newTestClient(store)

	resp := messages.CommandResponse{
		CommandID: "unknown-cmd",
		Type:      messages.CommandTypeLock,
		Result:    messages.CommandResultSuccess,
		Timestamp: time.Now().Unix(),
	}
	payload, _ := json.Marshal(resp)

	// Should not panic — just log and discard.
	c.handleCommandResponse("VIN1", payload)

	v := store.GetVehicle("VIN1")
	if len(v.Commands) != 0 {
		t.Errorf("expected no commands, got %d", len(v.Commands))
	}
}

func TestHandleCommandResponse_InvalidJSON(t *testing.T) {
	store := state.NewStore()
	c := newTestClient(store)

	// Should not panic on invalid JSON.
	c.handleCommandResponse("VIN1", []byte("not json"))
}

// --- handleTelemetry ---

func TestHandleTelemetry_UpdatesState(t *testing.T) {
	store := state.NewStore()
	store.RegisterVehicle("VIN1", "123456")

	c := newTestClient(store)

	locked := true
	doorOpen := false
	speed := 50.5
	lat := 48.1351
	lon := 11.5820
	parking := true

	tel := messages.TelemetryMessage{
		VIN:                  "VIN1",
		IsLocked:             &locked,
		IsDoorOpen:           &doorOpen,
		Speed:                &speed,
		Latitude:             &lat,
		Longitude:            &lon,
		ParkingSessionActive: &parking,
		Timestamp:            time.Now().Unix(),
	}
	payload, _ := json.Marshal(tel)

	c.handleTelemetry("VIN1", payload)

	v := store.GetVehicle("VIN1")
	if v == nil {
		t.Fatal("vehicle not found")
	}

	if v.IsLocked == nil || !*v.IsLocked {
		t.Errorf("IsLocked = %v, want true", v.IsLocked)
	}
	if v.IsDoorOpen == nil || *v.IsDoorOpen {
		t.Errorf("IsDoorOpen = %v, want false", v.IsDoorOpen)
	}
	if v.Speed == nil || *v.Speed != 50.5 {
		t.Errorf("Speed = %v, want 50.5", v.Speed)
	}
	if v.Latitude == nil || *v.Latitude != 48.1351 {
		t.Errorf("Latitude = %v, want 48.1351", v.Latitude)
	}
	if v.Longitude == nil || *v.Longitude != 11.5820 {
		t.Errorf("Longitude = %v, want 11.5820", v.Longitude)
	}
	if v.ParkingSessionActive == nil || !*v.ParkingSessionActive {
		t.Errorf("ParkingSessionActive = %v, want true", v.ParkingSessionActive)
	}
}

func TestHandleTelemetry_PartialUpdate(t *testing.T) {
	store := state.NewStore()
	store.RegisterVehicle("VIN1", "123456")

	// Set initial state.
	locked := true
	store.UpdateState("VIN1", &locked, nil, nil, nil, nil, nil)

	c := newTestClient(store)

	// Telemetry with only speed — should not clear locked state.
	speed := 30.0
	tel := messages.TelemetryMessage{
		VIN:       "VIN1",
		Speed:     &speed,
		Timestamp: time.Now().Unix(),
	}
	payload, _ := json.Marshal(tel)

	c.handleTelemetry("VIN1", payload)

	v := store.GetVehicle("VIN1")
	if v.IsLocked == nil || !*v.IsLocked {
		t.Errorf("IsLocked should still be true, got %v", v.IsLocked)
	}
	if v.Speed == nil || *v.Speed != 30.0 {
		t.Errorf("Speed = %v, want 30.0", v.Speed)
	}
}

func TestHandleTelemetry_InvalidJSON(t *testing.T) {
	store := state.NewStore()
	c := newTestClient(store)

	// Should not panic.
	c.handleTelemetry("VIN1", []byte("{bad"))
}

func TestHandleTelemetry_UnknownVIN(t *testing.T) {
	store := state.NewStore()
	c := newTestClient(store)

	// Telemetry for an unregistered VIN — should not panic.
	tel := messages.TelemetryMessage{
		VIN:       "UNKNOWN",
		Timestamp: time.Now().Unix(),
	}
	payload, _ := json.Marshal(tel)

	c.handleTelemetry("UNKNOWN", payload)
}

// --- handleRegistration ---

func TestHandleRegistration_NewVehicle(t *testing.T) {
	store := state.NewStore()
	c := newTestClient(store)

	reg := messages.RegistrationMessage{
		VIN:        "VIN1",
		PairingPIN: "123456",
		Timestamp:  time.Now().Unix(),
	}
	payload, _ := json.Marshal(reg)

	c.handleRegistration("VIN1", payload)

	v := store.GetVehicle("VIN1")
	if v == nil {
		t.Fatal("expected vehicle to be registered")
	}
	if v.VIN != "VIN1" {
		t.Errorf("VIN = %q, want %q", v.VIN, "VIN1")
	}
	if v.PairingPIN != "123456" {
		t.Errorf("PairingPIN = %q, want %q", v.PairingPIN, "123456")
	}
}

func TestHandleRegistration_ReRegistration(t *testing.T) {
	store := state.NewStore()
	store.RegisterVehicle("VIN1", "111111")

	c := newTestClient(store)

	// Re-register with new PIN.
	reg := messages.RegistrationMessage{
		VIN:        "VIN1",
		PairingPIN: "222222",
		Timestamp:  time.Now().Unix(),
	}
	payload, _ := json.Marshal(reg)

	c.handleRegistration("VIN1", payload)

	v := store.GetVehicle("VIN1")
	if v.PairingPIN != "222222" {
		t.Errorf("PairingPIN = %q, want %q (should be updated)", v.PairingPIN, "222222")
	}
}

func TestHandleRegistration_InvalidJSON(t *testing.T) {
	store := state.NewStore()
	c := newTestClient(store)

	// Should not panic.
	c.handleRegistration("VIN1", []byte("bad json"))

	v := store.GetVehicle("VIN1")
	if v != nil {
		t.Error("vehicle should not be registered from invalid JSON")
	}
}

func TestHandleRegistration_FallbackVIN(t *testing.T) {
	store := state.NewStore()
	c := newTestClient(store)

	// Registration message with empty VIN — should use topic VIN.
	reg := messages.RegistrationMessage{
		VIN:        "",
		PairingPIN: "654321",
		Timestamp:  time.Now().Unix(),
	}
	payload, _ := json.Marshal(reg)

	c.handleRegistration("TOPIC-VIN", payload)

	v := store.GetVehicle("TOPIC-VIN")
	if v == nil {
		t.Fatal("expected vehicle to be registered with topic VIN as fallback")
	}
}

// --- handleStatusResponse ---

func TestHandleStatusResponse_UpdatesState(t *testing.T) {
	store := state.NewStore()
	store.RegisterVehicle("VIN1", "123456")

	c := newTestClient(store)

	locked := true
	speed := 0.0
	resp := messages.StatusResponse{
		RequestID: "req-1",
		VIN:       "VIN1",
		IsLocked:  &locked,
		Speed:     &speed,
		Timestamp: time.Now().Unix(),
	}
	payload, _ := json.Marshal(resp)

	c.handleStatusResponse("VIN1", payload)

	v := store.GetVehicle("VIN1")
	if v.IsLocked == nil || !*v.IsLocked {
		t.Errorf("IsLocked = %v, want true", v.IsLocked)
	}
	if v.Speed == nil || *v.Speed != 0.0 {
		t.Errorf("Speed = %v, want 0.0", v.Speed)
	}
}

func TestHandleStatusResponse_InvalidJSON(t *testing.T) {
	store := state.NewStore()
	c := newTestClient(store)

	// Should not panic.
	c.handleStatusResponse("VIN1", []byte("{{"))
}

// --- parseTopic ---

func TestParseTopic(t *testing.T) {
	tests := []struct {
		topic      string
		wantVIN    string
		wantSuffix string
		wantOK     bool
	}{
		{"vehicles/VIN1/command_responses", "VIN1", "command_responses", true},
		{"vehicles/DEMO123/telemetry", "DEMO123", "telemetry", true},
		{"vehicles/VIN1/registration", "VIN1", "registration", true},
		{"vehicles/VIN1/status_response", "VIN1", "status_response", true},
		{"other/VIN1/commands", "", "", false},
		{"vehicles/VIN1", "", "", false},
		{"singlepart", "", "", false},
		{"", "", "", false},
	}

	for _, tt := range tests {
		t.Run(tt.topic, func(t *testing.T) {
			vin, suffix, ok := parseTopic(tt.topic)
			if ok != tt.wantOK {
				t.Errorf("parseTopic(%q) ok = %v, want %v", tt.topic, ok, tt.wantOK)
			}
			if vin != tt.wantVIN {
				t.Errorf("parseTopic(%q) vin = %q, want %q", tt.topic, vin, tt.wantVIN)
			}
			if suffix != tt.wantSuffix {
				t.Errorf("parseTopic(%q) suffix = %q, want %q", tt.topic, suffix, tt.wantSuffix)
			}
		})
	}
}
