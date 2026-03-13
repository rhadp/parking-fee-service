package databroker

import (
	"strings"
	"testing"
	"time"
)

// TS-02-12: Standard Signal Latitude metadata
// Requirement: 02-REQ-4.3
func TestSignalStandardLatitude(t *testing.T) {
	requireLiveDatabroker(t)
	startDatabroker(t)

	const signal = "Vehicle.CurrentLocation.Latitude"
	out, err := grpcGetMetadata(tcpEndpoint, signal)
	if err != nil {
		t.Fatalf("GetMetadata(%s) failed: %v\noutput: %s", signal, err, out)
	}
	lc := strings.ToUpper(out)
	if !strings.Contains(lc, "DOUBLE") {
		t.Errorf("GetMetadata(%s) does not show DOUBLE datatype: %s", signal, out)
	}
}

// TS-02-13: Standard Signal Longitude metadata
// Requirement: 02-REQ-4.4
func TestSignalStandardLongitude(t *testing.T) {
	requireLiveDatabroker(t)
	startDatabroker(t)

	const signal = "Vehicle.CurrentLocation.Longitude"
	out, err := grpcGetMetadata(tcpEndpoint, signal)
	if err != nil {
		t.Fatalf("GetMetadata(%s) failed: %v\noutput: %s", signal, err, out)
	}
	lc := strings.ToUpper(out)
	if !strings.Contains(lc, "DOUBLE") {
		t.Errorf("GetMetadata(%s) does not show DOUBLE datatype: %s", signal, out)
	}
}

// TS-02-14: Standard Signal Speed metadata
// Requirement: 02-REQ-4.5
func TestSignalStandardSpeed(t *testing.T) {
	requireLiveDatabroker(t)
	startDatabroker(t)

	const signal = "Vehicle.Speed"
	out, err := grpcGetMetadata(tcpEndpoint, signal)
	if err != nil {
		t.Fatalf("GetMetadata(%s) failed: %v\noutput: %s", signal, err, out)
	}
	lc := strings.ToUpper(out)
	if !strings.Contains(lc, "FLOAT") {
		t.Errorf("GetMetadata(%s) does not show FLOAT datatype: %s", signal, out)
	}
}

// TS-02-9: Custom Signal Set/Get Roundtrip
// Requirement: 02-REQ-3.4
func TestSignalCustomSetGet(t *testing.T) {
	requireLiveDatabroker(t)
	startDatabroker(t)

	const signal = "Vehicle.Parking.SessionActive"

	// Set the signal to true.
	setOut, err := grpcSetBool(tcpEndpoint, signal, true)
	if err != nil {
		t.Fatalf("Set(%s, true) failed: %v\noutput: %s", signal, err, setOut)
	}

	// Read it back and expect true.
	getOut, err := grpcGet(tcpEndpoint, signal)
	if err != nil {
		t.Fatalf("Get(%s) after Set failed: %v\noutput: %s", signal, err, getOut)
	}
	if !strings.Contains(getOut, "true") {
		t.Errorf("Get(%s) after Set(true): expected 'true' in response, got: %s", signal, getOut)
	}
}

// TS-02-15: Pub/Sub Notification
// Requirement: 02-REQ-5.1
func TestPubSubNotification(t *testing.T) {
	requireLiveDatabroker(t)
	startDatabroker(t)

	const signal = "Vehicle.Parking.SessionActive"

	captured := grpcSubscribeCapture(t, tcpEndpoint, signal, 8*time.Second, func() {
		if out, err := grpcSetBool(tcpEndpoint, signal, true); err != nil {
			t.Errorf("Set(%s, true) failed: %v\noutput: %s", signal, err, out)
		}
	})

	if !strings.Contains(captured, "true") {
		t.Errorf("Subscribe(%s): did not receive 'true' in notification stream:\n%s", signal, captured)
	}
}

// TS-02-16: Boolean Set/Get Roundtrip
// Requirement: 02-REQ-5.2
func TestBooleanRoundtrip(t *testing.T) {
	requireLiveDatabroker(t)
	startDatabroker(t)

	const signal = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked"

	setOut, err := grpcSetBool(tcpEndpoint, signal, true)
	if err != nil {
		t.Fatalf("Set(%s, true) failed: %v\noutput: %s", signal, err, setOut)
	}

	getOut, err := grpcGet(tcpEndpoint, signal)
	if err != nil {
		t.Fatalf("Get(%s) failed: %v\noutput: %s", signal, err, getOut)
	}
	if !strings.Contains(getOut, "true") {
		t.Errorf("Get(%s) after Set(true): expected 'true', got: %s", signal, getOut)
	}
}

// TS-02-17: String/JSON Set/Get Roundtrip
// Requirement: 02-REQ-5.3
func TestStringJsonRoundtrip(t *testing.T) {
	requireLiveDatabroker(t)
	startDatabroker(t)

	const signal = "Vehicle.Command.Door.Lock"
	const payload = `{"command_id":"abc-123","action":"lock","doors":["driver"],"source":"companion_app","vin":"WDB123","timestamp":1700000000}`

	setOut, err := grpcSetString(tcpEndpoint, signal, payload)
	if err != nil {
		t.Fatalf("Set(%s) failed: %v\noutput: %s", signal, err, setOut)
	}

	getOut, err := grpcGet(tcpEndpoint, signal)
	if err != nil {
		t.Fatalf("Get(%s) failed: %v\noutput: %s", signal, err, getOut)
	}
	// The response should contain key parts of the JSON payload.
	for _, fragment := range []string{"abc-123", "lock", "companion_app"} {
		if !strings.Contains(getOut, fragment) {
			t.Errorf("Get(%s): response missing %q from expected JSON payload:\n%s", signal, fragment, getOut)
		}
	}
}
