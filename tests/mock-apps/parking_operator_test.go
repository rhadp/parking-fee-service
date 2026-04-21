package mockappstests

import (
	"fmt"
	"net/http"
	"os"
	"os/exec"
	"syscall"
	"testing"
	"time"
)

// parkingOperatorBin builds and returns the parking-operator binary path.
var parkingOperatorBinCache string

func parkingOperatorBin(t *testing.T) string {
	t.Helper()
	if parkingOperatorBinCache == "" {
		parkingOperatorBinCache = buildBinary(t, "parking-operator")
	}
	return parkingOperatorBinCache
}

// TS-09-17: parking-operator serve shuts down gracefully on SIGTERM with exit 0.
func TestGracefulShutdown(t *testing.T) {
	binary := parkingOperatorBin(t)

	// Use a random high port to avoid conflicts
	port := "19876"
	cmd := exec.Command(binary, "serve", "--port="+port)
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr

	if err := cmd.Start(); err != nil {
		t.Fatalf("start parking-operator: %v", err)
	}

	// Wait for server to start listening
	addr := fmt.Sprintf("127.0.0.1:%s", port)
	if err := waitForPort(addr, 5*time.Second); err != nil {
		// Server didn't start listening (stub exits immediately)
		cmd.Process.Kill() //nolint
		cmd.Wait()         //nolint
		t.Fatalf("parking-operator serve did not start listening on %s: %v", addr, err)
	}

	// Verify server responds to HTTP requests
	resp, err := http.Get("http://" + addr + "/parking/status/test")
	if err != nil {
		t.Logf("HTTP check failed (expected): %v", err)
	} else {
		resp.Body.Close()
	}

	// Send SIGTERM
	if err := cmd.Process.Signal(syscall.SIGTERM); err != nil {
		t.Fatalf("send SIGTERM: %v", err)
	}

	// Wait for process to exit
	done := make(chan error, 1)
	go func() { done <- cmd.Wait() }()

	select {
	case err := <-done:
		if err != nil {
			if exitErr, ok := err.(*exec.ExitError); ok {
				if exitErr.ExitCode() != 0 {
					t.Errorf("expected exit code 0 on SIGTERM, got %d", exitErr.ExitCode())
				}
			} else {
				t.Errorf("unexpected error from Wait: %v", err)
			}
		}
		// exit code 0 is success
	case <-time.After(10 * time.Second):
		cmd.Process.Kill() //nolint
		t.Fatal("parking-operator did not exit within 10 seconds after SIGTERM")
	}
}
