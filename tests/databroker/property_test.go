package databroker

import (
	"strings"
	"testing"
	"time"
)

// catalogSignal holds expected signal metadata for property tests.
type catalogSignal struct {
	path     string
	datatype string // uppercase substring expected in GetMetadata response
}

var customSignals = []catalogSignal{
	{"Vehicle.Parking.SessionActive", "BOOL"},
	{"Vehicle.Command.Door.Lock", "STRING"},
	{"Vehicle.Command.Door.Response", "STRING"},
}

var standardSignals = []catalogSignal{
	{"Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", "BOOL"},
	{"Vehicle.Cabin.Door.Row1.DriverSide.IsOpen", "BOOL"},
	{"Vehicle.CurrentLocation.Latitude", "DOUBLE"},
	{"Vehicle.CurrentLocation.Longitude", "DOUBLE"},
	{"Vehicle.Speed", "FLOAT"},
}

// allCatalogSignals is the union of custom and standard signals.
var allCatalogSignals = append(customSignals, standardSignals...)

// TS-02-P1: Dual Listener Availability (Property 1)
// Validates: 02-REQ-1.1, 02-REQ-1.2, 02-REQ-1.4
func TestPropertyDualListenerAvailability(t *testing.T) {
	requireLiveDatabroker(t)
	startDatabroker(t)
	requireUDSSocket(t) // skip if socket not visible from host (e.g. Podman on macOS)

	for _, sig := range allCatalogSignals {
		t.Run(sig.path, func(t *testing.T) {
			tcpOut, tcpErr := grpcGetMetadata(tcpEndpoint, sig.path)
			udsOut, udsErr := grpcGetMetadata(udsEndpoint, sig.path)

			if tcpErr != nil {
				t.Errorf("TCP GetMetadata(%s) failed: %v\n%s", sig.path, tcpErr, tcpOut)
			}
			if udsErr != nil {
				t.Errorf("UDS GetMetadata(%s) failed: %v\n%s", sig.path, udsErr, udsOut)
			}

			if tcpErr == nil && udsErr == nil {
				tcpHas := strings.Contains(strings.ToUpper(tcpOut), sig.datatype)
				udsHas := strings.Contains(strings.ToUpper(udsOut), sig.datatype)
				if tcpHas != udsHas {
					t.Errorf("TCP and UDS differ on %s: TCP has %s=%v, UDS has %s=%v",
						sig.path, sig.datatype, tcpHas, sig.datatype, udsHas)
				}
			}
		})
	}
}

// TS-02-P2: Custom Signal Completeness (Property 2)
// Validates: 02-REQ-3.1, 02-REQ-3.2, 02-REQ-3.3
func TestPropertyCustomSignalCompleteness(t *testing.T) {
	requireLiveDatabroker(t)
	startDatabroker(t)

	for _, sig := range customSignals {
		t.Run(sig.path, func(t *testing.T) {
			out, err := grpcGetMetadata(tcpEndpoint, sig.path)
			if err != nil {
				t.Fatalf("GetMetadata(%s) failed: %v\n%s", sig.path, err, out)
			}
			if !strings.Contains(out, sig.path) {
				t.Errorf("GetMetadata response does not contain signal path %s:\n%s", sig.path, out)
			}
			if !strings.Contains(strings.ToUpper(out), sig.datatype) {
				t.Errorf("GetMetadata(%s): expected datatype %s in response:\n%s", sig.path, sig.datatype, out)
			}
		})
	}
}

// TS-02-P3: Standard Signal Availability (Property 3)
// Validates: 02-REQ-4.1, 02-REQ-4.2, 02-REQ-4.3, 02-REQ-4.4, 02-REQ-4.5
func TestPropertyStandardSignalAvailability(t *testing.T) {
	requireLiveDatabroker(t)
	startDatabroker(t)

	for _, sig := range standardSignals {
		t.Run(sig.path, func(t *testing.T) {
			out, err := grpcGetMetadata(tcpEndpoint, sig.path)
			if err != nil {
				t.Fatalf("GetMetadata(%s) failed: %v\n%s", sig.path, err, out)
			}
			if !strings.Contains(strings.ToUpper(out), sig.datatype) {
				t.Errorf("GetMetadata(%s): expected datatype %s in response:\n%s", sig.path, sig.datatype, out)
			}
		})
	}
}

// setGetCase describes a signal value pair for roundtrip tests.
type setGetCase struct {
	signal string
	setter func(endpoint string) (string, error)
	expect string // substring expected in Get response
}

// TS-02-P4: Set/Get Roundtrip Integrity (Property 4)
// Validates: 02-REQ-3.4, 02-REQ-5.2, 02-REQ-5.3
func TestPropertySetGetRoundtrip(t *testing.T) {
	requireLiveDatabroker(t)
	startDatabroker(t)

	cases := []setGetCase{
		{
			signal: "Vehicle.Parking.SessionActive",
			setter: func(ep string) (string, error) { return grpcSetBool(ep, "Vehicle.Parking.SessionActive", true) },
			expect: "true",
		},
		{
			signal: "Vehicle.Parking.SessionActive",
			setter: func(ep string) (string, error) { return grpcSetBool(ep, "Vehicle.Parking.SessionActive", false) },
			expect: "false",
		},
		{
			signal: "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked",
			setter: func(ep string) (string, error) {
				return grpcSetBool(ep, "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", true)
			},
			expect: "true",
		},
		{
			signal: "Vehicle.Speed",
			setter: func(ep string) (string, error) { return grpcSetFloat(ep, "Vehicle.Speed", "50.5") },
			expect: "50",
		},
		{
			signal: "Vehicle.Command.Door.Lock",
			setter: func(ep string) (string, error) {
				return grpcSetString(ep, "Vehicle.Command.Door.Lock", `{"command_id":"x","action":"lock"}`)
			},
			expect: "lock",
		},
		{
			signal: "Vehicle.Command.Door.Lock",
			setter: func(ep string) (string, error) {
				return grpcSetString(ep, "Vehicle.Command.Door.Lock", `{"command_id":"y","action":"unlock"}`)
			},
			expect: "unlock",
		},
	}

	for i, tc := range cases {
		t.Run(strings.ReplaceAll(tc.signal, ".", "_")+"_case"+strings.Repeat("I", i+1), func(t *testing.T) {
			setOut, err := tc.setter(tcpEndpoint)
			if err != nil {
				t.Fatalf("Set(%s) failed: %v\n%s", tc.signal, err, setOut)
			}

			getOut, err := grpcGet(tcpEndpoint, tc.signal)
			if err != nil {
				t.Fatalf("Get(%s) after Set failed: %v\n%s", tc.signal, err, getOut)
			}
			if !strings.Contains(getOut, tc.expect) {
				t.Errorf("Get(%s): expected %q in response, got: %s", tc.signal, tc.expect, getOut)
			}
		})
	}
}

// TS-02-P5: Pub/Sub Notification Delivery (Property 5)
// Validates: 02-REQ-5.1
func TestPropertyPubSubDelivery(t *testing.T) {
	requireLiveDatabroker(t)
	startDatabroker(t)

	subtests := []struct {
		signal string
		setter func() (string, error)
		expect string
	}{
		{
			signal: "Vehicle.Parking.SessionActive",
			setter: func() (string, error) { return grpcSetBool(tcpEndpoint, "Vehicle.Parking.SessionActive", true) },
			expect: "true",
		},
		{
			signal: "Vehicle.Speed",
			setter: func() (string, error) { return grpcSetFloat(tcpEndpoint, "Vehicle.Speed", "99.0") },
			expect: "99",
		},
	}

	for _, st := range subtests {
		t.Run(st.signal, func(t *testing.T) {
			captured := grpcSubscribeCapture(t, tcpEndpoint, st.signal, 8*time.Second, func() {
				out, err := st.setter()
				if err != nil {
					t.Errorf("Set(%s) failed in pub/sub test: %v\n%s", st.signal, err, out)
				}
			})
			if !strings.Contains(captured, st.expect) {
				t.Errorf("Subscribe(%s): expected %q in notification, got:\n%s", st.signal, st.expect, captured)
			}
		})
	}
}
