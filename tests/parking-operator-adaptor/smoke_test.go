package parkingoperatoradaptor_test

import (
	"context"
	"testing"
	"time"

	pa "github.com/rhadp/parking-fee-service/gen/parking_adaptor/v1"
)

// TestLockStartUnlockStopFlow verifies the end-to-end autonomous flow:
// lock event triggers session start, unlock event triggers session stop.
// TS-08-SMOKE-1 | Requirement: 08-REQ-3.3, 08-REQ-3.4, 08-REQ-4.1, 08-REQ-4.2
func TestLockStartUnlockStopFlow(t *testing.T) {
	skipIfTCPUnreachable(t)

	binary := buildAdaptor(t)
	conn := connectTCP(t)
	valClient := newVALClient(conn)
	v2Client := newV2Client(conn)
	mo := startMockOperator(t)
	grpcPort := getFreePort(t)

	env := adaptorEnv(grpcPort, mo.url())

	// Reset IsLocked to false before starting.
	setBool(t, v2Client, signalIsLocked, false)
	time.Sleep(300 * time.Millisecond)

	startAdaptor(t, binary, env...)
	adaptorClient := connectAdaptorGRPC(t, grpcPort)

	ctx := context.Background()

	// Step 1: Lock event → session should start.
	setBool(t, v2Client, signalIsLocked, true)
	mo.waitForStartCalled(t, 1, 10*time.Second)

	// Verify session is active via gRPC GetStatus.
	status, err := adaptorClient.GetStatus(ctx, &pa.GetStatusRequest{})
	if err != nil {
		t.Fatalf("GetStatus failed: %v", err)
	}
	if !status.Active {
		t.Error("expected active session after lock event")
	}
	if status.SessionId == "" {
		t.Error("expected non-empty session_id")
	}

	// Verify Vehicle.Parking.SessionActive is true in DATA_BROKER.
	waitForBool(t, valClient, signalSessionActive, true, 5*time.Second)

	// Step 2: Unlock event → session should stop.
	setBool(t, v2Client, signalIsLocked, false)
	mo.waitForStopCalled(t, 1, 10*time.Second)

	// Verify session is inactive via gRPC GetStatus.
	status, err = adaptorClient.GetStatus(ctx, &pa.GetStatusRequest{})
	if err != nil {
		t.Fatalf("GetStatus failed after unlock: %v", err)
	}
	if status.Active {
		t.Error("expected inactive session after unlock event")
	}

	// Verify Vehicle.Parking.SessionActive is false in DATA_BROKER.
	waitForBool(t, valClient, signalSessionActive, false, 5*time.Second)
}

// TestManualOverrideFlow verifies the end-to-end manual gRPC start/stop flow.
// TS-08-SMOKE-2 | Requirement: 08-REQ-1.2, 08-REQ-1.3, 08-REQ-5.1, 08-REQ-5.2
func TestManualOverrideFlow(t *testing.T) {
	skipIfTCPUnreachable(t)

	binary := buildAdaptor(t)
	conn := connectTCP(t)
	v2Client := newV2Client(conn)
	mo := startMockOperator(t)
	grpcPort := getFreePort(t)

	env := adaptorEnv(grpcPort, mo.url())

	// Reset IsLocked to false before starting.
	setBool(t, v2Client, signalIsLocked, false)
	time.Sleep(300 * time.Millisecond)

	startAdaptor(t, binary, env...)
	adaptorClient := connectAdaptorGRPC(t, grpcPort)

	ctx := context.Background()

	// Step 1: Manual StartSession via gRPC.
	startResp, err := adaptorClient.StartSession(ctx, &pa.StartSessionRequest{
		VehicleId: "DEMO-VIN-001",
		ZoneId:    "zone-manual",
	})
	if err != nil {
		t.Fatalf("StartSession failed: %v", err)
	}
	if startResp.SessionId == "" {
		t.Error("expected non-empty session_id from StartSession")
	}

	// Verify session is active.
	status, err := adaptorClient.GetStatus(ctx, &pa.GetStatusRequest{})
	if err != nil {
		t.Fatalf("GetStatus after start failed: %v", err)
	}
	if !status.Active {
		t.Error("expected active session after manual start")
	}

	// Step 2: Manual StopSession via gRPC.
	stopResp, err := adaptorClient.StopSession(ctx, &pa.StopSessionRequest{
		SessionId: startResp.SessionId,
	})
	if err != nil {
		t.Fatalf("StopSession failed: %v", err)
	}
	if stopResp.SessionId == "" {
		t.Error("expected non-empty session_id from StopSession")
	}

	// Verify session is inactive.
	status, err = adaptorClient.GetStatus(ctx, &pa.GetStatusRequest{})
	if err != nil {
		t.Fatalf("GetStatus after stop failed: %v", err)
	}
	if status.Active {
		t.Error("expected inactive session after manual stop")
	}
}

// TestOverrideThenAutonomousResume verifies that autonomous behavior resumes
// after a manual override.
// TS-08-SMOKE-3 | Requirement: 08-REQ-5.3, 08-REQ-5.E1
func TestOverrideThenAutonomousResume(t *testing.T) {
	skipIfTCPUnreachable(t)

	binary := buildAdaptor(t)
	conn := connectTCP(t)
	v2Client := newV2Client(conn)
	mo := startMockOperator(t)
	grpcPort := getFreePort(t)

	env := adaptorEnv(grpcPort, mo.url())

	// Reset IsLocked to false before starting.
	setBool(t, v2Client, signalIsLocked, false)
	time.Sleep(300 * time.Millisecond)

	startAdaptor(t, binary, env...)
	adaptorClient := connectAdaptorGRPC(t, grpcPort)

	ctx := context.Background()

	// Step 1: Autonomous start via lock event.
	setBool(t, v2Client, signalIsLocked, true)
	mo.waitForStartCalled(t, 1, 10*time.Second)

	status, err := adaptorClient.GetStatus(ctx, &pa.GetStatusRequest{})
	if err != nil {
		t.Fatalf("GetStatus failed: %v", err)
	}
	if !status.Active {
		t.Fatal("expected active session after lock event")
	}

	// Step 2: Manual stop (override).
	_, err = adaptorClient.StopSession(ctx, &pa.StopSessionRequest{
		SessionId: status.SessionId,
	})
	if err != nil {
		t.Fatalf("manual StopSession failed: %v", err)
	}

	status, err = adaptorClient.GetStatus(ctx, &pa.GetStatusRequest{})
	if err != nil {
		t.Fatalf("GetStatus after manual stop failed: %v", err)
	}
	if status.Active {
		t.Fatal("expected inactive session after manual stop")
	}

	// Step 3: Autonomous start via another lock event (override should not persist).
	// First unlock then lock to create a new cycle.
	setBool(t, v2Client, signalIsLocked, false)
	time.Sleep(500 * time.Millisecond)
	setBool(t, v2Client, signalIsLocked, true)
	mo.waitForStartCalled(t, 2, 10*time.Second)

	status, err = adaptorClient.GetStatus(ctx, &pa.GetStatusRequest{})
	if err != nil {
		t.Fatalf("GetStatus after second lock failed: %v", err)
	}
	if !status.Active {
		t.Error("expected active session after autonomous resume (lock event)")
	}

	// Step 4: Autonomous stop via unlock event.
	setBool(t, v2Client, signalIsLocked, false)
	mo.waitForStopCalled(t, 2, 10*time.Second)

	status, err = adaptorClient.GetStatus(ctx, &pa.GetStatusRequest{})
	if err != nil {
		t.Fatalf("GetStatus after final unlock failed: %v", err)
	}
	if status.Active {
		t.Error("expected inactive session after autonomous unlock")
	}
}
