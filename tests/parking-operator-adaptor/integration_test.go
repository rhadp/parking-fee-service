package parkingoperatoradaptor_test

import (
	"bufio"
	"context"
	"fmt"
	"os/exec"
	"runtime"
	"strings"
	"syscall"
	"testing"
	"time"

	"github.com/rhadp/parking-fee-service/gen/adapter"
)

// ---------------------------------------------------------------------------
// TS-08-15: Initial SessionActive Published
// Requirement: 08-REQ-4.3
// ---------------------------------------------------------------------------

// TestInitialSessionActive verifies that the service publishes
// Vehicle.Parking.SessionActive=false on startup.
func TestInitialSessionActive(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("signal tests not supported on Windows")
	}

	skipIfTCPUnreachable(t)

	bin := buildAdaptor(t)
	mock := newMockOperator(t)
	grpcPort := getFreePort(t)
	brokerClient := newBrokerClient(t)

	// Start the adaptor.
	env := adaptorEnv(mock.URL(), "http://"+tcpTarget, grpcPort)
	_ = startAdaptor(t, bin, env)

	// Verify SessionActive is false in DATA_BROKER after startup.
	val, ok := getBoolValue(t, brokerClient, signalSessionActive)
	if !ok {
		t.Fatal("Vehicle.Parking.SessionActive has no value after startup")
	}
	if val {
		t.Error("expected Vehicle.Parking.SessionActive=false after startup, got true")
	}
}

// ---------------------------------------------------------------------------
// TS-08-20: Startup Logging
// Requirement: 08-REQ-8.1, 08-REQ-8.2
// ---------------------------------------------------------------------------

// TestStartupLogging verifies the service logs config and ready message
// on startup.
func TestStartupLogging(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("signal tests not supported on Windows")
	}

	skipIfTCPUnreachable(t)

	bin := buildAdaptor(t)
	mock := newMockOperator(t)
	grpcPort := getFreePort(t)

	env := adaptorEnv(mock.URL(), "http://"+tcpTarget, grpcPort)
	proc := startAdaptor(t, bin, env)

	// The service should have logged its configuration and a ready message.
	checks := []struct {
		label  string
		substr string
	}{
		{"service name", "parking-operator-adaptor"},
		{"operator URL", mock.URL()},
		{"broker addr", tcpTarget},
		{"ready message", "ready"},
	}

	for _, check := range checks {
		if !proc.logs.contains(check.substr) {
			t.Errorf("expected log to contain %s (%q), but it did not\nLogs:\n%s",
				check.label, check.substr, joinLines(proc.logs.allLines()))
		}
	}
}

// ---------------------------------------------------------------------------
// TS-08-21: Graceful Shutdown
// Requirement: 08-REQ-8.3, 08-REQ-8.E1
// ---------------------------------------------------------------------------

// TestGracefulShutdown verifies the service exits cleanly on SIGTERM.
func TestGracefulShutdown(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("signal tests not supported on Windows")
	}

	skipIfTCPUnreachable(t)

	bin := buildAdaptor(t)
	mock := newMockOperator(t)
	grpcPort := getFreePort(t)

	env := adaptorEnv(mock.URL(), "http://"+tcpTarget, grpcPort)

	// Start without the cleanup auto-kill to manually control shutdown.
	cmd := exec.Command(bin)
	envSlice := []string{
		"PATH=" + getEnvOrEmpty("PATH"),
		"HOME=" + getEnvOrEmpty("HOME"),
	}
	for k, v := range env {
		envSlice = append(envSlice, k+"="+v)
	}
	cmd.Env = envSlice

	stdoutPipe, err := cmd.StdoutPipe()
	if err != nil {
		t.Fatalf("failed to create stdout pipe: %v", err)
	}
	stderrPipe, err := cmd.StderrPipe()
	if err != nil {
		t.Fatalf("failed to create stderr pipe: %v", err)
	}

	logs := newLogCapture()

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start adaptor: %v", err)
	}

	go func() {
		scanner := bufio.NewScanner(stdoutPipe)
		for scanner.Scan() {
			logs.appendLine(scanner.Text())
		}
	}()
	go func() {
		scanner := bufio.NewScanner(stderrPipe)
		for scanner.Scan() {
			logs.appendLine(scanner.Text())
		}
	}()

	if !logs.waitFor("ready", serviceReadyTimeout) {
		_ = cmd.Process.Kill()
		_ = cmd.Wait()
		t.Fatalf("adaptor did not become ready")
	}

	// Send SIGTERM.
	if err := cmd.Process.Signal(syscall.SIGTERM); err != nil {
		t.Fatalf("failed to send SIGTERM: %v", err)
	}

	// Wait for exit with timeout.
	done := make(chan error, 1)
	go func() {
		done <- cmd.Wait()
	}()

	select {
	case err := <-done:
		if err != nil {
			if exitErr, ok := err.(*exec.ExitError); ok {
				t.Errorf("expected exit 0 after SIGTERM, got %d", exitErr.ExitCode())
			} else {
				t.Errorf("wait error: %v", err)
			}
		}
		// Exit code 0 — success.
	case <-time.After(10 * time.Second):
		_ = cmd.Process.Kill()
		t.Fatal("timed out waiting for graceful shutdown after SIGTERM")
	}
}

// ---------------------------------------------------------------------------
// TS-08-E8: DATA_BROKER Unreachable on Startup
// Requirement: 08-REQ-3.E3
// ---------------------------------------------------------------------------

// TestDatabrokerUnreachable verifies the service retries connection to
// DATA_BROKER and exits with a non-zero code when unreachable.
func TestDatabrokerUnreachable(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("signal tests not supported on Windows")
	}

	bin := buildAdaptor(t)
	mock := newMockOperator(t)
	grpcPort := getFreePort(t)

	// Point DATA_BROKER_ADDR at a port that nothing is listening on.
	unreachablePort := getFreePort(t)
	env := adaptorEnv(mock.URL(),
		"http://localhost:"+itoa(unreachablePort),
		grpcPort,
	)

	proc := startAdaptorNoWait(t, bin, env)

	// Wait for the process to exit (should fail after retries).
	done := make(chan error, 1)
	go func() {
		done <- proc.cmd.Wait()
	}()

	select {
	case err := <-done:
		// Allow log capture goroutine to drain remaining stderr.
		time.Sleep(500 * time.Millisecond)

		if err == nil {
			t.Fatal("expected non-zero exit, got exit 0")
		}
		if exitErr, ok := err.(*exec.ExitError); ok {
			if exitErr.ExitCode() == 0 {
				t.Error("expected non-zero exit code")
			}
		}
		// Verify retry was attempted.
		if !proc.logs.contains("retrying") && !proc.logs.contains("failed") {
			allLogs := joinLines(proc.logs.allLines())
			t.Errorf("expected logs to indicate retry/failure\nLogs:\n%s", allLogs)
		}
	case <-time.After(60 * time.Second):
		t.Fatal("timed out waiting for adaptor to exit after DATA_BROKER unreachable")
	}
}

// ---------------------------------------------------------------------------
// TS-08-E12: Service Restart Loses Session
// Requirement: 08-REQ-6.E1
// ---------------------------------------------------------------------------

// TestSessionLostOnRestart verifies that session state is lost on restart.
func TestSessionLostOnRestart(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("signal tests not supported on Windows")
	}

	skipIfTCPUnreachable(t)

	bin := buildAdaptor(t)
	mock := newMockOperator(t)
	grpcPort := getFreePort(t)
	brokerClient := newBrokerClient(t)

	env := adaptorEnv(mock.URL(), "http://"+tcpTarget, grpcPort)

	// Start the adaptor (first instance).
	proc := startAdaptor(t, bin, env)

	// Start a session via lock event.
	setSignalBool(t, brokerClient, signalIsLocked, true)
	mock.waitForStartCalls(t, 1, 10*time.Second)

	// Verify session is active.
	adaptorClient := newAdaptorClient(t, grpcPort)
	ctx, cancel := context.WithTimeout(context.Background(), rpcTimeout)
	defer cancel()
	statusResp, err := adaptorClient.GetStatus(ctx, &adapter.GetStatusRequest{})
	if err != nil {
		t.Fatalf("GetStatus failed: %v", err)
	}
	if statusResp.Session == nil || !statusResp.Session.Active {
		t.Fatal("expected active session before restart")
	}

	// Kill the first instance (simulates restart).
	if proc.cmd.Process != nil {
		_ = proc.cmd.Process.Kill()
		_ = proc.cmd.Wait()
	}

	// Small delay to release the port.
	time.Sleep(1 * time.Second)

	// Reset mock operator counts for the second instance.
	mock2 := newMockOperator(t)
	env2 := adaptorEnv(mock2.URL(), "http://"+tcpTarget, grpcPort)

	// Start the adaptor (second instance).
	_ = startAdaptor(t, bin, env2)

	// GetStatus should return active=false (session lost on restart).
	adaptorClient2 := newAdaptorClient(t, grpcPort)
	ctx2, cancel2 := context.WithTimeout(context.Background(), rpcTimeout)
	defer cancel2()
	statusResp2, err := adaptorClient2.GetStatus(ctx2, &adapter.GetStatusRequest{})
	if err != nil {
		t.Fatalf("GetStatus after restart failed: %v", err)
	}
	if statusResp2.Session != nil && statusResp2.Session.Active {
		t.Error("expected active=false after restart, got active=true")
	}
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

func joinLines(lines []string) string {
	var b strings.Builder
	for _, l := range lines {
		b.WriteString(l)
		b.WriteByte('\n')
	}
	return b.String()
}

func getEnvOrEmpty(key string) string {
	val, _ := syscall.Getenv(key)
	return val
}

func itoa(n int) string {
	return fmt.Sprintf("%d", n)
}

