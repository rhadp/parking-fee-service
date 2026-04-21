// Package main tests for cloud-gateway lifecycle.
package main

import (
	"encoding/json"
	"io"
	"os"
	"os/exec"
	"strings"
	"syscall"
	"testing"
	"time"
)

// TestCompiles verifies the package compiles.
func TestCompiles(t *testing.T) {
	// placeholder: verifies this package compiles successfully
}

// buildBinary compiles the cloud-gateway binary for subprocess tests.
// It returns the path to the temporary binary file and a cleanup function.
func buildBinary(t *testing.T) string {
	t.Helper()
	bin, err := os.CreateTemp("", "cloud-gateway-test-*")
	if err != nil {
		t.Fatalf("create temp binary: %v", err)
	}
	bin.Close()
	binPath := bin.Name()
	t.Cleanup(func() { os.Remove(binPath) })

	cmd := exec.Command("go", "build", "-o", binPath, ".")
	cmd.Dir = "."
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("build binary: %v\n%s", err, out)
	}
	return binPath
}

// writeTestConfig writes a temporary config.json and returns its path.
func writeTestConfig(t *testing.T, port int, natsURL string) string {
	t.Helper()
	type tokenMapping struct {
		Token string `json:"token"`
		VIN   string `json:"vin"`
	}
	type cfg struct {
		Port                  int            `json:"port"`
		NatsURL               string         `json:"nats_url"`
		CommandTimeoutSeconds int            `json:"command_timeout_seconds"`
		Tokens                []tokenMapping `json:"tokens"`
	}
	c := cfg{
		Port:                  port,
		NatsURL:               natsURL,
		CommandTimeoutSeconds: 30,
		Tokens: []tokenMapping{
			{Token: "demo-token-001", VIN: "VIN12345"},
		},
	}
	data, err := json.Marshal(c)
	if err != nil {
		t.Fatalf("marshal config: %v", err)
	}
	f, err := os.CreateTemp("", "cloud-gateway-config-*.json")
	if err != nil {
		t.Fatalf("create temp config: %v", err)
	}
	defer f.Close()
	if _, err := f.Write(data); err != nil {
		t.Fatalf("write config: %v", err)
	}
	t.Cleanup(func() { os.Remove(f.Name()) })
	return f.Name()
}

// TestStartupLogging verifies that on startup the service logs port, NATS URL, and token count (TS-06-15).
// The test starts the binary with a NATS URL that is not reachable so the process exits after
// connection failures, but the startup log lines appear before the NATS connect attempt.
func TestStartupLogging(t *testing.T) {
	binPath := buildBinary(t)
	cfgPath := writeTestConfig(t, 18081, "nats://localhost:19999")

	cmd := exec.Command(binPath)
	cmd.Env = append(os.Environ(), "CONFIG_PATH="+cfgPath)

	// Capture combined output via a pipe so we can read it before/after exit.
	pr, pw, err := os.Pipe()
	if err != nil {
		t.Fatalf("create pipe: %v", err)
	}
	cmd.Stdout = pw
	cmd.Stderr = pw

	if err := cmd.Start(); err != nil {
		pw.Close()
		pr.Close()
		t.Fatalf("start binary: %v", err)
	}

	// Close the write end in the parent so we can detect EOF.
	pw.Close()

	// Wait for the process to exit (it will fail to connect to NATS and exit 1).
	done := make(chan struct{})
	var output []byte
	go func() {
		defer close(done)
		output, _ = io.ReadAll(pr)
	}()

	_ = cmd.Wait()
	pr.Close()

	select {
	case <-done:
	case <-time.After(30 * time.Second):
		t.Fatal("timed out waiting for output")
	}

	out := string(output)
	t.Logf("startup output:\n%s", out)

	// Verify startup log contains port, NATS URL, and token count (06-REQ-8.1).
	if !strings.Contains(out, "18081") {
		t.Errorf("startup log missing port 18081; output:\n%s", out)
	}
	if !strings.Contains(out, "nats://") {
		t.Errorf("startup log missing nats:// URL; output:\n%s", out)
	}
	if !strings.Contains(out, "token") {
		t.Errorf("startup log missing token count; output:\n%s", out)
	}
}

// TestGracefulShutdown verifies that on SIGTERM the service exits with code 0 (TS-06-14).
// This test requires a running NATS server; it is skipped when NATS is unavailable.
func TestGracefulShutdown(t *testing.T) {
	// Check NATS availability by looking at the environment.
	natsURL := os.Getenv("NATS_URL")
	if natsURL == "" {
		natsURL = "nats://localhost:4222"
	}

	// Probe NATS with a single fast attempt; skip if unreachable.
	probeCmd := exec.Command("sh", "-c",
		"timeout 2 bash -c 'echo > /dev/tcp/localhost/4222' 2>/dev/null")
	if err := probeCmd.Run(); err != nil {
		t.Skip("NATS server not available, skipping TestGracefulShutdown")
	}

	binPath := buildBinary(t)
	cfgPath := writeTestConfig(t, 18082, natsURL)

	cmd := exec.Command(binPath)
	cmd.Env = append(os.Environ(), "CONFIG_PATH="+cfgPath)
	cmd.Stdout = io.Discard
	cmd.Stderr = io.Discard

	if err := cmd.Start(); err != nil {
		t.Fatalf("start binary: %v", err)
	}

	// Give the service time to start up and connect.
	time.Sleep(500 * time.Millisecond)

	// Send SIGTERM.
	if err := cmd.Process.Signal(syscall.SIGTERM); err != nil {
		t.Fatalf("send SIGTERM: %v", err)
	}

	// Wait for process to exit.
	exitCh := make(chan error, 1)
	go func() {
		exitCh <- cmd.Wait()
	}()

	select {
	case err := <-exitCh:
		if err != nil {
			t.Errorf("expected exit code 0, got: %v", err)
		}
	case <-time.After(15 * time.Second):
		_ = cmd.Process.Kill()
		t.Error("timed out waiting for graceful shutdown")
	}
}
