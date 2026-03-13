package databroker

import (
	"strings"
	"testing"
)

// TS-02-4: Dual Listener Connectivity
// Requirement: 02-REQ-1.4
func TestLiveDualListener(t *testing.T) {
	requireLiveDatabroker(t)
	startDatabroker(t)

	t.Run("TCP", func(t *testing.T) {
		out, err := grpcGetMetadata(tcpEndpoint, "Vehicle.Speed")
		if err != nil {
			t.Fatalf("TCP gRPC call failed: %v\noutput: %s", err, out)
		}
		if !strings.Contains(out, "Vehicle.Speed") {
			t.Errorf("TCP GetMetadata response does not mention Vehicle.Speed: %s", out)
		}
	})

	t.Run("UDS", func(t *testing.T) {
		out, err := grpcGetMetadata(udsEndpoint, "Vehicle.Speed")
		if err != nil {
			t.Fatalf("UDS gRPC call failed: %v\noutput: %s", err, out)
		}
		if !strings.Contains(out, "Vehicle.Speed") {
			t.Errorf("UDS GetMetadata response does not mention Vehicle.Speed: %s", out)
		}
	})
}

// TS-02-6: Custom Signal SessionActive metadata
// Requirement: 02-REQ-3.1
func TestSignalCustomSessionActive(t *testing.T) {
	requireLiveDatabroker(t)
	startDatabroker(t)

	const signal = "Vehicle.Parking.SessionActive"
	out, err := grpcGetMetadata(tcpEndpoint, signal)
	if err != nil {
		t.Fatalf("GetMetadata(%s) failed: %v\noutput: %s", signal, err, out)
	}
	if !strings.Contains(out, signal) {
		t.Errorf("GetMetadata response does not mention %s: %s", signal, out)
	}
	// Expect boolean datatype in metadata.
	lc := strings.ToUpper(out)
	if !strings.Contains(lc, "BOOL") {
		t.Errorf("GetMetadata(%s) does not show BOOL datatype: %s", signal, out)
	}
}

// TS-02-7: Custom Signal Door Lock Command metadata
// Requirement: 02-REQ-3.2
func TestSignalCustomDoorLock(t *testing.T) {
	requireLiveDatabroker(t)
	startDatabroker(t)

	const signal = "Vehicle.Command.Door.Lock"
	out, err := grpcGetMetadata(tcpEndpoint, signal)
	if err != nil {
		t.Fatalf("GetMetadata(%s) failed: %v\noutput: %s", signal, err, out)
	}
	if !strings.Contains(out, signal) {
		t.Errorf("GetMetadata response does not mention %s: %s", signal, out)
	}
	lc := strings.ToUpper(out)
	if !strings.Contains(lc, "STRING") {
		t.Errorf("GetMetadata(%s) does not show STRING datatype: %s", signal, out)
	}
}

// TS-02-8: Custom Signal Door Response metadata
// Requirement: 02-REQ-3.3
func TestSignalCustomDoorResponse(t *testing.T) {
	requireLiveDatabroker(t)
	startDatabroker(t)

	const signal = "Vehicle.Command.Door.Response"
	out, err := grpcGetMetadata(tcpEndpoint, signal)
	if err != nil {
		t.Fatalf("GetMetadata(%s) failed: %v\noutput: %s", signal, err, out)
	}
	if !strings.Contains(out, signal) {
		t.Errorf("GetMetadata response does not mention %s: %s", signal, out)
	}
	lc := strings.ToUpper(out)
	if !strings.Contains(lc, "STRING") {
		t.Errorf("GetMetadata(%s) does not show STRING datatype: %s", signal, out)
	}
}

// TS-02-10: Standard Signal IsLocked metadata
// Requirement: 02-REQ-4.1
func TestSignalStandardIsLocked(t *testing.T) {
	requireLiveDatabroker(t)
	startDatabroker(t)

	const signal = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked"
	out, err := grpcGetMetadata(tcpEndpoint, signal)
	if err != nil {
		t.Fatalf("GetMetadata(%s) failed: %v\noutput: %s", signal, err, out)
	}
	lc := strings.ToUpper(out)
	if !strings.Contains(lc, "BOOL") {
		t.Errorf("GetMetadata(%s) does not show BOOL datatype: %s", signal, out)
	}
}

// TS-02-11: Standard Signal IsOpen metadata
// Requirement: 02-REQ-4.2
func TestSignalStandardIsOpen(t *testing.T) {
	requireLiveDatabroker(t)
	startDatabroker(t)

	const signal = "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen"
	out, err := grpcGetMetadata(tcpEndpoint, signal)
	if err != nil {
		t.Fatalf("GetMetadata(%s) failed: %v\noutput: %s", signal, err, out)
	}
	lc := strings.ToUpper(out)
	if !strings.Contains(lc, "BOOL") {
		t.Errorf("GetMetadata(%s) does not show BOOL datatype: %s", signal, out)
	}
}
