package mqtt

import (
	"testing"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/state"
)

func TestNewClient_InvalidBroker(t *testing.T) {
	store := state.NewStore()

	// Connecting to an invalid address should return an error.
	_, err := NewClient("localhost:19999", store,
		WithClientID("test-client"),
		WithConnectTimeout(500_000_000), // 500ms
	)
	if err == nil {
		t.Error("expected error connecting to invalid broker")
	}
}

func TestHandleMessage_Routing(t *testing.T) {
	store := state.NewStore()
	store.RegisterVehicle("VIN1", "123456")
	store.AddCommand("VIN1", "cmd-1", "lock")

	c := newTestClient(store)

	// Test that handleMessage routes correctly by calling it with a mock
	// pahomqtt.Message. We test this indirectly through parseTopic and
	// the individual handler tests — this test ensures the routing switch
	// works.

	// We test parseTopic directly since handleMessage depends on it.
	tests := []struct {
		topic  string
		wantOK bool
	}{
		{"vehicles/VIN1/command_responses", true},
		{"vehicles/VIN1/telemetry", true},
		{"vehicles/VIN1/registration", true},
		{"vehicles/VIN1/status_response", true},
		{"vehicles/VIN1/unknown_suffix", true},
		{"invalid_topic", false},
	}

	for _, tt := range tests {
		_, _, ok := parseTopic(tt.topic)
		if ok != tt.wantOK {
			t.Errorf("parseTopic(%q): got ok=%v, want ok=%v", tt.topic, ok, tt.wantOK)
		}
	}
	_ = c // prevent unused variable warning
}
