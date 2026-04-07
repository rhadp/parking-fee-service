package databroker_test

import (
	"context"
	"sync"
	"testing"
	"time"

	kuksapb "parking-fee-service/tests/databroker/kuksa"
)

// ---------------------------------------------------------------------------
// TS-02-10: Signal subscription via TCP
// Requirement: 02-REQ-10.1
// ---------------------------------------------------------------------------

// TestSubscriptionViaTCP verifies that a TCP subscriber receives a notification
// when a signal value is changed by another TCP client.
func TestSubscriptionViaTCP(t *testing.T) {
	// Two independent TCP connections: one for the subscriber, one for the writer.
	subConn := dialTCP(t)
	writerConn := dialTCP(t)
	subClient := valClient(subConn)
	writerClient := valClient(writerConn)

	const path = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked"

	// Open the subscription stream before writing so we don't miss the update.
	subCtxVal, subCancel := context.WithTimeout(context.Background(), subTimeout)
	defer subCancel()

	stream, err := subClient.Subscribe(subCtxVal, &kuksapb.SubscribeRequest{
		SignalPaths: []string{path},
	})
	if err != nil {
		t.Fatalf("TS-02-10: Subscribe failed: %v", err)
	}

	// Drain any initial-value notification that the databroker may send
	// immediately upon subscription (per Kuksa behaviour).
	drainInitial(t, stream)

	// Set the signal from the writer client.
	setSignalBool(t, writerClient, path, true)

	// Expect the subscriber to receive the update within the timeout.
	update, err := stream.Recv()
	if err != nil {
		t.Fatalf("TS-02-10: Subscribe Recv failed: %v", err)
	}
	dp, ok := update.Entries[path]
	if !ok || dp == nil || dp.Value == nil {
		t.Fatalf("TS-02-10: subscription update missing entry for %s", path)
	}
	got, ok := dp.Value.TypedValue.(*kuksapb.Value_Bool)
	if !ok {
		t.Fatalf("TS-02-10: expected bool in subscription update, got %T", dp.Value.TypedValue)
	}
	if !got.Bool {
		t.Errorf("TS-02-10: subscription update value: got false, want true")
	}
}

// ---------------------------------------------------------------------------
// TS-02-11: Signal subscription cross-transport (UDS subscribe, TCP write)
// Requirement: 02-REQ-10.1, 02-REQ-4.1
// ---------------------------------------------------------------------------

// TestSubscriptionCrossTransport verifies that a UDS subscriber receives a
// notification when a signal value is changed by a TCP client.
func TestSubscriptionCrossTransport(t *testing.T) {
	udsConn := dialUDS(t)
	tcpConn := dialTCP(t)
	udsClient := valClient(udsConn)
	tcpClient := valClient(tcpConn)

	const path = "Vehicle.Parking.SessionActive"

	subCtxVal, subCancel := context.WithTimeout(context.Background(), subTimeout)
	defer subCancel()

	stream, err := udsClient.Subscribe(subCtxVal, &kuksapb.SubscribeRequest{
		SignalPaths: []string{path},
	})
	if err != nil {
		t.Fatalf("TS-02-11: UDS Subscribe failed: %v", err)
	}

	// Drain any initial-value notification.
	drainInitial(t, stream)

	// Set the signal from the TCP client.
	setSignalBool(t, tcpClient, path, true)

	// Expect the UDS subscriber to receive the TCP-triggered update.
	update, err := stream.Recv()
	if err != nil {
		t.Fatalf("TS-02-11: UDS Subscribe Recv failed: %v", err)
	}
	dp, ok := update.Entries[path]
	if !ok || dp == nil || dp.Value == nil {
		t.Fatalf("TS-02-11: subscription update missing entry for %s", path)
	}
	got, ok := dp.Value.TypedValue.(*kuksapb.Value_Bool)
	if !ok {
		t.Fatalf("TS-02-11: expected bool in subscription update, got %T", dp.Value.TypedValue)
	}
	if !got.Bool {
		t.Errorf("TS-02-11: subscription update value: got false, want true")
	}
}

// ---------------------------------------------------------------------------
// TS-02-P4: Subscription delivery property
// Requirement: 02-REQ-10.1
// ---------------------------------------------------------------------------

// TestSubscriptionDelivery is a property test verifying that for any signal,
// a subscription receives an update when the signal value changes, and that
// the update reflects the new value.
func TestSubscriptionDelivery(t *testing.T) {
	testSignals := []signalDef{
		{path: "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", typeName: "bool"},
		{path: "Vehicle.Speed", typeName: "float"},
		{path: "Vehicle.CurrentLocation.Latitude", typeName: "double"},
		{path: "Vehicle.Command.Door.Response", typeName: "string"},
	}

	conn := dialTCP(t)
	client := valClient(conn)

	for _, sig := range testSignals {
		sig := sig
		t.Run(sig.path, func(t *testing.T) {
			want := testValueForType(sig.typeName)

			subCtxVal, subCancel := context.WithTimeout(context.Background(), subTimeout)
			defer subCancel()

			stream, err := client.Subscribe(subCtxVal, &kuksapb.SubscribeRequest{
				SignalPaths: []string{sig.path},
			})
			if err != nil {
				t.Fatalf("TS-02-P4: Subscribe(%q) failed: %v", sig.path, err)
			}

			// Drain any initial-value notification before setting new value.
			drainInitial(t, stream)

			// Write the signal from a separate goroutine to avoid blocking the
			// Recv below.
			var wg sync.WaitGroup
			wg.Add(1)
			go func() {
				defer wg.Done()
				publishSignal(t, client, sig.path, want)
			}()

			// Receive the update triggered by the write above.
			update, err := stream.Recv()
			wg.Wait()
			if err != nil {
				t.Fatalf("TS-02-P4: Subscribe Recv(%q) failed: %v", sig.path, err)
			}
			dp, ok := update.Entries[sig.path]
			if !ok || dp == nil || dp.Value == nil {
				t.Fatalf("TS-02-P4: subscription update for %q has no entry", sig.path)
			}
			if !valueEqual(dp.Value, want) {
				t.Errorf("TS-02-P4: subscription update for %q: got %s, want %s",
					sig.path, valueString(dp.Value), valueString(want))
			}
		})
	}
}

// drainInitial attempts to receive and discard one initial-value notification
// from the stream that Kuksa Databroker sends immediately upon subscription.
func drainInitial(t *testing.T, stream kuksapb.VAL_SubscribeClient) {
	t.Helper()
	done := make(chan struct{})
	go func() {
		defer close(done)
		_, _ = stream.Recv()
	}()
	select {
	case <-done:
		// Initial notification drained (or stream ended).
	case <-time.After(500 * time.Millisecond):
		// No initial notification within the window — nothing to drain.
	}
}
