package mockapps

import (
	"fmt"
	"net/http"
	"os/exec"
	"syscall"
	"testing"
	"time"
)

// TS-09-4: PARKING_OPERATOR Serve Starts Server
// Requirement: 09-REQ-2.1
//
// Starts parking-operator, waits for it to be ready, makes an HTTP request,
// and verifies that the server responds (404 for unknown session is acceptable).
func TestParkingOperatorServeStartsServer(t *testing.T) {
	// Build the binary.
	srcDir := parkingOperatorSrcDir(t)
	binPath := buildGoBinary(t, srcDir)

	port := findFreePort(t)
	portFlag := fmt.Sprintf("--port=%d", port)
	baseURL := fmt.Sprintf("http://127.0.0.1:%d", port)

	cmd := startProcess(t, binPath, "serve", portFlag)
	_ = cmd

	// Wait for the server to become ready.
	waitForHTTPReady(t, statusURL(baseURL, "probe"), 5*time.Second)

	// Verify it serves HTTP correctly: status for an unknown session → 404.
	resp, err := http.Get(statusURL(baseURL, "nonexistent"))
	if err != nil {
		t.Fatalf("GET /parking/status/nonexistent failed: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusNotFound {
		t.Errorf("expected HTTP 404 for unknown session, got %d", resp.StatusCode)
	}

	// Send SIGTERM to cleanly shut down.
	if err := cmd.Process.Signal(syscall.SIGTERM); err != nil {
		t.Logf("SIGTERM failed (process may have already exited): %v", err)
	}
}

// TS-09-8: PARKING_OPERATOR Graceful Shutdown
// Requirement: 09-REQ-2.5
//
// Starts parking-operator, sends SIGTERM, and verifies exit code 0.
func TestParkingOperatorGracefulShutdown(t *testing.T) {
	srcDir := parkingOperatorSrcDir(t)
	binPath := buildGoBinary(t, srcDir)

	port := findFreePort(t)
	portFlag := fmt.Sprintf("--port=%d", port)
	baseURL := fmt.Sprintf("http://127.0.0.1:%d", port)

	cmd := exec.Command(binPath, "serve", portFlag)
	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start parking-operator: %v", err)
	}

	// Wait for the server to be ready before sending SIGTERM.
	waitForHTTPReady(t, statusURL(baseURL, "probe"), 5*time.Second)

	// Send SIGTERM.
	if err := cmd.Process.Signal(syscall.SIGTERM); err != nil {
		t.Fatalf("failed to send SIGTERM: %v", err)
	}

	// Wait for exit with a timeout.
	done := make(chan error, 1)
	go func() { done <- cmd.Wait() }()

	select {
	case err := <-done:
		// cmd.Wait() returns a non-nil error if the exit code is non-zero.
		// For graceful shutdown the exit code should be 0, so err should be nil.
		if err != nil {
			t.Errorf("parking-operator exited with error after SIGTERM: %v", err)
		}
	case <-time.After(10 * time.Second):
		cmd.Process.Kill()
		t.Fatal("parking-operator did not exit within 10s after SIGTERM")
	}
}
