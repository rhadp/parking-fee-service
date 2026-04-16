// Integration tests for parking-operator (subprocess / server-level tests).
//
// Unit-level handler tests live in mock/parking-operator/server_test.go.
// This file covers tests that require running the full server binary.
package integration

import (
	"fmt"
	"net"
	"os/exec"
	"strings"
	"syscall"
	"testing"
	"time"
)

const parkingOperatorPkg = "github.com/sdv-demo/mock/parking-operator"

// waitForPort polls addr until a TCP connection succeeds or timeout expires.
func waitForPort(addr string, timeout time.Duration) error {
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		conn, err := net.DialTimeout("tcp", addr, 100*time.Millisecond)
		if err == nil {
			conn.Close()
			return nil
		}
		time.Sleep(50 * time.Millisecond)
	}
	return fmt.Errorf("timeout waiting for %s", addr)
}

// ── TS-09-17: parking-operator graceful shutdown ──────────────────────────

// TestGracefulShutdown verifies that `parking-operator serve` starts on the
// requested port and exits 0 after receiving SIGTERM.
func TestGracefulShutdown(t *testing.T) {
	binary := buildBinary(t, parkingOperatorPkg)

	// Use a fixed high port to avoid conflicts; if busy, test will time out.
	port := "19998"
	addr := "127.0.0.1:" + port

	cmd := exec.Command(binary, "serve", "--port="+port)
	if err := cmd.Start(); err != nil {
		t.Fatalf("start binary: %v", err)
	}

	// Wait up to 5s for the server to be ready.
	if err := waitForPort(addr, 5*time.Second); err != nil {
		cmd.Process.Kill() //nolint
		t.Fatalf("server never started: %v", err)
	}

	// Send SIGTERM and wait for graceful exit.
	if err := cmd.Process.Signal(syscall.SIGTERM); err != nil {
		t.Fatalf("signal: %v", err)
	}
	done := make(chan error, 1)
	go func() { done <- cmd.Wait() }()

	select {
	case err := <-done:
		if err != nil {
			t.Fatalf("expected exit 0 after SIGTERM, got: %v", err)
		}
	case <-time.After(5 * time.Second):
		cmd.Process.Kill() //nolint
		t.Fatal("server did not exit within 5s after SIGTERM")
	}
}

// TestHTTPError verifies companion-app-cli exits 1 when CLOUD_GATEWAY returns
// non-2xx, printing the status to output (TS-09-E11 for companion-app-cli).
// Note: this test uses companion-app-cli binary against a 500-returning mock.
func TestHTTPError(t *testing.T) {
	ms := startMockHTTPServer(t, 500, `{"error":"gateway down"}`)
	binary := buildBinary(t, companionPkg)

	cmd := exec.Command(binary,
		"lock",
		"--vin=VIN001",
		"--token=test-token",
		"--gateway-addr="+ms.URL,
	)
	out, err := cmd.CombinedOutput()
	if err == nil {
		t.Fatalf("expected exit 1 for 500 response, got 0\noutput: %s", out)
	}
	if !strings.Contains(string(out), "500") {
		t.Fatalf("expected HTTP status 500 in output, got: %s", out)
	}
}
