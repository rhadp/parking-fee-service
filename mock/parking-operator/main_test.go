package main

import (
	"fmt"
	"net"
	"net/http"
	"os"
	"os/exec"
	"runtime"
	"syscall"
	"testing"
	"time"
)

// ---------------------------------------------------------------------------
// TS-09-17: Parking Operator Graceful Shutdown
// Requirement: 09-REQ-8.1
// ---------------------------------------------------------------------------

func TestGracefulShutdown(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("SIGTERM not supported on Windows")
	}

	port := getFreePort(t)

	// Build the binary.
	binPath := t.TempDir() + "/parking-operator"
	build := exec.Command("go", "build", "-o", binPath, ".")
	build.Dir = "."
	if out, err := build.CombinedOutput(); err != nil {
		t.Fatalf("failed to build: %v\n%s", err, out)
	}

	// Start the server.
	cmd := exec.Command(binPath, "serve", fmt.Sprintf("--port=%d", port))
	cmd.Stdout = os.Stderr
	cmd.Stderr = os.Stderr
	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start: %v", err)
	}

	// Wait until the server is listening.
	addr := fmt.Sprintf("http://localhost:%d", port)
	if !waitForServer(addr, 5*time.Second) {
		_ = cmd.Process.Kill()
		t.Fatal("server did not start within timeout")
	}

	// Send SIGTERM.
	if err := cmd.Process.Signal(syscall.SIGTERM); err != nil {
		t.Fatalf("failed to send SIGTERM: %v", err)
	}

	// Wait for the process to exit.
	done := make(chan error, 1)
	go func() { done <- cmd.Wait() }()

	select {
	case err := <-done:
		if err != nil {
			t.Errorf("expected exit code 0, got error: %v", err)
		}
	case <-time.After(10 * time.Second):
		_ = cmd.Process.Kill()
		t.Fatal("server did not shut down within timeout")
	}
}

func TestServeSubcommandRequired(t *testing.T) {
	// run() with no args should return an error.
	err := run(nil)
	if err == nil {
		t.Error("expected error when no subcommand provided")
	}
}

// getFreePort returns an unused TCP port.
func getFreePort(t *testing.T) int {
	t.Helper()
	l, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("getFreePort: %v", err)
	}
	port := l.Addr().(*net.TCPAddr).Port
	l.Close()
	return port
}

// waitForServer polls the server until it responds or times out.
func waitForServer(addr string, timeout time.Duration) bool {
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		resp, err := http.Get(addr + "/parking/status/probe")
		if err == nil {
			resp.Body.Close()
			return true
		}
		time.Sleep(50 * time.Millisecond)
	}
	return false
}
