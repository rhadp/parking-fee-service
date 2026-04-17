package main

import (
	"bytes"
	"context"
	"fmt"
	"io"
	"net"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
	"syscall"
	"testing"
	"time"
)

// TestCompiles verifies the package compiles successfully (01-REQ-8.2).
func TestCompiles(t *testing.T) {
	// Compilation success is sufficient for this placeholder test.
}

// buildBinary compiles the parking-fee-service binary into a temporary
// directory and returns the binary path.
func buildBinary(t *testing.T) string {
	t.Helper()
	dir := t.TempDir()
	binPath := filepath.Join(dir, "parking-fee-service")
	if runtime.GOOS == "windows" {
		binPath += ".exe"
	}
	// Locate the module root (directory containing this file).
	_, currentFile, _, ok := runtime.Caller(0)
	if !ok {
		t.Fatal("could not determine source file path")
	}
	moduleDir := filepath.Dir(currentFile)

	cmd := exec.Command("go", "build", "-o", binPath, ".")
	cmd.Dir = moduleDir
	var stderr bytes.Buffer
	cmd.Stderr = &stderr
	if err := cmd.Run(); err != nil {
		t.Fatalf("go build failed: %v\nstderr: %s", err, stderr.String())
	}
	return binPath
}

// freePort returns an unused TCP port on localhost.
func freePort(t *testing.T) int {
	t.Helper()
	l, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("freePort: %v", err)
	}
	port := l.Addr().(*net.TCPAddr).Port
	l.Close()
	return port
}

// waitForReady polls /health until it returns 200 or the context expires.
func waitForReady(ctx context.Context, addr string) error {
	url := "http://" + addr + "/health"
	for {
		select {
		case <-ctx.Done():
			return fmt.Errorf("service at %s did not become ready: %w", url, ctx.Err())
		default:
			resp, err := http.Get(url) //nolint:noctx
			if err == nil && resp.StatusCode == http.StatusOK {
				resp.Body.Close()
				return nil
			}
			if resp != nil {
				resp.Body.Close()
			}
			time.Sleep(50 * time.Millisecond)
		}
	}
}

// TS-05-15: On startup the service logs version, port, zone count, operator
// count, and a ready message.
func TestStartupLogging(t *testing.T) {
	binPath := buildBinary(t)
	port := freePort(t)

	// Write a temporary config so the port is deterministic.
	cfgJSON := fmt.Sprintf(`{
		"port": %d,
		"proximity_threshold_meters": 500,
		"zones": [
			{
				"id": "z1",
				"name": "Zone One",
				"polygon": [
					{"lat": 48.14, "lon": 11.55},
					{"lat": 48.14, "lon": 11.56},
					{"lat": 48.13, "lon": 11.56},
					{"lat": 48.13, "lon": 11.55}
				]
			}
		],
		"operators": [
			{
				"id": "op1",
				"name": "Op One",
				"zone_id": "z1",
				"rate": {"type": "per-hour", "amount": 2.5, "currency": "EUR"},
				"adapter": {
					"image_ref": "registry/op1:v1",
					"checksum_sha256": "sha256:abc",
					"version": "1.0.0"
				}
			}
		]
	}`, port)

	cfgFile := filepath.Join(t.TempDir(), "config.json")
	if err := os.WriteFile(cfgFile, []byte(cfgJSON), 0o644); err != nil {
		t.Fatalf("writing config file: %v", err)
	}

	cmd := exec.Command(binPath)
	cmd.Env = append(os.Environ(), "CONFIG_PATH="+cfgFile)
	var output bytes.Buffer
	cmd.Stdout = io.MultiWriter(&output, os.Stdout)
	cmd.Stderr = io.MultiWriter(&output, os.Stderr)

	if err := cmd.Start(); err != nil {
		t.Fatalf("starting service: %v", err)
	}
	t.Cleanup(func() {
		if cmd.Process != nil {
			_ = cmd.Process.Signal(syscall.SIGTERM)
			_ = cmd.Wait()
		}
	})

	addr := fmt.Sprintf("127.0.0.1:%d", port)
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	if err := waitForReady(ctx, addr); err != nil {
		t.Fatalf("service did not start: %v\noutput: %s", err, output.String())
	}

	// Terminate cleanly before inspecting logs.
	_ = cmd.Process.Signal(syscall.SIGTERM)
	_ = cmd.Wait()

	logs := output.String()
	portStr := fmt.Sprintf("%d", port)
	if !strings.Contains(logs, portStr) {
		t.Errorf("startup logs do not contain port %q\nlogs:\n%s", portStr, logs)
	}
	if !strings.Contains(logs, "zone") {
		t.Errorf("startup logs do not mention 'zone'\nlogs:\n%s", logs)
	}
	if !strings.Contains(logs, "operator") {
		t.Errorf("startup logs do not mention 'operator'\nlogs:\n%s", logs)
	}
}

// TS-05-16: On SIGTERM, the service gracefully shuts down and exits with
// code 0.
func TestGracefulShutdown(t *testing.T) {
	binPath := buildBinary(t)
	port := freePort(t)

	cfgJSON := fmt.Sprintf(`{
		"port": %d,
		"proximity_threshold_meters": 500,
		"zones": [
			{
				"id": "z1",
				"name": "Zone One",
				"polygon": [
					{"lat": 48.14, "lon": 11.55},
					{"lat": 48.14, "lon": 11.56},
					{"lat": 48.13, "lon": 11.56},
					{"lat": 48.13, "lon": 11.55}
				]
			}
		],
		"operators": [
			{
				"id": "op1",
				"name": "Op One",
				"zone_id": "z1",
				"rate": {"type": "per-hour", "amount": 2.5, "currency": "EUR"},
				"adapter": {
					"image_ref": "registry/op1:v1",
					"checksum_sha256": "sha256:abc",
					"version": "1.0.0"
				}
			}
		]
	}`, port)

	cfgFile := filepath.Join(t.TempDir(), "config.json")
	if err := os.WriteFile(cfgFile, []byte(cfgJSON), 0o644); err != nil {
		t.Fatalf("writing config file: %v", err)
	}

	cmd := exec.Command(binPath)
	cmd.Env = append(os.Environ(), "CONFIG_PATH="+cfgFile)
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr

	if err := cmd.Start(); err != nil {
		t.Fatalf("starting service: %v", err)
	}

	addr := fmt.Sprintf("127.0.0.1:%d", port)
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	if err := waitForReady(ctx, addr); err != nil {
		_ = cmd.Process.Signal(syscall.SIGKILL)
		_ = cmd.Wait()
		t.Fatalf("service did not start: %v", err)
	}

	// Send SIGTERM and wait for exit.
	if err := cmd.Process.Signal(syscall.SIGTERM); err != nil {
		t.Fatalf("sending SIGTERM: %v", err)
	}

	done := make(chan error, 1)
	go func() { done <- cmd.Wait() }()

	select {
	case err := <-done:
		if err != nil {
			// On Unix, a graceful exit(0) results in nil from cmd.Wait().
			// Any non-nil error means non-zero exit.
			t.Errorf("service exited with error (want exit code 0): %v", err)
		}
	case <-time.After(5 * time.Second):
		_ = cmd.Process.Signal(syscall.SIGKILL)
		t.Error("service did not exit within 5s after SIGTERM")
	}
}
