package parkingoperatoradaptor_test

import (
	"context"
	"fmt"
	"os/exec"
	"syscall"
	"testing"
	"time"

	pa "github.com/rhadp/parking-fee-service/gen/parking_adaptor/v1"
)

// TestInitialSessionActive verifies that the adaptor publishes
// Vehicle.Parking.SessionActive=false on startup.
// TS-08-15 | Requirement: 08-REQ-4.3
func TestInitialSessionActive(t *testing.T) {
	skipIfTCPUnreachable(t)

	binary := buildAdaptor(t)
	conn := connectTCP(t)
	valClient := newVALClient(conn)
	v2Client := newV2Client(conn)
	mo := startMockOperator(t)
	grpcPort := getFreePort(t)

	// Reset IsLocked to false so the adaptor doesn't auto-start a session on
	// startup from leftover state of previous tests.
	setBool(t, v2Client, signalIsLocked, false)
	time.Sleep(300 * time.Millisecond)

	// Set SessionActive to true before starting the adaptor, so we can verify
	// that the adaptor resets it to false on startup.
	setBool(t, v2Client, signalSessionActive, true)

	env := adaptorEnv(grpcPort, mo.url())
	startAdaptor(t, binary, env...)

	// After startup, SessionActive should be false.
	waitForBool(t, valClient, signalSessionActive, false, 10*time.Second)

	val := getBool(t, valClient, signalSessionActive)
	if val == nil {
		t.Fatal("expected SessionActive to have a value, got nil")
	}
	if *val != false {
		t.Error("expected SessionActive = false after startup")
	}
}

// TestStartupLogging verifies that the adaptor logs config values and a ready
// message on startup.
// TS-08-20 | Requirement: 08-REQ-8.1, 08-REQ-8.2
func TestStartupLogging(t *testing.T) {
	skipIfTCPUnreachable(t)

	binary := buildAdaptor(t)
	mo := startMockOperator(t)
	grpcPort := getFreePort(t)

	env := adaptorEnv(grpcPort, mo.url())
	_, lc := startAdaptor(t, binary, env...)

	// Give a moment for logs to flush.
	time.Sleep(500 * time.Millisecond)

	logs := lc.contents()

	// Verify config values are logged (08-REQ-8.1).
	checks := []struct {
		name  string
		value string
	}{
		{"parking-operator-adaptor", "parking-operator-adaptor"},
		{"operator URL", mo.url()},
		{"DATA_BROKER_ADDR", tcpTarget},
		{"GRPC_PORT", fmt.Sprintf("%d", grpcPort)},
		{"VEHICLE_ID", "DEMO-VIN-001"},
		{"ZONE_ID", "zone-demo-1"},
		{"ready", "ready"},
	}

	for _, check := range checks {
		if !containsAny(logs, check.value) {
			t.Errorf("expected log output to contain %q (%s), but it did not.\nLogs:\n%s",
				check.value, check.name, logs)
		}
	}
}

// TestGracefulShutdown verifies that the adaptor exits cleanly on SIGTERM.
// TS-08-21 | Requirement: 08-REQ-8.3, 08-REQ-8.E1
func TestGracefulShutdown(t *testing.T) {
	skipIfTCPUnreachable(t)

	binary := buildAdaptor(t)
	mo := startMockOperator(t)
	grpcPort := getFreePort(t)

	env := adaptorEnv(grpcPort, mo.url())
	cmd, _ := startAdaptor(t, binary, env...)

	// Send SIGTERM.
	if err := cmd.Process.Signal(syscall.SIGTERM); err != nil {
		t.Fatalf("failed to send SIGTERM: %v", err)
	}

	// Wait for the process to exit with a timeout.
	exitCh := make(chan error, 1)
	go func() {
		exitCh <- cmd.Wait()
	}()

	select {
	case err := <-exitCh:
		if err != nil {
			// On some systems, a clean exit after SIGTERM may still report
			// an error due to signal handling. Check if exit code is 0.
			if exitErr, ok := err.(*exec.ExitError); ok {
				if exitErr.ExitCode() != 0 {
					t.Errorf("expected exit code 0, got %d", exitErr.ExitCode())
				}
			} else {
				t.Errorf("unexpected wait error: %v", err)
			}
		}
		// Exit code 0 or clean signal termination.
	case <-time.After(10 * time.Second):
		t.Fatal("process did not exit within 10 seconds after SIGTERM")
	}

	// Mark process as nil so cleanup doesn't try to kill it again.
	cmd.Process = nil
}

// TestDatabrokerUnreachable verifies the adaptor retries connection and
// exits non-zero when DATA_BROKER is unreachable.
// TS-08-E8 | Requirement: 08-REQ-3.E3
func TestDatabrokerUnreachable(t *testing.T) {
	binary := buildAdaptor(t)
	mo := startMockOperator(t)
	grpcPort := getFreePort(t)

	// Point DATA_BROKER_ADDR to a port nothing is listening on.
	unreachablePort := getFreePort(t)
	env := []string{
		fmt.Sprintf("GRPC_PORT=%d", grpcPort),
		fmt.Sprintf("PARKING_OPERATOR_URL=%s", mo.url()),
		fmt.Sprintf("DATA_BROKER_ADDR=http://localhost:%d", unreachablePort),
		"VEHICLE_ID=DEMO-VIN-001",
		"ZONE_ID=zone-demo-1",
		"RUST_LOG=info",
	}

	cmd, buf := startAdaptorNoWait(t, binary, env...)

	// Wait for the process to exit (should fail after retries).
	exitCh := make(chan error, 1)
	go func() {
		exitCh <- cmd.Wait()
	}()

	select {
	case err := <-exitCh:
		if err == nil {
			t.Fatal("expected non-zero exit code, but process exited successfully")
		}
		exitErr, ok := err.(*exec.ExitError)
		if !ok {
			t.Fatalf("unexpected error type: %v", err)
		}
		if exitErr.ExitCode() == 0 {
			t.Error("expected non-zero exit code when DATA_BROKER is unreachable")
		}
	case <-time.After(60 * time.Second):
		t.Fatal("process did not exit within 60 seconds")
	}

	// Mark process as nil so cleanup doesn't try to kill again.
	cmd.Process = nil

	// Give a moment for any final writes to the buffer.
	time.Sleep(100 * time.Millisecond)
	logs := buf.contents()

	// Verify logs contain retry indication.
	if !containsAny(logs, "retry", "retrying", "failed to connect") {
		t.Errorf("expected log output to indicate retries.\nLogs:\n%s", logs)
	}
}

// TestSessionLostOnRestart verifies that session state is lost when the
// adaptor restarts.
// TS-08-E12 | Requirement: 08-REQ-6.E1
func TestSessionLostOnRestart(t *testing.T) {
	skipIfTCPUnreachable(t)

	binary := buildAdaptor(t)
	conn := connectTCP(t)
	v2Client := newV2Client(conn)
	mo := startMockOperator(t)
	grpcPort := getFreePort(t)

	env := adaptorEnv(grpcPort, mo.url())

	// First run: start adaptor, trigger a session via lock event.
	cmd1, _ := startAdaptor(t, binary, env...)
	adaptorClient1 := connectAdaptorGRPC(t, grpcPort)

	// Trigger a lock event to start a session.
	setBool(t, v2Client, signalIsLocked, true)
	mo.waitForStartCalled(t, 1, 10*time.Second)

	// Verify session is active.
	ctx := context.Background()
	status, err := adaptorClient1.GetStatus(ctx, &pa.GetStatusRequest{})
	if err != nil {
		t.Fatalf("GetStatus failed: %v", err)
	}
	if !status.Active {
		t.Fatal("expected active session after lock event")
	}

	// Kill the first instance.
	if cmd1.Process != nil {
		_ = cmd1.Process.Kill()
		_ = cmd1.Wait()
		cmd1.Process = nil
	}

	// Reset IsLocked so the second instance doesn't immediately trigger.
	setBool(t, v2Client, signalIsLocked, false)
	time.Sleep(500 * time.Millisecond)

	// Second run: restart adaptor on the same port.
	cmd2, _ := startAdaptor(t, binary, env...)
	adaptorClient2 := connectAdaptorGRPC(t, grpcPort)

	// Verify session is NOT active (state was lost).
	status2, err := adaptorClient2.GetStatus(ctx, &pa.GetStatusRequest{})
	if err != nil {
		t.Fatalf("GetStatus after restart failed: %v", err)
	}
	if status2.Active {
		t.Error("expected no active session after restart")
	}

	// Clean up second instance.
	if cmd2.Process != nil {
		_ = cmd2.Process.Kill()
		_ = cmd2.Wait()
		cmd2.Process = nil
	}
}

// containsAny checks if s contains any of the given substrings.
func containsAny(s string, substrings ...string) bool {
	lower := toLower(s)
	for _, sub := range substrings {
		if indexOf(lower, toLower(sub)) >= 0 {
			return true
		}
	}
	return false
}

func toLower(s string) string {
	b := make([]byte, len(s))
	for i := range s {
		c := s[i]
		if c >= 'A' && c <= 'Z' {
			c += 'a' - 'A'
		}
		b[i] = c
	}
	return string(b)
}

func indexOf(s, sub string) int {
	if len(sub) == 0 {
		return 0
	}
	for i := 0; i <= len(s)-len(sub); i++ {
		if s[i:i+len(sub)] == sub {
			return i
		}
	}
	return -1
}

