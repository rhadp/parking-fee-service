// Live gRPC integration tests for DATA_BROKER signal operations.
// All tests in this file skip gracefully when the DATA_BROKER container is
// not running or grpcurl is not installed.
package databroker

import (
	"strings"
	"testing"
)

// --- Connectivity tests ---

// TestTCPConnectivity verifies that a gRPC client can connect to the DATA_BROKER via TCP
// and receive a successful response.
// Test Spec: TS-02-1
// Requirements: 02-REQ-2.1, 02-REQ-2.2
func TestTCPConnectivity(t *testing.T) {
	skipIfGrpcurlMissing(t)
	skipIfTCPNotReachable(t)

	// A successful metadata query for a known signal proves the channel is working.
	out := grpcurlTCP(t, "GetValue", `{"signal_id": {"path": "Vehicle.Speed"}}`)
	if out == "" {
		t.Fatalf("expected non-empty response from GetValue via TCP, got empty output")
	}
}

// TestUDSConnectivity verifies that a gRPC client can connect to the DATA_BROKER via UDS
// and receive a successful response.
// Test Spec: TS-02-2
// Requirements: 02-REQ-3.1, 02-REQ-3.2
func TestUDSConnectivity(t *testing.T) {
	skipIfGrpcurlMissing(t)
	skipIfTCPNotReachable(t) // skip if container is not up at all
	sockPath := effectiveUDSSocket(t)

	out := grpcurlUDS(t, sockPath, "GetValue", `{"signal_id": {"path": "Vehicle.Speed"}}`)
	if out == "" {
		t.Fatalf("expected non-empty response from GetValue via UDS, got empty output")
	}
}

// --- Standard VSS signal metadata tests ---

// TestStandardVSSSignalMetadata verifies that all 5 standard VSS v5.1 signals are present
// in the DATA_BROKER metadata with the correct data types.
// Test Spec: TS-02-4, TS-02-P1
// Requirements: 02-REQ-5.1, 02-REQ-5.2
func TestStandardVSSSignalMetadata(t *testing.T) {
	skipIfGrpcurlMissing(t)
	skipIfTCPNotReachable(t)

	type signalSpec struct {
		path     string
		typeHint string // substring expected in the data type field (case-insensitive)
	}

	signals := []signalSpec{
		{"Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", "bool"},
		{"Vehicle.Cabin.Door.Row1.DriverSide.IsOpen", "bool"},
		{"Vehicle.CurrentLocation.Latitude", "double"},
		{"Vehicle.CurrentLocation.Longitude", "double"},
		{"Vehicle.Speed", "float"},
	}

	for _, sig := range signals {
		t.Run(sig.path, func(t *testing.T) {
			// Use ListMetadata to introspect signal metadata.
			reqJSON := `{"root": "` + sig.path + `"}`
			out := grpcurlTCP(t, "ListMetadata", reqJSON)
			outLower := strings.ToLower(out)

			if !strings.Contains(out, sig.path) {
				t.Errorf("ListMetadata response does not mention signal %q\noutput: %s", sig.path, out)
			}
			if !strings.Contains(outLower, sig.typeHint) {
				t.Errorf("expected type hint %q in ListMetadata response for %q\noutput: %s",
					sig.typeHint, sig.path, out)
			}
		})
	}
}

// --- Custom VSS signal metadata tests ---

// TestCustomVSSSignalMetadata verifies that all 3 custom VSS signals from the overlay are
// present in the DATA_BROKER with the correct data types.
// Test Spec: TS-02-5, TS-02-P1
// Requirements: 02-REQ-6.1, 02-REQ-6.2, 02-REQ-6.3, 02-REQ-6.4
func TestCustomVSSSignalMetadata(t *testing.T) {
	skipIfGrpcurlMissing(t)
	skipIfTCPNotReachable(t)

	type signalSpec struct {
		path     string
		typeHint string
	}

	signals := []signalSpec{
		{"Vehicle.Parking.SessionActive", "bool"},
		{"Vehicle.Command.Door.Lock", "string"},
		{"Vehicle.Command.Door.Response", "string"},
	}

	for _, sig := range signals {
		t.Run(sig.path, func(t *testing.T) {
			reqJSON := `{"root": "` + sig.path + `"}`
			out := grpcurlTCP(t, "ListMetadata", reqJSON)
			outLower := strings.ToLower(out)

			if !strings.Contains(out, sig.path) {
				t.Errorf("ListMetadata response does not mention signal %q (overlay may not be loaded)\noutput: %s",
					sig.path, out)
			}
			if !strings.Contains(outLower, sig.typeHint) {
				t.Errorf("expected type hint %q in ListMetadata response for %q\noutput: %s",
					sig.typeHint, sig.path, out)
			}
		})
	}
}

// --- Signal set/get tests via TCP ---

// TestSignalSetGetViaTCP verifies that signals of various types can be set and retrieved
// via the TCP gRPC interface.
// Test Spec: TS-02-6
// Requirements: 02-REQ-8.1, 02-REQ-8.2
func TestSignalSetGetViaTCP(t *testing.T) {
	skipIfGrpcurlMissing(t)
	skipIfTCPNotReachable(t)

	t.Run("Vehicle.Speed float", func(t *testing.T) {
		grpcurlTCP(t, "PublishValue",
			`{"signal_id": {"path": "Vehicle.Speed"}, "value": {"float_value": 50.0}}`)
		out := grpcurlTCP(t, "GetValue",
			`{"signal_id": {"path": "Vehicle.Speed"}}`)
		if !strings.Contains(out, "50") {
			t.Errorf("expected value 50 in GetValue response\noutput: %s", out)
		}
	})

	t.Run("Vehicle.Parking.SessionActive bool", func(t *testing.T) {
		grpcurlTCP(t, "PublishValue",
			`{"signal_id": {"path": "Vehicle.Parking.SessionActive"}, "value": {"bool_value": true}}`)
		out := grpcurlTCP(t, "GetValue",
			`{"signal_id": {"path": "Vehicle.Parking.SessionActive"}}`)
		if !strings.Contains(strings.ToLower(out), "true") {
			t.Errorf("expected bool value true in GetValue response\noutput: %s", out)
		}
	})

	t.Run("Vehicle.Command.Door.Lock string", func(t *testing.T) {
		const payload = `{"command_id":"abc123","action":"lock"}`
		grpcurlTCP(t, "PublishValue",
			`{"signal_id": {"path": "Vehicle.Command.Door.Lock"}, "value": {"string_value": "`+payload+`"}}`)
		out := grpcurlTCP(t, "GetValue",
			`{"signal_id": {"path": "Vehicle.Command.Door.Lock"}}`)
		if !strings.Contains(out, "abc123") {
			t.Errorf("expected command_id abc123 in GetValue response\noutput: %s", out)
		}
	})
}

// --- Signal set/get tests via UDS ---

// TestSignalSetGetViaUDS verifies that signals can be set and retrieved via the UDS interface.
// Test Spec: TS-02-7
// Requirements: 02-REQ-9.1
func TestSignalSetGetViaUDS(t *testing.T) {
	skipIfGrpcurlMissing(t)
	skipIfTCPNotReachable(t)
	sockPath := effectiveUDSSocket(t)

	t.Run("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked bool", func(t *testing.T) {
		grpcurlUDS(t, sockPath, "PublishValue",
			`{"signal_id": {"path": "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked"}, "value": {"bool_value": true}}`)
		out := grpcurlUDS(t, sockPath, "GetValue",
			`{"signal_id": {"path": "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked"}}`)
		if !strings.Contains(strings.ToLower(out), "true") {
			t.Errorf("expected bool value true in UDS GetValue response\noutput: %s", out)
		}
	})

	t.Run("Vehicle.CurrentLocation.Latitude double", func(t *testing.T) {
		grpcurlUDS(t, sockPath, "PublishValue",
			`{"signal_id": {"path": "Vehicle.CurrentLocation.Latitude"}, "value": {"double_value": 48.1351}}`)
		out := grpcurlUDS(t, sockPath, "GetValue",
			`{"signal_id": {"path": "Vehicle.CurrentLocation.Latitude"}}`)
		if !strings.Contains(out, "48") {
			t.Errorf("expected latitude value 48.1351 in UDS GetValue response\noutput: %s", out)
		}
	})
}

// --- Cross-transport consistency tests ---

// TestCrossTransportTCPWriteUDSRead verifies that a value written via TCP is readable via UDS.
// Test Spec: TS-02-8
// Requirements: 02-REQ-4.1, 02-REQ-9.2
func TestCrossTransportTCPWriteUDSRead(t *testing.T) {
	skipIfGrpcurlMissing(t)
	skipIfTCPNotReachable(t)
	sockPath := effectiveUDSSocket(t)

	// Write a distinctive speed value via TCP.
	grpcurlTCP(t, "PublishValue",
		`{"signal_id": {"path": "Vehicle.Speed"}, "value": {"float_value": 75.5}}`)

	// Read the same signal via UDS and verify the value.
	out := grpcurlUDS(t, sockPath, "GetValue",
		`{"signal_id": {"path": "Vehicle.Speed"}}`)
	if !strings.Contains(out, "75") {
		t.Errorf("expected speed value 75.5 written via TCP to be readable via UDS\noutput: %s", out)
	}
}

// TestCrossTransportUDSWriteTCPRead verifies that a value written via UDS is readable via TCP.
// Test Spec: TS-02-9
// Requirements: 02-REQ-4.1, 02-REQ-9.2
func TestCrossTransportUDSWriteTCPRead(t *testing.T) {
	skipIfGrpcurlMissing(t)
	skipIfTCPNotReachable(t)
	sockPath := effectiveUDSSocket(t)

	// Write a value via UDS.
	grpcurlUDS(t, sockPath, "PublishValue",
		`{"signal_id": {"path": "Vehicle.Parking.SessionActive"}, "value": {"bool_value": true}}`)

	// Read the same signal via TCP.
	out := grpcurlTCP(t, "GetValue",
		`{"signal_id": {"path": "Vehicle.Parking.SessionActive"}}`)
	if !strings.Contains(strings.ToLower(out), "true") {
		t.Errorf("expected bool value true written via UDS to be readable via TCP\noutput: %s", out)
	}
}

// --- Permissive mode test ---

// TestPermissiveMode verifies that the DATA_BROKER accepts requests without authorization tokens.
// Test Spec: TS-02-12
// Requirement: 02-REQ-7.1
func TestPermissiveMode(t *testing.T) {
	skipIfGrpcurlMissing(t)
	skipIfTCPNotReachable(t)

	// A plain request with no credentials must succeed.
	grpcurlTCP(t, "PublishValue",
		`{"signal_id": {"path": "Vehicle.Speed"}, "value": {"float_value": 10.0}}`)
	// Success is confirmed by grpcurlTCP not calling t.Fatal.
}
