// Integration tests for PARKING_OPERATOR_ADAPTOR.
//
// All tests use a mock DATA_BROKER and mock PARKING_OPERATOR HTTP server, so
// they run without any external infrastructure.
//
// TestDatabrokerUnreachable tests the failure-mode behavior when DATA_BROKER
// is unavailable and always passes regardless of infrastructure.
//
// Test Specs:
//   - TS-08-15: TestInitialSessionActive
//   - TS-08-20: TestStartupLogging
//   - TS-08-21: TestGracefulShutdown
//   - TS-08-E8: TestDatabrokerUnreachable
//   - TS-08-E12: TestSessionLostOnRestart
package parking_operator_adaptor_test

import (
	"context"
	"fmt"
	"strings"
	"syscall"
	"testing"
	"time"

	parkingpb "github.com/sdv-demo/tests/parking-operator-adaptor/pb/parking"
)

// ── TestStartupLogging (TS-08-20) ─────────────────────────────────────────

// TestStartupLogging verifies that on startup the adaptor logs its version,
// configuration values, and a "ready" message.
//
// Requirements: 08-REQ-8.1, 08-REQ-8.2
// Test Spec: TS-08-20
func TestStartupLogging(t *testing.T) {
	binary := getAdaptorBinary(t)
	broker, brokerAddr := newMockDataBroker(t)
	mockOp := startMockOperator(t)

	ap := startAdaptor(t, binary,
		"DATA_BROKER_ADDR="+brokerAddr,
		"PARKING_OPERATOR_URL="+mockOp.URL,
		"VEHICLE_ID=TEST-VIN-LOG-001",
		"ZONE_ID=zone-log-test",
	)

	// Wait for gRPC server to be ready.
	waitForGRPCReady(t, ap, 10*time.Second)

	// Give a moment for the "ready" log to flush.
	time.Sleep(300 * time.Millisecond)

	output := ap.getOutput()

	checks := []struct {
		name    string
		pattern string
	}{
		{"service name", "parking-operator-adaptor"},
		{"operator URL", mockOp.URL},
		{"vehicle ID", "TEST-VIN-LOG-001"},
		{"grpc port", fmt.Sprintf("%d", ap.port)},
		{"ready message", "ready"},
	}

	for _, c := range checks {
		if !strings.Contains(output, c.pattern) {
			t.Errorf("startup log missing %s (%q); output:\n%s", c.name, c.pattern, output)
		}
	}

	_ = broker // broker runs in background; kept for cleanup
}

// ── TestGracefulShutdown (TS-08-21) ───────────────────────────────────────

// TestGracefulShutdown verifies that the adaptor exits cleanly (code 0) when
// sent SIGTERM.
//
// Requirements: 08-REQ-8.3
// Test Spec: TS-08-21
func TestGracefulShutdown(t *testing.T) {
	binary := getAdaptorBinary(t)
	_, brokerAddr := newMockDataBroker(t)
	mockOp := startMockOperator(t)

	ap := startAdaptor(t, binary,
		"DATA_BROKER_ADDR="+brokerAddr,
		"PARKING_OPERATOR_URL="+mockOp.URL,
	)

	// Wait for the gRPC server to be ready before sending SIGTERM.
	waitForGRPCReady(t, ap, 10*time.Second)

	// Send SIGTERM.
	if err := ap.cmd.Process.Signal(syscall.SIGTERM); err != nil {
		t.Fatalf("send SIGTERM: %v", err)
	}

	code, err := waitForExit(ap, 10*time.Second)
	if err != nil {
		t.Fatalf("wait for exit: %v", err)
	}
	if code != 0 {
		t.Errorf("expected exit code 0 after SIGTERM, got %d\noutput:\n%s", code, ap.getOutput())
	}
}

// ── TestInitialSessionActive (TS-08-15) ───────────────────────────────────

// TestInitialSessionActive verifies that the adaptor publishes
// Vehicle.Parking.SessionActive = false in DATA_BROKER on startup.
//
// Requirements: 08-REQ-4.3
// Test Spec: TS-08-15
func TestInitialSessionActive(t *testing.T) {
	binary := getAdaptorBinary(t)
	broker, brokerAddr := newMockDataBroker(t)
	mockOp := startMockOperator(t)

	ap := startAdaptor(t, binary,
		"DATA_BROKER_ADDR="+brokerAddr,
		"PARKING_OPERATOR_URL="+mockOp.URL,
	)

	// Wait for gRPC server to be ready (adaptor has published initial state by then).
	waitForGRPCReady(t, ap, 10*time.Second)
	time.Sleep(200 * time.Millisecond)

	// Verify that the adaptor set SessionActive=false via the mock broker.
	val, found := broker.getSignal("Vehicle.Parking.SessionActive")
	if !found {
		t.Error("expected adaptor to set Vehicle.Parking.SessionActive on startup, but no Set call recorded")
	} else if val {
		t.Errorf("expected Vehicle.Parking.SessionActive=false on startup, got true")
	}

	// Also verify via gRPC that no session is active.
	client := newParkingClient(t, ap)
	status := grpcGetStatus(t, client)
	if status.Active {
		t.Errorf("expected GetStatus.active=false on startup, got true")
	}
}

// ── TestDatabrokerUnreachable (TS-08-E8) ──────────────────────────────────

// TestDatabrokerUnreachable verifies that when DATA_BROKER is unreachable the
// adaptor retries and then exits with a non-zero exit code.
//
// This test does NOT use a mock DATA_BROKER — it explicitly tests the
// failure case.
//
// Requirements: 08-REQ-3.E3
// Test Spec: TS-08-E8
func TestDatabrokerUnreachable(t *testing.T) {
	binary := getAdaptorBinary(t)
	mockOp := startMockOperator(t)

	// Find a port that is almost certainly not listening.
	freePort := findFreePort(t)
	unusableAddr := fmt.Sprintf("http://localhost:%d", freePort)

	ap := startAdaptor(t, binary,
		"DATA_BROKER_ADDR="+unusableAddr,
		"PARKING_OPERATOR_URL="+mockOp.URL,
	)

	// The adaptor tries 5 times with delays 1s, 2s, 4s, 8s = ~15s total.
	// Allow up to 35s for the exit to handle slow CI environments.
	code, err := waitForExit(ap, 35*time.Second)
	if err != nil {
		t.Fatalf("wait for exit: %v", err)
	}
	if code == 0 {
		t.Errorf("expected non-zero exit code when DATA_BROKER unreachable, got 0")
	}
}

// ── TestSessionLostOnRestart (TS-08-E12) ──────────────────────────────────

// TestSessionLostOnRestart verifies that in-memory session state is lost when
// the adaptor process is restarted.  After a restart, GetStatus must return
// active=false regardless of any previous session.
//
// Requirements: 08-REQ-6.E1
// Test Spec: TS-08-E12
func TestSessionLostOnRestart(t *testing.T) {
	binary := getAdaptorBinary(t)
	_, brokerAddr := newMockDataBroker(t)
	mockOp := startMockOperator(t)

	// Start first adaptor instance.
	ap1 := startAdaptor(t, binary,
		"DATA_BROKER_ADDR="+brokerAddr,
		"PARKING_OPERATOR_URL="+mockOp.URL,
		"VEHICLE_ID=TEST-VIN-RESTART",
		"ZONE_ID=zone-restart-test",
	)
	waitForGRPCReady(t, ap1, 10*time.Second)

	client1 := newParkingClient(t, ap1)

	// Start a session manually via gRPC.
	ctx1, cancel1 := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel1()
	startResp, err := client1.StartSession(ctx1, &parkingpb.StartSessionRequest{
		ZoneId: "zone-restart-test",
	})
	if err != nil {
		t.Fatalf("StartSession: %v", err)
	}
	if startResp.SessionId == "" {
		t.Fatal("StartSession returned empty session_id")
	}

	// Verify session is active on first instance.
	status := grpcGetStatus(t, client1)
	if !status.Active {
		t.Fatalf("expected session active after StartSession, got inactive")
	}

	// Terminate the first adaptor process.
	_ = ap1.cmd.Process.Signal(syscall.SIGTERM)
	waitForExit(ap1, 5*time.Second) //nolint

	// Start a second adaptor instance with the same mock broker.
	// New process = no in-memory session state.
	ap2 := startAdaptor(t, binary,
		"DATA_BROKER_ADDR="+brokerAddr,
		"PARKING_OPERATOR_URL="+mockOp.URL,
		"VEHICLE_ID=TEST-VIN-RESTART",
		"ZONE_ID=zone-restart-test",
	)
	waitForGRPCReady(t, ap2, 10*time.Second)

	client2 := newParkingClient(t, ap2)

	// Session state must be gone after restart.
	status2 := grpcGetStatus(t, client2)
	if status2.Active {
		t.Errorf("expected no active session after restart (in-memory state lost), got active=true")
	}
	if status2.SessionId != "" {
		t.Errorf("expected empty session_id after restart, got %q", status2.SessionId)
	}
}
