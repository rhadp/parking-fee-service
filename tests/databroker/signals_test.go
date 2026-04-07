package databroker_test

import (
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
		got := getSignalValue(t, client, path)
		gv, ok := got.TypedValue.(*kuksapb.Value_Float)
		if !ok {
			t.Fatalf("TS-02-6: expected float value for %s, got %T", path, got.TypedValue)
		}
		if gv.Float != want {
			t.Errorf("TS-02-6: Vehicle.Speed: got %v, want %v", gv.Float, want)
		}
	})

	t.Run("Vehicle.Parking.SessionActive/bool", func(t *testing.T) {
		const path = "Vehicle.Parking.SessionActive"
		setSignalBool(t, client, path, true)
		got := getSignalValue(t, client, path)
		gv, ok := got.TypedValue.(*kuksapb.Value_Bool)
		if !ok {
			t.Fatalf("TS-02-6: expected bool value for %s, got %T", path, got.TypedValue)
		}
		if !gv.Bool {
			t.Errorf("TS-02-6: Vehicle.Parking.SessionActive: got false, want true")
		}
	})

	t.Run("Vehicle.Command.Door.Lock/string", func(t *testing.T) {
		const path = "Vehicle.Command.Door.Lock"
		const want = `{"command_id":"abc","action":"lock"}`
		setSignalString(t, client, path, want)
		got := getSignalValue(t, client, path)
		gv, ok := got.TypedValue.(*kuksapb.Value_String_)
		if !ok {
			t.Fatalf("TS-02-6: expected string value for %s, got %T", path, got.TypedValue)
		}
		if gv.String_ != want {
			t.Errorf("TS-02-6: Vehicle.Command.Door.Lock: got %q, want %q", gv.String_, want)
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
		got := getSignalValue(t, client, path)
		gv, ok := got.TypedValue.(*kuksapb.Value_Bool)
		if !ok {
			t.Fatalf("TS-02-7: expected bool value for %s, got %T", path, got.TypedValue)
		}
		if !gv.Bool {
			t.Errorf("TS-02-7: IsLocked: got false, want true")
		}
	})

	t.Run("Vehicle.CurrentLocation.Latitude/double", func(t *testing.T) {
		const path = "Vehicle.CurrentLocation.Latitude"
		const want float64 = 48.1351
		setSignalDouble(t, client, path, want)
		got := getSignalValue(t, client, path)
		gv, ok := got.TypedValue.(*kuksapb.Value_Double)
		if !ok {
			t.Fatalf("TS-02-7: expected double value for %s, got %T", path, got.TypedValue)
		}
		if gv.Double != want {
			t.Errorf("TS-02-7: Latitude: got %v, want %v", gv.Double, want)
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

	got := getSignalValue(t, udsClient, path)
	gv, ok := got.TypedValue.(*kuksapb.Value_Float)
	if !ok {
		t.Fatalf("TS-02-8: expected float value via UDS for %s, got %T", path, got.TypedValue)
	}
	if gv.Float != want {
		t.Errorf("TS-02-8: TCP write / UDS read: got %v, want %v", gv.Float, want)
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

	got := getSignalValue(t, tcpClient, path)
	gv, ok := got.TypedValue.(*kuksapb.Value_Bool)
	if !ok {
		t.Fatalf("TS-02-9: expected bool value via TCP for %s, got %T", path, got.TypedValue)
	}
	if !gv.Bool {
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
			publishSignal(t, client, sig.path, want)

			// Read it back.
			got := getSignalValue(t, client, sig.path)
			if !valueEqual(got, want) {
				t.Errorf("TS-02-P2: roundtrip for %q: got %s, want %s",
					sig.path, valueString(got), valueString(want))
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
			publishSignal(t, tcpClient, sig.path, want)

			// Read via TCP.
			tcpVal := getSignalValue(t, tcpClient, sig.path)
			// Read via UDS.
			udsVal := getSignalValue(t, udsClient, sig.path)

			if !valueEqual(tcpVal, udsVal) {
				t.Errorf("TS-02-P3: cross-transport mismatch for %q: TCP=%s UDS=%s",
					sig.path, valueString(tcpVal), valueString(udsVal))
			}
			if !valueEqual(tcpVal, want) {
				t.Errorf("TS-02-P3: TCP GetValue(%q) != written value: got %s, want %s",
					sig.path, valueString(tcpVal), valueString(want))
			}
		})

		t.Run(sig.path+"/uds-write", func(t *testing.T) {
			// Use a different value to confirm the UDS write propagates.
			want := zeroValueForType(sig.typeName)

			// Write via UDS.
			publishSignal(t, udsClient, sig.path, want)

			// Read via TCP.
			tcpVal := getSignalValue(t, tcpClient, sig.path)
			// Read via UDS.
			udsVal := getSignalValue(t, udsClient, sig.path)

			if !valueEqual(tcpVal, udsVal) {
				t.Errorf("TS-02-P3: cross-transport mismatch after UDS write for %q: TCP=%s UDS=%s",
					sig.path, valueString(tcpVal), valueString(udsVal))
			}
		})
	}
}

// ---------------------------------------------------------------------------
// TS-02-E1: Set non-existent signal
// Requirement: 02-REQ-8.E1
// ---------------------------------------------------------------------------

// TestSetNonExistentSignal verifies that attempting to set a signal that does
// not exist in the VSS tree results in an error.
func TestSetNonExistentSignal(t *testing.T) {
	conn := dialTCP(t)
	client := valClient(conn)

	ctx, cancel := opCtx()
	defer cancel()

	_, err := client.PublishValue(ctx, &kuksapb.PublishValueRequest{
		SignalId:  signalID("Vehicle.NonExistent.Signal"),
		DataPoint: &kuksapb.Datapoint{Value: &kuksapb.Value{TypedValue: &kuksapb.Value_Float{Float: 42.0}}},
	})
	if err != nil {
		// gRPC-level error (e.g., NOT_FOUND status) is the expected outcome.
		t.Logf("TS-02-E1: got expected gRPC error for non-existent signal: %v", err)
		return
	}
	// If no error, the databroker accepted a non-existent signal (unexpected).
	t.Errorf("TS-02-E1: PublishValue for non-existent signal succeeded; expected an error")
}
