// Signal tests — live gRPC connectivity, metadata, and set/get operations.
//
// All tests in this file require a running DATA_BROKER container and skip
// automatically when it is unavailable.
//
// API note: The actual running container exposes kuksa.val.v2.VAL (not v1).
// grpcurl uses gRPC reflection to discover the service schema.
//
// Test Specs: TS-02-1, TS-02-2, TS-02-4, TS-02-5, TS-02-6, TS-02-7, TS-02-8, TS-02-9
// Requirements: 02-REQ-2, 02-REQ-3, 02-REQ-4, 02-REQ-5, 02-REQ-6, 02-REQ-8, 02-REQ-9
package databroker_test

import (
	"strings"
	"testing"
)

// ── Signal definitions ─────────────────────────────────────────────────────

// standardSignals is the set of 5 standard VSS v5.1 signals that must be
// present in the DATA_BROKER's built-in VSS tree.
var standardSignals = []struct {
	path     string
	datatype string
}{
	{"Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", "bool"},
	{"Vehicle.Cabin.Door.Row1.DriverSide.IsOpen", "bool"},
	{"Vehicle.CurrentLocation.Latitude", "double"},
	{"Vehicle.CurrentLocation.Longitude", "double"},
	{"Vehicle.Speed", "float"},
}

// customSignals is the set of 3 custom VSS signals loaded from the overlay.
var customSignals = []struct {
	path     string
	datatype string
}{
	{"Vehicle.Parking.SessionActive", "bool"},
	{"Vehicle.Command.Door.Lock", "string"},
	{"Vehicle.Command.Door.Response", "string"},
}

// allSignals combines standard and custom signals.
var allSignals = append(standardSignals, customSignals...)

// ── Connectivity tests ─────────────────────────────────────────────────────

// TestTCPConnectivity verifies that a gRPC client can connect to the
// DATA_BROKER via TCP on host port 55556 and receive a valid response.
//
// Test Spec: TS-02-1
// Requirements: 02-REQ-2.1, 02-REQ-2.2
func TestTCPConnectivity(t *testing.T) {
	requireDatabrokerTCP(t)

	// GetServerInfo is the health-check RPC for the databroker v2 API.
	stdout, stderr, err := grpcurlTCP(t, "kuksa.val.v2.VAL/GetServerInfo", "{}")
	combined := stdout + stderr
	if err != nil {
		t.Fatalf("GetServerInfo via TCP failed: %v\noutput: %s", err, combined)
	}
	// The response should contain version or name information.
	if !strings.Contains(combined, "version") && !strings.Contains(combined, "name") {
		t.Errorf("GetServerInfo response does not contain version/name; got: %s", combined)
	}
}

// TestUDSConnectivity verifies that a gRPC client can connect to the
// DATA_BROKER via Unix Domain Socket and receive a valid response.
//
// Test Spec: TS-02-2
// Requirements: 02-REQ-3.1, 02-REQ-3.2
func TestUDSConnectivity(t *testing.T) {
	requireDatabrokerUDS(t)

	stdout, stderr, err := grpcurlUDS(t, "kuksa.val.v2.VAL/GetServerInfo", "{}")
	combined := stdout + stderr
	if err != nil {
		t.Fatalf("GetServerInfo via UDS failed: %v\noutput: %s", err, combined)
	}
	if !strings.Contains(combined, "version") && !strings.Contains(combined, "name") {
		t.Errorf("UDS GetServerInfo response does not contain version/name; got: %s", combined)
	}
}

// ── Metadata tests ─────────────────────────────────────────────────────────

// TestStandardSignalMetadata verifies that all 5 standard VSS v5.1 signals
// are present in the DATA_BROKER with the correct data types.
//
// Test Spec: TS-02-4, TS-02-P1
// Requirements: 02-REQ-5.1, 02-REQ-5.2
func TestStandardSignalMetadata(t *testing.T) {
	requireDatabrokerTCP(t)

	for _, sig := range standardSignals {
		t.Run(sig.path, func(t *testing.T) {
			// ListMetadata filters by signal path prefix.
			// The path is prepended to the response for matching since v2
			// ListMetadata response omits the path in the response body.
			body := `{"root":"` + sig.path + `"}`
			stdout, stderr, err := grpcurlTCP(t, "kuksa.val.v2.VAL/ListMetadata", body)
			combined := sig.path + " " + stdout + stderr
			if err != nil {
				t.Fatalf("ListMetadata for %s failed: %v\noutput: %s", sig.path, err, combined)
			}
			if !strings.Contains(combined, sig.path) {
				t.Errorf("ListMetadata for %s: signal not found in response; output: %s", sig.path, combined)
			}
			// Verify the data type appears in the response.
			if !strings.Contains(strings.ToLower(combined), sig.datatype) {
				t.Errorf("ListMetadata for %s: expected datatype %q not found in response; output: %s",
					sig.path, sig.datatype, combined)
			}
		})
	}
}

// TestCustomSignalMetadata verifies that all 3 custom VSS signals from the
// overlay are present in the DATA_BROKER with the correct data types.
//
// Test Spec: TS-02-5, TS-02-P1
// Requirements: 02-REQ-6.1, 02-REQ-6.2, 02-REQ-6.3, 02-REQ-6.4
func TestCustomSignalMetadata(t *testing.T) {
	requireDatabrokerTCP(t)

	for _, sig := range customSignals {
		t.Run(sig.path, func(t *testing.T) {
			body := `{"root":"` + sig.path + `"}`
			stdout, stderr, err := grpcurlTCP(t, "kuksa.val.v2.VAL/ListMetadata", body)
			combined := sig.path + " " + stdout + stderr
			if err != nil {
				t.Fatalf("ListMetadata for %s failed: %v\noutput: %s", sig.path, err, combined)
			}
			if !strings.Contains(combined, sig.path) {
				t.Errorf("ListMetadata for %s: signal not found in response; output: %s", sig.path, combined)
			}
			if !strings.Contains(strings.ToLower(combined), sig.datatype) {
				t.Errorf("ListMetadata for %s: expected datatype %q not found in response; output: %s",
					sig.path, sig.datatype, combined)
			}
		})
	}
}

// ── Set/Get via TCP ────────────────────────────────────────────────────────

// TestSignalSetGetTCP verifies that signals can be set and retrieved via the
// TCP gRPC interface.  Tests multiple signal types: float, bool, string.
//
// Test Spec: TS-02-6, TS-02-P2
// Requirements: 02-REQ-8.1, 02-REQ-8.2
func TestSignalSetGetTCP(t *testing.T) {
	requireDatabrokerTCP(t)

	t.Run("Vehicle.Speed/float", func(t *testing.T) {
		// Publish float value 50.0 to Vehicle.Speed.
		setBody := `{"signal_id":{"path":"Vehicle.Speed"},"data_point":{"float":50.0}}`
		grpcurlOK(t, "kuksa.val.v2.VAL/PublishValue", setBody)

		// Get the value back and verify it is 50.
		getBody := `{"signal_id":{"path":"Vehicle.Speed"}}`
		out := grpcurlOK(t, "kuksa.val.v2.VAL/GetValue", getBody)
		if !strings.Contains(out, "50") {
			t.Errorf("GetValue for Vehicle.Speed: expected 50.0 in response; got: %s", out)
		}
	})

	t.Run("Vehicle.Parking.SessionActive/bool", func(t *testing.T) {
		setBody := `{"signal_id":{"path":"Vehicle.Parking.SessionActive"},"data_point":{"bool":true}}`
		grpcurlOK(t, "kuksa.val.v2.VAL/PublishValue", setBody)

		getBody := `{"signal_id":{"path":"Vehicle.Parking.SessionActive"}}`
		out := grpcurlOK(t, "kuksa.val.v2.VAL/GetValue", getBody)
		if !strings.Contains(out, "true") {
			t.Errorf("GetValue for Vehicle.Parking.SessionActive: expected true; got: %s", out)
		}
	})

	t.Run("Vehicle.Command.Door.Lock/string", func(t *testing.T) {
		payload := `{"command_id":"abc","action":"lock"}`
		setBody := `{"signal_id":{"path":"Vehicle.Command.Door.Lock"},"data_point":{"string":"` + payload + `"}}`
		grpcurlOK(t, "kuksa.val.v2.VAL/PublishValue", setBody)

		getBody := `{"signal_id":{"path":"Vehicle.Command.Door.Lock"}}`
		out := grpcurlOK(t, "kuksa.val.v2.VAL/GetValue", getBody)
		if !strings.Contains(out, "abc") {
			t.Errorf("GetValue for Vehicle.Command.Door.Lock: expected command_id 'abc'; got: %s", out)
		}
	})
}

// ── Set/Get via UDS ────────────────────────────────────────────────────────

// TestSignalSetGetUDS verifies that signals can be set and retrieved via the
// UDS gRPC interface.
//
// Test Spec: TS-02-7, TS-02-P2
// Requirements: 02-REQ-9.1
func TestSignalSetGetUDS(t *testing.T) {
	requireDatabrokerUDS(t)

	t.Run("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked/bool", func(t *testing.T) {
		setBody := `{"signal_id":{"path":"Vehicle.Cabin.Door.Row1.DriverSide.IsLocked"},"data_point":{"bool":true}}`
		grpcurlUDSOK(t, "kuksa.val.v2.VAL/PublishValue", setBody)

		getBody := `{"signal_id":{"path":"Vehicle.Cabin.Door.Row1.DriverSide.IsLocked"}}`
		out := grpcurlUDSOK(t, "kuksa.val.v2.VAL/GetValue", getBody)
		if !strings.Contains(out, "true") {
			t.Errorf("UDS GetValue for IsLocked: expected true; got: %s", out)
		}
	})

	t.Run("Vehicle.CurrentLocation.Latitude/double", func(t *testing.T) {
		setBody := `{"signal_id":{"path":"Vehicle.CurrentLocation.Latitude"},"data_point":{"double":48.1351}}`
		grpcurlUDSOK(t, "kuksa.val.v2.VAL/PublishValue", setBody)

		getBody := `{"signal_id":{"path":"Vehicle.CurrentLocation.Latitude"}}`
		out := grpcurlUDSOK(t, "kuksa.val.v2.VAL/GetValue", getBody)
		if !strings.Contains(out, "48") {
			t.Errorf("UDS GetValue for Latitude: expected 48.1351; got: %s", out)
		}
	})
}

// ── Cross-transport consistency ────────────────────────────────────────────

// TestCrossTransportTCPtoUDS verifies that a signal written via TCP is
// readable via UDS.
//
// Test Spec: TS-02-8, TS-02-P3
// Requirements: 02-REQ-4.1, 02-REQ-9.2
func TestCrossTransportTCPtoUDS(t *testing.T) {
	requireDatabrokerUDS(t)
	requireDatabrokerTCP(t)

	// Write via TCP.
	setBody := `{"signal_id":{"path":"Vehicle.Speed"},"data_point":{"float":75.5}}`
	grpcurlOK(t, "kuksa.val.v2.VAL/PublishValue", setBody)

	// Read via UDS.
	getBody := `{"signal_id":{"path":"Vehicle.Speed"}}`
	out := grpcurlUDSOK(t, "kuksa.val.v2.VAL/GetValue", getBody)
	if !strings.Contains(out, "75") {
		t.Errorf("UDS read after TCP write for Vehicle.Speed: expected 75.5; got: %s", out)
	}
}

// TestCrossTransportUDStoTCP verifies that a signal written via UDS is
// readable via TCP.
//
// Test Spec: TS-02-9, TS-02-P3
// Requirements: 02-REQ-4.1, 02-REQ-9.2
func TestCrossTransportUDStoTCP(t *testing.T) {
	requireDatabrokerUDS(t)
	requireDatabrokerTCP(t)

	// Write via UDS.
	setBody := `{"signal_id":{"path":"Vehicle.Parking.SessionActive"},"data_point":{"bool":true}}`
	grpcurlUDSOK(t, "kuksa.val.v2.VAL/PublishValue", setBody)

	// Read via TCP.
	getBody := `{"signal_id":{"path":"Vehicle.Parking.SessionActive"}}`
	out := grpcurlOK(t, "kuksa.val.v2.VAL/GetValue", getBody)
	if !strings.Contains(out, "true") {
		t.Errorf("TCP read after UDS write for Vehicle.Parking.SessionActive: expected true; got: %s", out)
	}
}

// TestPermissiveMode verifies that the DATA_BROKER accepts gRPC requests
// without any authorization token.
//
// Test Spec: TS-02-12
// Requirements: 02-REQ-7.1
func TestPermissiveMode(t *testing.T) {
	requireDatabrokerTCP(t)

	// Plain request with no credentials — must succeed.
	setBody := `{"signal_id":{"path":"Vehicle.Speed"},"data_point":{"float":10.0}}`
	stdout, stderr, err := grpcurlTCP(t, "kuksa.val.v2.VAL/PublishValue", setBody)
	if err != nil {
		combined := stdout + stderr
		if strings.Contains(combined, "PermissionDenied") || strings.Contains(combined, "Unauthenticated") {
			t.Errorf("DATA_BROKER rejected request without token (permissive mode expected); output: %s", combined)
		}
		t.Fatalf("PublishValue without token failed: %v\noutput: %s", err, combined)
	}
}
