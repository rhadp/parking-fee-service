package databroker_test

import (
	"context"
	"testing"
	"time"

	"github.com/rhadp/parking-fee-service/gen/kuksa"
)

// TestPropertySignalCompleteness verifies that all 8 expected VSS signals
// (5 standard + 3 custom) are present and accessible in the DATA_BROKER.
//
// Test Spec: TS-02-P1
// Requirements: 02-REQ-5.1, 02-REQ-5.2, 02-REQ-6.1, 02-REQ-6.2, 02-REQ-6.3
func TestPropertySignalCompleteness(t *testing.T) {
	skipIfTCPUnreachable(t)

	client := newTCPClient(t)

	signals := allSignals()
	foundCount := 0

	for _, sig := range signals {
		t.Run(sig.Path, func(t *testing.T) {
			entry, err := getSignalValue(t, client, sig.Path)
			if err != nil {
				t.Errorf("signal %s not accessible: %v", sig.Path, err)
				return
			}
			if entry == nil {
				t.Errorf("signal %s returned nil entry", sig.Path)
				return
			}
			foundCount++
		})
	}

	if foundCount != 8 {
		t.Errorf("expected 8 signals, found %d", foundCount)
	}
}

// TestPropertyWriteReadRoundtrip verifies that for any signal, setting a
// value and immediately getting it returns exactly the same value.
//
// Test Spec: TS-02-P2
// Requirements: 02-REQ-8.1, 02-REQ-9.1
func TestPropertyWriteReadRoundtrip(t *testing.T) {
	skipIfTCPUnreachable(t)

	client := newTCPClient(t)

	for _, sig := range allSignals() {
		testValues := testValuesForType(sig.DataType)
		for _, val := range testValues {
			name := sig.Path
			t.Run(name, func(t *testing.T) {
				setSignalByType(t, client, sig, val)

				entry, err := getSignalValue(t, client, sig.Path)
				if err != nil {
					t.Fatalf("Get after Set failed: %v", err)
				}
				assertDatapointValue(t, entry.Value, sig.DataType, val)
			})
		}
	}
}

// TestPropertyCrossTransportEquivalence verifies that for any signal, the
// value read via TCP equals the value read via UDS after a write on either
// transport.
//
// Test Spec: TS-02-P3
// Requirements: 02-REQ-4.1, 02-REQ-9.2
func TestPropertyCrossTransportEquivalence(t *testing.T) {
	skipIfTCPUnreachable(t)
	skipIfUDSUnreachable(t)

	tcpClient := newTCPClient(t)
	udsClient := newUDSClient(t)

	for _, sig := range allSignals() {
		vals := testValuesForType(sig.DataType)
		if len(vals) == 0 {
			continue
		}

		t.Run("TCP_write_"+sig.Path, func(t *testing.T) {
			val := vals[0]
			setSignalByType(t, tcpClient, sig, val)

			// Read via TCP.
			tcpEntry, err := getSignalValue(t, tcpClient, sig.Path)
			if err != nil {
				t.Fatalf("TCP Get failed: %v", err)
			}

			// Read via UDS.
			udsEntry, err := getSignalValue(t, udsClient, sig.Path)
			if err != nil {
				t.Fatalf("UDS Get failed: %v", err)
			}

			// Both should have the same value.
			assertDatapointValue(t, tcpEntry.Value, sig.DataType, val)
			assertDatapointValue(t, udsEntry.Value, sig.DataType, val)
		})

		if len(vals) > 1 {
			t.Run("UDS_write_"+sig.Path, func(t *testing.T) {
				val := vals[1]
				setSignalByType(t, udsClient, sig, val)

				// Read via TCP.
				tcpEntry, err := getSignalValue(t, tcpClient, sig.Path)
				if err != nil {
					t.Fatalf("TCP Get failed: %v", err)
				}

				// Read via UDS.
				udsEntry, err := getSignalValue(t, udsClient, sig.Path)
				if err != nil {
					t.Fatalf("UDS Get failed: %v", err)
				}

				assertDatapointValue(t, tcpEntry.Value, sig.DataType, val)
				assertDatapointValue(t, udsEntry.Value, sig.DataType, val)
			})
		}
	}
}

// TestPropertySubscriptionDelivery verifies that for any active subscription
// on a signal, a value change is delivered to the subscriber.
//
// Test Spec: TS-02-P4
// Requirements: 02-REQ-10.1
func TestPropertySubscriptionDelivery(t *testing.T) {
	skipIfTCPUnreachable(t)

	client := newTCPClient(t)

	// Test a representative subset of signals covering each data type.
	testCases := []struct {
		sig signalDef
		val interface{}
	}{
		{signalDef{"Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", "bool"}, true},
		{signalDef{"Vehicle.Speed", "float"}, float32(42.5)},
		{signalDef{"Vehicle.CurrentLocation.Latitude", "double"}, float64(52.5200)},
		{signalDef{"Vehicle.Command.Door.Lock", "string"}, `{"command_id":"test"}`},
	}

	for _, tc := range testCases {
		t.Run(tc.sig.Path, func(t *testing.T) {
			ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
			defer cancel()

			stream, err := client.Subscribe(ctx, &kuksa.SubscribeRequest{
				Entries: []*kuksa.SubscribeEntry{
					{
						Path:   tc.sig.Path,
						View:   kuksa.View_VIEW_CURRENT_VALUE,
						Fields: []kuksa.Field{kuksa.Field_FIELD_VALUE},
					},
				},
			})
			if err != nil {
				t.Fatalf("Subscribe failed: %v", err)
			}

			// Drain initial-value notification.
			drainCtx, drainCancel := context.WithTimeout(context.Background(), 2*time.Second)
			drainInitialNotification(t, stream, drainCtx)
			drainCancel()

			// Set the signal value.
			setSignalByType(t, client, tc.sig, tc.val)

			// Receive subscription update.
			resp, err := stream.Recv()
			if err != nil {
				t.Fatalf("failed to receive subscription update: %v", err)
			}

			if len(resp.Updates) == 0 {
				t.Fatal("subscription response contained no updates")
			}

			found := false
			for _, update := range resp.Updates {
				if update.Entry != nil && update.Entry.Path == tc.sig.Path {
					assertDatapointValue(t, update.Entry.Value, tc.sig.DataType, tc.val)
					found = true
					break
				}
			}
			if !found {
				t.Errorf("subscription update did not contain entry for %s", tc.sig.Path)
			}
		})
	}
}
