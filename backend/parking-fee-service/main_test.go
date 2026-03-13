package main

import (
	"bytes"
	"fmt"
	"net/http"
	"os"
	"os/exec"
	"strings"
	"syscall"
	"testing"
	"time"
)

// TestParkingFeeServiceCompiles verifies the component compiles.
func TestParkingFeeServiceCompiles(t *testing.T) {
	t.Log("parking-fee-service compiles successfully")
}

// TS-05-15: On startup, the service logs version, port, zone count, operator count.
func TestStartupLogging(t *testing.T) {
	bin := buildBinary(t)

	cmd := exec.Command(bin)
	var out bytes.Buffer
	cmd.Stdout = &out
	cmd.Stderr = &out
	cmd.Env = append(os.Environ(), "CONFIG_PATH=/nonexistent/config.json")

	if err := cmd.Start(); err != nil {
		t.Fatalf("cmd.Start: %v", err)
	}

	// Give the service a moment to print startup logs.
	time.Sleep(300 * time.Millisecond)

	// Terminate the service gracefully.
	_ = cmd.Process.Signal(syscall.SIGTERM)
	done := make(chan error, 1)
	go func() { done <- cmd.Wait() }()

	select {
	case <-done:
	case <-time.After(5 * time.Second):
		_ = cmd.Process.Kill()
		t.Fatal("service did not exit within 5 seconds after SIGTERM")
	}

	output := out.String()
	t.Logf("startup output:\n%s", output)

	// The startup log must contain the port, zone count, and operator count.
	checks := []string{"8080", "zones", "operators"}
	for _, want := range checks {
		if !strings.Contains(output, want) {
			t.Errorf("startup log does not contain %q; full output:\n%s", want, output)
		}
	}
}

// TS-05-16: On SIGTERM, the service exits with code 0.
// This test first waits for the service to be reachable on its health endpoint
// before sending SIGTERM, confirming it actually started an HTTP server.
func TestGracefulShutdown(t *testing.T) {
	const port = "18080"
	bin := buildBinary(t)

	cmd := exec.Command(bin)
	cmd.Env = append(os.Environ(),
		"CONFIG_PATH=/nonexistent/config.json",
		"PORT="+port,
	)

	if err := cmd.Start(); err != nil {
		t.Fatalf("cmd.Start: %v", err)
	}

	// Wait for the health endpoint to respond (max 3 s).
	healthURL := fmt.Sprintf("http://localhost:%s/health", port)
	var ready bool
	deadline := time.Now().Add(3 * time.Second)
	for time.Now().Before(deadline) {
		resp, err := http.Get(healthURL)
		if err == nil {
			resp.Body.Close()
			if resp.StatusCode == http.StatusOK {
				ready = true
				break
			}
		}
		time.Sleep(50 * time.Millisecond)
	}
	if !ready {
		_ = cmd.Process.Kill()
		t.Fatal("service did not become ready on /health within 3 seconds")
	}

	if err := cmd.Process.Signal(syscall.SIGTERM); err != nil {
		t.Fatalf("Signal(SIGTERM): %v", err)
	}

	done := make(chan error, 1)
	go func() { done <- cmd.Wait() }()

	select {
	case err := <-done:
		if err != nil {
			if exitErr, ok := err.(*exec.ExitError); ok {
				t.Errorf("service exited with code %d, want 0", exitErr.ExitCode())
			} else {
				t.Errorf("cmd.Wait: %v", err)
			}
		}
	case <-time.After(5 * time.Second):
		_ = cmd.Process.Kill()
		t.Fatal("service did not exit within 5 seconds after SIGTERM")
	}
}

// buildBinary compiles the parking-fee-service binary and returns the path.
func buildBinary(t *testing.T) string {
	t.Helper()
	bin := t.TempDir() + "/parking-fee-service"
	cmd := exec.Command("go", "build", "-o", bin, ".")
	cmd.Dir = "."
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("go build failed: %v\n%s", err, out)
	}
	return bin
}
