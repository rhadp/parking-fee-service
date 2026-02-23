package mqtt

import "testing"

func TestCommandTopic(t *testing.T) {
	tests := []struct {
		vin      string
		expected string
	}{
		{"VIN12345", "vehicles/VIN12345/commands"},
		{"WBAPH5C55BA270000", "vehicles/WBAPH5C55BA270000/commands"},
		{"VIN_A", "vehicles/VIN_A/commands"},
	}

	for _, tc := range tests {
		got := CommandTopic(tc.vin)
		if got != tc.expected {
			t.Errorf("CommandTopic(%q) = %q, want %q", tc.vin, got, tc.expected)
		}
	}
}

func TestResponseTopic(t *testing.T) {
	got := ResponseTopic("VIN12345")
	expected := "vehicles/VIN12345/command_responses"
	if got != expected {
		t.Errorf("ResponseTopic = %q, want %q", got, expected)
	}
}

func TestTelemetryTopic(t *testing.T) {
	got := TelemetryTopic("VIN12345")
	expected := "vehicles/VIN12345/telemetry"
	if got != expected {
		t.Errorf("TelemetryTopic = %q, want %q", got, expected)
	}
}

func TestWildcardResponseTopic(t *testing.T) {
	got := WildcardResponseTopic()
	expected := "vehicles/+/command_responses"
	if got != expected {
		t.Errorf("WildcardResponseTopic = %q, want %q", got, expected)
	}
}

func TestWildcardTelemetryTopic(t *testing.T) {
	got := WildcardTelemetryTopic()
	expected := "vehicles/+/telemetry"
	if got != expected {
		t.Errorf("WildcardTelemetryTopic = %q, want %q", got, expected)
	}
}

func TestExtractVINFromTopic(t *testing.T) {
	tests := []struct {
		topic   string
		wantVIN string
		wantOK  bool
	}{
		{"vehicles/VIN12345/commands", "VIN12345", true},
		{"vehicles/VIN12345/command_responses", "VIN12345", true},
		{"vehicles/VIN12345/telemetry", "VIN12345", true},
		{"vehicles/WBAPH5C55BA270000/commands", "WBAPH5C55BA270000", true},
		{"vehicles//commands", "", false},
		{"other/VIN12345/commands", "", false},
		{"vehicles/", "", false},
		{"vehicles", "", false},
		{"", "", false},
	}

	for _, tc := range tests {
		vin, ok := ExtractVINFromTopic(tc.topic)
		if ok != tc.wantOK || vin != tc.wantVIN {
			t.Errorf("ExtractVINFromTopic(%q) = (%q, %v), want (%q, %v)",
				tc.topic, vin, ok, tc.wantVIN, tc.wantOK)
		}
	}
}
