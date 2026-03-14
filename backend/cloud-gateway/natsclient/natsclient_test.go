package natsclient_test

import (
	"testing"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/natsclient"
)

// TS-06-P7: NATS Subject Correctness Property
// For any VIN, the command subject is exactly vehicles.{vin}.commands.
func TestPropertyNATSSubjects(t *testing.T) {
	vins := []string{"VIN12345", "VIN67890", "ABC", "test-vin-001"}
	for _, vin := range vins {
		expected := "vehicles." + vin + ".commands"
		got := natsclient.CommandSubject(vin)
		if got != expected {
			t.Errorf("CommandSubject(%q) = %q, want %q", vin, got, expected)
		}
	}

	respExpected := "vehicles.*.command_responses"
	if got := natsclient.ResponseSubject(); got != respExpected {
		t.Errorf("ResponseSubject() = %q, want %q", got, respExpected)
	}

	telExpected := "vehicles.*.telemetry"
	if got := natsclient.TelemetrySubject(); got != telExpected {
		t.Errorf("TelemetrySubject() = %q, want %q", got, telExpected)
	}
}
