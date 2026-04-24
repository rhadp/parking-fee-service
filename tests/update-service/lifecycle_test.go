package updateservice_test

import (
	"bufio"
	"encoding/json"
	"fmt"
	"net"
	"os"
	"os/exec"
	"runtime"
	"strings"
	"syscall"
	"testing"
	"time"
)

// startServiceForLifecycle starts the update-service binary for lifecycle
// testing WITHOUT registering a t.Cleanup kill. The caller is responsible
// for stopping the process. This avoids a race between t.Cleanup's Kill()
// and the test's own Wait() when testing signal-driven shutdown.
func startServiceForLifecycle(t *testing.T, binaryPath string) (*exec.Cmd, *logCapture, int) {
	t.Helper()

	// Get a free port.
	l, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to get free port: %v", err)
	}
	port := l.Addr().(*net.TCPAddr).Port
	l.Close()

	// Write config file.
	cfg := map[string]interface{}{
		"grpc_port":               port,
		"registry_url":            "",
		"inactivity_timeout_secs": 86400,
		"container_storage_path":  "/var/lib/containers/adapters/",
	}
	data, err := json.Marshal(cfg)
	if err != nil {
		t.Fatalf("failed to marshal config: %v", err)
	}
	f, err := os.CreateTemp("", "lifecycle-config-*.json")
	if err != nil {
		t.Fatalf("failed to create temp config: %v", err)
	}
	if _, err := f.Write(data); err != nil {
		t.Fatalf("failed to write config: %v", err)
	}
	f.Close()
	t.Cleanup(func() { os.Remove(f.Name()) })

	cmd := exec.Command(binaryPath)
	cmd.Env = []string{
		fmt.Sprintf("CONFIG_PATH=%s", f.Name()),
		fmt.Sprintf("PATH=%s", os.Getenv("PATH")),
		fmt.Sprintf("HOME=%s", os.Getenv("HOME")),
	}
	cmd.SysProcAttr = &syscall.SysProcAttr{Setpgid: true}

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
		t.Fatalf("failed to start update-service: %v", err)
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

	if !logs.waitFor("UPDATE_SERVICE ready", serviceReadyTimeout) {
		_ = cmd.Process.Kill()
		_ = cmd.Wait()
		allLogs := strings.Join(logs.allLines(), "\n")
		t.Fatalf("update-service did not become ready within %v\nLogs:\n%s",
			serviceReadyTimeout, allLogs)
	}

	return cmd, logs, port
}

// TestStartupLogging verifies that on startup, the service logs its
// configuration (port number, inactivity timeout) and a ready message.
//
// Requirements: 07-REQ-10.1
// Test Spec: TS-07-17
func TestStartupLogging(t *testing.T) {
	bin := buildUpdateService(t)
	cmd, logs, port := startServiceForLifecycle(t, bin)
	defer func() {
		_ = cmd.Process.Kill()
		_ = cmd.Wait()
	}()

	// Check for port number in logs.
	portStr := fmt.Sprintf("%d", port)
	if !logs.contains(portStr) {
		t.Errorf("expected startup log to contain port number %q", portStr)
	}

	// Check for inactivity timeout in logs.
	if !logs.contains("86400") {
		t.Error("expected startup log to contain inactivity timeout '86400'")
	}

	// Check for ready indicator in logs.
	if !logs.contains("ready") {
		t.Error("expected startup log to contain 'ready' indicator")
	}
}

// TestGracefulShutdown verifies that SIGTERM causes the update-service
// to stop accepting RPCs and exit with code 0.
//
// Requirements: 07-REQ-10.2
// Test Spec: TS-07-18
func TestGracefulShutdown(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("signal tests not supported on Windows")
	}

	bin := buildUpdateService(t)
	cmd, _, _ := startServiceForLifecycle(t, bin)

	// Send SIGTERM.
	if err := cmd.Process.Signal(syscall.SIGTERM); err != nil {
		t.Fatalf("failed to send SIGTERM: %v", err)
	}

	// Wait for the process to exit.
	errCh := make(chan error, 1)
	go func() {
		errCh <- cmd.Wait()
	}()

	select {
	case err := <-errCh:
		if err != nil {
			if exitErr, ok := err.(*exec.ExitError); ok {
				t.Errorf("expected exit code 0 after SIGTERM, got %d", exitErr.ExitCode())
			} else {
				t.Errorf("wait error: %v", err)
			}
		}
		// err == nil means exit code 0, which is expected.
	case <-time.After(10 * time.Second):
		_ = cmd.Process.Kill()
		_ = cmd.Wait()
		t.Fatal("update-service did not exit within 10 seconds after SIGTERM")
	}
}

// TestGracefulShutdownSIGINT verifies that SIGINT also triggers a
// clean exit with code 0.
//
// Requirements: 07-REQ-10.2
func TestGracefulShutdownSIGINT(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("signal tests not supported on Windows")
	}

	bin := buildUpdateService(t)
	cmd, _, _ := startServiceForLifecycle(t, bin)

	// Send SIGINT.
	if err := cmd.Process.Signal(syscall.SIGINT); err != nil {
		t.Fatalf("failed to send SIGINT: %v", err)
	}

	// Wait for the process to exit.
	errCh := make(chan error, 1)
	go func() {
		errCh <- cmd.Wait()
	}()

	select {
	case err := <-errCh:
		if err != nil {
			if exitErr, ok := err.(*exec.ExitError); ok {
				t.Errorf("expected exit code 0 after SIGINT, got %d", exitErr.ExitCode())
			} else {
				t.Errorf("wait error: %v", err)
			}
		}
	case <-time.After(10 * time.Second):
		_ = cmd.Process.Kill()
		_ = cmd.Wait()
		t.Fatal("update-service did not exit within 10 seconds after SIGINT")
	}
}
