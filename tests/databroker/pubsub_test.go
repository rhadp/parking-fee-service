package databroker_test

import (
	"context"
	"testing"
	"time"

	kuksa "github.com/rhadp/parking-fee-service/gen/kuksa"
	"google.golang.org/protobuf/types/known/timestamppb"
)

// TestSubscriptionViaTCP verifies that a TCP subscriber receives notifications
// when a signal value changes.
// Test Spec: TS-02-10
// Requirement: 02-REQ-10.1
func TestSubscriptionViaTCP(t *testing.T) {
	skipIfTCPUnreachable(t)
	_, subscriberClient := dialTCP(t)
	_, publisherClient := dialTCP(t)

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	// Subscribe to the signal.
	stream, err := subscriberClient.Subscribe(ctx, &kuksa.SubscribeRequest{
		SignalPaths: []string{"Vehicle.Cabin.Door.Row1.DriverSide.IsLocked"},
	})
	if err != nil {
		t.Fatalf("failed to subscribe: %v", err)
	}

	// Drain any initial current-value notification that Kuksa may send.
	drainStream(stream, 1*time.Second)

	// Publish a new value from a second client.
	pubCtx, pubCancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer pubCancel()
	_, err = publisherClient.PublishValue(pubCtx, &kuksa.PublishValueRequest{
		SignalId: &kuksa.SignalID{Signal: &kuksa.SignalID_Path{
			Path: "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked",
		}},
		DataPoint: &kuksa.Datapoint{Timestamp: timestamppb.Now(), Value: boolValue(true)},
	})
	if err != nil {
		t.Fatalf("failed to publish: %v", err)
	}

	// Receive the subscription notification.
	received := make(chan *kuksa.SubscribeResponse, 1)
	errCh := make(chan error, 1)
	go func() {
		resp, recvErr := stream.Recv()
		if recvErr != nil {
			errCh <- recvErr
			return
		}
		received <- resp
	}()

	select {
	case resp := <-received:
		entry, ok := resp.Entries["Vehicle.Cabin.Door.Row1.DriverSide.IsLocked"]
		if !ok {
			t.Error("subscription response missing expected signal entry")
		} else if entry.GetValue().GetBool() != true {
			t.Errorf("subscription: expected true, got %v", entry.GetValue().GetBool())
		}
	case recvErr := <-errCh:
		t.Fatalf("subscription recv error: %v", recvErr)
	case <-ctx.Done():
		t.Fatal("subscription: timed out waiting for notification")
	}
}

// TestSubscriptionCrossTransport verifies that a UDS subscriber receives
// notifications when a signal is set via TCP.
// Test Spec: TS-02-11
// Requirement: 02-REQ-10.1, 02-REQ-4.1
func TestSubscriptionCrossTransport(t *testing.T) {
	skipIfTCPUnreachable(t)
	skipIfUDSUnreachable(t)
	_, udsClient := dialUDS(t)
	_, tcpClient := dialTCP(t)

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	// Subscribe via UDS.
	stream, err := udsClient.Subscribe(ctx, &kuksa.SubscribeRequest{
		SignalPaths: []string{"Vehicle.Parking.SessionActive"},
	})
	if err != nil {
		t.Fatalf("failed to subscribe via UDS: %v", err)
	}

	// Drain any initial current-value notification.
	drainStream(stream, 1*time.Second)

	// Publish via TCP.
	pubCtx, pubCancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer pubCancel()
	_, err = tcpClient.PublishValue(pubCtx, &kuksa.PublishValueRequest{
		SignalId:  &kuksa.SignalID{Signal: &kuksa.SignalID_Path{Path: "Vehicle.Parking.SessionActive"}},
		DataPoint: &kuksa.Datapoint{Timestamp: timestamppb.Now(), Value: boolValue(true)},
	})
	if err != nil {
		t.Fatalf("failed to publish via TCP: %v", err)
	}

	// Receive the cross-transport subscription notification.
	received := make(chan *kuksa.SubscribeResponse, 1)
	errCh := make(chan error, 1)
	go func() {
		resp, recvErr := stream.Recv()
		if recvErr != nil {
			errCh <- recvErr
			return
		}
		received <- resp
	}()

	select {
	case resp := <-received:
		entry, ok := resp.Entries["Vehicle.Parking.SessionActive"]
		if !ok {
			t.Error("cross-transport subscription response missing expected signal entry")
		} else if entry.GetValue().GetBool() != true {
			t.Errorf("cross-transport subscription: expected true, got %v", entry.GetValue().GetBool())
		}
	case recvErr := <-errCh:
		t.Fatalf("cross-transport subscription recv error: %v", recvErr)
	case <-ctx.Done():
		t.Fatal("cross-transport subscription: timed out waiting for notification")
	}
}
