package databroker_test

import (
	"context"
	"testing"

	kuksapb "parking-fee-service/tests/databroker/kuksa"
)

// ---------------------------------------------------------------------------
// TS-02-1: TCP connectivity
// Requirement: 02-REQ-2.1, 02-REQ-2.2
// ---------------------------------------------------------------------------

// TestTCPConnectivity verifies that a gRPC client can establish a connection to
// the DATA_BROKER via TCP on host port 55556 and perform a successful GetValue RPC.
func TestTCPConnectivity(t *testing.T) {
	conn := dialTCP(t)
	client := valClient(conn)

	ctx, cancel := opCtx()
	defer cancel()

	// A GetValue for a known standard signal confirms the channel is live and the
	// databroker is responsive. Using Vehicle.Speed (always present in VSS v5.1).
	resp, err := client.GetValue(ctx, &kuksapb.GetValueRequest{
		SignalId: signalID("Vehicle.Speed"),
	})
	if err != nil {
		t.Fatalf("TS-02-1: TCP GetValue(Vehicle.Speed) failed: %v", err)
	}
	// Any non-error response (even with no value yet set) confirms connectivity.
	_ = resp
	t.Log("TS-02-1: TCP connectivity OK")
}

// ---------------------------------------------------------------------------
// TS-02-2: UDS connectivity
// Requirement: 02-REQ-3.1, 02-REQ-3.2
// ---------------------------------------------------------------------------

// TestUDSConnectivity verifies that a gRPC client can establish a connection to
// the DATA_BROKER via Unix Domain Socket and perform a successful GetValue RPC.
func TestUDSConnectivity(t *testing.T) {
	conn := dialUDS(t)
	client := valClient(conn)

	ctx, cancel := opCtx()
	defer cancel()

	resp, err := client.GetValue(ctx, &kuksapb.GetValueRequest{
		SignalId: signalID("Vehicle.Speed"),
	})
	if err != nil {
		t.Fatalf("TS-02-2: UDS GetValue(Vehicle.Speed) failed: %v", err)
	}
	_ = resp
	t.Log("TS-02-2: UDS connectivity OK")
}

// ---------------------------------------------------------------------------
// TS-02-12: Permissive mode (no auth required)
// Requirement: 02-REQ-7.1
// ---------------------------------------------------------------------------

// TestPermissiveMode verifies that the DATA_BROKER accepts gRPC requests
// sent without any authorization credentials.
func TestPermissiveMode(t *testing.T) {
	// dialTCP already uses insecure credentials with no auth token — this test
	// confirms that the absence of credentials does not trigger PERMISSION_DENIED.
	conn := dialTCP(t)
	client := valClient(conn)

	ctx, cancel := opCtx()
	defer cancel()

	_, err := client.PublishValue(ctx, &kuksapb.PublishValueRequest{
		SignalId:  signalID("Vehicle.Speed"),
		DataPoint: &kuksapb.Datapoint{Value: &kuksapb.Value{TypedValue: &kuksapb.Value_Float{Float: 10.0}}},
	})
	if err != nil {
		t.Fatalf("TS-02-12: permissive mode PublishValue failed: %v", err)
	}
}

// ---------------------------------------------------------------------------
// TS-02-E4: Permissive mode with arbitrary token
// Requirement: 02-REQ-7.E1
// ---------------------------------------------------------------------------

// TestPermissiveModeWithArbitraryToken verifies that the DATA_BROKER accepts
// requests even when an invalid/arbitrary authorization token is sent.
func TestPermissiveModeWithArbitraryToken(t *testing.T) {
	conn := dialTCPWithToken(t, "invalid-token-12345")
	client := valClient(conn)

	ctx, cancel := context.WithTimeout(context.Background(), opTimeout)
	defer cancel()

	_, err := client.GetValue(ctx, &kuksapb.GetValueRequest{
		SignalId: signalID("Vehicle.Speed"),
	})
	if err != nil {
		t.Fatalf("TS-02-E4: GetValue with arbitrary token failed: %v", err)
	}
	t.Log("TS-02-E4: arbitrary token accepted (permissive mode confirmed)")
}
