package databroker_test

import (
	"context"
	"testing"
	"time"

	"github.com/rhadp/parking-fee-service/gen/kuksa"
)

// TestSubscriptionViaTCP verifies that a TCP subscriber receives
// notifications when a signal value changes.
//
// Test Spec: TS-02-10
// Requirements: 02-REQ-10.1
func TestSubscriptionViaTCP(t *testing.T) {
	skipIfTCPUnreachable(t)

	subscriber := newTCPClient(t)
	publisher := newTCPClient(t)

	signalPath := "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked"

	// Subscribe to the signal.
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	stream, err := subscriber.Subscribe(ctx, &kuksa.SubscribeRequest{
		Entries: []*kuksa.SubscribeEntry{
			{
				Path:   signalPath,
				View:   kuksa.View_VIEW_CURRENT_VALUE,
				Fields: []kuksa.Field{kuksa.Field_FIELD_VALUE},
			},
		},
	})
	if err != nil {
		t.Fatalf("Subscribe failed: %v", err)
	}

	// Drain any initial-value notification that kuksa-databroker sends
	// on subscription establishment.
	drainCtx, drainCancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer drainCancel()
	drainInitialNotification(t, stream, drainCtx)

	// Set the signal from a different client.
	setSignalBool(t, publisher, signalPath, true)

	// Receive the subscription update.
	resp, err := stream.Recv()
	if err != nil {
		t.Fatalf("failed to receive subscription update: %v", err)
	}

	if len(resp.Updates) == 0 {
		t.Fatal("subscription response contained no updates")
	}

	found := false
	for _, update := range resp.Updates {
		if update.Entry != nil && update.Entry.Path == signalPath {
			assertDatapointValue(t, update.Entry.Value, "bool", true)
			found = true
			break
		}
	}
	if !found {
		t.Errorf("subscription update did not contain entry for %s", signalPath)
	}
}

// TestSubscriptionCrossTransport verifies that a UDS subscriber receives
// notifications when a signal is set via TCP.
//
// Test Spec: TS-02-11
// Requirements: 02-REQ-10.1, 02-REQ-4.1
func TestSubscriptionCrossTransport(t *testing.T) {
	skipIfTCPUnreachable(t)
	skipIfUDSUnreachable(t)

	udsSubscriber := newUDSClient(t)
	tcpPublisher := newTCPClient(t)

	signalPath := "Vehicle.Parking.SessionActive"

	// Subscribe via UDS.
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	stream, err := udsSubscriber.Subscribe(ctx, &kuksa.SubscribeRequest{
		Entries: []*kuksa.SubscribeEntry{
			{
				Path:   signalPath,
				View:   kuksa.View_VIEW_CURRENT_VALUE,
				Fields: []kuksa.Field{kuksa.Field_FIELD_VALUE},
			},
		},
	})
	if err != nil {
		t.Fatalf("Subscribe via UDS failed: %v", err)
	}

	// Drain initial-value notification.
	drainCtx, drainCancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer drainCancel()
	drainInitialNotification(t, stream, drainCtx)

	// Set via TCP.
	setSignalBool(t, tcpPublisher, signalPath, true)

	// Receive update via UDS subscription.
	resp, err := stream.Recv()
	if err != nil {
		t.Fatalf("failed to receive cross-transport subscription update: %v", err)
	}

	if len(resp.Updates) == 0 {
		t.Fatal("cross-transport subscription response contained no updates")
	}

	found := false
	for _, update := range resp.Updates {
		if update.Entry != nil && update.Entry.Path == signalPath {
			assertDatapointValue(t, update.Entry.Value, "bool", true)
			found = true
			break
		}
	}
	if !found {
		t.Errorf("cross-transport subscription update did not contain entry for %s", signalPath)
	}
}

// drainInitialNotification reads and discards any initial-value notification
// that kuksa-databroker sends upon subscription establishment. This prevents
// the initial notification from being incorrectly counted as a value-change
// update in subscription tests.
func drainInitialNotification(t *testing.T, stream kuksa.VAL_SubscribeClient, ctx context.Context) {
	t.Helper()
	done := make(chan struct{})
	go func() {
		defer close(done)
		// Attempt to read one message; if it arrives before the context
		// deadline, it's the initial notification and we discard it.
		_, _ = stream.Recv()
	}()
	select {
	case <-done:
		// Drained successfully.
	case <-ctx.Done():
		// No initial notification received within timeout; that's OK.
	}
}
