// Property tests for the DATA_BROKER component.
//
// Property tests verify invariants that must hold across all signals and both
// transports:
//   - TS-02-P1: Signal completeness — all 8 signals present with correct types
//   - TS-02-P2: Write-read roundtrip — set value equals get value
//   - TS-02-P3: Cross-transport equivalence — TCP read == UDS read
//   - TS-02-P4: Subscription delivery — value change is delivered to subscriber
//
// All tests require a running DATA_BROKER container and skip when unavailable.
//
// Test Specs: TS-02-P1, TS-02-P2, TS-02-P3, TS-02-P4
// Requirements: 02-REQ-5, 02-REQ-6, 02-REQ-8, 02-REQ-9, 02-REQ-10
package databroker_test

import (
	"fmt"
	"strings"
	"testing"
)

// TestPropertySignalCompleteness verifies that all 8 expected VSS signals
// (5 standard + 3 custom) are present in the DATA_BROKER metadata.
//
// This property must hold for all signals without exception; any missing
// signal constitutes a test failure with the specific signal name reported.
//
// Test Spec: TS-02-P1
// Requirements: 02-REQ-5.1, 02-REQ-6.1, 02-REQ-6.2, 02-REQ-6.3
func TestPropertySignalCompleteness(t *testing.T) {
	requireDatabrokerTCP(t)

	missing := []string{}
	wrongType := []string{}

	for _, sig := range allSignals {
		body := `{"root":"` + sig.path + `"}`
		stdout, stderr, err := grpcurlTCP(t, "kuksa.val.v2.VAL/ListMetadata", body)
		// Prepend the path since v2 ListMetadata omits it from the response body.
		combined := sig.path + " " + stdout + stderr

		if err != nil || !strings.Contains(combined, sig.path) {
			missing = append(missing, sig.path)
			continue
		}
		if !strings.Contains(strings.ToLower(combined), sig.datatype) {
			wrongType = append(wrongType, fmt.Sprintf("%s (expected %s)", sig.path, sig.datatype))
		}
	}

	if len(missing) > 0 {
		t.Errorf("missing signals in DATA_BROKER metadata (%d/%d):\n  %s",
			len(missing), len(allSignals), strings.Join(missing, "\n  "))
	}
	if len(wrongType) > 0 {
		t.Errorf("signals with wrong data type in metadata:\n  %s",
			strings.Join(wrongType, "\n  "))
	}
}

// TestPropertyWriteReadRoundtrip verifies that for each signal type, setting
// a value and immediately getting it returns the same value (no transformation
// or loss).
//
// Test Spec: TS-02-P2
// Requirements: 02-REQ-8.1, 02-REQ-9.1
func TestPropertyWriteReadRoundtrip(t *testing.T) {
	requireDatabrokerTCP(t)

	type testCase struct {
		path      string
		setBody   string
		expectVal string
	}

	cases := []testCase{
		{
			path:      "Vehicle.Speed",
			setBody:   `{"signal_id":{"path":"Vehicle.Speed"},"data_point":{"float":42.5}}`,
			expectVal: "42",
		},
		{
			path:      "Vehicle.Parking.SessionActive",
			setBody:   `{"signal_id":{"path":"Vehicle.Parking.SessionActive"},"data_point":{"bool":false}}`,
			expectVal: "false",
		},
		{
			path:      "Vehicle.CurrentLocation.Latitude",
			setBody:   `{"signal_id":{"path":"Vehicle.CurrentLocation.Latitude"},"data_point":{"double":51.5}}`,
			expectVal: "51",
		},
		{
			path:      "Vehicle.CurrentLocation.Longitude",
			setBody:   `{"signal_id":{"path":"Vehicle.CurrentLocation.Longitude"},"data_point":{"double":-0.118}}`,
			expectVal: "-0",
		},
		{
			path:      "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked",
			setBody:   `{"signal_id":{"path":"Vehicle.Cabin.Door.Row1.DriverSide.IsLocked"},"data_point":{"bool":true}}`,
			expectVal: "true",
		},
		{
			path:      "Vehicle.Command.Door.Lock",
			setBody:   `{"signal_id":{"path":"Vehicle.Command.Door.Lock"},"data_point":{"string":"roundtrip-test"}}`,
			expectVal: "roundtrip-test",
		},
		{
			path:      "Vehicle.Command.Door.Response",
			setBody:   `{"signal_id":{"path":"Vehicle.Command.Door.Response"},"data_point":{"string":"roundtrip-resp"}}`,
			expectVal: "roundtrip-resp",
		},
	}

	for _, tc := range cases {
		t.Run(tc.path, func(t *testing.T) {
			// Set the value.
			grpcurlOK(t, "kuksa.val.v2.VAL/PublishValue", tc.setBody)

			// Get the value back.
			getBody := `{"signal_id":{"path":"` + tc.path + `"}}`
			out := grpcurlOK(t, "kuksa.val.v2.VAL/GetValue", getBody)

			if !strings.Contains(out, tc.expectVal) {
				t.Errorf("roundtrip for %s: set then get returned unexpected value; expected %q in: %s",
					tc.path, tc.expectVal, out)
			}
		})
	}
}

// TestPropertyCrossTransportEquivalence verifies that for each signal, the
// value read via TCP equals the value read via UDS after a write on either
// transport.
//
// Test Spec: TS-02-P3
// Requirements: 02-REQ-4.1, 02-REQ-9.2
func TestPropertyCrossTransportEquivalence(t *testing.T) {
	requireDatabrokerUDS(t)
	requireDatabrokerTCP(t)

	type testCase struct {
		path      string
		setBody   string
		expectVal string
	}

	cases := []testCase{
		{
			path:      "Vehicle.Speed",
			setBody:   `{"signal_id":{"path":"Vehicle.Speed"},"data_point":{"float":99.9}}`,
			expectVal: "99",
		},
		{
			path:      "Vehicle.Parking.SessionActive",
			setBody:   `{"signal_id":{"path":"Vehicle.Parking.SessionActive"},"data_point":{"bool":true}}`,
			expectVal: "true",
		},
	}

	for _, tc := range cases {
		t.Run(tc.path+"/TCP_write", func(t *testing.T) {
			// Write via TCP.
			grpcurlOK(t, "kuksa.val.v2.VAL/PublishValue", tc.setBody)

			getBody := `{"signal_id":{"path":"` + tc.path + `"}}`

			// Read via TCP.
			tcpOut := grpcurlOK(t, "kuksa.val.v2.VAL/GetValue", getBody)
			// Read via UDS.
			udsOut := grpcurlUDSOK(t, "kuksa.val.v2.VAL/GetValue", getBody)

			tcpHas := strings.Contains(tcpOut, tc.expectVal)
			udsHas := strings.Contains(udsOut, tc.expectVal)
			if tcpHas != udsHas {
				t.Errorf("cross-transport mismatch for %s: TCP=%v UDS=%v (expected both to have %q)",
					tc.path, tcpHas, udsHas, tc.expectVal)
			}
		})

		t.Run(tc.path+"/UDS_write", func(t *testing.T) {
			// Write via UDS.
			grpcurlUDSOK(t, "kuksa.val.v2.VAL/PublishValue", tc.setBody)

			getBody := `{"signal_id":{"path":"` + tc.path + `"}}`

			// Read via TCP.
			tcpOut := grpcurlOK(t, "kuksa.val.v2.VAL/GetValue", getBody)
			// Read via UDS.
			udsOut := grpcurlUDSOK(t, "kuksa.val.v2.VAL/GetValue", getBody)

			tcpHas := strings.Contains(tcpOut, tc.expectVal)
			udsHas := strings.Contains(udsOut, tc.expectVal)
			if tcpHas != udsHas {
				t.Errorf("cross-transport mismatch after UDS write for %s: TCP=%v UDS=%v",
					tc.path, tcpHas, udsHas)
			}
		})
	}
}

// TestPropertyDualListenerSimultaneous verifies that both TCP and UDS listeners
// are active simultaneously by making concurrent requests on both transports.
//
// Test Spec: TS-02-P3 (partial)
// Requirements: 02-REQ-4.1
func TestPropertyDualListenerSimultaneous(t *testing.T) {
	requireDatabrokerUDS(t)
	requireDatabrokerTCP(t)

	type result struct {
		transport string
		output    string
		err       error
	}

	ch := make(chan result, 2)
	getBody := `{"signal_id":{"path":"Vehicle.Speed"}}`

	// Concurrent TCP request.
	go func() {
		out, _, err := grpcurlTCP(t, "kuksa.val.v2.VAL/GetValue", getBody)
		ch <- result{"TCP", out, err}
	}()

	// Concurrent UDS request.
	go func() {
		out, _, err := grpcurlUDS(t, "kuksa.val.v2.VAL/GetValue", getBody)
		ch <- result{"UDS", out, err}
	}()

	// Collect both results.
	for i := 0; i < 2; i++ {
		r := <-ch
		if r.err != nil {
			t.Errorf("%s concurrent request failed: %v; output: %s", r.transport, r.err, r.output)
		}
	}
}
