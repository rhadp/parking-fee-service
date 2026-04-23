package databroker_test

import (
	"testing"

	pb "github.com/rhadp/parking-fee-service/gen/kuksa/val/v1"
)

// TestPropertySignalCompleteness verifies that all 8 expected VSS signals
// (5 standard + 3 custom) are present in the DATA_BROKER with correct types.
// TS-02-P1 | Requirement: 02-REQ-5.1, 02-REQ-5.2, 02-REQ-6.1, 02-REQ-6.2, 02-REQ-6.3
func TestPropertySignalCompleteness(t *testing.T) {
	skipIfTCPUnreachable(t)
	conn := connectTCP(t)
	client := newVALClient(conn)

	foundCount := 0
	for _, sig := range allSignals {
		t.Run(sig.Path, func(t *testing.T) {
			entry := getMetadata(t, client, sig.Path)
			if entry.Metadata == nil {
				t.Errorf("no metadata returned for %s", sig.Path)
				return
			}
			if entry.Metadata.DataType != sig.DataType {
				t.Errorf("expected data type %v for %s, got %v",
					sig.DataType, sig.Path, entry.Metadata.DataType)
				return
			}
			foundCount++
		})
	}

	if foundCount != 8 {
		t.Errorf("expected 8 signals, found %d", foundCount)
	}
}

// TestPropertyWriteReadRoundtrip verifies that for any signal, setting a value
// and immediately getting it returns the same value with no transformation.
// TS-02-P2 | Requirement: 02-REQ-8.1, 02-REQ-8.2, 02-REQ-9.1
func TestPropertyWriteReadRoundtrip(t *testing.T) {
	skipIfTCPUnreachable(t)
	conn := connectTCP(t)
	client := newVALClient(conn)

	type testCase struct {
		path string
		dp   *pb.Datapoint
		desc string
		// check is a function that validates the retrieved value.
		check func(t *testing.T, entry *pb.DataEntry)
	}

	cases := []testCase{
		{
			path: "Vehicle.Speed",
			desc: "float_0",
			dp:   &pb.Datapoint{Value: &pb.Datapoint_FloatValue{FloatValue: 0.0}},
			check: func(t *testing.T, e *pb.DataEntry) {
				if e.Value.GetFloatValue() != 0.0 {
					t.Errorf("expected 0.0, got %v", e.Value.GetFloatValue())
				}
			},
		},
		{
			path: "Vehicle.Speed",
			desc: "float_50",
			dp:   &pb.Datapoint{Value: &pb.Datapoint_FloatValue{FloatValue: 50.0}},
			check: func(t *testing.T, e *pb.DataEntry) {
				if e.Value.GetFloatValue() != 50.0 {
					t.Errorf("expected 50.0, got %v", e.Value.GetFloatValue())
				}
			},
		},
		{
			path: "Vehicle.Speed",
			desc: "float_999.9",
			dp:   &pb.Datapoint{Value: &pb.Datapoint_FloatValue{FloatValue: 999.9}},
			check: func(t *testing.T, e *pb.DataEntry) {
				if e.Value.GetFloatValue() != 999.9 {
					t.Errorf("expected 999.9, got %v", e.Value.GetFloatValue())
				}
			},
		},
		{
			path: "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked",
			desc: "bool_true",
			dp:   &pb.Datapoint{Value: &pb.Datapoint_BoolValue{BoolValue: true}},
			check: func(t *testing.T, e *pb.DataEntry) {
				if e.Value == nil {
					t.Fatal("expected non-nil Datapoint")
				}
				bv, ok := e.Value.Value.(*pb.Datapoint_BoolValue)
				if !ok {
					t.Fatalf("expected BoolValue oneof variant, got %T", e.Value.Value)
				}
				if !bv.BoolValue {
					t.Error("expected true, got false")
				}
			},
		},
		{
			path: "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked",
			desc: "bool_false",
			dp:   &pb.Datapoint{Value: &pb.Datapoint_BoolValue{BoolValue: false}},
			check: func(t *testing.T, e *pb.DataEntry) {
				// The bool_true case above set this signal to true.
				// Now we set it to false and verify the value changed.
				// In proto3, GetBoolValue() returns false for both unset
				// and explicitly-set-false. We use a type assertion on the
				// oneof variant to definitively confirm the server returned
				// a BoolValue (not an unset or different-type value).
				if e.Value == nil {
					t.Fatal("expected non-nil Datapoint")
				}
				bv, ok := e.Value.Value.(*pb.Datapoint_BoolValue)
				if !ok {
					t.Fatalf("expected BoolValue oneof variant, got %T", e.Value.Value)
				}
				if bv.BoolValue {
					t.Error("expected false after setting bool to false, got true")
				}
			},
		},
		{
			path: "Vehicle.CurrentLocation.Latitude",
			desc: "double_48.1351",
			dp:   &pb.Datapoint{Value: &pb.Datapoint_DoubleValue{DoubleValue: 48.1351}},
			check: func(t *testing.T, e *pb.DataEntry) {
				if e.Value.GetDoubleValue() != 48.1351 {
					t.Errorf("expected 48.1351, got %v", e.Value.GetDoubleValue())
				}
			},
		},
		{
			path: "Vehicle.CurrentLocation.Longitude",
			desc: "double_neg122.4194",
			dp:   &pb.Datapoint{Value: &pb.Datapoint_DoubleValue{DoubleValue: -122.4194}},
			check: func(t *testing.T, e *pb.DataEntry) {
				if e.Value.GetDoubleValue() != -122.4194 {
					t.Errorf("expected -122.4194, got %v", e.Value.GetDoubleValue())
				}
			},
		},
		{
			path: "Vehicle.Command.Door.Lock",
			desc: "string_json",
			dp:   &pb.Datapoint{Value: &pb.Datapoint_StringValue{StringValue: `{"command_id":"x"}`}},
			check: func(t *testing.T, e *pb.DataEntry) {
				expected := `{"command_id":"x"}`
				if e.Value.GetStringValue() != expected {
					t.Errorf("expected %q, got %q", expected, e.Value.GetStringValue())
				}
			},
		},
		{
			path: "Vehicle.Command.Door.Response",
			desc: "string_empty_json",
			dp:   &pb.Datapoint{Value: &pb.Datapoint_StringValue{StringValue: "{}"}},
			check: func(t *testing.T, e *pb.DataEntry) {
				if e.Value.GetStringValue() != "{}" {
					t.Errorf("expected %q, got %q", "{}", e.Value.GetStringValue())
				}
			},
		},
	}

	for _, tc := range cases {
		t.Run(tc.path+"/"+tc.desc, func(t *testing.T) {
			setValue(t, client, tc.path, tc.dp)
			entry := getValue(t, client, tc.path)
			if entry.Value == nil {
				t.Fatal("expected non-nil value after set")
			}
			tc.check(t, entry)
		})
	}
}

// TestPropertyCrossTransportEquivalence verifies that for any signal, the
// value read via TCP equals the value read via UDS after a write on either
// transport.
// TS-02-P3 | Requirement: 02-REQ-4.1, 02-REQ-9.1, 02-REQ-9.2
func TestPropertyCrossTransportEquivalence(t *testing.T) {
	skipIfTCPUnreachable(t)
	sockPath := skipIfUDSUnreachable(t)

	tcpConn := connectTCP(t)
	udsConn := connectUDS(t, sockPath)
	tcpClient := newVALClient(tcpConn)
	udsClient := newVALClient(udsConn)

	type crossTransportCase struct {
		path    string
		dp      *pb.Datapoint
		desc    string
		compare func(a, b *pb.DataEntry) bool
	}

	cases := []crossTransportCase{
		{
			path: "Vehicle.Speed",
			desc: "float_via_tcp",
			dp:   &pb.Datapoint{Value: &pb.Datapoint_FloatValue{FloatValue: 42.5}},
			compare: func(a, b *pb.DataEntry) bool {
				return a.Value.GetFloatValue() == b.Value.GetFloatValue()
			},
		},
		{
			path: "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen",
			desc: "bool_via_tcp",
			dp:   &pb.Datapoint{Value: &pb.Datapoint_BoolValue{BoolValue: true}},
			compare: func(a, b *pb.DataEntry) bool {
				return a.Value.GetBoolValue() == b.Value.GetBoolValue()
			},
		},
		{
			path: "Vehicle.Command.Door.Lock",
			desc: "string_via_tcp",
			dp:   &pb.Datapoint{Value: &pb.Datapoint_StringValue{StringValue: `{"test":"cross"}`}},
			compare: func(a, b *pb.DataEntry) bool {
				return a.Value.GetStringValue() == b.Value.GetStringValue()
			},
		},
	}

	for _, tc := range cases {
		// Write via TCP, verify both reads match.
		t.Run("tcp_write/"+tc.desc, func(t *testing.T) {
			setValue(t, tcpClient, tc.path, tc.dp)
			tcpEntry := getValue(t, tcpClient, tc.path)
			udsEntry := getValue(t, udsClient, tc.path)
			if tcpEntry.Value == nil || udsEntry.Value == nil {
				t.Fatal("expected non-nil values from both transports")
			}
			if !tc.compare(tcpEntry, udsEntry) {
				t.Error("TCP and UDS reads returned different values after TCP write")
			}
		})

		// Write via UDS, verify both reads match.
		t.Run("uds_write/"+tc.desc, func(t *testing.T) {
			setValue(t, udsClient, tc.path, tc.dp)
			tcpEntry := getValue(t, tcpClient, tc.path)
			udsEntry := getValue(t, udsClient, tc.path)
			if tcpEntry.Value == nil || udsEntry.Value == nil {
				t.Fatal("expected non-nil values from both transports")
			}
			if !tc.compare(tcpEntry, udsEntry) {
				t.Error("TCP and UDS reads returned different values after UDS write")
			}
		})
	}
}
