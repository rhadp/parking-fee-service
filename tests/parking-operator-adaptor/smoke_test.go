package parkingoperatoradaptortests

import (
	"context"
	"testing"
	"time"

	parkingpb "github.com/rhadp/parking-fee-service/tests/parking-operator-adaptor/pb/parking"
)

// ── TS-08-SMOKE-1: Lock-Start-Unlock-Stop Flow ────────────────────────────────

// TestLockStartUnlockStopFlow verifies the end-to-end autonomous session flow:
// lock event → session start, unlock event → session stop.
//
// Test Spec: TS-08-SMOKE-1
// Requirements: 08-REQ-3.3, 08-REQ-3.4, 08-REQ-4.1, 08-REQ-4.2
func TestLockStartUnlockStopFlow(t *testing.T) {
	binPath := buildAdaptorBinary(t)
	broker, brokerAddr := newMockDataBroker(t)
	op := newMockParkingOperator(t)

	const grpcPort = "50090"
	proc := startAdaptor(t, binPath, map[string]string{
		"GRPC_PORT":            grpcPort,
		"DATA_BROKER_ADDR":     "http://" + brokerAddr,
		"PARKING_OPERATOR_URL": op.URL(),
		"VEHICLE_ID":           "DEMO-VIN-001",
		"ZONE_ID":              "zone-demo-1",
		"RUST_LOG":             "info",
	})

	if !broker.WaitForSubscription(10 * time.Second) {
		t.Fatalf("adaptor did not subscribe to DATA_BROKER within 10s\noutput:\n%s", proc.output.String())
	}
	if err := waitForPort("127.0.0.1:"+grpcPort, 5*time.Second); err != nil {
		t.Fatalf("adaptor gRPC not ready: %v\noutput:\n%s", err, proc.output.String())
	}

	adaptorClient := newParkingAdaptorClient(t, "127.0.0.1:"+grpcPort)

	// ── Step 1: Lock event → expect session start ──

	broker.SendIsLocked(true)

	if !op.WaitForStart(1, 10*time.Second) {
		t.Fatalf("operator did not receive /parking/start within 10s\noutput:\n%s", proc.output.String())
	}

	// ── Step 2: GetStatus returns active ──

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	status, err := adaptorClient.GetStatus(ctx, &parkingpb.GetStatusRequest{})
	if err != nil {
		t.Fatalf("GetStatus failed: %v", err)
	}
	if !status.Active {
		t.Errorf("expected session active after lock event\noutput:\n%s", proc.output.String())
	}
	if status.SessionId == "" {
		t.Errorf("expected non-empty session_id after lock event")
	}

	// ── Step 3: Vehicle.Parking.SessionActive = true ──

	if !broker.WaitForSessionActive(true, 5*time.Second) {
		val, ok := broker.LastSessionActive()
		t.Errorf("expected SessionActive=true after session start; got value=%v ok=%v", val, ok)
	}

	// ── Step 4: Unlock event → expect session stop ──

	broker.SendIsLocked(false)

	if !op.WaitForStop(1, 10*time.Second) {
		t.Fatalf("operator did not receive /parking/stop within 10s\noutput:\n%s", proc.output.String())
	}

	// ── Step 5: GetStatus returns inactive ──

	ctx2, cancel2 := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel2()

	// Poll for inactive state (the stop is async).
	deadline := time.Now().Add(5 * time.Second)
	var lastStatus *parkingpb.GetStatusResponse
	for time.Now().Before(deadline) {
		lastStatus, err = adaptorClient.GetStatus(ctx2, &parkingpb.GetStatusRequest{})
		if err != nil {
			t.Fatalf("GetStatus (post-stop) failed: %v", err)
		}
		if !lastStatus.Active {
			break
		}
		time.Sleep(100 * time.Millisecond)
	}
	if lastStatus.Active {
		t.Errorf("expected session inactive after unlock event\noutput:\n%s", proc.output.String())
	}

	// ── Step 6: Vehicle.Parking.SessionActive = false ──

	if !broker.WaitForSessionActive(false, 5*time.Second) {
		val, ok := broker.LastSessionActive()
		t.Errorf("expected SessionActive=false after session stop; got value=%v ok=%v", val, ok)
	}
}

// ── TS-08-SMOKE-2: Manual Override Flow ──────────────────────────────────────

// TestManualOverrideFlow verifies the manual gRPC start/stop flow.
//
// Test Spec: TS-08-SMOKE-2
// Requirements: 08-REQ-5.1, 08-REQ-5.2, 08-REQ-1.2, 08-REQ-1.3
func TestManualOverrideFlow(t *testing.T) {
	binPath := buildAdaptorBinary(t)
	broker, brokerAddr := newMockDataBroker(t)
	op := newMockParkingOperator(t)

	const grpcPort = "50091"
	proc := startAdaptor(t, binPath, map[string]string{
		"GRPC_PORT":            grpcPort,
		"DATA_BROKER_ADDR":     "http://" + brokerAddr,
		"PARKING_OPERATOR_URL": op.URL(),
		"RUST_LOG":             "info",
	})

	if !broker.WaitForSubscription(10 * time.Second) {
		t.Fatalf("adaptor did not subscribe within 10s\noutput:\n%s", proc.output.String())
	}
	if err := waitForPort("127.0.0.1:"+grpcPort, 5*time.Second); err != nil {
		t.Fatalf("adaptor gRPC not ready: %v\noutput:\n%s", err, proc.output.String())
	}

	adaptorClient := newParkingAdaptorClient(t, "127.0.0.1:"+grpcPort)

	// ── Step 1: Manual StartSession ──

	ctx1, cancel1 := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel1()
	startResp, err := adaptorClient.StartSession(ctx1, &parkingpb.StartSessionRequest{ZoneId: "zone-manual"})
	if err != nil {
		t.Fatalf("StartSession failed: %v", err)
	}
	if startResp.SessionId == "" {
		t.Errorf("expected non-empty session_id in StartSession response")
	}

	// ── Step 2: GetStatus returns active ──

	ctx2, cancel2 := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel2()
	status, err := adaptorClient.GetStatus(ctx2, &parkingpb.GetStatusRequest{})
	if err != nil {
		t.Fatalf("GetStatus failed: %v", err)
	}
	if !status.Active {
		t.Errorf("expected session active after manual StartSession\noutput:\n%s", proc.output.String())
	}

	// ── Step 3: Manual StopSession ──

	ctx3, cancel3 := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel3()
	stopResp, err := adaptorClient.StopSession(ctx3, &parkingpb.StopSessionRequest{})
	if err != nil {
		t.Fatalf("StopSession failed: %v", err)
	}
	if stopResp.SessionId == "" {
		t.Errorf("expected non-empty session_id in StopSession response")
	}

	// ── Step 4: GetStatus returns inactive ──

	ctx4, cancel4 := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel4()
	status2, err := adaptorClient.GetStatus(ctx4, &parkingpb.GetStatusRequest{})
	if err != nil {
		t.Fatalf("GetStatus (post-stop) failed: %v", err)
	}
	if status2.Active {
		t.Errorf("expected session inactive after manual StopSession\noutput:\n%s", proc.output.String())
	}
}

// ── TS-08-SMOKE-3: Override Then Autonomous Resume ────────────────────────────

// TestOverrideThenAutonomousResume verifies that autonomous lock/unlock behavior
// resumes after a manual override.
//
// Sequence:
//  1. Lock event → autonomous session start
//  2. Manual StopSession (override)
//  3. Unlock event → no-op (no session to stop)
//  4. Lock event → autonomous session start resumes
//  5. Unlock event → autonomous session stop
//
// Note: To avoid the issue where a duplicate IsLocked=true signal might not
// trigger a new session start on a real DATA_BROKER (which only delivers
// change notifications), step 3 sends IsLocked=false first to clear the state.
// Our mock DATA_BROKER delivers all signals unconditionally.
//
// Test Spec: TS-08-SMOKE-3
// Requirements: 08-REQ-5.3, 08-REQ-5.E1
func TestOverrideThenAutonomousResume(t *testing.T) {
	binPath := buildAdaptorBinary(t)
	broker, brokerAddr := newMockDataBroker(t)
	op := newMockParkingOperator(t)

	const grpcPort = "50092"
	proc := startAdaptor(t, binPath, map[string]string{
		"GRPC_PORT":            grpcPort,
		"DATA_BROKER_ADDR":     "http://" + brokerAddr,
		"PARKING_OPERATOR_URL": op.URL(),
		"RUST_LOG":             "info",
	})

	if !broker.WaitForSubscription(10 * time.Second) {
		t.Fatalf("adaptor did not subscribe within 10s\noutput:\n%s", proc.output.String())
	}
	if err := waitForPort("127.0.0.1:"+grpcPort, 5*time.Second); err != nil {
		t.Fatalf("adaptor gRPC not ready: %v\noutput:\n%s", err, proc.output.String())
	}

	adaptorClient := newParkingAdaptorClient(t, "127.0.0.1:"+grpcPort)

	// ── Step 1: Lock event → autonomous session start ──

	broker.SendIsLocked(true)
	if !op.WaitForStart(1, 10*time.Second) {
		t.Fatalf("operator did not receive first /parking/start\noutput:\n%s", proc.output.String())
	}

	// Confirm active.
	ctx1, cancel1 := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel1()
	status1, err := adaptorClient.GetStatus(ctx1, &parkingpb.GetStatusRequest{})
	if err != nil {
		t.Fatalf("GetStatus failed: %v", err)
	}
	if !status1.Active {
		t.Fatalf("expected session active after first lock event\noutput:\n%s", proc.output.String())
	}

	// ── Step 2: Manual StopSession (override) ──

	ctx2, cancel2 := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel2()
	_, err = adaptorClient.StopSession(ctx2, &parkingpb.StopSessionRequest{})
	if err != nil {
		t.Fatalf("manual StopSession failed: %v", err)
	}

	// Confirm stopped.
	ctx2b, cancel2b := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel2b()
	status2, err := adaptorClient.GetStatus(ctx2b, &parkingpb.GetStatusRequest{})
	if err != nil {
		t.Fatalf("GetStatus (post-stop) failed: %v", err)
	}
	if status2.Active {
		t.Fatalf("expected session inactive after manual StopSession\noutput:\n%s", proc.output.String())
	}

	// ── Step 3: Unlock event → no-op (no session active) ──

	broker.SendIsLocked(false)
	time.Sleep(200 * time.Millisecond) // Brief pause for event processing.
	// Operator stop count should still be 1 (only the manual stop).
	if op.StopCount() != 1 {
		t.Errorf("expected stop count=1 after unlock with no session, got %d", op.StopCount())
	}

	// ── Step 4: Lock event → autonomous session resumes ──

	broker.SendIsLocked(true)
	if !op.WaitForStart(2, 10*time.Second) {
		t.Fatalf("operator did not receive second /parking/start (autonomous resume)\noutput:\n%s",
			proc.output.String())
	}

	// Confirm active again.
	ctx4, cancel4 := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel4()

	// Poll for active state.
	deadline := time.Now().Add(5 * time.Second)
	var lastStatus *parkingpb.GetStatusResponse
	for time.Now().Before(deadline) {
		lastStatus, err = adaptorClient.GetStatus(ctx4, &parkingpb.GetStatusRequest{})
		if err != nil {
			t.Fatalf("GetStatus failed: %v", err)
		}
		if lastStatus.Active {
			break
		}
		time.Sleep(100 * time.Millisecond)
	}
	if !lastStatus.Active {
		t.Fatalf("expected session active after second lock event (autonomous resume)\noutput:\n%s",
			proc.output.String())
	}

	// ── Step 5: Unlock event → autonomous session stop ──

	broker.SendIsLocked(false)
	if !op.WaitForStop(2, 10*time.Second) {
		t.Fatalf("operator did not receive second /parking/stop (autonomous stop)\noutput:\n%s",
			proc.output.String())
	}

	// Confirm inactive.
	ctx5, cancel5 := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel5()

	deadline2 := time.Now().Add(5 * time.Second)
	for time.Now().Before(deadline2) {
		lastStatus, err = adaptorClient.GetStatus(ctx5, &parkingpb.GetStatusRequest{})
		if err != nil {
			t.Fatalf("GetStatus (final) failed: %v", err)
		}
		if !lastStatus.Active {
			break
		}
		time.Sleep(100 * time.Millisecond)
	}
	if lastStatus.Active {
		t.Errorf("expected session inactive after second unlock event\noutput:\n%s", proc.output.String())
	}

	// Summary: operator received exactly 2 starts and 2 stops.
	if op.StartCount() != 2 {
		t.Errorf("expected operator start_count=2, got %d", op.StartCount())
	}
	if op.StopCount() != 2 {
		t.Errorf("expected operator stop_count=2, got %d", op.StopCount())
	}
}
