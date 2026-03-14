package model_test

import (
	"testing"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
)

// TS-06-2: Command Payload Structure
// Valid payload parses successfully with all fields.
func TestCommandPayloadStructure_Valid(t *testing.T) {
	data := []byte(`{"command_id":"c1","type":"lock","doors":["driver"]}`)
	cmd, err := model.ParseCommand(data)
	if err != nil {
		t.Fatalf("ParseCommand(valid) returned error: %v", err)
	}
	if cmd == nil {
		t.Fatal("ParseCommand(valid) returned nil command")
	}
	if cmd.CommandID != "c1" {
		t.Errorf("CommandID = %q, want %q", cmd.CommandID, "c1")
	}
	if cmd.Type != "lock" {
		t.Errorf("Type = %q, want %q", cmd.Type, "lock")
	}
	if len(cmd.Doors) != 1 || cmd.Doors[0] != "driver" {
		t.Errorf("Doors = %v, want [driver]", cmd.Doors)
	}
}

// TS-06-2: Missing type field returns error.
func TestCommandPayloadStructure_MissingType(t *testing.T) {
	data := []byte(`{"command_id":"c1","doors":["driver"]}`)
	_, err := model.ParseCommand(data)
	if err == nil {
		t.Error("ParseCommand(missing type) returned nil error, want error")
	}
}

// TS-06-2: Missing doors field returns error.
func TestCommandPayloadStructure_MissingDoors(t *testing.T) {
	data := []byte(`{"command_id":"c1","type":"lock"}`)
	_, err := model.ParseCommand(data)
	if err == nil {
		t.Error("ParseCommand(missing doors) returned nil error, want error")
	}
}

// TS-06-2: Missing command_id field returns error.
func TestCommandPayloadStructure_MissingCommandID(t *testing.T) {
	data := []byte(`{"type":"lock","doors":["driver"]}`)
	_, err := model.ParseCommand(data)
	if err == nil {
		t.Error("ParseCommand(missing command_id) returned nil error, want error")
	}
}

// TS-06-2: Invalid type value returns error.
func TestCommandPayloadStructure_InvalidType(t *testing.T) {
	data := []byte(`{"command_id":"c1","type":"open","doors":["driver"]}`)
	_, err := model.ParseCommand(data)
	if err == nil {
		t.Error("ParseCommand(invalid type 'open') returned nil error, want error")
	}
}

// TS-06-2: Invalid JSON returns error.
func TestCommandPayloadStructure_InvalidJSON(t *testing.T) {
	data := []byte(`{invalid json`)
	_, err := model.ParseCommand(data)
	if err == nil {
		t.Error("ParseCommand(invalid JSON) returned nil error, want error")
	}
}

// TS-06-10: Response Payload Parsing
// Success response parses with empty reason.
func TestResponsePayloadParsing_Success(t *testing.T) {
	data := []byte(`{"command_id":"cmd-005","status":"success"}`)
	resp, err := model.ParseResponse(data)
	if err != nil {
		t.Fatalf("ParseResponse(success) returned error: %v", err)
	}
	if resp == nil {
		t.Fatal("ParseResponse(success) returned nil")
	}
	if resp.CommandID != "cmd-005" {
		t.Errorf("CommandID = %q, want %q", resp.CommandID, "cmd-005")
	}
	if resp.Status != "success" {
		t.Errorf("Status = %q, want %q", resp.Status, "success")
	}
	if resp.Reason != "" {
		t.Errorf("Reason = %q, want empty", resp.Reason)
	}
}

// TS-06-10: Failed response parses with reason.
func TestResponsePayloadParsing_Failed(t *testing.T) {
	data := []byte(`{"command_id":"cmd-006","status":"failed","reason":"door_open"}`)
	resp, err := model.ParseResponse(data)
	if err != nil {
		t.Fatalf("ParseResponse(failed) returned error: %v", err)
	}
	if resp == nil {
		t.Fatal("ParseResponse(failed) returned nil")
	}
	if resp.Reason != "door_open" {
		t.Errorf("Reason = %q, want %q", resp.Reason, "door_open")
	}
}

// TS-06-P5: Payload Validation Property
// For any invalid payload, ParseCommand returns an error.
func TestPropertyPayloadValidation(t *testing.T) {
	invalidPayloads := []struct {
		name string
		data string
	}{
		{"empty object", `{}`},
		{"missing type", `{"command_id":"c1","doors":["d"]}`},
		{"missing doors", `{"command_id":"c1","type":"lock"}`},
		{"missing command_id", `{"type":"lock","doors":["d"]}`},
		{"invalid type start", `{"command_id":"c1","type":"start","doors":["d"]}`},
		{"invalid type open", `{"command_id":"c1","type":"open","doors":["d"]}`},
		{"invalid type LOCK", `{"command_id":"c1","type":"LOCK","doors":["d"]}`},
		{"not json", `not json`},
		{"empty string", ``},
		{"null body", `null`},
		{"empty doors array", `{"command_id":"c1","type":"lock","doors":[]}`},
	}
	for _, tc := range invalidPayloads {
		t.Run(tc.name, func(t *testing.T) {
			_, err := model.ParseCommand([]byte(tc.data))
			if err == nil {
				t.Errorf("ParseCommand(%s) returned nil error, want error", tc.name)
			}
		})
	}
}
