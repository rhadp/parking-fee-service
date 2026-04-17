// Smoke tests for PARKING_OPERATOR_ADAPTOR — end-to-end flows.
//
// All smoke tests use a mock DATA_BROKER and mock PARKING_OPERATOR, so they
// run without any external infrastructure.
//
// Test Specs:
//   - TS-08-SMOKE-1: TestLockStartUnlockStopFlow
//   - TS-08-SMOKE-2: TestManualOverrideFlow
//   - TS-08-SMOKE-3: TestOverrideThenAutonomousResume
package parking_operator_adaptor_test

import (
	"context"
	"testing"
	"time"

	parkingpb "github.com/sdv-demo/tests/parking-operator-adaptor/pb/parking"
)

// ── TestLockStartUnlockStopFlow (TS-08-SMOKE-1) ───────────────────────────

// TestLockStartUnlockStopFlow verifies the end-to-end autonomous flow:
//   - PushLockEvent(true) → adaptor calls PARKING_OPERATOR /start, session active
//   - GetStatus returns active=true with session_id
//   - PushLockEvent(false) → adaptor calls PARKING_OPERATOR /stop, session inactive
//   - GetStatus returns active=false
//
// Requirements: 08-REQ-3.3, 08-REQ-3.4, 08-REQ-4.1, 08-REQ-4.2
// Test Spec: TS-08-SMOKE-1
func TestLockStartUnlockStopFlow(t *testing.T) {
	binary := getAdaptorBinary(t)
	broker, brokerAddr := newMockDataBroker(t)
	mockOp := startMockOperator(t)

	ap := startAdaptor(t, binary,
		"DATA_BROKER_ADDR="+brokerAddr,
		"PARKING_OPERATOR_URL="+mockOp.URL,
		"VEHICLE_ID=SMOKE-VIN-001",
		"ZONE_ID=zone-smoke-1",
	)
	waitForGRPCReady(t, ap, 10*time.Second)

	// Wait until the adaptor has subscribed to the DATA_BROKER stream.
	waitForAdaptorSubscribed(t, broker, 5*time.Second)

	client := newParkingClient(t, ap)

	// Verify no session active initially.
	status := grpcGetStatus(t, client)
	if status.Active {
		t.Fatal("expected no active session before lock event")
	}

	// Push IsLocked=true to trigger autonomous session start.
	broker.PushLockEvent(true)

	// Wait for mock operator's /parking/start to be called.
	waitForCondition(t,
		func() bool { return mockOp.startCount.Load() >= 1 },
		10*time.Second, "operator start called after lock event",
	)

	// Verify session is active.
	status = grpcGetStatus(t, client)
	if !status.Active {
		t.Errorf("expected active session after lock event, got inactive")
	}
	if status.SessionId == "" {
		t.Errorf("expected non-empty session_id after lock event")
	}

	// Verify DATA_BROKER SessionActive was set to true.
	waitForCondition(t, func() bool {
		val, found := broker.getSignal("Vehicle.Parking.SessionActive")
		return found && val == true
	}, 5*time.Second, "SessionActive=true in DATA_BROKER")

	// Push IsLocked=false to trigger autonomous session stop.
	broker.PushLockEvent(false)

	// Wait for mock operator's /parking/stop to be called.
	waitForCondition(t,
		func() bool { return mockOp.stopCount.Load() >= 1 },
		10*time.Second, "operator stop called after unlock event",
	)

	// Verify session is inactive.
	status = grpcGetStatus(t, client)
	if status.Active {
		t.Errorf("expected inactive session after unlock event, got active")
	}

	// Verify DATA_BROKER SessionActive was set to false.
	waitForCondition(t, func() bool {
		val, found := broker.getSignal("Vehicle.Parking.SessionActive")
		return found && val == false
	}, 5*time.Second, "SessionActive=false in DATA_BROKER after stop")
}

// ── TestManualOverrideFlow (TS-08-SMOKE-2) ────────────────────────────────

// TestManualOverrideFlow verifies manual gRPC start/stop works regardless
// of the vehicle lock state.
//
// Requirements: 08-REQ-5.1, 08-REQ-5.2
// Test Spec: TS-08-SMOKE-2
func TestManualOverrideFlow(t *testing.T) {
	binary := getAdaptorBinary(t)
	_, brokerAddr := newMockDataBroker(t)
	mockOp := startMockOperator(t)

	ap := startAdaptor(t, binary,
		"DATA_BROKER_ADDR="+brokerAddr,
		"PARKING_OPERATOR_URL="+mockOp.URL,
		"VEHICLE_ID=SMOKE-VIN-002",
		"ZONE_ID=zone-smoke-2",
	)
	waitForGRPCReady(t, ap, 10*time.Second)

	client := newParkingClient(t, ap)

	// Manual StartSession.
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	startResp, err := client.StartSession(ctx, &parkingpb.StartSessionRequest{
		ZoneId: "zone-manual",
	})
	if err != nil {
		t.Fatalf("StartSession: %v", err)
	}
	if startResp.SessionId == "" {
		t.Errorf("StartSession returned empty session_id")
	}
	if startResp.Rate == nil {
		t.Errorf("StartSession returned nil rate")
	}

	// Verify active.
	status := grpcGetStatus(t, client)
	if !status.Active {
		t.Errorf("expected active session after manual StartSession, got inactive")
	}
	if status.SessionId != startResp.SessionId {
		t.Errorf("GetStatus session_id %q != StartSession session_id %q",
			status.SessionId, startResp.SessionId)
	}

	// Manual StopSession.
	ctx2, cancel2 := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel2()
	stopResp, err := client.StopSession(ctx2, &parkingpb.StopSessionRequest{})
	if err != nil {
		t.Fatalf("StopSession: %v", err)
	}
	if stopResp.SessionId == "" {
		t.Errorf("StopSession returned empty session_id")
	}
	if stopResp.DurationSeconds == 0 {
		t.Errorf("expected non-zero duration_seconds in StopSession response")
	}

	// Verify inactive.
	status = grpcGetStatus(t, client)
	if status.Active {
		t.Errorf("expected inactive session after manual StopSession, got active")
	}
}

// ── TestOverrideThenAutonomousResume (TS-08-SMOKE-3) ──────────────────────

// TestOverrideThenAutonomousResume verifies that after a manual StopSession
// override, the next lock event resumes autonomous session management
// (override non-persistent).
//
// Requirements: 08-REQ-5.3, 08-REQ-5.E1
// Test Spec: TS-08-SMOKE-3
func TestOverrideThenAutonomousResume(t *testing.T) {
	binary := getAdaptorBinary(t)
	broker, brokerAddr := newMockDataBroker(t)
	mockOp := startMockOperator(t)

	ap := startAdaptor(t, binary,
		"DATA_BROKER_ADDR="+brokerAddr,
		"PARKING_OPERATOR_URL="+mockOp.URL,
		"VEHICLE_ID=SMOKE-VIN-003",
		"ZONE_ID=zone-smoke-3",
	)
	waitForGRPCReady(t, ap, 10*time.Second)

	// Wait until the adaptor has subscribed to the DATA_BROKER stream.
	waitForAdaptorSubscribed(t, broker, 5*time.Second)

	client := newParkingClient(t, ap)

	// Step 1: Lock event starts session autonomously.
	broker.PushLockEvent(true)
	waitForCondition(t,
		func() bool { return mockOp.startCount.Load() >= 1 },
		10*time.Second, "first autonomous start",
	)
	status := grpcGetStatus(t, client)
	if !status.Active {
		t.Fatalf("expected active session after first lock event")
	}

	// Step 2: Manual StopSession override.
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	_, err := client.StopSession(ctx, &parkingpb.StopSessionRequest{})
	if err != nil {
		t.Fatalf("manual StopSession: %v", err)
	}
	status = grpcGetStatus(t, client)
	if status.Active {
		t.Fatalf("expected inactive session after manual stop")
	}

	// Push unlock event (no-op since session already stopped).
	broker.PushLockEvent(false)
	// Brief pause for the unlock event to be processed.
	time.Sleep(300 * time.Millisecond)

	// Step 3: Lock event should start a new session autonomously (override non-persistent).
	broker.PushLockEvent(true)
	waitForCondition(t,
		func() bool { return mockOp.startCount.Load() >= 2 },
		10*time.Second, "second autonomous start after override",
	)
	status = grpcGetStatus(t, client)
	if !status.Active {
		t.Errorf("expected active session after second lock event (override non-persistent)")
	}

	// Step 4: Unlock stops session autonomously.
	broker.PushLockEvent(false)
	waitForCondition(t,
		func() bool { return mockOp.stopCount.Load() >= 2 },
		10*time.Second, "second autonomous stop",
	)
	status = grpcGetStatus(t, client)
	if status.Active {
		t.Errorf("expected inactive session after second unlock event")
	}
}
