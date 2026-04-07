package parkingoperatoradaptor_test

import (
	"testing"
	"time"

	adaptorpb "parking-fee-service/tests/parking-operator-adaptor/adaptorpb"
)

// ---------------------------------------------------------------------------
// TS-08-SMOKE-1: Lock-Start-Unlock-Stop Flow
// Requirements: 08-REQ-3.3, 08-REQ-3.4, 08-REQ-4.1, 08-REQ-4.2
// ---------------------------------------------------------------------------

// TestSmokeLockStartUnlockStop verifies the end-to-end flow: lock event
// triggers session start, unlock event triggers session stop.
func TestSmokeLockStartUnlockStop(t *testing.T) {
	requirePodman(t)
	dbClient := ensureDatabrokerReachable(t)
	resetSignals(t, dbClient)

	mockOp := startMockOperator(t)
	binary := buildAdaptor(t)
	grpcPort := freePort(t)
	_ = startAdaptor(t, binary, grpcPort, mockOp.URL())
	time.Sleep(500 * time.Millisecond)

	conn := dialAdaptor(t, grpcPort)
	client := adaptorClient(conn)

	// Lock → start session.
	setSignalBool(t, dbClient, signalIsLocked, true)
	mockOp.waitForStartCount(t, 1, 10*time.Second)

	// Verify session is active via gRPC.
	ctx, cancel := opCtx()
	status, err := client.GetStatus(ctx, &adaptorpb.GetStatusRequest{})
	cancel()
	if err != nil {
		t.Fatalf("TS-08-SMOKE-1: GetStatus after lock failed: %v", err)
	}
	if !status.Active {
		t.Error("TS-08-SMOKE-1: expected active=true after lock event")
	}
	if status.SessionId == "" {
		t.Error("TS-08-SMOKE-1: expected non-empty session_id after lock event")
	}

	// Verify SessionActive=true in DATA_BROKER.
	waitForSignalBool(t, dbClient, signalSessionActive, true, 5*time.Second)

	// Unlock → stop session.
	setSignalBool(t, dbClient, signalIsLocked, false)
	mockOp.waitForStopCount(t, 1, 10*time.Second)

	// Verify session is inactive via gRPC.
	ctx2, cancel2 := opCtx()
	status2, err := client.GetStatus(ctx2, &adaptorpb.GetStatusRequest{})
	cancel2()
	if err != nil {
		t.Fatalf("TS-08-SMOKE-1: GetStatus after unlock failed: %v", err)
	}
	if status2.Active {
		t.Error("TS-08-SMOKE-1: expected active=false after unlock event")
	}

	// Verify SessionActive=false in DATA_BROKER.
	waitForSignalBool(t, dbClient, signalSessionActive, false, 5*time.Second)
}

// ---------------------------------------------------------------------------
// TS-08-SMOKE-2: Manual Override Flow
// Requirements: 08-REQ-5.1, 08-REQ-5.2
// ---------------------------------------------------------------------------

// TestSmokeManualOverride verifies manual StartSession and StopSession via gRPC.
func TestSmokeManualOverride(t *testing.T) {
	requirePodman(t)
	dbClient := ensureDatabrokerReachable(t)
	resetSignals(t, dbClient)

	mockOp := startMockOperator(t)
	binary := buildAdaptor(t)
	grpcPort := freePort(t)
	_ = startAdaptor(t, binary, grpcPort, mockOp.URL())
	time.Sleep(500 * time.Millisecond)

	conn := dialAdaptor(t, grpcPort)
	client := adaptorClient(conn)

	// Manual StartSession.
	ctx, cancel := opCtx()
	startResp, err := client.StartSession(ctx, &adaptorpb.StartSessionRequest{
		ZoneId: "zone-manual",
	})
	cancel()
	if err != nil {
		t.Fatalf("TS-08-SMOKE-2: StartSession failed: %v", err)
	}
	if startResp.SessionId == "" {
		t.Error("TS-08-SMOKE-2: expected non-empty session_id from StartSession")
	}

	// Verify active via GetStatus.
	ctx2, cancel2 := opCtx()
	status, err := client.GetStatus(ctx2, &adaptorpb.GetStatusRequest{})
	cancel2()
	if err != nil {
		t.Fatalf("TS-08-SMOKE-2: GetStatus after start failed: %v", err)
	}
	if !status.Active {
		t.Error("TS-08-SMOKE-2: expected active=true after manual start")
	}

	// Manual StopSession.
	ctx3, cancel3 := opCtx()
	stopResp, err := client.StopSession(ctx3, &adaptorpb.StopSessionRequest{})
	cancel3()
	if err != nil {
		t.Fatalf("TS-08-SMOKE-2: StopSession failed: %v", err)
	}
	if stopResp.SessionId == "" {
		t.Error("TS-08-SMOKE-2: expected non-empty session_id from StopSession")
	}

	// Verify inactive via GetStatus.
	ctx4, cancel4 := opCtx()
	status2, err := client.GetStatus(ctx4, &adaptorpb.GetStatusRequest{})
	cancel4()
	if err != nil {
		t.Fatalf("TS-08-SMOKE-2: GetStatus after stop failed: %v", err)
	}
	if status2.Active {
		t.Error("TS-08-SMOKE-2: expected active=false after manual stop")
	}
}

// ---------------------------------------------------------------------------
// TS-08-SMOKE-3: Override Then Autonomous Resume
// Requirements: 08-REQ-5.3, 08-REQ-5.E1
// ---------------------------------------------------------------------------

// TestSmokeOverrideThenAutonomous verifies that after a manual stop, the next
// lock/unlock cycle resumes autonomous behavior.
func TestSmokeOverrideThenAutonomous(t *testing.T) {
	requirePodman(t)
	dbClient := ensureDatabrokerReachable(t)
	resetSignals(t, dbClient)

	mockOp := startMockOperator(t)
	binary := buildAdaptor(t)
	grpcPort := freePort(t)
	_ = startAdaptor(t, binary, grpcPort, mockOp.URL())
	time.Sleep(500 * time.Millisecond)

	conn := dialAdaptor(t, grpcPort)
	client := adaptorClient(conn)

	// Step 1: Lock event → start session autonomously.
	setSignalBool(t, dbClient, signalIsLocked, true)
	mockOp.waitForStartCount(t, 1, 10*time.Second)

	ctx, cancel := opCtx()
	status, err := client.GetStatus(ctx, &adaptorpb.GetStatusRequest{})
	cancel()
	if err != nil {
		t.Fatalf("TS-08-SMOKE-3: GetStatus after lock failed: %v", err)
	}
	if !status.Active {
		t.Error("TS-08-SMOKE-3: expected active=true after lock event")
	}

	// Step 2: Manual StopSession (override).
	ctx2, cancel2 := opCtx()
	_, err = client.StopSession(ctx2, &adaptorpb.StopSessionRequest{})
	cancel2()
	if err != nil {
		t.Fatalf("TS-08-SMOKE-3: manual StopSession failed: %v", err)
	}

	ctx3, cancel3 := opCtx()
	status2, err := client.GetStatus(ctx3, &adaptorpb.GetStatusRequest{})
	cancel3()
	if err != nil {
		t.Fatalf("TS-08-SMOKE-3: GetStatus after manual stop failed: %v", err)
	}
	if status2.Active {
		t.Error("TS-08-SMOKE-3: expected active=false after manual stop")
	}

	// Need to reset lock state first so we can trigger a new lock event.
	setSignalBool(t, dbClient, signalIsLocked, false)
	time.Sleep(500 * time.Millisecond)

	// Step 3: Lock event → should start new session autonomously.
	setSignalBool(t, dbClient, signalIsLocked, true)
	mockOp.waitForStartCount(t, 2, 10*time.Second)

	ctx4, cancel4 := opCtx()
	status3, err := client.GetStatus(ctx4, &adaptorpb.GetStatusRequest{})
	cancel4()
	if err != nil {
		t.Fatalf("TS-08-SMOKE-3: GetStatus after second lock failed: %v", err)
	}
	if !status3.Active {
		t.Error("TS-08-SMOKE-3: expected active=true after autonomous restart")
	}

	// Step 4: Unlock event → should stop session autonomously.
	setSignalBool(t, dbClient, signalIsLocked, false)
	mockOp.waitForStopCount(t, 2, 10*time.Second)

	ctx5, cancel5 := opCtx()
	status4, err := client.GetStatus(ctx5, &adaptorpb.GetStatusRequest{})
	cancel5()
	if err != nil {
		t.Fatalf("TS-08-SMOKE-3: GetStatus after autonomous stop failed: %v", err)
	}
	if status4.Active {
		t.Error("TS-08-SMOKE-3: expected active=false after autonomous stop")
	}
}
