package databroker_test

// property_test.go — property-based invariant tests for the DATA_BROKER.
//
// These tests verify cross-cutting properties that must hold for all signals:
// signal completeness, write-read idempotency, and cross-transport equivalence.
//
// API value field names in kuksa.val.v2 Datapoint.value (oneof):
//   "float", "bool", "double", "string" (NOT float_value etc.)
//
// Tests: TS-02-P1, TS-02-P2, TS-02-P3.
// Requirements: 02-REQ-5.1, 02-REQ-5.2, 02-REQ-6.1–6.4, 02-REQ-4.1,
//               02-REQ-8.1, 02-REQ-9.1, 02-REQ-9.2.

import (
	"fmt"
	"strings"
	"testing"
)

// TestSignalCompleteness (TS-02-P1) verifies that ALL 8 expected signals are
// present in the DATA_BROKER metadata with the correct data types.
//
// A successful ListMetadata call (grpcurl exits 0) confirms the signal exists.
// The response must contain the expected DATA_TYPE_* string.
func TestSignalCompleteness(t *testing.T) {
	requireTCPReachable(t)
	requireGrpcurl(t)

	for _, sig := range allSignals {
		t.Run(sig.path, func(t *testing.T) {
			data := `{"root": "` + sig.path + `"}`
			out := grpcurlTCP(t, "kuksa.val.v2.VAL/ListMetadata", data)
			// grpcurlTCP calls t.Fatalf on non-zero exit (signal not found).
			// Here we additionally verify the type hint appears in the response.
			if !strings.Contains(out, sig.typeHint) {
				t.Errorf("signal %q: expected type hint %q in metadata, got:\n%s",
					sig.path, sig.typeHint, out)
			}
		})
	}
}

// TestWriteReadRoundtrip (TS-02-P2) verifies that for each signal, setting a
// value and immediately getting it returns exactly the value that was set.
func TestWriteReadRoundtrip(t *testing.T) {
	requireTCPReachable(t)
	requireGrpcurl(t)

	type testCase struct {
		signal   string
		field    string // field name inside "value": {...}
		setVal   string // raw JSON value
		checkVal string // expected substring in GetValue response
	}

	cases := []testCase{
		{
			signal:   "Vehicle.Speed",
			field:    "float",
			setVal:   "123",
			checkVal: "123",
		},
		{
			signal:   "Vehicle.CurrentLocation.Latitude",
			field:    "double",
			setVal:   "48.1351",
			checkVal: "48.1351",
		},
		{
			signal:   "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked",
			field:    "bool",
			setVal:   "false",
			checkVal: `"bool": false`,
		},
		{
			signal:   "Vehicle.Parking.SessionActive",
			field:    "bool",
			setVal:   "true",
			checkVal: `"bool": true`,
		},
		{
			signal:   "Vehicle.Command.Door.Lock",
			field:    "string",
			setVal:   `"{\"command_id\":\"roundtrip-test\"}"`,
			checkVal: "roundtrip-test",
		},
	}

	for _, tc := range cases {
		t.Run(fmt.Sprintf("%s=%s", tc.signal, tc.setVal), func(t *testing.T) {
			setData := fmt.Sprintf(
				`{"signal_id": {"path": %q}, "data_point": {"value": {%q: %s}}}`,
				tc.signal, tc.field, tc.setVal,
			)
			grpcurlTCP(t, "kuksa.val.v2.VAL/PublishValue", setData)

			getData := fmt.Sprintf(`{"signal_id": {"path": %q}}`, tc.signal)
			out := grpcurlTCP(t, "kuksa.val.v2.VAL/GetValue", getData)
			if !strings.Contains(out, tc.checkVal) {
				t.Errorf("roundtrip for %q: expected %q in response, got:\n%s",
					tc.signal, tc.checkVal, out)
			}
		})
	}
}

// TestCrossTransportEquivalence (TS-02-P3) verifies that for any signal, the
// value read via TCP equals the value read via UDS, regardless of which
// transport was used to write the value.
func TestCrossTransportEquivalence(t *testing.T) {
	requireTCPReachable(t)
	requireUDSSocket(t)
	requireGrpcurl(t)

	type testCase struct {
		signal string
		field  string
		valA   string // write via TCP, check in UDS read
		valB   string // write via UDS, check in TCP read
	}

	cases := []testCase{
		{
			signal: "Vehicle.Speed",
			field:  "float",
			valA:   "11",
			valB:   "22",
		},
		{
			signal: "Vehicle.Parking.SessionActive",
			field:  "bool",
			valA:   "true",
			valB:   "false",
		},
	}

	getReq := func(signal string) string {
		return fmt.Sprintf(`{"signal_id": {"path": %q}}`, signal)
	}
	setReq := func(signal, field, val string) string {
		return fmt.Sprintf(`{"signal_id": {"path": %q}, "data_point": {"value": {%q: %s}}}`,
			signal, field, val)
	}

	for _, tc := range cases {
		t.Run(tc.signal, func(t *testing.T) {
			// Direction 1: write via TCP, read via UDS.
			grpcurlTCP(t, "kuksa.val.v2.VAL/PublishValue", setReq(tc.signal, tc.field, tc.valA))
			udOut := grpcurlUDS(t, "kuksa.val.v2.VAL/GetValue", getReq(tc.signal))
			if !strings.Contains(udOut, tc.valA) {
				t.Errorf("TCP→UDS for %q: expected %q in UDS read, got:\n%s",
					tc.signal, tc.valA, udOut)
			}

			// Direction 2: write via UDS, read via TCP.
			grpcurlUDS(t, "kuksa.val.v2.VAL/PublishValue", setReq(tc.signal, tc.field, tc.valB))
			tcpOut := grpcurlTCP(t, "kuksa.val.v2.VAL/GetValue", getReq(tc.signal))
			if !strings.Contains(tcpOut, tc.valB) {
				t.Errorf("UDS→TCP for %q: expected %q in TCP read, got:\n%s",
					tc.signal, tc.valB, tcpOut)
			}
		})
	}
}
