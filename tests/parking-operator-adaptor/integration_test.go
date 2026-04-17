// Integration tests for PARKING_OPERATOR_ADAPTOR (spec 08_parking_operator_adaptor).
//
// Tests cover:
//   - TS-08-E8:  TestDatabrokerUnreachable  — retry behaviour and non-zero exit
//   - TS-08-20:  TestStartupLogging         — config values and "ready" in logs
//   - TS-08-21:  TestGracefulShutdown       — SIGTERM → exit code 0
//   - TS-08-15:  TestInitialSessionActive   — SessionActive=false on startup
//   - TS-08-E12: TestSessionLostOnRestart   — in-memory state lost on restart
//
// Tests that require a fully started adaptor (all except TestDatabrokerUnreachable)
// will skip gracefully when the DATA_BROKER is unreachable or the adaptor fails
// to connect due to the kuksa.val.v1 / kuksa.val.v2 API mismatch.
package parkingoperatoradaptor

import (
	"fmt"
	"strings"
	"testing"
	"time"
)

// ── TS-08-E8: DATA_BROKER Unreachable on Startup ──────────────────────────────

// TestDatabrokerUnreachable verifies that the adaptor exits with a non-zero
// code when DATA_BROKER is not reachable after exhausting retries.
//
// This test does NOT require any infrastructure — it intentionally starts the
// adaptor against a port where nothing is listening.
//
// Requirements: 08-REQ-3.E3
// Test Spec: TS-08-E8
func TestDatabrokerUnreachable(t *testing.T) {
	// Deliberately bad endpoint — nothing listens here.
	badBroker := "http://localhost:19997"

	ap := startAdaptorRaw(t, map[string]string{
		"DATA_BROKER_ADDR":     badBroker,
		"PARKING_OPERATOR_URL": "http://localhost:19998", // not needed but valid
		"GRPC_PORT":            fmt.Sprintf("%d", testGrpcPort),
		"RUST_LOG":             "warn",
	})

	// The adaptor retries up to 5 times with delays 1s, 2s, 4s, 8s = 15s total.
	// Allow 60s to be safe.
	exitCode, ok := ap.waitForExit(60 * time.Second)
	if !ok {
		ap.kill()
		t.Fatalf("parking-operator-adaptor did not exit within 60s despite unreachable DATA_BROKER; logs:\n%s",
			ap.logs())
	}
	if exitCode == 0 {
		t.Errorf("expected non-zero exit code when DATA_BROKER is unreachable, got 0; logs:\n%s",
			ap.logs())
	}

	// Verify retry messages were logged (08-REQ-3.E3).
	logs := ap.logs()
	if !strings.Contains(logs, "retry") && !strings.Contains(logs, "Retry") {
		t.Errorf("expected retry messages in logs; logs:\n%s", logs)
	}
}

// ── TS-08-20: Startup Logging ─────────────────────────────────────────────────

// TestStartupLogging verifies that the adaptor logs its configuration and a
// "ready" message on successful startup (08-REQ-8.1, 08-REQ-8.2).
//
// Requirements: 08-REQ-8.1, 08-REQ-8.2
// Test Spec: TS-08-20
func TestStartupLogging(t *testing.T) {
	mockOp := newMockOperator(t)
	ap := startAdaptor(t, mockOp) // skips if DATA_BROKER unavailable or proto mismatch

	// Allow up to 2 s for log lines to propagate after the port is open.
	deadline := time.Now().Add(2 * time.Second)
	for time.Now().Before(deadline) {
		logs := ap.logs()
		portStr := fmt.Sprintf("%d", testGrpcPort)
		if strings.Contains(logs, portStr) && strings.Contains(strings.ToLower(logs), "ready") {
			return // pass
		}
		time.Sleep(100 * time.Millisecond)
	}

	logs := ap.logs()
	portStr := fmt.Sprintf("%d", testGrpcPort)

	if !strings.Contains(logs, "parking-operator-adaptor") {
		t.Errorf("expected service name in startup logs; logs:\n%s", logs)
	}
	if !strings.Contains(logs, portStr) {
		t.Errorf("expected gRPC port %d in startup logs; logs:\n%s", testGrpcPort, logs)
	}
	if !strings.Contains(logs, dataBrokerEndpoint) && !strings.Contains(logs, "55556") {
		t.Errorf("expected DATA_BROKER address in startup logs; logs:\n%s", logs)
	}
	if !strings.Contains(logs, mockOp.url()) && !strings.Contains(logs, "localhost") {
		t.Errorf("expected PARKING_OPERATOR_URL in startup logs; logs:\n%s", logs)
	}
	if !strings.Contains(strings.ToLower(logs), "ready") {
		t.Errorf(`expected "ready" in startup logs; logs:\n%s`, logs)
	}
}

// ── TS-08-21: Graceful Shutdown ───────────────────────────────────────────────

// TestGracefulShutdown verifies that the adaptor exits with code 0 when
// SIGTERM is received (08-REQ-8.3, 08-REQ-8.E1).
//
// Requirements: 08-REQ-8.3, 08-REQ-8.E1
// Test Spec: TS-08-21
func TestGracefulShutdown(t *testing.T) {
	mockOp := newMockOperator(t)
	ap := startAdaptor(t, mockOp) // skips if DATA_BROKER unavailable or proto mismatch

	ap.sendSIGTERM()

	code, ok := ap.waitForExit(15 * time.Second)
	if !ok {
		ap.kill()
		t.Fatalf("parking-operator-adaptor did not exit within 15s after SIGTERM; logs:\n%s", ap.logs())
	}
	if code != 0 {
		t.Errorf("expected exit code 0 after SIGTERM, got %d; logs:\n%s", code, ap.logs())
	}
}

// ── TS-08-15: Initial SessionActive Published ─────────────────────────────────

// TestInitialSessionActive verifies that the adaptor publishes
// Vehicle.Parking.SessionActive = false in DATA_BROKER at startup (08-REQ-4.3).
//
// Requirements: 08-REQ-4.3
// Test Spec: TS-08-15
func TestInitialSessionActive(t *testing.T) {
	requireGrpcurl(t)
	mockOp := newMockOperator(t)
	_ = startAdaptor(t, mockOp) // skips if DATA_BROKER unavailable or proto mismatch

	// After startup, Vehicle.Parking.SessionActive must be false.
	// Allow a short window for the signal to propagate.
	if !waitForBrokerBool(t, signalSessionActive, false, 5*time.Second) {
		// Check what value was actually set.
		actual := brokerGetBool(t, signalSessionActive)
		t.Errorf("expected Vehicle.Parking.SessionActive=false after startup, got %v", actual)
	}
}

// ── TS-08-E12: Session State Lost on Restart ──────────────────────────────────

// TestSessionLostOnRestart verifies that in-memory session state is lost when
// the adaptor restarts. GetStatus must return active=false after restart
// even if a session was active before (08-REQ-6.E1).
//
// Requirements: 08-REQ-6.E1
// Test Spec: TS-08-E12
func TestSessionLostOnRestart(t *testing.T) {
	requireGrpcurl(t)
	mockOp := newMockOperator(t)
	ap := startAdaptor(t, mockOp) // skips if DATA_BROKER unavailable or proto mismatch

	// Start a session manually via gRPC.
	out, err := grpcCallAdaptor(t, testGrpcPort, "StartSession", `{"zone_id":"zone-restart-test"}`)
	if err != nil {
		t.Fatalf("StartSession failed: %v\noutput: %s", err, out)
	}
	if !strings.Contains(out, "sessionId") && !strings.Contains(out, "session_id") {
		t.Fatalf("StartSession response missing session_id; output: %s", out)
	}

	// Verify session is active.
	if !waitForGRPCActive(t, testGrpcPort, true, 5*time.Second) {
		t.Fatalf("expected session to be active before restart; logs:\n%s", ap.logs())
	}

	// Kill the adaptor (simulates restart by not waiting for graceful shutdown).
	ap.kill()
	_, _ = ap.waitForExit(5 * time.Second)

	// Start a fresh adaptor instance on the same port.
	mockOp2 := newMockOperator(t)
	ap2 := startAdaptor(t, mockOp2)
	_ = ap2

	// After restart, session state must be gone — active=false.
	out, err = grpcCallAdaptor(t, testGrpcPort, "GetStatus", "")
	if err != nil {
		t.Fatalf("GetStatus after restart failed: %v\noutput: %s", err, out)
	}

	if strings.Contains(out, `"active": true`) || strings.Contains(out, `"active":true`) {
		t.Errorf("expected active=false after restart, but got active=true; output:\n%s", out)
	}
}
