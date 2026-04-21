// Property-based integration tests for DATA_BROKER invariants.
// These tests exercise correctness properties across the full signal set and both
// transports. All tests skip when the DATA_BROKER container is unavailable or
// grpcurl is not installed.
package databroker

import (
	"strings"
	"testing"
)

// allSignals is the complete set of 8 expected VSS signals (5 standard + 3 custom overlay).
// Used by property tests to exercise all signals systematically.
var allSignals = []struct {
	path     string
	typeHint string // expected in the ListMetadata response (case-insensitive)
}{
	{"Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", "bool"},
	{"Vehicle.Cabin.Door.Row1.DriverSide.IsOpen", "bool"},
	{"Vehicle.CurrentLocation.Latitude", "double"},
	{"Vehicle.CurrentLocation.Longitude", "double"},
	{"Vehicle.Speed", "float"},
	{"Vehicle.Parking.SessionActive", "bool"},
	{"Vehicle.Command.Door.Lock", "string"},
	{"Vehicle.Command.Door.Response", "string"},
}

// TestPropertySignalCompleteness verifies that every expected signal is present in
// the DATA_BROKER metadata with the correct type. Missing signals are reported by name.
// Test Spec: TS-02-P1
// Requirements: 02-REQ-5.1, 02-REQ-5.2, 02-REQ-6.1, 02-REQ-6.2, 02-REQ-6.3
func TestPropertySignalCompleteness(t *testing.T) {
	skipIfGrpcurlMissing(t)
	skipIfTCPNotReachable(t)

	for _, sig := range allSignals {
		t.Run(sig.path, func(t *testing.T) {
			reqJSON := `{"root": "` + sig.path + `"}`
			out := grpcurlTCP(t, "ListMetadata", reqJSON)
			outLower := strings.ToLower(out)

			if !strings.Contains(out, sig.path) {
				t.Errorf("signal %q missing from DATA_BROKER metadata", sig.path)
			}
			if !strings.Contains(outLower, sig.typeHint) {
				t.Errorf("signal %q: expected type %q in metadata, got: %s", sig.path, sig.typeHint, out)
			}
		})
	}
}

// TestPropertyWriteReadRoundtrip verifies that for each signal, setting a value and
// immediately reading it back returns the same value (write-read idempotency).
// Per TS-02-P2 pseudocode, multiple values per type are tested:
//   BOOL: [true, false], FLOAT: [0.0, 50.0, 999.9],
//   DOUBLE: [48.1351, -122.4194], STRING: ['{"command_id":"x"}', '{}']
// Roundtrip is tested via TCP for all signals.
// Test Spec: TS-02-P2
// Requirements: 02-REQ-8.1, 02-REQ-9.1
func TestPropertyWriteReadRoundtrip(t *testing.T) {
	skipIfGrpcurlMissing(t)
	skipIfTCPNotReachable(t)

	type testCase struct {
		signal  string
		value   string // JSON field name and value for PublishValueRequest
		checkIn string // substring to expect in GetValue response
	}

	cases := []testCase{
		// BOOL signals: test both true and false
		{signal: "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", value: `"bool_value": true`, checkIn: "true"},
		{signal: "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", value: `"bool_value": false`, checkIn: "false"},
		{signal: "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen", value: `"bool_value": true`, checkIn: "true"},
		{signal: "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen", value: `"bool_value": false`, checkIn: "false"},
		{signal: "Vehicle.Parking.SessionActive", value: `"bool_value": true`, checkIn: "true"},
		{signal: "Vehicle.Parking.SessionActive", value: `"bool_value": false`, checkIn: "false"},
		// FLOAT signal: test 0.0, 50.0, 999.9
		{signal: "Vehicle.Speed", value: `"float_value": 0.0`, checkIn: "0"},
		{signal: "Vehicle.Speed", value: `"float_value": 50.0`, checkIn: "50"},
		{signal: "Vehicle.Speed", value: `"float_value": 999.9`, checkIn: "999"},
		// DOUBLE signals: test 48.1351 and -122.4194
		{signal: "Vehicle.CurrentLocation.Latitude", value: `"double_value": 48.1351`, checkIn: "48"},
		{signal: "Vehicle.CurrentLocation.Latitude", value: `"double_value": -122.4194`, checkIn: "-122"},
		{signal: "Vehicle.CurrentLocation.Longitude", value: `"double_value": 11.5820`, checkIn: "11"},
		{signal: "Vehicle.CurrentLocation.Longitude", value: `"double_value": -122.4194`, checkIn: "-122"},
		// STRING signals: test JSON payloads and empty object
		{signal: "Vehicle.Command.Door.Lock", value: `"string_value": "{\"command_id\":\"x\"}"`, checkIn: "command_id"},
		{signal: "Vehicle.Command.Door.Lock", value: `"string_value": "{}"`, checkIn: "{}"},
		{signal: "Vehicle.Command.Door.Response", value: `"string_value": "{\"command_id\":\"y\"}"`, checkIn: "command_id"},
		{signal: "Vehicle.Command.Door.Response", value: `"string_value": "{}"`, checkIn: "{}"},
	}

	for _, tc := range cases {
		name := tc.signal + "/" + tc.checkIn
		t.Run(name, func(t *testing.T) {
			publishReq := `{"signal_id": {"path": "` + tc.signal + `"}, "value": {` + tc.value + `}}`
			grpcurlTCP(t, "PublishValue", publishReq)

			getReq := `{"signal_id": {"path": "` + tc.signal + `"}}`
			out := grpcurlTCP(t, "GetValue", getReq)

			if !strings.Contains(strings.ToLower(out), strings.ToLower(tc.checkIn)) {
				t.Errorf("write-read roundtrip failed for %q: expected %q in response\noutput: %s",
					tc.signal, tc.checkIn, out)
			}
		})
	}
}

// TestPropertyWriteReadRoundtripUDS verifies write-read idempotency via the UDS transport.
// This complements TestPropertyWriteReadRoundtrip (TCP) by exercising the UDS-write/UDS-read
// path for each signal type.
// Test Spec: TS-02-P2
// Requirements: 02-REQ-9.1
func TestPropertyWriteReadRoundtripUDS(t *testing.T) {
	skipIfGrpcurlMissing(t)
	skipIfTCPNotReachable(t)
	sockPath := effectiveUDSSocket(t)

	type testCase struct {
		signal  string
		value   string
		checkIn string
	}

	cases := []testCase{
		{signal: "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", value: `"bool_value": true`, checkIn: "true"},
		{signal: "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen", value: `"bool_value": false`, checkIn: "false"},
		{signal: "Vehicle.CurrentLocation.Latitude", value: `"double_value": 48.1351`, checkIn: "48"},
		{signal: "Vehicle.CurrentLocation.Longitude", value: `"double_value": 11.5820`, checkIn: "11"},
		{signal: "Vehicle.Speed", value: `"float_value": 50.0`, checkIn: "50"},
		{signal: "Vehicle.Parking.SessionActive", value: `"bool_value": true`, checkIn: "true"},
		{signal: "Vehicle.Command.Door.Lock", value: `"string_value": "uds-roundtrip"`, checkIn: "uds-roundtrip"},
		{signal: "Vehicle.Command.Door.Response", value: `"string_value": "uds-resp"`, checkIn: "uds-resp"},
	}

	for _, tc := range cases {
		t.Run(tc.signal, func(t *testing.T) {
			publishReq := `{"signal_id": {"path": "` + tc.signal + `"}, "value": {` + tc.value + `}}`
			grpcurlUDS(t, sockPath, "PublishValue", publishReq)

			getReq := `{"signal_id": {"path": "` + tc.signal + `"}}`
			out := grpcurlUDS(t, sockPath, "GetValue", getReq)

			if !strings.Contains(strings.ToLower(out), strings.ToLower(tc.checkIn)) {
				t.Errorf("UDS write-read roundtrip failed for %q: expected %q in response\noutput: %s",
					tc.signal, tc.checkIn, out)
			}
		})
	}
}

// TestPropertyCrossTransportEquivalence verifies that for every signal in the full
// set of 8, reading via TCP and reading via UDS return identical values after a write
// on either transport. Per TS-02-P3 pseudocode, this covers all signals (not just a subset).
// Test Spec: TS-02-P3
// Requirements: 02-REQ-4.1, 02-REQ-9.2
func TestPropertyCrossTransportEquivalence(t *testing.T) {
	skipIfGrpcurlMissing(t)
	skipIfTCPNotReachable(t)
	sockPath := effectiveUDSSocket(t)

	type testCase struct {
		signal  string
		pubReq  string // full PublishValueRequest JSON
		checkIn string
	}

	// All 8 signals covered for cross-transport equivalence.
	cases := []testCase{
		{
			signal:  "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked",
			pubReq:  `{"signal_id": {"path": "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked"}, "value": {"bool_value": true}}`,
			checkIn: "true",
		},
		{
			signal:  "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen",
			pubReq:  `{"signal_id": {"path": "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen"}, "value": {"bool_value": true}}`,
			checkIn: "true",
		},
		{
			signal:  "Vehicle.CurrentLocation.Latitude",
			pubReq:  `{"signal_id": {"path": "Vehicle.CurrentLocation.Latitude"}, "value": {"double_value": 37.7749}}`,
			checkIn: "37",
		},
		{
			signal:  "Vehicle.CurrentLocation.Longitude",
			pubReq:  `{"signal_id": {"path": "Vehicle.CurrentLocation.Longitude"}, "value": {"double_value": -122.4194}}`,
			checkIn: "-122",
		},
		{
			signal:  "Vehicle.Speed",
			pubReq:  `{"signal_id": {"path": "Vehicle.Speed"}, "value": {"float_value": 42.0}}`,
			checkIn: "42",
		},
		{
			signal:  "Vehicle.Parking.SessionActive",
			pubReq:  `{"signal_id": {"path": "Vehicle.Parking.SessionActive"}, "value": {"bool_value": true}}`,
			checkIn: "true",
		},
		{
			signal:  "Vehicle.Command.Door.Lock",
			pubReq:  `{"signal_id": {"path": "Vehicle.Command.Door.Lock"}, "value": {"string_value": "cross-xport-lock"}}`,
			checkIn: "cross-xport-lock",
		},
		{
			signal:  "Vehicle.Command.Door.Response",
			pubReq:  `{"signal_id": {"path": "Vehicle.Command.Door.Response"}, "value": {"string_value": "cross-xport-resp"}}`,
			checkIn: "cross-xport-resp",
		},
	}

	getReqFor := func(signal string) string {
		return `{"signal_id": {"path": "` + signal + `"}}`
	}

	for _, tc := range cases {
		t.Run("TCP_write/"+tc.signal, func(t *testing.T) {
			grpcurlTCP(t, "PublishValue", tc.pubReq)

			tcpOut := grpcurlTCP(t, "GetValue", getReqFor(tc.signal))
			udsOut := grpcurlUDS(t, sockPath, "GetValue", getReqFor(tc.signal))

			if !strings.Contains(strings.ToLower(tcpOut), strings.ToLower(tc.checkIn)) {
				t.Errorf("TCP read after TCP write: expected %q\noutput: %s", tc.checkIn, tcpOut)
			}
			if !strings.Contains(strings.ToLower(udsOut), strings.ToLower(tc.checkIn)) {
				t.Errorf("UDS read after TCP write: expected %q\noutput: %s", tc.checkIn, udsOut)
			}
		})

		t.Run("UDS_write/"+tc.signal, func(t *testing.T) {
			grpcurlUDS(t, sockPath, "PublishValue", tc.pubReq)

			tcpOut := grpcurlTCP(t, "GetValue", getReqFor(tc.signal))
			udsOut := grpcurlUDS(t, sockPath, "GetValue", getReqFor(tc.signal))

			if !strings.Contains(strings.ToLower(tcpOut), strings.ToLower(tc.checkIn)) {
				t.Errorf("TCP read after UDS write: expected %q\noutput: %s", tc.checkIn, tcpOut)
			}
			if !strings.Contains(strings.ToLower(udsOut), strings.ToLower(tc.checkIn)) {
				t.Errorf("UDS read after UDS write: expected %q\noutput: %s", tc.checkIn, udsOut)
			}
		})
	}
}
