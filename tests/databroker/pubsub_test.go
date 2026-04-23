package databroker_test

import (
	"context"
	"testing"
	"time"

	pb "github.com/rhadp/parking-fee-service/gen/kuksa/val/v1"
	"google.golang.org/grpc"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/credentials/insecure"
	"google.golang.org/grpc/metadata"
	"google.golang.org/grpc/status"
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

// TestSubscriptionReconnect verifies that after a subscriber disconnects and
// reconnects, it can re-subscribe and receive subsequent updates without
// missing the current value.
//
// This covers 02-REQ-10.E1: IF a subscriber disconnects and reconnects, THEN
// the subscriber SHALL be able to re-subscribe and receive subsequent updates
// without missing the current value.
//
// TS-02-10 (edge case: reconnect) | Requirement: 02-REQ-10.E1
func TestSubscriptionReconnect(t *testing.T) {
	skipIfTCPUnreachable(t)

	const signalPath = "Vehicle.CurrentLocation.Longitude"
	// Phase 1: establish initial subscription and set a value.
	subConn1 := connectTCP(t)
	setConn := connectTCP(t)
	setClient := newVALClient(setConn)

	ctx1, cancel1 := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel1()

	stream1, err := pb.NewVALClient(subConn1).Subscribe(ctx1, &pb.SubscribeRequest{
		Entries: []*pb.SubscribeEntry{
			{
				Path:   signalPath,
				View:   pb.View_VIEW_CURRENT_VALUE,
				Fields: []pb.Field{pb.Field_FIELD_VALUE},
			},
		},
	})
	if err != nil {
		t.Fatalf("initial Subscribe failed: %v", err)
	}

	// Allow subscription to establish before setting a value.
	time.Sleep(300 * time.Millisecond)

	// Set an initial value; the subscription stream may deliver this.
	setValue(t, setClient, signalPath, &pb.Datapoint{
		Value: &pb.Datapoint_DoubleValue{DoubleValue: 11.11},
	})

	// Drain any pending events from the first subscription.
	drainCtx, drainCancel := context.WithTimeout(context.Background(), 2*time.Second)
	for {
		_, err := stream1.Recv()
		if err != nil {
			// Stream ended or timed out — stop draining.
			break
		}
		select {
		case <-drainCtx.Done():
			goto drained
		default:
		}
	}
drained:
	drainCancel()

	// Phase 2: disconnect the first subscriber.
	cancel1()
	subConn1.Close()

	// Phase 3: reconnect and re-subscribe.
	subConn2 := connectTCP(t)
	subClient2 := pb.NewVALClient(subConn2)

	ctx2, cancel2 := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel2()

	stream2, err := subClient2.Subscribe(ctx2, &pb.SubscribeRequest{
		Entries: []*pb.SubscribeEntry{
			{
				Path:   signalPath,
				View:   pb.View_VIEW_CURRENT_VALUE,
				Fields: []pb.Field{pb.Field_FIELD_VALUE},
			},
		},
	})
	if err != nil {
		t.Fatalf("re-subscribe after reconnect failed: %v", err)
	}

	// Allow the new subscription to establish.
	time.Sleep(300 * time.Millisecond)

	// Phase 4: set a new value that the reconnected subscriber must receive.
	const newValue = 22.22
	setValue(t, setClient, signalPath, &pb.Datapoint{
		Value: &pb.Datapoint_DoubleValue{DoubleValue: newValue},
	})

	// Phase 5: verify the reconnected subscriber receives the new value.
	// The server may deliver the current value on subscription, so we look
	// for either the exact new value or any update from the server.
	deadline := time.After(5 * time.Second)
	for {
		select {
		case <-deadline:
			t.Fatal("timed out waiting for subscription update after reconnect")
		default:
			resp, recvErr := stream2.Recv()
			if recvErr != nil {
				t.Fatalf("stream.Recv() after reconnect failed: %v", recvErr)
			}
			for _, update := range resp.Updates {
				if update.Path == signalPath && update.Value != nil &&
					update.Value.GetDoubleValue() == newValue {
					// Success: reconnected subscriber received the new value.
					return
				}
			}
		}
	}
}

// TestAPICompatibilityCheck is a diagnostic test that explicitly validates
// the DATA_BROKER is serving a compatible API version. When the container
// exposes only the v2 API (kuksa.val.v2), calls via the v1 proto client will
// return empty responses rather than gRPC errors, making other tests silently
// incorrect. This test detects that condition and fails with a clear message.
//
// This addresses the critical review finding about v1/v2 API mismatch
// (errata §6 in docs/errata/02_data_broker_spec_contradictions.md).
//
// Requirement: 02-REQ-5.2 (signal retrieval via gRPC)
func TestAPICompatibilityCheck(t *testing.T) {
	skipIfTCPUnreachable(t)
	conn := connectTCP(t)
	client := newVALClient(conn)

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	// Request metadata for a well-known standard signal.
	resp, err := client.Get(ctx, &pb.GetRequest{
		Entries: []*pb.EntryRequest{
			{
				Path:   "Vehicle.Speed",
				View:   pb.View_VIEW_METADATA,
				Fields: []pb.Field{pb.Field_FIELD_METADATA},
			},
		},
	})
	if err != nil {
		// A gRPC error at this level is a transport or protocol error.
		st, _ := status.FromError(err)
		if st.Code() == codes.Unimplemented {
			t.Fatalf("DATA_BROKER returned UNIMPLEMENTED for v1 Get method: "+
				"the container is likely serving kuksa.val.v2 only (not v1). "+
				"Update compose.yml to use a databroker image that serves kuksa.val.v1 "+
				"(see docs/errata/02_data_broker_spec_contradictions.md §6). Error: %v", err)
		}
		t.Fatalf("v1 Get request to DATA_BROKER failed: %v", err)
	}

	// If the response has zero entries, the server may be ignoring v1 requests
	// (v2-only server silently drops unrecognised proto fields).
	if len(resp.Entries) == 0 {
		t.Fatalf("DATA_BROKER returned 0 entries for Vehicle.Speed metadata request. "+
			"This is the expected symptom of a v2-only server receiving v1 proto requests "+
			"(kuksa.val.v1.Get silently returns empty on a v2-only server). "+
			"Update compose.yml to use a databroker image that serves kuksa.val.v1 "+
			"(see docs/errata/02_data_broker_spec_contradictions.md §6).")
	}

	// If the entry has no Metadata, the API is not returning expected data.
	if resp.Entries[0].Metadata == nil {
		t.Fatalf("DATA_BROKER returned an entry for Vehicle.Speed with nil Metadata. "+
			"This indicates the server is not populating fields expected by kuksa.val.v1. "+
			"Check if the server is serving a compatible API version "+
			"(see docs/errata/02_data_broker_spec_contradictions.md §6).")
	}

	t.Logf("API compatibility check passed: kuksa.val.v1 Get returned metadata for Vehicle.Speed")
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
