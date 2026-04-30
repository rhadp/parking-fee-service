package parkingoperatoradaptor_test

import (
	"context"
	"syscall"
	"testing"
	"time"

	"github.com/rhadp/parking-fee-service/gen/adapter"
)

// TestLockStartUnlockStopFlow verifies end-to-end autonomous flow:
// lock event triggers session start, unlock event triggers session stop
// (TS-08-SMOKE-1).
func TestLockStartUnlockStopFlow(t *testing.T) {
	skipIfTCPUnreachable(t)

	binary := buildAdaptor(t)
	mockOp := newMockOperator(t)
	grpcPort := getFreePort(t)

	_, valClient := dialDatabroker(t)

	env := adaptorEnv(grpcPort, mockOp.URL())
	proc, lines := startAdaptorWithStderrScanner(t, binary, env)
	waitReady(t, lines, 30*time.Second)
	defer func() {
		_ = proc.sendSignal(syscall.SIGTERM)
		proc.waitExit(5 * time.Second)
	}()

	adaptorClient := dialAdaptor(t, grpcPort)
	ctx := context.Background()

	// Lock -> start session.
	publishValue(t, valClient, signalIsLocked, boolValue(true))
	mockOp.waitForStartCalls(t, 1, 10*time.Second)

	// Verify operator POST /parking/start was called.
	startCalls := mockOp.getStartCalls()
	if len(startCalls) < 1 {
		t.Fatal("expected at least 1 start call")
	}
	if startCalls[0].Path != "/parking/start" {
		t.Errorf("expected POST /parking/start, got %s", startCalls[0].Path)
	}

	// Verify session active via gRPC.
	status, err := adaptorClient.GetStatus(ctx, &adapter.GetStatusRequest{})
	if err != nil {
		t.Fatalf("GetStatus failed: %v", err)
	}
	if status.GetSession() == nil || !status.GetSession().GetActive() {
		t.Fatal("expected active session after lock")
	}
	if status.GetSession().GetSessionId() == "" {
		t.Error("expected non-empty session_id")
	}

	// Verify SessionActive=true in DATA_BROKER.
	waitForSessionActive(t, valClient, true, 10*time.Second)

	// Unlock -> stop session.
	publishValue(t, valClient, signalIsLocked, boolValue(false))
	mockOp.waitForStopCalls(t, 1, 10*time.Second)

	// Verify operator POST /parking/stop was called.
	stopCalls := mockOp.getStopCalls()
	if len(stopCalls) < 1 {
		t.Fatal("expected at least 1 stop call")
	}
	if stopCalls[0].Path != "/parking/stop" {
		t.Errorf("expected POST /parking/stop, got %s", stopCalls[0].Path)
	}

	// Verify session inactive via gRPC.
	status, err = adaptorClient.GetStatus(ctx, &adapter.GetStatusRequest{})
	if err != nil {
		t.Fatalf("GetStatus after unlock failed: %v", err)
	}
	if status.GetSession() != nil && status.GetSession().GetActive() {
		t.Fatal("expected inactive session after unlock")
	}

	// Verify SessionActive=false in DATA_BROKER.
	waitForSessionActive(t, valClient, false, 10*time.Second)
}

// TestManualOverrideFlow verifies manual gRPC start/stop (TS-08-SMOKE-2).
func TestManualOverrideFlow(t *testing.T) {
	skipIfTCPUnreachable(t)

	binary := buildAdaptor(t)
	mockOp := newMockOperator(t)
	grpcPort := getFreePort(t)

	env := adaptorEnv(grpcPort, mockOp.URL())
	proc, lines := startAdaptorWithStderrScanner(t, binary, env)
	waitReady(t, lines, 30*time.Second)
	defer func() {
		_ = proc.sendSignal(syscall.SIGTERM)
		proc.waitExit(5 * time.Second)
	}()

	adaptorClient := dialAdaptor(t, grpcPort)
	ctx := context.Background()

	// Manual StartSession.
	startResp, err := adaptorClient.StartSession(ctx, &adapter.StartSessionRequest{
		ZoneId: "zone-manual",
	})
	if err != nil {
		t.Fatalf("StartSession failed: %v", err)
	}
	if startResp.GetSession() == nil || startResp.GetSession().GetSessionId() == "" {
		t.Fatal("expected session_id in StartSession response")
	}

	// Verify operator was called.
	mockOp.waitForStartCalls(t, 1, 10*time.Second)

	// Verify session active.
	status, err := adaptorClient.GetStatus(ctx, &adapter.GetStatusRequest{})
	if err != nil {
		t.Fatalf("GetStatus failed: %v", err)
	}
	if !status.GetSession().GetActive() {
		t.Fatal("expected active session")
	}

	// Manual StopSession.
	stopResp, err := adaptorClient.StopSession(ctx, &adapter.StopSessionRequest{})
	if err != nil {
		t.Fatalf("StopSession failed: %v", err)
	}
	if stopResp.GetSession() == nil {
		t.Fatal("expected session in StopSession response")
	}

	// Verify operator was called.
	mockOp.waitForStopCalls(t, 1, 10*time.Second)

	// Verify session inactive.
	status, err = adaptorClient.GetStatus(ctx, &adapter.GetStatusRequest{})
	if err != nil {
		t.Fatalf("GetStatus after stop failed: %v", err)
	}
	if status.GetSession() != nil && status.GetSession().GetActive() {
		t.Fatal("expected inactive session after manual stop")
	}
}

// TestOverrideThenAutonomousResume verifies manual stop followed by autonomous
// lock/unlock cycle (TS-08-SMOKE-3).
func TestOverrideThenAutonomousResume(t *testing.T) {
	skipIfTCPUnreachable(t)

	binary := buildAdaptor(t)
	mockOp := newMockOperator(t)
	grpcPort := getFreePort(t)

	_, valClient := dialDatabroker(t)

	env := adaptorEnv(grpcPort, mockOp.URL())
	proc, lines := startAdaptorWithStderrScanner(t, binary, env)
	waitReady(t, lines, 30*time.Second)
	defer func() {
		_ = proc.sendSignal(syscall.SIGTERM)
		proc.waitExit(5 * time.Second)
	}()

	adaptorClient := dialAdaptor(t, grpcPort)
	ctx := context.Background()

	// Step 1: Lock event -> autonomous start.
	publishValue(t, valClient, signalIsLocked, boolValue(true))
	mockOp.waitForStartCalls(t, 1, 10*time.Second)

	status, err := adaptorClient.GetStatus(ctx, &adapter.GetStatusRequest{})
	if err != nil {
		t.Fatalf("GetStatus failed: %v", err)
	}
	if !status.GetSession().GetActive() {
		t.Fatal("expected active session after lock")
	}

	// Step 2: Manual StopSession (override).
	_, err = adaptorClient.StopSession(ctx, &adapter.StopSessionRequest{})
	if err != nil {
		t.Fatalf("manual StopSession failed: %v", err)
	}

	status, err = adaptorClient.GetStatus(ctx, &adapter.GetStatusRequest{})
	if err != nil {
		t.Fatalf("GetStatus after manual stop failed: %v", err)
	}
	if status.GetSession() != nil && status.GetSession().GetActive() {
		t.Fatal("expected inactive session after manual stop")
	}

	// Step 3: New lock event -> autonomous start (override does not persist).
	// First toggle to false to ensure a state change is published.
	publishValue(t, valClient, signalIsLocked, boolValue(false))
	time.Sleep(500 * time.Millisecond) // Allow the unlock no-op to process.
	publishValue(t, valClient, signalIsLocked, boolValue(true))
	mockOp.waitForStartCalls(t, 2, 10*time.Second) // 2 total start calls.

	status, err = adaptorClient.GetStatus(ctx, &adapter.GetStatusRequest{})
	if err != nil {
		t.Fatalf("GetStatus after second lock failed: %v", err)
	}
	if !status.GetSession().GetActive() {
		t.Fatal("expected active session after autonomous restart")
	}

	// Step 4: Unlock event -> autonomous stop.
	publishValue(t, valClient, signalIsLocked, boolValue(false))
	// 2 total stop calls: 1 manual (step 2) + 1 autonomous (step 4).
	mockOp.waitForStopCalls(t, 2, 10*time.Second)

	status, err = adaptorClient.GetStatus(ctx, &adapter.GetStatusRequest{})
	if err != nil {
		t.Fatalf("GetStatus after autonomous unlock failed: %v", err)
	}
	if status.GetSession() != nil && status.GetSession().GetActive() {
		t.Fatal("expected inactive session after autonomous stop")
	}
}
