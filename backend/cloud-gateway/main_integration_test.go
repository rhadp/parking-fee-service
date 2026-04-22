//go:build integration

package main

import (
	"encoding/json"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"syscall"
	"testing"
	"time"
)

// buildBinary builds the cloud-gateway binary for testing and returns its path.
func buildBinary(t *testing.T) string {
	t.Helper()
	binPath := filepath.Join(t.TempDir(), "cloud-gateway")
	cmd := exec.Command("go", "build", "-o", binPath, ".")
	cmd.Dir = "."
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("failed to build binary: %v\n%s", err, out)
	}
	return binPath
}

// writeTestConfig writes a JSON config file and returns its path.
func writeTestConfig(t *testing.T, port int, timeoutSeconds int) string {
	t.Helper()
	cfg := map[string]any{
		"port":                    port,
		"nats_url":                "nats://localhost:4222",
		"command_timeout_seconds": timeoutSeconds,
		"tokens": []map[string]string{
			{"token": "demo-token-001", "vin": "VIN12345"},
			{"token": "demo-token-002", "vin": "VIN67890"},
		},
	}
	data, err := json.Marshal(cfg)
	if err != nil {
		t.Fatal(err)
	}
	cfgPath := filepath.Join(t.TempDir(), "config.json")
	if err := os.WriteFile(cfgPath, data, 0o644); err != nil {
		t.Fatal(err)
	}
	return cfgPath
}

// TS-06-15: Startup Logging
// Requirement: 06-REQ-8.1
// On startup, the service logs port, NATS URL, and token count.
func TestStartupLogging(t *testing.T) {
	binPath := buildBinary(t)
	cfgPath := writeTestConfig(t, 18081, 30)

	cmd := exec.Command(binPath)
	cmd.Env = append(os.Environ(), "CONFIG_PATH="+cfgPath)

	// Capture combined output (slog writes to stderr by default).
	var output strings.Builder
	cmd.Stdout = &output
	cmd.Stderr = &output

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start binary: %v", err)
	}

	// Give the service time to start and log.
	time.Sleep(2 * time.Second)

	// Send SIGTERM to shut down.
	if err := cmd.Process.Signal(syscall.SIGTERM); err != nil {
		t.Fatalf("failed to send SIGTERM: %v", err)
	}
	_ = cmd.Wait()

	logs := output.String()

	// Verify startup log contains port.
	if !strings.Contains(logs, "18081") {
		t.Errorf("startup logs do not contain port '18081':\n%s", logs)
	}

	// Verify startup log contains NATS URL.
	if !strings.Contains(logs, "nats://") {
		t.Errorf("startup logs do not contain 'nats://':\n%s", logs)
	}

	// Verify startup log contains token count reference.
	if !strings.Contains(logs, "token") {
		t.Errorf("startup logs do not contain 'token':\n%s", logs)
	}
}

// TS-06-14: Graceful Shutdown
// Requirement: 06-REQ-8.2
// On SIGTERM, the service drains NATS and exits with code 0.
func TestGracefulShutdown(t *testing.T) {
	binPath := buildBinary(t)
	cfgPath := writeTestConfig(t, 18082, 30)

	cmd := exec.Command(binPath)
	cmd.Env = append(os.Environ(), "CONFIG_PATH="+cfgPath)

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start binary: %v", err)
	}

	// Give the service time to start.
	time.Sleep(2 * time.Second)

	// Send SIGTERM.
	if err := cmd.Process.Signal(syscall.SIGTERM); err != nil {
		t.Fatalf("failed to send SIGTERM: %v", err)
	}

	// Wait for process to exit.
	err := cmd.Wait()
	if err != nil {
		t.Errorf("expected clean exit (code 0), got error: %v", err)
	}
}
