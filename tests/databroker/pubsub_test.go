package databroker_test

import (
	"context"
	"testing"
	"time"

	pb "github.com/rhadp/parking-fee-service/gen/kuksa/val/v1"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
	"google.golang.org/grpc/metadata"
)

// TestSubscriptionViaTCP verifies that a TCP subscriber receives notifications
// when a signal value changes.
// TS-02-10 | Requirement: 02-REQ-10.1
func TestSubscriptionViaTCP(t *testing.T) {
	skipIfTCPUnreachable(t)

	// Use two separate connections: one for subscribe, one for set.
	subConn := connectTCP(t)
	setConn := connectTCP(t)
	subClient := newVALClient(subConn)
	setClient := newVALClient(setConn)

	// Subscribe to signal changes.
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	stream, err := subClient.Subscribe(ctx, &pb.SubscribeRequest{
		Entries: []*pb.SubscribeEntry{
			{
				Path:   "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked",
				View:   pb.View_VIEW_CURRENT_VALUE,
				Fields: []pb.Field{pb.Field_FIELD_VALUE},
			},
		},
	})
	if err != nil {
		t.Fatalf("Subscribe failed: %v", err)
	}

	// Allow subscription to establish.
	time.Sleep(500 * time.Millisecond)

	// Set the signal value from another connection.
	setValue(t, setClient, "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", &pb.Datapoint{
		Value: &pb.Datapoint_BoolValue{BoolValue: true},
	})

	// Wait for the subscription to deliver the update.
	// The databroker may deliver an initial current-value event on subscription,
	// so we look for any event with the expected value within the timeout.
	deadline := time.After(5 * time.Second)
	for {
		select {
		case <-deadline:
			t.Fatal("timed out waiting for subscription update")
		default:
			resp, err := stream.Recv()
			if err != nil {
				t.Fatalf("stream.Recv() failed: %v", err)
			}
			for _, update := range resp.Updates {
				if update.Path == "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked" &&
					update.Value != nil &&
					update.Value.GetBoolValue() {
					// Success: received the expected update.
					return
				}
			}
		}
	}
}

// TestSubscriptionCrossTransport verifies that a UDS subscriber receives
// notifications when a signal is set via TCP.
// TS-02-11 | Requirement: 02-REQ-10.1, 02-REQ-4.1
func TestSubscriptionCrossTransport(t *testing.T) {
	skipIfTCPUnreachable(t)
	sockPath := skipIfUDSUnreachable(t)

	udsConn := connectUDS(t, sockPath)
	tcpConn := connectTCP(t)
	udsClient := newVALClient(udsConn)
	tcpClient := newVALClient(tcpConn)

	// Subscribe via UDS.
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	stream, err := udsClient.Subscribe(ctx, &pb.SubscribeRequest{
		Entries: []*pb.SubscribeEntry{
			{
				Path:   "Vehicle.Parking.SessionActive",
				View:   pb.View_VIEW_CURRENT_VALUE,
				Fields: []pb.Field{pb.Field_FIELD_VALUE},
			},
		},
	})
	if err != nil {
		t.Fatalf("Subscribe via UDS failed: %v", err)
	}

	// Allow subscription to establish.
	time.Sleep(500 * time.Millisecond)

	// Set via TCP.
	setValue(t, tcpClient, "Vehicle.Parking.SessionActive", &pb.Datapoint{
		Value: &pb.Datapoint_BoolValue{BoolValue: true},
	})

	// Wait for the update on the UDS subscription.
	deadline := time.After(5 * time.Second)
	for {
		select {
		case <-deadline:
			t.Fatal("timed out waiting for cross-transport subscription update")
		default:
			resp, err := stream.Recv()
			if err != nil {
				t.Fatalf("stream.Recv() failed: %v", err)
			}
			for _, update := range resp.Updates {
				if update.Path == "Vehicle.Parking.SessionActive" &&
					update.Value != nil &&
					update.Value.GetBoolValue() {
					return
				}
			}
		}
	}
}

// TestPermissiveModeWithArbitraryToken verifies that the DATA_BROKER accepts
// requests even when an invalid/arbitrary authorization token is provided.
// TS-02-E4 | Requirement: 02-REQ-7.E1
func TestPermissiveModeWithArbitraryToken(t *testing.T) {
	skipIfTCPUnreachable(t)

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	conn, err := grpc.DialContext(ctx, tcpTarget,
		grpc.WithTransportCredentials(insecure.NewCredentials()),
		grpc.WithBlock(),
	)
	if err != nil {
		t.Fatalf("failed to connect: %v", err)
	}
	defer conn.Close()

	client := pb.NewVALClient(conn)

	// Send a request with an arbitrary Authorization header.
	md := metadata.Pairs("authorization", "Bearer invalid-token-12345")
	ctx = metadata.NewOutgoingContext(ctx, md)

	resp, err := client.Get(ctx, &pb.GetRequest{
		Entries: []*pb.EntryRequest{
			{
				Path:   "Vehicle.Speed",
				View:   pb.View_VIEW_CURRENT_VALUE,
				Fields: []pb.Field{pb.Field_FIELD_VALUE},
			},
		},
	})
	if err != nil {
		t.Fatalf("request with arbitrary token should succeed in permissive mode, got: %v", err)
	}
	if len(resp.Entries) == 0 {
		t.Error("expected at least one entry in response")
	}
}
