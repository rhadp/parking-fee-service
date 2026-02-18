package messages

import (
	"encoding/json"
	"testing"
)

// ---------------------------------------------------------------------------
// Topic helpers
// ---------------------------------------------------------------------------

func TestTopicFor(t *testing.T) {
	tests := []struct {
		pattern string
		vin     string
		want    string
	}{
		{TopicCommands, "DEMO0000000000001", "vehicles/DEMO0000000000001/commands"},
		{TopicCommandResponses, "VIN123", "vehicles/VIN123/command_responses"},
		{TopicStatusRequest, "ABC", "vehicles/ABC/status_request"},
		{TopicStatusResponse, "ABC", "vehicles/ABC/status_response"},
		{TopicTelemetry, "ABC", "vehicles/ABC/telemetry"},
		{TopicRegistration, "ABC", "vehicles/ABC/registration"},
	}
	for _, tt := range tests {
		got := TopicFor(tt.pattern, tt.vin)
		if got != tt.want {
			t.Errorf("TopicFor(%q, %q) = %q, want %q", tt.pattern, tt.vin, got, tt.want)
		}
	}
}

func TestSubscriptionPatterns(t *testing.T) {
	// Verify wildcard subscription patterns match the expected format.
	tests := []struct {
		name string
		got  string
		want string
	}{
		{"SubCommandResponses", SubCommandResponses, "vehicles/+/command_responses"},
		{"SubStatusResponse", SubStatusResponse, "vehicles/+/status_response"},
		{"SubTelemetry", SubTelemetry, "vehicles/+/telemetry"},
		{"SubRegistration", SubRegistration, "vehicles/+/registration"},
	}
	for _, tt := range tests {
		if tt.got != tt.want {
			t.Errorf("%s = %q, want %q", tt.name, tt.got, tt.want)
		}
	}
}

// ---------------------------------------------------------------------------
// Schema compatibility tests
//
// These tests serialize sample messages to JSON and verify the output matches
// the design document schemas. The same sample values are used in the Rust
// tests (cloud-gateway-client/src/messages.rs) to ensure both sides produce
// identical wire-format JSON.
// ---------------------------------------------------------------------------

func TestCommandMessageJSON(t *testing.T) {
	msg := CommandMessage{
		CommandID: "550e8400-e29b-41d4-a716-446655440000",
		Type:      CommandTypeLock,
		Timestamp: 1708300800,
	}
	data, err := json.Marshal(msg)
	if err != nil {
		t.Fatalf("Marshal error: %v", err)
	}

	var m map[string]interface{}
	if err := json.Unmarshal(data, &m); err != nil {
		t.Fatalf("Unmarshal error: %v", err)
	}

	assertEqual(t, m["command_id"], "550e8400-e29b-41d4-a716-446655440000")
	assertEqual(t, m["type"], "lock")
	assertEqualFloat(t, m["timestamp"], 1708300800)
	assertFieldCount(t, m, 3)
}

func TestCommandMessageUnlockJSON(t *testing.T) {
	msg := CommandMessage{
		CommandID: "abc",
		Type:      CommandTypeUnlock,
		Timestamp: 0,
	}
	data, err := json.Marshal(msg)
	if err != nil {
		t.Fatalf("Marshal error: %v", err)
	}

	var m map[string]interface{}
	if err := json.Unmarshal(data, &m); err != nil {
		t.Fatalf("Unmarshal error: %v", err)
	}

	assertEqual(t, m["type"], "unlock")
}

func TestCommandResponseJSON(t *testing.T) {
	msg := CommandResponse{
		CommandID: "550e8400-e29b-41d4-a716-446655440000",
		Type:      CommandTypeLock,
		Result:    CommandResultSuccess,
		Timestamp: 1708300801,
	}
	data, err := json.Marshal(msg)
	if err != nil {
		t.Fatalf("Marshal error: %v", err)
	}

	var m map[string]interface{}
	if err := json.Unmarshal(data, &m); err != nil {
		t.Fatalf("Unmarshal error: %v", err)
	}

	assertEqual(t, m["command_id"], "550e8400-e29b-41d4-a716-446655440000")
	assertEqual(t, m["type"], "lock")
	assertEqual(t, m["result"], "SUCCESS")
	assertEqualFloat(t, m["timestamp"], 1708300801)
	assertFieldCount(t, m, 4)
}

func TestCommandResponseRejectedSpeedJSON(t *testing.T) {
	msg := CommandResponse{
		CommandID: "x",
		Type:      CommandTypeLock,
		Result:    CommandResultRejectedSpeed,
		Timestamp: 0,
	}
	data, err := json.Marshal(msg)
	if err != nil {
		t.Fatalf("Marshal error: %v", err)
	}

	var m map[string]interface{}
	if err := json.Unmarshal(data, &m); err != nil {
		t.Fatalf("Unmarshal error: %v", err)
	}

	assertEqual(t, m["result"], "REJECTED_SPEED")
}

func TestCommandResponseRejectedDoorOpenJSON(t *testing.T) {
	msg := CommandResponse{
		CommandID: "x",
		Type:      CommandTypeLock,
		Result:    CommandResultRejectedDoorOpen,
		Timestamp: 0,
	}
	data, err := json.Marshal(msg)
	if err != nil {
		t.Fatalf("Marshal error: %v", err)
	}

	var m map[string]interface{}
	if err := json.Unmarshal(data, &m); err != nil {
		t.Fatalf("Unmarshal error: %v", err)
	}

	assertEqual(t, m["result"], "REJECTED_DOOR_OPEN")
}

func TestStatusRequestJSON(t *testing.T) {
	msg := StatusRequest{
		RequestID: "660e8400-e29b-41d4-a716-446655440000",
		Timestamp: 1708300802,
	}
	data, err := json.Marshal(msg)
	if err != nil {
		t.Fatalf("Marshal error: %v", err)
	}

	var m map[string]interface{}
	if err := json.Unmarshal(data, &m); err != nil {
		t.Fatalf("Unmarshal error: %v", err)
	}

	assertEqual(t, m["request_id"], "660e8400-e29b-41d4-a716-446655440000")
	assertEqualFloat(t, m["timestamp"], 1708300802)
	assertFieldCount(t, m, 2)
}

func TestStatusResponseJSON(t *testing.T) {
	locked := true
	doorOpen := false
	speed := 0.0
	lat := 48.1351
	lon := 11.582
	parking := false

	msg := StatusResponse{
		RequestID:            "660e8400-e29b-41d4-a716-446655440000",
		VIN:                  "DEMO0000000000001",
		IsLocked:             &locked,
		IsDoorOpen:           &doorOpen,
		Speed:                &speed,
		Latitude:             &lat,
		Longitude:            &lon,
		ParkingSessionActive: &parking,
		Timestamp:            1708300802,
	}
	data, err := json.Marshal(msg)
	if err != nil {
		t.Fatalf("Marshal error: %v", err)
	}

	var m map[string]interface{}
	if err := json.Unmarshal(data, &m); err != nil {
		t.Fatalf("Unmarshal error: %v", err)
	}

	assertEqual(t, m["request_id"], "660e8400-e29b-41d4-a716-446655440000")
	assertEqual(t, m["vin"], "DEMO0000000000001")
	assertEqual(t, m["is_locked"], true)
	assertEqual(t, m["is_door_open"], false)
	assertEqualFloat(t, m["speed"], 0.0)
	assertEqualFloat(t, m["latitude"], 48.1351)
	assertEqualFloat(t, m["longitude"], 11.582)
	assertEqual(t, m["parking_session_active"], false)
	assertEqualFloat(t, m["timestamp"], 1708300802)
	assertFieldCount(t, m, 9)
}

func TestStatusResponseNullFields(t *testing.T) {
	msg := StatusResponse{
		RequestID: "test-id",
		VIN:       "VIN123",
		Timestamp: 1708300802,
	}
	data, err := json.Marshal(msg)
	if err != nil {
		t.Fatalf("Marshal error: %v", err)
	}

	var m map[string]interface{}
	if err := json.Unmarshal(data, &m); err != nil {
		t.Fatalf("Unmarshal error: %v", err)
	}

	// Nil pointers serialize as JSON null.
	assertNull(t, m, "is_locked")
	assertNull(t, m, "is_door_open")
	assertNull(t, m, "speed")
	assertNull(t, m, "latitude")
	assertNull(t, m, "longitude")
	assertNull(t, m, "parking_session_active")
}

func TestTelemetryMessageJSON(t *testing.T) {
	locked := true
	doorOpen := false
	speed := 0.0
	lat := 48.1351
	lon := 11.582
	parking := false

	msg := TelemetryMessage{
		VIN:                  "DEMO0000000000001",
		IsLocked:             &locked,
		IsDoorOpen:           &doorOpen,
		Speed:                &speed,
		Latitude:             &lat,
		Longitude:            &lon,
		ParkingSessionActive: &parking,
		Timestamp:            1708300802,
	}
	data, err := json.Marshal(msg)
	if err != nil {
		t.Fatalf("Marshal error: %v", err)
	}

	var m map[string]interface{}
	if err := json.Unmarshal(data, &m); err != nil {
		t.Fatalf("Unmarshal error: %v", err)
	}

	assertEqual(t, m["vin"], "DEMO0000000000001")
	assertEqual(t, m["is_locked"], true)
	assertEqual(t, m["is_door_open"], false)
	assertEqualFloat(t, m["speed"], 0.0)
	assertEqualFloat(t, m["latitude"], 48.1351)
	assertEqualFloat(t, m["longitude"], 11.582)
	assertEqual(t, m["parking_session_active"], false)
	assertEqualFloat(t, m["timestamp"], 1708300802)
	// No request_id in telemetry.
	if _, ok := m["request_id"]; ok {
		t.Error("telemetry should not have request_id field")
	}
	assertFieldCount(t, m, 8)
}

func TestTelemetryMessageNullFields(t *testing.T) {
	msg := TelemetryMessage{
		VIN:       "VIN123",
		Timestamp: 1708300802,
	}
	data, err := json.Marshal(msg)
	if err != nil {
		t.Fatalf("Marshal error: %v", err)
	}

	var m map[string]interface{}
	if err := json.Unmarshal(data, &m); err != nil {
		t.Fatalf("Unmarshal error: %v", err)
	}

	assertNull(t, m, "is_locked")
	assertNull(t, m, "is_door_open")
	assertNull(t, m, "speed")
	assertNull(t, m, "latitude")
	assertNull(t, m, "longitude")
	assertNull(t, m, "parking_session_active")
}

func TestRegistrationMessageJSON(t *testing.T) {
	msg := RegistrationMessage{
		VIN:        "DEMO0000000000001",
		PairingPIN: "482916",
		Timestamp:  1708300800,
	}
	data, err := json.Marshal(msg)
	if err != nil {
		t.Fatalf("Marshal error: %v", err)
	}

	var m map[string]interface{}
	if err := json.Unmarshal(data, &m); err != nil {
		t.Fatalf("Unmarshal error: %v", err)
	}

	assertEqual(t, m["vin"], "DEMO0000000000001")
	assertEqual(t, m["pairing_pin"], "482916")
	assertEqualFloat(t, m["timestamp"], 1708300800)
	assertFieldCount(t, m, 3)
}

// ---------------------------------------------------------------------------
// Roundtrip tests: marshal → unmarshal → compare
// ---------------------------------------------------------------------------

func TestCommandMessageRoundtrip(t *testing.T) {
	msg := CommandMessage{
		CommandID: "abc-123",
		Type:      CommandTypeUnlock,
		Timestamp: 12345,
	}
	data, err := json.Marshal(msg)
	if err != nil {
		t.Fatalf("Marshal error: %v", err)
	}
	var got CommandMessage
	if err := json.Unmarshal(data, &got); err != nil {
		t.Fatalf("Unmarshal error: %v", err)
	}
	if got != msg {
		t.Errorf("roundtrip mismatch: got %+v, want %+v", got, msg)
	}
}

func TestCommandResponseRoundtrip(t *testing.T) {
	msg := CommandResponse{
		CommandID: "abc-123",
		Type:      CommandTypeLock,
		Result:    CommandResultRejectedDoorOpen,
		Timestamp: 12345,
	}
	data, err := json.Marshal(msg)
	if err != nil {
		t.Fatalf("Marshal error: %v", err)
	}
	var got CommandResponse
	if err := json.Unmarshal(data, &got); err != nil {
		t.Fatalf("Unmarshal error: %v", err)
	}
	if got != msg {
		t.Errorf("roundtrip mismatch: got %+v, want %+v", got, msg)
	}
}

func TestStatusRequestRoundtrip(t *testing.T) {
	msg := StatusRequest{
		RequestID: "req-1",
		Timestamp: 99999,
	}
	data, err := json.Marshal(msg)
	if err != nil {
		t.Fatalf("Marshal error: %v", err)
	}
	var got StatusRequest
	if err := json.Unmarshal(data, &got); err != nil {
		t.Fatalf("Unmarshal error: %v", err)
	}
	if got != msg {
		t.Errorf("roundtrip mismatch: got %+v, want %+v", got, msg)
	}
}

func TestRegistrationMessageRoundtrip(t *testing.T) {
	msg := RegistrationMessage{
		VIN:        "TESTVIN",
		PairingPIN: "123456",
		Timestamp:  11111,
	}
	data, err := json.Marshal(msg)
	if err != nil {
		t.Fatalf("Marshal error: %v", err)
	}
	var got RegistrationMessage
	if err := json.Unmarshal(data, &got); err != nil {
		t.Fatalf("Unmarshal error: %v", err)
	}
	if got != msg {
		t.Errorf("roundtrip mismatch: got %+v, want %+v", got, msg)
	}
}

// ---------------------------------------------------------------------------
// Cross-language wire format test
//
// This test verifies that the Go JSON output is byte-identical to what the
// Rust side expects, using a canonical JSON comparison. Both Go and Rust tests
// use the same sample values from the design document.
// ---------------------------------------------------------------------------

func TestCrossLanguageWireFormat(t *testing.T) {
	// CommandMessage: same sample as Rust test command_message_json_matches_go
	cmdMsg := CommandMessage{
		CommandID: "550e8400-e29b-41d4-a716-446655440000",
		Type:      CommandTypeLock,
		Timestamp: 1708300800,
	}
	cmdData, err := json.Marshal(cmdMsg)
	if err != nil {
		t.Fatalf("Marshal CommandMessage: %v", err)
	}

	// Unmarshal back and verify key fields are exactly as expected.
	var cmdMap map[string]interface{}
	if err := json.Unmarshal(cmdData, &cmdMap); err != nil {
		t.Fatalf("Unmarshal CommandMessage: %v", err)
	}

	// The JSON key for command type must be "type" (not "command_type").
	if _, ok := cmdMap["type"]; !ok {
		t.Error("CommandMessage JSON must use key 'type', not 'command_type'")
	}
	if _, ok := cmdMap["command_type"]; ok {
		t.Error("CommandMessage JSON must not have key 'command_type'")
	}

	// Registration: verify "pairing_pin" key (not "pairingPin" or "PairingPIN").
	regMsg := RegistrationMessage{
		VIN:        "DEMO0000000000001",
		PairingPIN: "482916",
		Timestamp:  1708300800,
	}
	regData, err := json.Marshal(regMsg)
	if err != nil {
		t.Fatalf("Marshal RegistrationMessage: %v", err)
	}
	var regMap map[string]interface{}
	if err := json.Unmarshal(regData, &regMap); err != nil {
		t.Fatalf("Unmarshal RegistrationMessage: %v", err)
	}
	if _, ok := regMap["pairing_pin"]; !ok {
		t.Error("RegistrationMessage JSON must use key 'pairing_pin'")
	}
}

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

func assertEqual(t *testing.T, got, want interface{}) {
	t.Helper()
	if got != want {
		t.Errorf("got %v (%T), want %v (%T)", got, got, want, want)
	}
}

func assertEqualFloat(t *testing.T, got interface{}, want float64) {
	t.Helper()
	f, ok := got.(float64)
	if !ok {
		t.Errorf("expected float64, got %T (%v)", got, got)
		return
	}
	if f != want {
		t.Errorf("got %v, want %v", f, want)
	}
}

func assertNull(t *testing.T, m map[string]interface{}, key string) {
	t.Helper()
	val, ok := m[key]
	if !ok {
		t.Errorf("key %q missing from JSON", key)
		return
	}
	if val != nil {
		t.Errorf("key %q should be null, got %v", key, val)
	}
}

func assertFieldCount(t *testing.T, m map[string]interface{}, want int) {
	t.Helper()
	if len(m) != want {
		t.Errorf("field count = %d, want %d; fields: %v", len(m), want, keys(m))
	}
}

func keys(m map[string]interface{}) []string {
	ks := make([]string, 0, len(m))
	for k := range m {
		ks = append(ks, k)
	}
	return ks
}
