package parkingoperatoradaptor_test

import (
	"context"
	"runtime"
	"testing"
	"time"

	"github.com/rhadp/parking-fee-service/gen/adapter"
)

// ---------------------------------------------------------------------------
// TS-08-SMOKE-1: Lock-Start-Unlock-Stop Flow
// Requirements: 08-REQ-3.3, 08-REQ-3.4, 08-REQ-4.1, 08-REQ-4.2
// ---------------------------------------------------------------------------

// TestLockStartUnlockStopFlow is an end-to-end test: lock event triggers
// session start, unlock event triggers session stop. Verifies both gRPC
// status and DATA_BROKER signals.
func TestLockStartUnlockStopFlow(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("signal tests not supported on Windows")
	}

	skipIfTCPUnreachable(t)

	bin := buildAdaptor(t)
	mock := newMockOperator(t)
	grpcPort := getFreePort(t)
	brokerClient := newBrokerClient(t)

	env := adaptorEnv(mock.URL(), "http://"+tcpTarget, grpcPort)
	_ = startAdaptor(t, bin, env)

	adaptorClient := newAdaptorClient(t, grpcPort)

	// Step 1: Lock event → start session.
	setSignalBool(t, brokerClient, signalIsLocked, true)
	mock.waitForStartCalls(t, 1, 10*time.Second)

	// Step 2: Verify GetStatus returns active=true with a session_id.
	ctx, cancel := context.WithTimeout(context.Background(), rpcTimeout)
	defer cancel()
	statusResp, err := adaptorClient.GetStatus(ctx, &adapter.GetStatusRequest{})
	if err != nil {
		t.Fatalf("GetStatus failed: %v", err)
	}
	if statusResp.Session == nil {
		t.Fatal("GetStatus returned nil session")
	}
	if !statusResp.Session.Active {
		t.Error("expected active=true after lock event")
	}
	if statusResp.Session.SessionId == "" {
		t.Error("expected non-empty session_id")
	}

	// Step 3: Verify Vehicle.Parking.SessionActive=true in DATA_BROKER.
	waitForBoolSignal(t, brokerClient, signalSessionActive, true, 5*time.Second)

	// Step 4: Unlock event → stop session.
	setSignalBool(t, brokerClient, signalIsLocked, false)
	mock.waitForStopCalls(t, 1, 10*time.Second)

	// Step 5: Verify GetStatus returns active=false.
	ctx2, cancel2 := context.WithTimeout(context.Background(), rpcTimeout)
	defer cancel2()
	statusResp2, err := adaptorClient.GetStatus(ctx2, &adapter.GetStatusRequest{})
	if err != nil {
		t.Fatalf("GetStatus after unlock failed: %v", err)
	}
	if statusResp2.Session != nil && statusResp2.Session.Active {
		t.Error("expected active=false after unlock event")
	}

	// Step 6: Verify Vehicle.Parking.SessionActive=false in DATA_BROKER.
	waitForBoolSignal(t, brokerClient, signalSessionActive, false, 5*time.Second)
}

// ---------------------------------------------------------------------------
// TS-08-SMOKE-2: Manual Override Flow
// Requirements: 08-REQ-1.2, 08-REQ-1.3, 08-REQ-5.1, 08-REQ-5.2
// ---------------------------------------------------------------------------

// TestManualOverrideFlow is an end-to-end test: manual StartSession and
// StopSession via gRPC, verifying operator calls and status.
func TestManualOverrideFlow(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("signal tests not supported on Windows")
	}

	skipIfTCPUnreachable(t)

	bin := buildAdaptor(t)
	mock := newMockOperator(t)
	grpcPort := getFreePort(t)

	env := adaptorEnv(mock.URL(), "http://"+tcpTarget, grpcPort)
	_ = startAdaptor(t, bin, env)

	adaptorClient := newAdaptorClient(t, grpcPort)

	// Step 1: Manual StartSession.
	ctx, cancel := context.WithTimeout(context.Background(), rpcTimeout)
	defer cancel()
	startResp, err := adaptorClient.StartSession(ctx, &adapter.StartSessionRequest{
		ZoneId: "zone-manual",
	})
	if err != nil {
		t.Fatalf("StartSession failed: %v", err)
	}
	if startResp.Session == nil {
		t.Fatal("StartSession returned nil session")
	}
	if startResp.Session.SessionId == "" {
		t.Error("expected non-empty session_id from StartSession")
	}

	// Step 2: Verify GetStatus returns active=true.
	ctx2, cancel2 := context.WithTimeout(context.Background(), rpcTimeout)
	defer cancel2()
	statusResp, err := adaptorClient.GetStatus(ctx2, &adapter.GetStatusRequest{})
	if err != nil {
		t.Fatalf("GetStatus failed: %v", err)
	}
	if statusResp.Session == nil || !statusResp.Session.Active {
		t.Error("expected active=true after manual StartSession")
	}

	// Step 3: Manual StopSession.
	ctx3, cancel3 := context.WithTimeout(context.Background(), rpcTimeout)
	defer cancel3()
	stopResp, err := adaptorClient.StopSession(ctx3, &adapter.StopSessionRequest{})
	if err != nil {
		t.Fatalf("StopSession failed: %v", err)
	}
	if stopResp.Session == nil {
		t.Fatal("StopSession returned nil session")
	}
	if stopResp.Session.SessionId == "" {
		t.Error("expected non-empty session_id from StopSession")
	}

	// Step 4: Verify GetStatus returns active=false.
	ctx4, cancel4 := context.WithTimeout(context.Background(), rpcTimeout)
	defer cancel4()
	statusResp2, err := adaptorClient.GetStatus(ctx4, &adapter.GetStatusRequest{})
	if err != nil {
		t.Fatalf("GetStatus after stop failed: %v", err)
	}
	if statusResp2.Session != nil && statusResp2.Session.Active {
		t.Error("expected active=false after manual StopSession")
	}

	// Verify operator was called.
	if mock.getStartCallCount() < 1 {
		t.Error("expected at least 1 start call to operator")
	}
	if mock.getStopCallCount() < 1 {
		t.Error("expected at least 1 stop call to operator")
	}
}

// ---------------------------------------------------------------------------
// TS-08-SMOKE-3: Override Then Autonomous Resume
// Requirements: 08-REQ-5.3, 08-REQ-5.E1
// ---------------------------------------------------------------------------

// TestOverrideThenAutonomousResume verifies that after a manual stop, the
// next lock/unlock cycle resumes autonomous behavior.
func TestOverrideThenAutonomousResume(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("signal tests not supported on Windows")
	}

	skipIfTCPUnreachable(t)

	bin := buildAdaptor(t)
	mock := newMockOperator(t)
	grpcPort := getFreePort(t)
	brokerClient := newBrokerClient(t)

	env := adaptorEnv(mock.URL(), "http://"+tcpTarget, grpcPort)
	_ = startAdaptor(t, bin, env)

	adaptorClient := newAdaptorClient(t, grpcPort)

	// Step 1: Lock event → start session autonomously.
	setSignalBool(t, brokerClient, signalIsLocked, true)
	mock.waitForStartCalls(t, 1, 10*time.Second)

	ctx, cancel := context.WithTimeout(context.Background(), rpcTimeout)
	defer cancel()
	statusResp, err := adaptorClient.GetStatus(ctx, &adapter.GetStatusRequest{})
	if err != nil {
		t.Fatalf("GetStatus failed: %v", err)
	}
	if statusResp.Session == nil || !statusResp.Session.Active {
		t.Fatal("expected active session after lock event")
	}

	// Step 2: Manual StopSession (override).
	ctx2, cancel2 := context.WithTimeout(context.Background(), rpcTimeout)
	defer cancel2()
	_, err = adaptorClient.StopSession(ctx2, &adapter.StopSessionRequest{})
	if err != nil {
		t.Fatalf("StopSession failed: %v", err)
	}

	ctx3, cancel3 := context.WithTimeout(context.Background(), rpcTimeout)
	defer cancel3()
	statusResp2, err := adaptorClient.GetStatus(ctx3, &adapter.GetStatusRequest{})
	if err != nil {
		t.Fatalf("GetStatus after stop failed: %v", err)
	}
	if statusResp2.Session != nil && statusResp2.Session.Active {
		t.Error("expected active=false after manual StopSession")
	}

	// Step 3: Lock event → should start new session autonomously.
	// First unlock to reset state, then lock again.
	setSignalBool(t, brokerClient, signalIsLocked, false)
	// Small delay for event to process (unlock is a no-op since session
	// was already stopped manually).
	time.Sleep(1 * time.Second)

	setSignalBool(t, brokerClient, signalIsLocked, true)
	mock.waitForStartCalls(t, 2, 10*time.Second)

	ctx4, cancel4 := context.WithTimeout(context.Background(), rpcTimeout)
	defer cancel4()
	statusResp3, err := adaptorClient.GetStatus(ctx4, &adapter.GetStatusRequest{})
	if err != nil {
		t.Fatalf("GetStatus after 2nd lock failed: %v", err)
	}
	if statusResp3.Session == nil || !statusResp3.Session.Active {
		t.Error("expected active=true after autonomous lock event (resume)")
	}

	// Step 4: Unlock event → should stop session autonomously.
	setSignalBool(t, brokerClient, signalIsLocked, false)
	mock.waitForStopCalls(t, 2, 10*time.Second)

	ctx5, cancel5 := context.WithTimeout(context.Background(), rpcTimeout)
	defer cancel5()
	statusResp4, err := adaptorClient.GetStatus(ctx5, &adapter.GetStatusRequest{})
	if err != nil {
		t.Fatalf("GetStatus after final unlock failed: %v", err)
	}
	if statusResp4.Session != nil && statusResp4.Session.Active {
		t.Error("expected active=false after autonomous unlock event")
	}
}
