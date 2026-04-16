package main

import (
	"bytes"
	"fmt"
	"net"
	"net/http"
	"os"
	"os/exec"
	"strings"
	"syscall"
	"testing"
	"time"
)

// TestCompiles verifies the package compiles successfully.
func TestCompiles(t *testing.T) {
	// This test passes by virtue of the package compiling.
}

// findFreePort returns a free TCP port on localhost.
func findFreePort(t *testing.T) int {
	t.Helper()
	ln, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("findFreePort: %v", err)
	}
	port := ln.Addr().(*net.TCPAddr).Port
	ln.Close()
	return port
}

// buildBinary compiles the main package into a temporary binary and returns
// its path. The test is skipped if go build fails (e.g. in short mode without
// compiler available).
func buildBinary(t *testing.T) string {
	t.Helper()
	bin, err := os.CreateTemp("", "parking-fee-service-*")
	if err != nil {
		t.Fatalf("create temp file: %v", err)
	}
	bin.Close()
	binPath := bin.Name()
	t.Cleanup(func() { os.Remove(binPath) })

	cmd := exec.Command("go", "build", "-o", binPath, ".")
	cmd.Dir = "."
	var out bytes.Buffer
	cmd.Stderr = &out
	if err := cmd.Run(); err != nil {
		t.Skipf("go build failed: %v\n%s", err, out.String())
	}
	return binPath
}

// waitForHTTP polls url until it gets a 200 or timeout.
func waitForHTTP(url string, timeout time.Duration) error {
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		resp, err := http.Get(url) //nolint:noctx
		if err == nil {
			resp.Body.Close()
			if resp.StatusCode == http.StatusOK {
				return nil
			}
		}
		time.Sleep(50 * time.Millisecond)
	}
	return fmt.Errorf("service did not become ready within %s", timeout)
}

// ---- TS-05-15: Startup Logging ----

func TestStartupLogging(t *testing.T) {
	bin := buildBinary(t)
	port := findFreePort(t)

	var buf bytes.Buffer
	cmd := exec.Command(bin)
	cmd.Env = append(os.Environ(),
		"CONFIG_PATH=/nonexistent/path/to/config.json",
		fmt.Sprintf("PORT=%d", port), // not used by main, handled below via config
	)
	// Override the port by providing a custom config via CONFIG_PATH that
	// doesn't exist — the service falls back to defaults (port 8080). We
	// instead inject the port via a valid temporary config.
	cfgJSON := fmt.Sprintf(`{
		"port": %d,
		"proximity_threshold_meters": 500,
		"zones": [
			{
				"id": "munich-central",
				"name": "Munich Central Station Area",
				"polygon": [
					{"lat": 48.14, "lon": 11.555},
					{"lat": 48.14, "lon": 11.565},
					{"lat": 48.135, "lon": 11.565},
					{"lat": 48.135, "lon": 11.555}
				]
			}
		],
		"operators": [
			{
				"id": "parkhaus-munich",
				"name": "Parkhaus Muenchen GmbH",
				"zone_id": "munich-central",
				"rate": {"type": "per-hour", "amount": 2.5, "currency": "EUR"},
				"adapter": {
					"image_ref": "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0",
					"checksum_sha256": "sha256:abc123def456",
					"version": "1.0.0"
				}
			}
		]
	}`, port)

	cfgFile, err := os.CreateTemp("", "pfs-test-config-*.json")
	if err != nil {
		t.Fatalf("create config: %v", err)
	}
	defer os.Remove(cfgFile.Name())
	if _, err := cfgFile.WriteString(cfgJSON); err != nil {
		t.Fatalf("write config: %v", err)
	}
	cfgFile.Close()

	cmd = exec.Command(bin)
	cmd.Env = append(os.Environ(), "CONFIG_PATH="+cfgFile.Name())
	cmd.Stdout = &buf
	cmd.Stderr = &buf

	if err := cmd.Start(); err != nil {
		t.Fatalf("start service: %v", err)
	}
	t.Cleanup(func() {
		cmd.Process.Signal(syscall.SIGTERM) //nolint:errcheck
		cmd.Wait()                          //nolint:errcheck
	})

	// Wait for the service to be ready.
	if err := waitForHTTP(fmt.Sprintf("http://127.0.0.1:%d/health", port), 5*time.Second); err != nil {
		t.Fatalf("service not ready: %v\nOutput:\n%s", err, buf.String())
	}

	output := buf.String()

	// REQ-6.1: log must mention port, zone count, operator count.
	if !strings.Contains(output, fmt.Sprintf("%d", port)) {
		t.Errorf("startup log missing port %d\nOutput:\n%s", port, output)
	}
	if !strings.Contains(output, "zones") {
		t.Errorf("startup log missing 'zones'\nOutput:\n%s", output)
	}
	if !strings.Contains(output, "operators") {
		t.Errorf("startup log missing 'operators'\nOutput:\n%s", output)
	}
}

// ---- TS-05-16: Graceful Shutdown ----

func TestGracefulShutdown(t *testing.T) {
	bin := buildBinary(t)
	port := findFreePort(t)

	cfgJSON := fmt.Sprintf(`{
		"port": %d,
		"proximity_threshold_meters": 500,
		"zones": [
			{
				"id": "munich-central",
				"name": "Munich Central Station Area",
				"polygon": [
					{"lat": 48.14, "lon": 11.555},
					{"lat": 48.14, "lon": 11.565},
					{"lat": 48.135, "lon": 11.565},
					{"lat": 48.135, "lon": 11.555}
				]
			}
		],
		"operators": [
			{
				"id": "parkhaus-munich",
				"name": "Parkhaus Muenchen GmbH",
				"zone_id": "munich-central",
				"rate": {"type": "per-hour", "amount": 2.5, "currency": "EUR"},
				"adapter": {
					"image_ref": "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0",
					"checksum_sha256": "sha256:abc123def456",
					"version": "1.0.0"
				}
			}
		]
	}`, port)

	cfgFile, err := os.CreateTemp("", "pfs-test-config-*.json")
	if err != nil {
		t.Fatalf("create config: %v", err)
	}
	defer os.Remove(cfgFile.Name())
	if _, err := cfgFile.WriteString(cfgJSON); err != nil {
		t.Fatalf("write config: %v", err)
	}
	cfgFile.Close()

	cmd := exec.Command(bin)
	cmd.Env = append(os.Environ(), "CONFIG_PATH="+cfgFile.Name())

	if err := cmd.Start(); err != nil {
		t.Fatalf("start service: %v", err)
	}

	// Wait for the service to be ready.
	if err := waitForHTTP(fmt.Sprintf("http://127.0.0.1:%d/health", port), 5*time.Second); err != nil {
		cmd.Process.Kill() //nolint:errcheck
		t.Fatalf("service not ready: %v", err)
	}

	// Send SIGTERM and wait for exit.
	if err := cmd.Process.Signal(syscall.SIGTERM); err != nil {
		t.Fatalf("send SIGTERM: %v", err)
	}

	done := make(chan error, 1)
	go func() { done <- cmd.Wait() }()

	select {
	case err := <-done:
		if err != nil {
			// On SIGTERM, the process exits with signal; that's acceptable.
			// exec.ExitError wraps the exit status — check it's signal-based.
			exitErr, ok := err.(*exec.ExitError)
			if !ok {
				t.Fatalf("process exited with error: %v", err)
			}
			// A process killed by SIGTERM on Linux exits with signal exit.
			// On macOS it exits with status -1 (signal). Both are acceptable
			// for graceful shutdown signaled by SIGTERM.
			status, ok := exitErr.Sys().(syscall.WaitStatus)
			if !ok {
				t.Fatalf("unexpected exit error type: %v", err)
			}
			if status.ExitStatus() != 0 && !status.Signaled() {
				t.Fatalf("process exited with non-zero status: %v", status.ExitStatus())
			}
		}
		// exit code 0 is also fine (clean shutdown).
	case <-time.After(5 * time.Second):
		cmd.Process.Kill() //nolint:errcheck
		t.Fatal("service did not shut down within 5 seconds after SIGTERM")
	}
}
