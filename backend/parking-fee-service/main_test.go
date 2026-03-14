package main

import (
	"bytes"
	"fmt"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"syscall"
	"testing"
	"time"
)

// preBuiltBinary is the path to the binary compiled once in TestMain,
// shared across all tests to avoid concurrent go-build races when
// sub-packages run in parallel under go test -p N.
var preBuiltBinary string

// TestMain compiles the service binary once before running any test,
// so that buildBinary() never races with the go test framework's own
// compilation of sub-packages.
func TestMain(m *testing.M) {
	tmp, err := os.MkdirTemp("", "pfs-test-binary-*")
	if err != nil {
		fmt.Fprintf(os.Stderr, "TestMain: MkdirTemp: %v\n", err)
		os.Exit(1)
	}
	defer os.RemoveAll(tmp)

	bin := filepath.Join(tmp, "parking-fee-service")
	out, err := exec.Command("go", "build", "-o", bin, ".").CombinedOutput()
	if err != nil {
		fmt.Fprintf(os.Stderr, "TestMain: go build: %v\n%s", err, out)
		os.Exit(1)
	}
	preBuiltBinary = bin

	os.Exit(m.Run())
}

// TestParkingFeeServiceCompiles verifies the component compiles.
func TestParkingFeeServiceCompiles(t *testing.T) {
	t.Log("parking-fee-service compiles successfully")
}

// TS-05-15: On startup, the service logs version, port, zone count, operator count.
func TestStartupLogging(t *testing.T) {
	// Use a non-default port to avoid conflicts with concurrent service instances.
	const port = "18081"

	bin := buildBinary(t)

	cmd := exec.Command(bin)
	var out bytes.Buffer
	cmd.Stdout = &out
	cmd.Stderr = &out
	cmd.Env = append(os.Environ(),
		"CONFIG_PATH=/nonexistent/config.json",
		"PORT="+port,
	)

	if err := cmd.Start(); err != nil {
		t.Fatalf("cmd.Start: %v", err)
	}

	// Poll the health endpoint rather than sleeping a fixed amount.  Under
	// high CPU load (e.g. many parallel test packages) the subprocess may not
	// be scheduled quickly enough for a fixed 300 ms sleep to be reliable.
	healthURL := fmt.Sprintf("http://localhost:%s/health", port)
	var ready bool
	deadline := time.Now().Add(5 * time.Second)
	for time.Now().Before(deadline) {
		resp, err := http.Get(healthURL)
		if err == nil {
			resp.Body.Close()
			if resp.StatusCode == http.StatusOK {
				ready = true
				break
			}
		}
		time.Sleep(10 * time.Millisecond)
	}
	if !ready {
		_ = cmd.Process.Kill()
		_ = cmd.Wait()
		t.Fatal("service did not become ready within 5 seconds")
	}

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
	checks := []string{port, "zones", "operators"}
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

// buildBinary returns the path to the service binary compiled once in TestMain.
// Using a pre-built binary avoids a race between this test's go build call and
// the concurrent compilation of sub-packages under go test -p N.
func buildBinary(t *testing.T) string {
	t.Helper()
	if preBuiltBinary == "" {
		t.Fatal("preBuiltBinary is empty — did TestMain run?")
	}
	return preBuiltBinary
}
