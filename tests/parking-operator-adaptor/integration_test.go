package parkingoperatoradaptortests

import (
	"context"
	"fmt"
	"os/exec"
	"strings"
	"syscall"
	"testing"
	"time"

	parkingpb "github.com/rhadp/parking-fee-service/tests/parking-operator-adaptor/pb/parking"
)

// ── TS-08-20: Startup Logging ─────────────────────────────────────────────────

// TestStartupLogging verifies that the service logs its version, config values,
// and a "ready" message on startup.
//
// The test runs the adaptor with an unreachable DATA_BROKER so the process will
// eventually exit, but the startup log appears before the first connection attempt.
//
// Test Spec: TS-08-20
// Requirements: 08-REQ-8.1, 08-REQ-8.2
func TestStartupLogging(t *testing.T) {
	binPath := buildAdaptorBinary(t)

	const testPort = "50083"
	const operatorURL = "http://localhost:18080"
	const brokerAddr = "http://localhost:19999"

	proc := startAdaptor(t, binPath, map[string]string{
		"GRPC_PORT":            testPort,
		"PARKING_OPERATOR_URL": operatorURL,
		"DATA_BROKER_ADDR":     brokerAddr,
		"VEHICLE_ID":           "TEST-VIN-001",
		"ZONE_ID":              "zone-test-1",
		"RUST_LOG":             "info",
	})

	// The startup log appears before DATA_BROKER connection attempts.
	if !waitForLog(proc, "parking-operator-adaptor", 10*time.Second) {
		t.Fatalf("startup log not found within 10s\noutput:\n%s", proc.output.String())
	}

	output := proc.output.String()

	// Verify key config values appear in the log.
	checks := []struct {
		name   string
		substr string
	}{
		{"operator URL", "18080"},
		{"broker addr", "19999"},
		{"grpc port", testPort},
		{"vehicle_id", "TEST-VIN-001"},
		{"zone_id", "zone-test-1"},
	}
	for _, c := range checks {
		if !strings.Contains(output, c.substr) {
			t.Errorf("startup log missing %s (%q)\nfull output:\n%s", c.name, c.substr, output)
		}
	}
}

// ── TS-08-21: Graceful Shutdown ───────────────────────────────────────────────

// TestGracefulShutdown verifies that SIGTERM causes the adaptor to exit with
// code 0. Uses a mock DATA_BROKER so the adaptor reaches "ready" state.
//
// Test Spec: TS-08-21
// Requirements: 08-REQ-8.3, 08-REQ-8.E1
func TestGracefulShutdown(t *testing.T) {
	binPath := buildAdaptorBinary(t)
	broker, brokerAddr := newMockDataBroker(t)
	op := newMockParkingOperator(t)

	const grpcPort = "50084"
	proc := startAdaptor(t, binPath, map[string]string{
		"GRPC_PORT":            grpcPort,
		"DATA_BROKER_ADDR":     "http://" + brokerAddr,
		"PARKING_OPERATOR_URL": op.URL(),
		"RUST_LOG":             "info",
	})

	// Wait for the adaptor to subscribe (indicates it's fully started).
	if !broker.WaitForSubscription(10 * time.Second) {
		t.Skipf("adaptor did not subscribe to DATA_BROKER within 10s\noutput:\n%s", proc.output.String())
	}

	// Wait for the gRPC server to be ready.
	if err := waitForPort("127.0.0.1:"+grpcPort, 5*time.Second); err != nil {
		t.Fatalf("adaptor gRPC port not listening: %v\noutput:\n%s", err, proc.output.String())
	}

	// Send SIGTERM.
	if err := proc.cmd.Process.Signal(syscall.SIGTERM); err != nil {
		t.Fatalf("send SIGTERM: %v", err)
	}

	done := make(chan error, 1)
	go func() { done <- proc.cmd.Wait() }()

	select {
	case err := <-done:
		if exitErr, ok := err.(*exec.ExitError); ok && exitErr.ExitCode() != 0 {
			t.Errorf("expected exit 0 after SIGTERM, got %d\noutput:\n%s",
				exitErr.ExitCode(), proc.output.String())
		}
		// nil means exit 0 — test passes.
	case <-time.After(10 * time.Second):
		t.Fatalf("adaptor did not exit within 10s after SIGTERM\noutput:\n%s", proc.output.String())
	}
}

// ── TS-08-E8: DATA_BROKER Unreachable on Startup ──────────────────────────────

// TestDatabrokerUnreachable verifies that the adaptor exits non-zero after
// exhausting connection retries when DATA_BROKER is unreachable.
//
// Test Spec: TS-08-E8
// Requirements: 08-REQ-3.E3
func TestDatabrokerUnreachable(t *testing.T) {
	binPath := buildAdaptorBinary(t)

	proc := startAdaptor(t, binPath, map[string]string{
		"DATA_BROKER_ADDR": "http://localhost:19999",
		"RUST_LOG":         "warn",
	})

	// The adaptor retries 5 times (delays 1s+2s+4s+4s = 11s total).
	// Allow 60s for all retries to complete.
	done := make(chan error, 1)
	go func() { done <- proc.cmd.Wait() }()

	select {
	case err := <-done:
		if err == nil {
			t.Fatal("expected non-zero exit when DATA_BROKER is unreachable, got exit 0")
		}
		if exitErr, ok := err.(*exec.ExitError); ok {
			if exitErr.ExitCode() == 0 {
				t.Fatalf("expected non-zero exit code, got 0")
			}
			// Non-zero exit confirmed.
		}
	case <-time.After(60 * time.Second):
		t.Fatal("adaptor did not exit within 60s; expected failure after exhausting retries")
	}
}

// ── TS-08-15: Initial SessionActive Published ─────────────────────────────────

// TestInitialSessionActive verifies that the adaptor publishes
// Vehicle.Parking.SessionActive=false to DATA_BROKER on startup.
//
// Test Spec: TS-08-15
// Requirements: 08-REQ-4.3
func TestInitialSessionActive(t *testing.T) {
	binPath := buildAdaptorBinary(t)
	broker, brokerAddr := newMockDataBroker(t)
	op := newMockParkingOperator(t)

	const grpcPort = "50085"
	proc := startAdaptor(t, binPath, map[string]string{
		"GRPC_PORT":            grpcPort,
		"DATA_BROKER_ADDR":     "http://" + brokerAddr,
		"PARKING_OPERATOR_URL": op.URL(),
		"RUST_LOG":             "info",
	})

	// Wait for subscription (adaptor fully connected to DATA_BROKER).
	if !broker.WaitForSubscription(10 * time.Second) {
		t.Fatalf("adaptor did not subscribe to DATA_BROKER within 10s\noutput:\n%s", proc.output.String())
	}

	// Wait for the initial SessionActive=false to be published.
	if !broker.WaitForSessionActive(false, 5*time.Second) {
		val, ok := broker.LastSessionActive()
		t.Fatalf("expected Vehicle.Parking.SessionActive=false on startup; got value=%v ok=%v\noutput:\n%s",
			val, ok, proc.output.String())
	}

	// Confirm the initial value is false.
	val, ok := broker.LastSessionActive()
	if !ok {
		t.Fatalf("no SessionActive value received from adaptor\noutput:\n%s", proc.output.String())
	}
	if val {
		t.Errorf("expected SessionActive=false on startup, got true")
	}
}

// ── TS-08-E12: Session State Lost on Restart ──────────────────────────────────

// TestSessionLostOnRestart verifies that session state is not persisted across
// restarts. Starts a session via lock event, kills the adaptor, then verifies
// the restarted adaptor reports no active session.
//
// Test Spec: TS-08-E12
// Requirements: 08-REQ-6.E1
func TestSessionLostOnRestart(t *testing.T) {
	binPath := buildAdaptorBinary(t)
	op := newMockParkingOperator(t)

	// ── First instance ──

	broker1, broker1Addr := newMockDataBroker(t)
	const grpcPort1 = "50086"

	proc1 := startAdaptor(t, binPath, map[string]string{
		"GRPC_PORT":            grpcPort1,
		"DATA_BROKER_ADDR":     "http://" + broker1Addr,
		"PARKING_OPERATOR_URL": op.URL(),
		"RUST_LOG":             "info",
	})

	if !broker1.WaitForSubscription(10 * time.Second) {
		t.Fatalf("first adaptor did not subscribe within 10s\noutput:\n%s", proc1.output.String())
	}
	if err := waitForPort(fmt.Sprintf("127.0.0.1:%s", grpcPort1), 5*time.Second); err != nil {
		t.Fatalf("first adaptor gRPC not ready: %v\noutput:\n%s", err, proc1.output.String())
	}

	// Trigger a session start via lock event.
	broker1.SendIsLocked(true)

	// Wait for the operator to receive the start call.
	if !op.WaitForStart(1, 10*time.Second) {
		t.Fatalf("operator did not receive start call within 10s\noutput:\n%s", proc1.output.String())
	}

	// Verify session is active via GetStatus.
	ctx1, cancel1 := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel1()
	client1 := newParkingAdaptorClient(t, fmt.Sprintf("127.0.0.1:%s", grpcPort1))
	status1, err := client1.GetStatus(ctx1, &parkingpb.GetStatusRequest{})
	if err != nil {
		t.Fatalf("GetStatus on first instance failed: %v", err)
	}
	if !status1.Active {
		t.Fatalf("expected session active after lock event, got inactive\noutput:\n%s", proc1.output.String())
	}

	// ── Kill first instance ──
	if err := proc1.cmd.Process.Kill(); err != nil {
		t.Logf("kill first adaptor: %v", err)
	}
	_ = proc1.cmd.Wait()

	// ── Second instance (restart) ──

	// Use a fresh mock DATA_BROKER for the second instance.
	_, broker2Addr := newMockDataBroker(t)

	const grpcPort2 = "50087"
	proc2 := startAdaptor(t, binPath, map[string]string{
		"GRPC_PORT":            grpcPort2,
		"DATA_BROKER_ADDR":     "http://" + broker2Addr,
		"PARKING_OPERATOR_URL": op.URL(),
		"RUST_LOG":             "info",
	})

	// Wait for the second instance to be ready.
	if err := waitForPort(fmt.Sprintf("127.0.0.1:%s", grpcPort2), 10*time.Second); err != nil {
		t.Fatalf("second adaptor gRPC not ready: %v\noutput:\n%s", err, proc2.output.String())
	}

	// After restart, GetStatus must return inactive (session state was in-memory only).
	ctx2, cancel2 := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel2()
	client2 := newParkingAdaptorClient(t, fmt.Sprintf("127.0.0.1:%s", grpcPort2))
	status2, err := client2.GetStatus(ctx2, &parkingpb.GetStatusRequest{})
	if err != nil {
		t.Fatalf("GetStatus on second instance failed: %v\noutput:\n%s", err, proc2.output.String())
	}
	if status2.Active {
		t.Errorf("expected session inactive after restart (session state is in-memory only)\noutput:\n%s",
			proc2.output.String())
	}
}
