package databroker_test

// signal_test.go — live gRPC tests for connectivity, metadata, set/get,
// and cross-transport consistency.
//
// All tests in this file skip gracefully when:
//   - The grpcurl binary is not installed.
//   - The databroker TCP port (55556) is not reachable.
//   - The UDS socket is not accessible from the host (macOS + Podman VM).
//
// API: kuksa.val.v2.VAL (Kuksa Databroker 0.5.0).
//   - Value fields in Datapoint.value: "float", "bool", "double", "string"
//     (NOT float_value/bool_value — see errata 02_data_broker_compose_flags.md).
//   - ListMetadata response contains a "metadata" array; the signal path is NOT
//     in the response body. Existence is verified by successful (non-error) call.
//   - GetValue response: {"dataPoint": {"value": {"float": 50}}}
//   - PublishValue request: {"signal_id": {"path": "..."}, "data_point": {"value": {"float": 50}}}
//
// Tests: TS-02-1, TS-02-2, TS-02-4, TS-02-5, TS-02-6, TS-02-7, TS-02-8,
//        TS-02-9, TS-02-12.
// Requirements: 02-REQ-2.1, 02-REQ-2.2, 02-REQ-3.1, 02-REQ-3.2, 02-REQ-4.1,
//               02-REQ-5.1, 02-REQ-5.2, 02-REQ-6.1–6.4, 02-REQ-7.1,
//               02-REQ-8.1, 02-REQ-8.2, 02-REQ-9.1, 02-REQ-9.2.

import (
	"strings"
	"testing"
)

// ---- signal definitions used by multiple tests ----

type signalDef struct {
	path     string
	typeHint string // substring expected in ListMetadata dataType field
}

// Standard VSS v5.1 signals (loaded from the bundled vss_release_4.0.json).
// Actual Kuksa v2 API data types use DATA_TYPE_BOOLEAN (not DATA_TYPE_BOOL).
var standardSignals = []signalDef{
	{"Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", "DATA_TYPE_BOOLEAN"},
	{"Vehicle.Cabin.Door.Row1.DriverSide.IsOpen", "DATA_TYPE_BOOLEAN"},
	{"Vehicle.CurrentLocation.Latitude", "DATA_TYPE_DOUBLE"},
	{"Vehicle.CurrentLocation.Longitude", "DATA_TYPE_DOUBLE"},
	{"Vehicle.Speed", "DATA_TYPE_FLOAT"},
}

// Custom overlay signals (from deployments/vss-overlay.json).
var customSignals = []signalDef{
	{"Vehicle.Parking.SessionActive", "DATA_TYPE_BOOLEAN"},
	{"Vehicle.Command.Door.Lock", "DATA_TYPE_STRING"},
	{"Vehicle.Command.Door.Response", "DATA_TYPE_STRING"},
}

// allSignals is the full set of 8 expected signals (used in property tests).
var allSignals = append(standardSignals, customSignals...) //nolint:gochecknoglobals

// ---- connectivity tests ----

// TestTCPConnectivity verifies that a gRPC client can connect to the DATA_BROKER
// via TCP on host port 55556 and receive a valid GetServerInfo response (TS-02-1,
// 02-REQ-2.1, 02-REQ-2.2).
func TestTCPConnectivity(t *testing.T) {
	requireTCPReachable(t)
	requireGrpcurl(t)

	out := grpcurlTCP(t, "kuksa.val.v2.VAL/GetServerInfo", "{}")
	if strings.TrimSpace(out) == "" {
		t.Error("GetServerInfo returned empty response")
	}
	// Expect a version field in the response.
	if !strings.Contains(out, "version") {
		t.Errorf("GetServerInfo response missing 'version' field, got:\n%s", out)
	}
}

// TestUDSConnectivity verifies that a gRPC client can connect to the DATA_BROKER
// via UDS and receive a valid response (TS-02-2, 02-REQ-3.1, 02-REQ-3.2).
func TestUDSConnectivity(t *testing.T) {
	requireUDSSocket(t)
	requireGrpcurl(t)

	out := grpcurlUDS(t, "kuksa.val.v2.VAL/GetServerInfo", "{}")
	if strings.TrimSpace(out) == "" {
		t.Error("GetServerInfo via UDS returned empty response")
	}
	if !strings.Contains(out, "version") {
		t.Errorf("GetServerInfo via UDS response missing 'version' field, got:\n%s", out)
	}
}

// ---- metadata tests ----

// TestStandardVSSSignalMetadata verifies that all 5 standard VSS v5.1 signals
// are present in the DATA_BROKER metadata with the correct data types
// (TS-02-4, 02-REQ-5.1, 02-REQ-5.2).
//
// Note: ListMetadata succeeds (exit 0) when the signal exists; the response
// contains a "metadata" array with the dataType. A NotFound error means the
// signal is missing (standard VSS tree not loaded).
func TestStandardVSSSignalMetadata(t *testing.T) {
	requireTCPReachable(t)
	requireGrpcurl(t)

	for _, sig := range standardSignals {
		t.Run(sig.path, func(t *testing.T) {
			data := `{"root": "` + sig.path + `"}`
			out := grpcurlTCP(t, "kuksa.val.v2.VAL/ListMetadata", data)
			if !strings.Contains(out, sig.typeHint) {
				t.Errorf("expected data type %q in ListMetadata response for %q, got:\n%s",
					sig.typeHint, sig.path, out)
			}
		})
	}
}

// TestCustomVSSSignalMetadata verifies that all 3 custom overlay signals are
// present in the DATA_BROKER with correct data types (TS-02-5, 02-REQ-6.1–6.4).
func TestCustomVSSSignalMetadata(t *testing.T) {
	requireTCPReachable(t)
	requireGrpcurl(t)

	for _, sig := range customSignals {
		t.Run(sig.path, func(t *testing.T) {
			data := `{"root": "` + sig.path + `"}`
			out := grpcurlTCP(t, "kuksa.val.v2.VAL/ListMetadata", data)
			if !strings.Contains(out, sig.typeHint) {
				t.Errorf("expected data type %q in ListMetadata response for %q, got:\n%s",
					sig.typeHint, sig.path, out)
			}
		})
	}
}

// ---- set/get tests via TCP ----

// TestSignalSetGetTCP verifies that signals of different types can be set and
// retrieved via TCP gRPC (TS-02-6, 02-REQ-8.1, 02-REQ-8.2).
func TestSignalSetGetTCP(t *testing.T) {
	requireTCPReachable(t)
	requireGrpcurl(t)

	t.Run("Vehicle.Speed/float", func(t *testing.T) {
		grpcurlTCP(t, "kuksa.val.v2.VAL/PublishValue",
			`{"signal_id": {"path": "Vehicle.Speed"}, "data_point": {"value": {"float": 50}}}`)

		out := grpcurlTCP(t, "kuksa.val.v2.VAL/GetValue",
			`{"signal_id": {"path": "Vehicle.Speed"}}`)
		if !strings.Contains(out, "50") {
			t.Errorf("expected value 50 in GetValue response, got:\n%s", out)
		}
	})

	t.Run("Vehicle.Parking.SessionActive/bool", func(t *testing.T) {
		grpcurlTCP(t, "kuksa.val.v2.VAL/PublishValue",
			`{"signal_id": {"path": "Vehicle.Parking.SessionActive"}, "data_point": {"value": {"bool": true}}}`)

		out := grpcurlTCP(t, "kuksa.val.v2.VAL/GetValue",
			`{"signal_id": {"path": "Vehicle.Parking.SessionActive"}}`)
		if !strings.Contains(out, "true") {
			t.Errorf("expected bool true in GetValue response, got:\n%s", out)
		}
	})

	t.Run("Vehicle.Command.Door.Lock/string", func(t *testing.T) {
		grpcurlTCP(t, "kuksa.val.v2.VAL/PublishValue",
			`{"signal_id": {"path": "Vehicle.Command.Door.Lock"}, "data_point": {"value": {"string": "{\"command_id\":\"abc\",\"action\":\"lock\"}"}}}`)

		out := grpcurlTCP(t, "kuksa.val.v2.VAL/GetValue",
			`{"signal_id": {"path": "Vehicle.Command.Door.Lock"}}`)
		if !strings.Contains(out, "abc") {
			t.Errorf("expected command_id 'abc' in GetValue response, got:\n%s", out)
		}
	})
}

// ---- set/get tests via UDS ----

// TestSignalSetGetUDS verifies that signals can be set and retrieved via UDS
// gRPC (TS-02-7, 02-REQ-9.1).
func TestSignalSetGetUDS(t *testing.T) {
	requireUDSSocket(t)
	requireGrpcurl(t)

	t.Run("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked/bool", func(t *testing.T) {
		grpcurlUDS(t, "kuksa.val.v2.VAL/PublishValue",
			`{"signal_id": {"path": "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked"}, "data_point": {"value": {"bool": true}}}`)

		out := grpcurlUDS(t, "kuksa.val.v2.VAL/GetValue",
			`{"signal_id": {"path": "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked"}}`)
		if !strings.Contains(out, "true") {
			t.Errorf("expected bool true in UDS GetValue response, got:\n%s", out)
		}
	})

	t.Run("Vehicle.CurrentLocation.Latitude/double", func(t *testing.T) {
		grpcurlUDS(t, "kuksa.val.v2.VAL/PublishValue",
			`{"signal_id": {"path": "Vehicle.CurrentLocation.Latitude"}, "data_point": {"value": {"double": 48.1351}}}`)

		out := grpcurlUDS(t, "kuksa.val.v2.VAL/GetValue",
			`{"signal_id": {"path": "Vehicle.CurrentLocation.Latitude"}}`)
		if !strings.Contains(out, "48.1351") {
			t.Errorf("expected latitude 48.1351 in UDS GetValue response, got:\n%s", out)
		}
	})
}

// ---- cross-transport consistency ----

// TestCrossTransportTCPToUDS verifies that a signal written via TCP is readable
// via UDS (TS-02-8, 02-REQ-4.1, 02-REQ-9.2).
func TestCrossTransportTCPToUDS(t *testing.T) {
	requireTCPReachable(t)
	requireUDSSocket(t)
	requireGrpcurl(t)

	// Write via TCP.
	grpcurlTCP(t, "kuksa.val.v2.VAL/PublishValue",
		`{"signal_id": {"path": "Vehicle.Speed"}, "data_point": {"value": {"float": 75.5}}}`)

	// Read via UDS.
	out := grpcurlUDS(t, "kuksa.val.v2.VAL/GetValue",
		`{"signal_id": {"path": "Vehicle.Speed"}}`)
	if !strings.Contains(out, "75.5") {
		t.Errorf("expected 75.5 in UDS GetValue after TCP write, got:\n%s", out)
	}
}

// TestCrossTransportUDSToTCP verifies that a signal written via UDS is readable
// via TCP (TS-02-9, 02-REQ-4.1, 02-REQ-9.2).
func TestCrossTransportUDSToTCP(t *testing.T) {
	requireTCPReachable(t)
	requireUDSSocket(t)
	requireGrpcurl(t)

	// Write via UDS.
	grpcurlUDS(t, "kuksa.val.v2.VAL/PublishValue",
		`{"signal_id": {"path": "Vehicle.Parking.SessionActive"}, "data_point": {"value": {"bool": true}}}`)

	// Read via TCP.
	out := grpcurlTCP(t, "kuksa.val.v2.VAL/GetValue",
		`{"signal_id": {"path": "Vehicle.Parking.SessionActive"}}`)
	if !strings.Contains(out, "true") {
		t.Errorf("expected true in TCP GetValue after UDS write, got:\n%s", out)
	}
}

// ---- permissive mode ----

// TestPermissiveModeNoAuth verifies that the DATA_BROKER accepts requests
// without any authorization token (TS-02-12, 02-REQ-7.1).
func TestPermissiveModeNoAuth(t *testing.T) {
	requireTCPReachable(t)
	requireGrpcurl(t)

	// Set a signal without credentials — should succeed.
	// grpcurlTCP fails the test on non-zero exit; reaching here means success.
	grpcurlTCP(t, "kuksa.val.v2.VAL/PublishValue",
		`{"signal_id": {"path": "Vehicle.Speed"}, "data_point": {"value": {"float": 10}}}`)
}
