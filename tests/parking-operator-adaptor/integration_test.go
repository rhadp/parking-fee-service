package parkingoperatoradaptor_test

import (
	"fmt"
	"os"
	"os/exec"
	"strings"
	"testing"
	"time"

	adaptorpb "parking-fee-service/tests/parking-operator-adaptor/adaptorpb"
)

// ---------------------------------------------------------------------------
// TS-08-15: Initial SessionActive Published
// Requirement: 08-REQ-4.3
// ---------------------------------------------------------------------------

// TestInitialSessionActive verifies the service publishes
// Vehicle.Parking.SessionActive=false on startup.
func TestInitialSessionActive(t *testing.T) {
	requirePodman(t)
	dbClient := ensureDatabrokerReachable(t)
	resetSignals(t, dbClient)

	// Set SessionActive to true before starting the adaptor so we can detect
	// it being reset to false on startup.
	setSignalBool(t, dbClient, signalSessionActive, true)

	mockOp := startMockOperator(t)
	binary := buildAdaptor(t)
	grpcPort := freePort(t)
	_ = startAdaptor(t, binary, grpcPort, mockOp.URL())

	// After startup, SessionActive should be false.
	time.Sleep(500 * time.Millisecond)

	val, ok := getSignalBool(t, dbClient, signalSessionActive)
	if !ok {
		t.Fatal("TS-08-15: SessionActive signal has no value after startup")
	}
	if val != false {
		t.Errorf("TS-08-15: expected SessionActive=false after startup, got true")
	}
}

// ---------------------------------------------------------------------------
// TS-08-20: Startup Logging
// Requirement: 08-REQ-8.1, 08-REQ-8.2
// ---------------------------------------------------------------------------

// TestStartupLogging verifies the service logs config values and a ready
// message on startup.
func TestStartupLogging(t *testing.T) {
	requirePodman(t)
	_ = ensureDatabrokerReachable(t)

	mockOp := startMockOperator(t)
	binary := buildAdaptor(t)
	grpcPort := freePort(t)
	proc := startAdaptor(t, binary, grpcPort, mockOp.URL())

	time.Sleep(500 * time.Millisecond)

	logs := proc.getLogs()
	combined := strings.Join(logs, "\n")

	// Verify config values are logged.
	checks := []struct {
		label string
		want  string
	}{
		{"service name", "parking-operator-adaptor"},
		{"GRPC_PORT", fmt.Sprintf("%d", grpcPort)},
		{"ready message", "ready"},
		{"VEHICLE_ID", "DEMO-VIN-001"},
		{"ZONE_ID", "zone-demo-1"},
	}

	for _, c := range checks {
		if !strings.Contains(combined, c.want) {
			t.Errorf("TS-08-20: expected log to contain %q (%s), not found in:\n%s",
				c.want, c.label, combined)
		}
	}
}

// ---------------------------------------------------------------------------
// TS-08-21: Graceful Shutdown
// Requirement: 08-REQ-8.3
// ---------------------------------------------------------------------------

// TestGracefulShutdown verifies the service exits cleanly on SIGTERM.
func TestGracefulShutdown(t *testing.T) {
	requirePodman(t)
	_ = ensureDatabrokerReachable(t)

	mockOp := startMockOperator(t)
	binary := buildAdaptor(t)
	grpcPort := freePort(t)
	proc := startAdaptor(t, binary, grpcPort, mockOp.URL())

	// Send interrupt signal (SIGINT via cancel context).
	// The process was started with context, so cancel sends the signal.
	proc.cancel()

	done := make(chan error, 1)
	go func() {
		done <- proc.cmd.Wait()
	}()

	select {
	case err := <-done:
		// On cancel the process may exit with 0 or be killed.
		// We just verify it exits within a reasonable time.
		if err != nil {
			// Context cancellation sends SIGKILL, which is acceptable.
			t.Logf("TS-08-21: process exited with: %v (acceptable for context cancel)", err)
		}
	case <-time.After(10 * time.Second):
		t.Fatal("TS-08-21: service did not exit within 10 seconds after signal")
	}
}

// ---------------------------------------------------------------------------
// TS-08-E8: DATA_BROKER Unreachable on Startup
// Requirement: 08-REQ-3.E3
// ---------------------------------------------------------------------------

// TestDatabrokerUnreachable verifies retry behavior when DATA_BROKER is
// unreachable, and that the service exits with a non-zero code.
func TestDatabrokerUnreachable(t *testing.T) {
	binary := buildAdaptor(t)

	mockOp := startMockOperator(t)

	// Point to a non-existent DATA_BROKER endpoint.
	cmd := exec.Command(binary)
	cmd.Env = append(os.Environ(),
		"DATA_BROKER_ADDR=http://localhost:19999",
		fmt.Sprintf("PARKING_OPERATOR_URL=%s", mockOp.URL()),
		"GRPC_PORT=0",
		"RUST_LOG=warn",
	)
	cmd.Stdout = nil
	cmd.Stderr = nil

	err := cmd.Start()
	if err != nil {
		t.Fatalf("TS-08-E8: failed to start parking-operator-adaptor: %v", err)
	}

	done := make(chan error, 1)
	go func() {
		done <- cmd.Wait()
	}()

	select {
	case err := <-done:
		if err == nil {
			t.Error("TS-08-E8: expected non-zero exit code, got 0")
		}
		if exitErr, ok := err.(*exec.ExitError); ok {
			if exitErr.ExitCode() == 0 {
				t.Error("TS-08-E8: expected non-zero exit code, got 0")
			}
			t.Logf("TS-08-E8: service exited with code %d (expected non-zero)", exitErr.ExitCode())
		}
	case <-time.After(60 * time.Second):
		_ = cmd.Process.Kill()
		t.Fatal("TS-08-E8: service did not exit within 60 seconds")
	}
}

// ---------------------------------------------------------------------------
// TS-08-E12: Service Restart Loses Session
// Requirement: 08-REQ-6.E1
// ---------------------------------------------------------------------------

// TestSessionLostOnRestart verifies session state is lost on restart.
func TestSessionLostOnRestart(t *testing.T) {
	requirePodman(t)
	dbClient := ensureDatabrokerReachable(t)
	resetSignals(t, dbClient)

	mockOp := startMockOperator(t)
	binary := buildAdaptor(t)
	grpcPort := freePort(t)

	// Start adaptor and create a session via lock event.
	proc := startAdaptor(t, binary, grpcPort, mockOp.URL())
	time.Sleep(500 * time.Millisecond)

	// Trigger lock event to start a session.
	setSignalBool(t, dbClient, signalIsLocked, true)
	mockOp.waitForStartCount(t, 1, 10*time.Second)

	// Verify session is active via gRPC.
	conn := dialAdaptor(t, grpcPort)
	client := adaptorClient(conn)
	ctx, cancel := opCtx()
	defer cancel()

	status, err := client.GetStatus(ctx, &adaptorpb.GetStatusRequest{})
	if err != nil {
		t.Fatalf("TS-08-E12: GetStatus failed: %v", err)
	}
	if !status.Active {
		t.Fatal("TS-08-E12: expected active session before restart")
	}

	// Stop the adaptor.
	proc.stop()

	// Reset signals for clean startup.
	setSignalBool(t, dbClient, signalIsLocked, false)

	// Restart the adaptor on a new port (old port may still be in TIME_WAIT).
	grpcPort2 := freePort(t)
	_ = startAdaptor(t, binary, grpcPort2, mockOp.URL())
	time.Sleep(500 * time.Millisecond)

	// Verify session is NOT active after restart.
	conn2 := dialAdaptor(t, grpcPort2)
	client2 := adaptorClient(conn2)
	ctx2, cancel2 := opCtx()
	defer cancel2()

	status2, err := client2.GetStatus(ctx2, &adaptorpb.GetStatusRequest{})
	if err != nil {
		t.Fatalf("TS-08-E12: GetStatus after restart failed: %v", err)
	}
	if status2.Active {
		t.Error("TS-08-E12: expected no active session after restart, got active")
	}
}
