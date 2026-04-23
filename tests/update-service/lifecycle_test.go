package updateservice_test

import (
	"fmt"
	"strings"
	"syscall"
	"testing"
	"time"
)

// TestStartupLogging verifies the service logs its configuration (port,
// inactivity timeout) and a ready message on startup.
// TS-07-17 | Requirement: 07-REQ-10.1
func TestStartupLogging(t *testing.T) {
	binary := buildUpdateService(t)
	port := findFreePort(t)
	configPath := writeConfigFile(t, port)

	si := startUpdateServiceWithCleanup(t, binary, port, configPath)

	logs := si.getLogs()
	joined := strings.Join(logs, "\n")

	// REQ-10.1: log output must contain port number.
	portStr := fmt.Sprintf("%d", port)
	if !strings.Contains(joined, portStr) {
		t.Errorf("startup logs do not contain port %s; logs:\n%s", portStr, joined)
	}

	// REQ-10.1: log output must contain a "ready" indicator.
	if !strings.Contains(strings.ToLower(joined), "ready") {
		t.Errorf("startup logs do not contain 'ready'; logs:\n%s", joined)
	}
}

// TestGracefulShutdown verifies the service exits with code 0 when receiving
// SIGTERM while idle (no RPCs in flight).
// TS-07-18 | Requirement: 07-REQ-10.2
func TestGracefulShutdown(t *testing.T) {
	binary := buildUpdateService(t)
	port := findFreePort(t)
	configPath := writeConfigFile(t, port)

	si := startUpdateService(t, binary, port, configPath)

	// Send SIGTERM to the service process.
	if err := si.cmd.Process.Signal(syscall.SIGTERM); err != nil {
		t.Fatalf("failed to send SIGTERM: %v", err)
	}

	// Wait for the process to exit.
	done := make(chan error, 1)
	go func() {
		done <- si.cmd.Wait()
	}()

	select {
	case err := <-done:
		if err != nil {
			t.Errorf("expected clean exit, got error: %v", err)
		}
		exitCode := si.cmd.ProcessState.ExitCode()
		if exitCode != 0 {
			t.Errorf("expected exit code 0, got %d", exitCode)
		}
	case <-time.After(15 * time.Second):
		_ = si.cmd.Process.Kill()
		_ = si.cmd.Wait()
		t.Fatal("service did not exit within 15 seconds after SIGTERM")
	}
}

// TestGracefulShutdownSIGINT verifies the service exits with code 0 when
// receiving SIGINT (Ctrl+C).
// Requirement: 07-REQ-10.2
func TestGracefulShutdownSIGINT(t *testing.T) {
	binary := buildUpdateService(t)
	port := findFreePort(t)
	configPath := writeConfigFile(t, port)

	si := startUpdateService(t, binary, port, configPath)

	// Send SIGINT to the service process.
	if err := si.cmd.Process.Signal(syscall.SIGINT); err != nil {
		t.Fatalf("failed to send SIGINT: %v", err)
	}

	// Wait for the process to exit.
	done := make(chan error, 1)
	go func() {
		done <- si.cmd.Wait()
	}()

	select {
	case err := <-done:
		if err != nil {
			t.Errorf("expected clean exit, got error: %v", err)
		}
		exitCode := si.cmd.ProcessState.ExitCode()
		if exitCode != 0 {
			t.Errorf("expected exit code 0, got %d", exitCode)
		}
	case <-time.After(15 * time.Second):
		_ = si.cmd.Process.Kill()
		_ = si.cmd.Wait()
		t.Fatal("service did not exit within 15 seconds after SIGINT")
	}
}
