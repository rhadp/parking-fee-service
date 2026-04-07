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
		Paths: []string{path},
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
	if len(update.Entries) == 0 {
		t.Fatal("TS-02-10: received subscription update with no entries")
	}
	entry := update.Entries[0]
	got, ok := entry.Value.Value.(*kuksapb.Datapoint_BoolValue)
	if !ok {
		t.Fatalf("TS-02-10: expected bool in subscription update, got %T", entry.Value.Value)
	}
	if !got.BoolValue {
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
		Paths: []string{path},
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
	if len(update.Entries) == 0 {
		t.Fatal("TS-02-11: received subscription update with no entries")
	}
	entry := update.Entries[0]
	got, ok := entry.Value.Value.(*kuksapb.Datapoint_BoolValue)
	if !ok {
		t.Fatalf("TS-02-11: expected bool in subscription update, got %T", entry.Value.Value)
	}
	if !got.BoolValue {
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
//
// Note: The Kuksa Databroker may send an initial value notification immediately
// upon subscription. This test drains that initial notification (if present)
// before writing, then expects exactly one update per write.
func TestSubscriptionDelivery(t *testing.T) {
	// Use a subset of signals to keep the test runtime bounded.
	// One representative from each type.
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
				Paths: []string{sig.path},
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
				ctx, cancel := context.WithTimeout(context.Background(), opTimeout)
				defer cancel()
				resp, err := client.Set(ctx, &kuksapb.SetRequest{
					Entries: []*kuksapb.DataEntry{
						{Path: sig.path, Value: want},
					},
				})
				if err != nil {
					t.Errorf("TS-02-P4: Set(%q) gRPC error: %v", sig.path, err)
					return
				}
				if !resp.Success {
					t.Errorf("TS-02-P4: Set(%q) rejected: %s", sig.path, resp.Error)
				}
			}()

			// Receive the update triggered by the Set above.
			update, err := stream.Recv()
			wg.Wait()
			if err != nil {
				t.Fatalf("TS-02-P4: Subscribe Recv(%q) failed: %v", sig.path, err)
			}
			if len(update.Entries) == 0 {
				t.Fatalf("TS-02-P4: subscription update for %q has no entries", sig.path)
			}
			got := update.Entries[0].Value
			if !datapointEqual(got, want) {
				t.Errorf("TS-02-P4: subscription update for %q: got %s, want %s",
					sig.path, datapointString(got), datapointString(want))
			}
		})
	}
}

// drainInitial attempts to receive and discard one initial-value notification
// from the stream that Kuksa Databroker sends immediately upon subscription.
// If no notification arrives within a short window, the drain is a no-op.
// This prevents the initial notification from interfering with assertions about
// subsequent writes.
func drainInitial(t *testing.T, stream kuksapb.VAL_SubscribeClient) {
	t.Helper()
	// Use a very short timeout: if Kuksa sends an initial value it arrives
	// almost immediately; we don't want to wait long if it doesn't.
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
