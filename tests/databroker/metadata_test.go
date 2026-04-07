package databroker_test

import (
	"context"
	"testing"

	kuksapb "parking-fee-service/tests/databroker/kuksa"
)

// ---------------------------------------------------------------------------
// TS-02-4: Standard VSS signal metadata
// Requirement: 02-REQ-5.1, 02-REQ-5.2
// ---------------------------------------------------------------------------

// TestStandardVSSSignalMetadata verifies that all 5 standard VSS v5.1 signals
// are present in the DATA_BROKER. Signal existence is verified by successfully
// performing a GetValue RPC.
func TestStandardVSSSignalMetadata(t *testing.T) {
	conn := dialTCP(t)
	client := valClient(conn)

	for _, sig := range standardSignals {
		sig := sig
		t.Run(sig.path, func(t *testing.T) {
			ctx, cancel := context.WithTimeout(context.Background(), opTimeout)
			defer cancel()

			_, err := client.GetValue(ctx, &kuksapb.GetValueRequest{
				SignalId: signalID(sig.path),
			})
			if err != nil {
				t.Errorf("TS-02-4: standard signal %q not found in databroker: %v", sig.path, err)
			}
		})
	}
}

// TestStandardVSSSignalTypeCompatibility verifies that the 5 standard signals
// accept values of their declared types by performing a zero-value PublishValue.
// Requirement: 02-REQ-5.1, 02-REQ-5.2
func TestStandardVSSSignalTypeCompatibility(t *testing.T) {
	conn := dialTCP(t)
	client := valClient(conn)

	for _, sig := range standardSignals {
		sig := sig
		t.Run(sig.path, func(t *testing.T) {
			ctx, cancel := context.WithTimeout(context.Background(), opTimeout)
			defer cancel()

			_, err := client.PublishValue(ctx, &kuksapb.PublishValueRequest{
				SignalId:  signalID(sig.path),
				DataPoint: &kuksapb.Datapoint{Value: zeroValueForType(sig.typeName)},
			})
			if err != nil {
				t.Errorf("TS-02-4 type check: PublishValue zero-value for %q (%s) failed: %v",
					sig.path, sig.typeName, err)
			}
		})
	}
}

// ---------------------------------------------------------------------------
// TS-02-5: Custom VSS signal metadata
// Requirement: 02-REQ-6.1, 02-REQ-6.2, 02-REQ-6.3, 02-REQ-6.4
// ---------------------------------------------------------------------------

// TestCustomVSSSignalMetadata verifies that all 3 custom VSS signals defined in
// the overlay file are present in the DATA_BROKER after startup.
func TestCustomVSSSignalMetadata(t *testing.T) {
	conn := dialTCP(t)
	client := valClient(conn)

	for _, sig := range customSignals {
		sig := sig
		t.Run(sig.path, func(t *testing.T) {
			ctx, cancel := context.WithTimeout(context.Background(), opTimeout)
			defer cancel()

			_, err := client.GetValue(ctx, &kuksapb.GetValueRequest{
				SignalId: signalID(sig.path),
			})
			if err != nil {
				t.Errorf("TS-02-5: custom signal %q not found in databroker: %v", sig.path, err)
			}
		})
	}
}

// TestCustomVSSSignalTypeCompatibility verifies that the 3 custom overlay
// signals accept values of their declared types.
// Requirement: 02-REQ-6.1, 02-REQ-6.2, 02-REQ-6.3
func TestCustomVSSSignalTypeCompatibility(t *testing.T) {
	conn := dialTCP(t)
	client := valClient(conn)

	for _, sig := range customSignals {
		sig := sig
		t.Run(sig.path, func(t *testing.T) {
			ctx, cancel := context.WithTimeout(context.Background(), opTimeout)
			defer cancel()

			_, err := client.PublishValue(ctx, &kuksapb.PublishValueRequest{
				SignalId:  signalID(sig.path),
				DataPoint: &kuksapb.Datapoint{Value: zeroValueForType(sig.typeName)},
			})
			if err != nil {
				t.Errorf("TS-02-5 type check: PublishValue zero-value for %q (%s) failed: %v",
					sig.path, sig.typeName, err)
			}
		})
	}
}

// ---------------------------------------------------------------------------
// TS-02-P1: Signal completeness property
// Requirement: 02-REQ-5.1, 02-REQ-5.2, 02-REQ-6.1, 02-REQ-6.2, 02-REQ-6.3, 02-REQ-6.4
// ---------------------------------------------------------------------------

// TestSignalCompleteness is a property test verifying that all 8 expected VSS
// signals (5 standard + 3 custom) are present in the DATA_BROKER and accept
// values of their declared types.
func TestSignalCompleteness(t *testing.T) {
	conn := dialTCP(t)
	client := valClient(conn)

	foundCount := 0
	for _, sig := range allSignals {
		sig := sig
		t.Run(sig.path, func(t *testing.T) {
			ctx, cancel := context.WithTimeout(context.Background(), opTimeout)
			defer cancel()

			// Step 1: signal exists (GetValue succeeds)
			_, err := client.GetValue(ctx, &kuksapb.GetValueRequest{
				SignalId: signalID(sig.path),
			})
			if err != nil {
				t.Errorf("TS-02-P1: signal %q missing from databroker: %v", sig.path, err)
				return
			}
			foundCount++

			// Step 2: signal accepts its declared type (PublishValue with zero value succeeds)
			ctx2, cancel2 := context.WithTimeout(context.Background(), opTimeout)
			defer cancel2()

			_, err = client.PublishValue(ctx2, &kuksapb.PublishValueRequest{
				SignalId:  signalID(sig.path),
				DataPoint: &kuksapb.Datapoint{Value: zeroValueForType(sig.typeName)},
			})
			if err != nil {
				t.Errorf("TS-02-P1: type check for %q (%s) failed: %v",
					sig.path, sig.typeName, err)
			}
		})
	}

	expectedCount := len(allSignals) // 8
	if foundCount != expectedCount {
		t.Errorf("TS-02-P1: found %d/%d signals; missing signals detected", foundCount, expectedCount)
	}
}
