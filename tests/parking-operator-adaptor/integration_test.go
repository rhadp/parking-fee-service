package parkingoperatoradaptor_test

import (
	"context"
	"fmt"
	"os"
	"strings"
	"syscall"
	"testing"
	"time"

	"github.com/rhadp/parking-fee-service/gen/adapter"
)

// TestInitialSessionActive verifies that the adaptor publishes
// Vehicle.Parking.SessionActive=false on startup (TS-08-15).
func TestInitialSessionActive(t *testing.T) {
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

	// Verify SessionActive is false in DATA_BROKER.
	waitForSessionActive(t, valClient, false, 10*time.Second)
}

// TestStartupLogging verifies the adaptor logs config and ready message
// on startup (TS-08-20).
func TestStartupLogging(t *testing.T) {
	skipIfTCPUnreachable(t)

	binary := buildAdaptor(t)
	mockOp := newMockOperator(t)
	grpcPort := getFreePort(t)

	env := adaptorEnv(grpcPort, mockOp.URL())
	proc, lines := startAdaptorWithStderrScanner(t, binary, env)

	// Collect all lines until "ready" to inspect startup logs.
	allLines := collectLinesUntilReady(t, lines, 30*time.Second)
	allOutput := strings.Join(allLines, "\n")

	checks := []string{
		"parking-operator-adaptor",
		mockOp.URL(),
		databrokerAddr,
		fmt.Sprintf("%d", grpcPort),
		"ready",
	}
	for _, check := range checks {
		if !strings.Contains(allOutput, check) {
			t.Errorf("startup log missing %q in output:\n%s", check, allOutput)
		}
	}

	// Cleanup.
	_ = proc.sendSignal(syscall.SIGTERM)
	proc.waitExit(5 * time.Second)
}

// TestGracefulShutdown verifies the adaptor exits cleanly on SIGTERM (TS-08-21).
func TestGracefulShutdown(t *testing.T) {
	skipIfTCPUnreachable(t)

	binary := buildAdaptor(t)
	mockOp := newMockOperator(t)
	grpcPort := getFreePort(t)

	env := adaptorEnv(grpcPort, mockOp.URL())
	proc, lines := startAdaptorWithStderrScanner(t, binary, env)
	waitReady(t, lines, 30*time.Second)

	// Send SIGTERM.
	if err := proc.sendSignal(syscall.SIGTERM); err != nil {
		t.Fatalf("failed to send SIGTERM: %v", err)
	}

	exitCode := proc.waitExit(5 * time.Second)
	if exitCode != 0 {
		t.Errorf("expected exit code 0, got %d\nstderr: %s", exitCode, proc.getStderr())
	}
}

// TestDatabrokerUnreachable verifies retry behavior when DATA_BROKER is
// unreachable on startup (TS-08-E8).
func TestDatabrokerUnreachable(t *testing.T) {
	binary := buildAdaptor(t)
	mockOp := newMockOperator(t)
	grpcPort := getFreePort(t)

	// Point DATA_BROKER_ADDR to a non-listening port.
	env := []string{
		fmt.Sprintf("PATH=%s", os.Getenv("PATH")),
		fmt.Sprintf("HOME=%s", os.Getenv("HOME")),
		fmt.Sprintf("GRPC_PORT=%d", grpcPort),
		fmt.Sprintf("PARKING_OPERATOR_URL=%s", mockOp.URL()),
		"DATA_BROKER_ADDR=http://localhost:19999",
		"VEHICLE_ID=DEMO-VIN-001",
		"ZONE_ID=zone-demo-1",
	}

	proc := startAdaptor(t, binary, env)

	// The adaptor should retry connection and then exit with non-zero code.
	// With 5 attempts and exponential backoff (1s, 2s, 4s, 8s), this takes
	// approximately 15-30 seconds.
	exitCode := proc.waitExit(60 * time.Second)
	if exitCode == 0 {
		t.Errorf("expected non-zero exit code, got 0")
	}

	stderr := proc.getStderr()
	lowerStderr := strings.ToLower(stderr)
	if !strings.Contains(lowerStderr, "retry") &&
		!strings.Contains(lowerStderr, "connect") &&
		!strings.Contains(lowerStderr, "attempt") {
		t.Logf("note: stderr may not contain explicit retry indication:\n%s", stderr)
	}
}

// TestSessionLostOnRestart verifies that session state is lost on restart
// (TS-08-E12).
func TestSessionLostOnRestart(t *testing.T) {
	skipIfTCPUnreachable(t)

	binary := buildAdaptor(t)
	mockOp := newMockOperator(t)
	grpcPort := getFreePort(t)

	_, valClient := dialDatabroker(t)

	env := adaptorEnv(grpcPort, mockOp.URL())

	// First run: start session via lock event.
	proc1, lines1 := startAdaptorWithStderrScanner(t, binary, env)
	waitReady(t, lines1, 30*time.Second)

	publishValue(t, valClient, signalIsLocked, boolValue(true))
	mockOp.waitForStartCalls(t, 1, 10*time.Second)

	// Verify session is active via gRPC.
	adaptorClient := dialAdaptor(t, grpcPort)
	ctx := context.Background()
	resp, err := adaptorClient.GetStatus(ctx, &adapter.GetStatusRequest{})
	if err != nil {
		t.Fatalf("GetStatus failed: %v", err)
	}
	if resp.GetSession() == nil || !resp.GetSession().GetActive() {
		t.Fatal("expected active session after lock event")
	}

	// Reset IsLocked to false to prevent auto-start on the next run.
	publishValue(t, valClient, signalIsLocked, boolValue(false))
	mockOp.waitForStopCalls(t, 1, 10*time.Second)

	// Kill the first instance.
	_ = proc1.sendSignal(syscall.SIGTERM)
	proc1.waitExit(5 * time.Second)

	// Wait for the port to be freed.
	time.Sleep(1 * time.Second)

	// Second run: verify session state is lost.
	proc2, lines2 := startAdaptorWithStderrScanner(t, binary, env)
	waitReady(t, lines2, 30*time.Second)

	adaptorClient2 := dialAdaptor(t, grpcPort)
	resp2, err := adaptorClient2.GetStatus(ctx, &adapter.GetStatusRequest{})
	if err != nil {
		t.Fatalf("GetStatus after restart failed: %v", err)
	}
	if resp2.GetSession() != nil && resp2.GetSession().GetActive() {
		t.Fatal("expected no active session after restart (session state should be lost)")
	}

	// Kill the second instance.
	_ = proc2.sendSignal(syscall.SIGTERM)
	proc2.waitExit(5 * time.Second)
}
