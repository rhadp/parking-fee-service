package databroker_test

import (
	"context"
	"testing"

	kuksapb "parking-fee-service/tests/databroker/kuksa"
)

// ---------------------------------------------------------------------------
// TS-02-6: Signal set/get via TCP
// Requirement: 02-REQ-8.1, 02-REQ-8.2
// ---------------------------------------------------------------------------

// TestSignalSetGetViaTCP verifies that signals of all types can be written and
// immediately read back via the TCP gRPC interface.
func TestSignalSetGetViaTCP(t *testing.T) {
	conn := dialTCP(t)
	client := valClient(conn)

	t.Run("Vehicle.Speed/float", func(t *testing.T) {
		const path = "Vehicle.Speed"
		const want float32 = 50.0
		setSignalFloat(t, client, path, want)
		entry := getSignal(t, client, path)
		got, ok := entry.Value.Value.(*kuksapb.Datapoint_FloatValue)
		if !ok {
			t.Fatalf("TS-02-6: expected float value for %s, got %T", path, entry.Value.Value)
		}
		if got.FloatValue != want {
			t.Errorf("TS-02-6: Vehicle.Speed: got %v, want %v", got.FloatValue, want)
		}
	})

	t.Run("Vehicle.Parking.SessionActive/bool", func(t *testing.T) {
		const path = "Vehicle.Parking.SessionActive"
		setSignalBool(t, client, path, true)
		entry := getSignal(t, client, path)
		got, ok := entry.Value.Value.(*kuksapb.Datapoint_BoolValue)
		if !ok {
			t.Fatalf("TS-02-6: expected bool value for %s, got %T", path, entry.Value.Value)
		}
		if !got.BoolValue {
			t.Errorf("TS-02-6: Vehicle.Parking.SessionActive: got false, want true")
		}
	})

	t.Run("Vehicle.Command.Door.Lock/string", func(t *testing.T) {
		const path = "Vehicle.Command.Door.Lock"
		const want = `{"command_id":"abc","action":"lock"}`
		setSignalString(t, client, path, want)
		entry := getSignal(t, client, path)
		got, ok := entry.Value.Value.(*kuksapb.Datapoint_StringValue)
		if !ok {
			t.Fatalf("TS-02-6: expected string value for %s, got %T", path, entry.Value.Value)
		}
		if got.StringValue != want {
			t.Errorf("TS-02-6: Vehicle.Command.Door.Lock: got %q, want %q", got.StringValue, want)
		}
	})
}

// ---------------------------------------------------------------------------
// TS-02-7: Signal set/get via UDS
// Requirement: 02-REQ-9.1
// ---------------------------------------------------------------------------

// TestSignalSetGetViaUDS verifies that signals can be written and read back via
// the Unix Domain Socket gRPC interface.
func TestSignalSetGetViaUDS(t *testing.T) {
	conn := dialUDS(t)
	client := valClient(conn)

	t.Run("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked/bool", func(t *testing.T) {
		const path = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked"
		setSignalBool(t, client, path, true)
		entry := getSignal(t, client, path)
		got, ok := entry.Value.Value.(*kuksapb.Datapoint_BoolValue)
		if !ok {
			t.Fatalf("TS-02-7: expected bool value for %s, got %T", path, entry.Value.Value)
		}
		if !got.BoolValue {
			t.Errorf("TS-02-7: IsLocked: got false, want true")
		}
	})

	t.Run("Vehicle.CurrentLocation.Latitude/double", func(t *testing.T) {
		const path = "Vehicle.CurrentLocation.Latitude"
		const want float64 = 48.1351
		setSignalDouble(t, client, path, want)
		entry := getSignal(t, client, path)
		got, ok := entry.Value.Value.(*kuksapb.Datapoint_DoubleValue)
		if !ok {
			t.Fatalf("TS-02-7: expected double value for %s, got %T", path, entry.Value.Value)
		}
		if got.DoubleValue != want {
			t.Errorf("TS-02-7: Latitude: got %v, want %v", got.DoubleValue, want)
		}
	})
}

// ---------------------------------------------------------------------------
// TS-02-8: Cross-transport consistency (TCP write, UDS read)
// Requirement: 02-REQ-4.1, 02-REQ-9.2
// ---------------------------------------------------------------------------

// TestCrossTransportTCPWriteUDSRead verifies that a signal written via TCP is
// immediately readable with the same value via UDS.
func TestCrossTransportTCPWriteUDSRead(t *testing.T) {
	tcpConn := dialTCP(t)
	udsConn := dialUDS(t)
	tcpClient := valClient(tcpConn)
	udsClient := valClient(udsConn)

	const path = "Vehicle.Speed"
	const want float32 = 75.5

	setSignalFloat(t, tcpClient, path, want)

	entry := getSignal(t, udsClient, path)
	got, ok := entry.Value.Value.(*kuksapb.Datapoint_FloatValue)
	if !ok {
		t.Fatalf("TS-02-8: expected float value via UDS for %s, got %T", path, entry.Value.Value)
	}
	if got.FloatValue != want {
		t.Errorf("TS-02-8: TCP write / UDS read: got %v, want %v", got.FloatValue, want)
	}
}

// ---------------------------------------------------------------------------
// TS-02-9: Cross-transport consistency (UDS write, TCP read)
// Requirement: 02-REQ-4.1, 02-REQ-9.2
// ---------------------------------------------------------------------------

// TestCrossTransportUDSWriteTCPRead verifies that a signal written via UDS is
// immediately readable with the same value via TCP.
func TestCrossTransportUDSWriteTCPRead(t *testing.T) {
	tcpConn := dialTCP(t)
	udsConn := dialUDS(t)
	tcpClient := valClient(tcpConn)
	udsClient := valClient(udsConn)

	const path = "Vehicle.Parking.SessionActive"

	setSignalBool(t, udsClient, path, true)

	entry := getSignal(t, tcpClient, path)
	got, ok := entry.Value.Value.(*kuksapb.Datapoint_BoolValue)
	if !ok {
		t.Fatalf("TS-02-9: expected bool value via TCP for %s, got %T", path, entry.Value.Value)
	}
	if !got.BoolValue {
		t.Errorf("TS-02-9: UDS write / TCP read: got false, want true")
	}
}

// ---------------------------------------------------------------------------
// TS-02-P2: Write-read roundtrip property
// Requirement: 02-REQ-8.1, 02-REQ-8.2, 02-REQ-9.1
// ---------------------------------------------------------------------------

// TestWriteReadRoundtrip is a property test verifying that for every signal in
// the expected set, setting a typed value and immediately getting it returns the
// identical value (no transformation or loss).
func TestWriteReadRoundtrip(t *testing.T) {
	conn := dialTCP(t)
	client := valClient(conn)

	for _, sig := range allSignals {
		sig := sig
		t.Run(sig.path, func(t *testing.T) {
			want := testValueForType(sig.typeName)

			// Write the test value.
			ctx, cancel := context.WithTimeout(context.Background(), opTimeout)
			resp, err := client.Set(ctx, &kuksapb.SetRequest{
				Entries: []*kuksapb.DataEntry{
					{Path: sig.path, Value: want},
				},
			})
			cancel()
			if err != nil {
				t.Fatalf("TS-02-P2: Set(%q) gRPC error: %v", sig.path, err)
			}
			if !resp.Success {
				t.Fatalf("TS-02-P2: Set(%q) rejected: %s", sig.path, resp.Error)
			}

			// Read it back.
			entry := getSignal(t, client, sig.path)
			if entry.Value == nil {
				t.Fatalf("TS-02-P2: Get(%q) returned nil value", sig.path)
			}
			if !datapointEqual(entry.Value, want) {
				t.Errorf("TS-02-P2: roundtrip for %q: got %s, want %s",
					sig.path, datapointString(entry.Value), datapointString(want))
			}
		})
	}
}

// ---------------------------------------------------------------------------
// TS-02-P3: Cross-transport equivalence property
// Requirement: 02-REQ-4.1, 02-REQ-9.2
// ---------------------------------------------------------------------------

// TestCrossTransportEquivalence is a property test verifying that for every
// signal, a value written via one transport reads identically via both transports.
func TestCrossTransportEquivalence(t *testing.T) {
	tcpConn := dialTCP(t)
	udsConn := dialUDS(t)
	tcpClient := valClient(tcpConn)
	udsClient := valClient(udsConn)

	for _, sig := range allSignals {
		sig := sig
		t.Run(sig.path+"/tcp-write", func(t *testing.T) {
			want := testValueForType(sig.typeName)

			// Write via TCP.
			ctx, cancel := context.WithTimeout(context.Background(), opTimeout)
			resp, err := tcpClient.Set(ctx, &kuksapb.SetRequest{
				Entries: []*kuksapb.DataEntry{
					{Path: sig.path, Value: want},
				},
			})
			cancel()
			if err != nil {
				t.Fatalf("TS-02-P3: TCP Set(%q) gRPC error: %v", sig.path, err)
			}
			if !resp.Success {
				t.Fatalf("TS-02-P3: TCP Set(%q) rejected: %s", sig.path, resp.Error)
			}

			// Read via TCP.
			tcpEntry := getSignal(t, tcpClient, sig.path)
			// Read via UDS.
			udsEntry := getSignal(t, udsClient, sig.path)

			if !datapointEqual(tcpEntry.Value, udsEntry.Value) {
				t.Errorf("TS-02-P3: cross-transport mismatch for %q: TCP=%s UDS=%s",
					sig.path, datapointString(tcpEntry.Value), datapointString(udsEntry.Value))
			}
			if !datapointEqual(tcpEntry.Value, want) {
				t.Errorf("TS-02-P3: TCP Get(%q) != written value: got %s, want %s",
					sig.path, datapointString(tcpEntry.Value), datapointString(want))
			}
		})

		t.Run(sig.path+"/uds-write", func(t *testing.T) {
			// Use a different value to confirm the UDS write propagates.
			want := zeroValueForType(sig.typeName)

			// Write via UDS.
			ctx, cancel := context.WithTimeout(context.Background(), opTimeout)
			resp, err := udsClient.Set(ctx, &kuksapb.SetRequest{
				Entries: []*kuksapb.DataEntry{
					{Path: sig.path, Value: want},
				},
			})
			cancel()
			if err != nil {
				t.Fatalf("TS-02-P3: UDS Set(%q) gRPC error: %v", sig.path, err)
			}
			if !resp.Success {
				t.Fatalf("TS-02-P3: UDS Set(%q) rejected: %s", sig.path, resp.Error)
			}

			// Read via TCP.
			tcpEntry := getSignal(t, tcpClient, sig.path)
			// Read via UDS.
			udsEntry := getSignal(t, udsClient, sig.path)

			if !datapointEqual(tcpEntry.Value, udsEntry.Value) {
				t.Errorf("TS-02-P3: cross-transport mismatch after UDS write for %q: TCP=%s UDS=%s",
					sig.path, datapointString(tcpEntry.Value), datapointString(udsEntry.Value))
			}
		})
	}
}

// ---------------------------------------------------------------------------
// TS-02-E1: Set non-existent signal
// Requirement: 02-REQ-8.E1
// ---------------------------------------------------------------------------

// TestSetNonExistentSignal verifies that attempting to set a signal that does
// not exist in the VSS tree results in an error (either a gRPC-level error or
// a SetResponse with success=false and a descriptive error message).
func TestSetNonExistentSignal(t *testing.T) {
	conn := dialTCP(t)
	client := valClient(conn)

	ctx, cancel := opCtx()
	defer cancel()

	resp, err := client.Set(ctx, &kuksapb.SetRequest{
		Entries: []*kuksapb.DataEntry{
			{
				Path:  "Vehicle.NonExistent.Signal",
				Value: &kuksapb.Datapoint{Value: &kuksapb.Datapoint_FloatValue{FloatValue: 42.0}},
			},
		},
	})
	if err != nil {
		// gRPC-level error (e.g., NOT_FOUND status) is acceptable.
		t.Logf("TS-02-E1: got expected gRPC error for non-existent signal: %v", err)
		return
	}
	// Application-level error (success=false) is also acceptable.
	if resp.Success {
		t.Errorf("TS-02-E1: Set non-existent signal returned success=true; expected an error")
	} else {
		t.Logf("TS-02-E1: got expected application error for non-existent signal: %s", resp.Error)
	}
}
