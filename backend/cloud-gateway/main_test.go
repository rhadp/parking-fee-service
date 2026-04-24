package main

import (
	"bufio"
	"fmt"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"syscall"
	"testing"
	"time"
)

func TestCompiles(t *testing.T) {
	// Placeholder test: verifies the module compiles successfully.
}

// natsAvailable checks if a NATS server is reachable at the given address.
func natsAvailable(t *testing.T, addr string) bool {
	t.Helper()
	conn, err := net.DialTimeout("tcp", addr, 1*time.Second)
	if err != nil {
		return false
	}
	conn.Close()
	return true
}

// buildBinary compiles the service binary and returns the path to it.
func buildBinary(t *testing.T) string {
	t.Helper()
	binPath := filepath.Join(t.TempDir(), "cloud-gateway")
	cmd := exec.Command("go", "build", "-o", binPath, ".")
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	if err := cmd.Run(); err != nil {
		t.Fatalf("failed to build binary: %v", err)
	}
	return binPath
}

// getFreePort returns a free TCP port number.
func getFreePort(t *testing.T) int {
	t.Helper()
	ln, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to find free port: %v", err)
	}
	port := ln.Addr().(*net.TCPAddr).Port
	ln.Close()
	return port
}

// writeTestConfig writes a cloud-gateway config JSON file and returns the path.
func writeTestConfig(t *testing.T, port int, natsURL string, timeoutSecs int) string {
	t.Helper()
	dir := t.TempDir()
	path := filepath.Join(dir, "config.json")
	data := fmt.Sprintf(`{
  "port": %d,
  "nats_url": %q,
  "command_timeout_seconds": %d,
  "tokens": [
    {"token": "demo-token-001", "vin": "VIN12345"},
    {"token": "demo-token-002", "vin": "VIN67890"}
  ]
}`, port, natsURL, timeoutSecs)
	if err := os.WriteFile(path, []byte(data), 0644); err != nil {
		t.Fatalf("failed to write test config: %v", err)
	}
	return path
}

// ---------------------------------------------------------------------------
// TS-06-15: Startup Logging
// Requirement: 06-REQ-8.1
// Description: On startup, the service logs port, NATS URL, token count.
// ---------------------------------------------------------------------------

func TestStartupLogging(t *testing.T) {
	if !natsAvailable(t, "localhost:4222") {
		t.Skip("NATS not available at localhost:4222, skipping startup logging test")
	}

	binPath := buildBinary(t)
	port := getFreePort(t)
	configPath := writeTestConfig(t, port, "nats://localhost:4222", 30)

	cmd := exec.Command(binPath)
	cmd.Env = append(os.Environ(), "CONFIG_PATH="+configPath)

	stderrPipe, err := cmd.StderrPipe()
	if err != nil {
		t.Fatal(err)
	}

	if err := cmd.Start(); err != nil {
		t.Fatal(err)
	}

	// Continuously drain stderr; signal when the "ready" line is found.
	var output strings.Builder
	readyCh := make(chan struct{}, 1)
	scanDone := make(chan struct{})
	go func() {
		defer close(scanDone)
		scanner := bufio.NewScanner(stderrPipe)
		for scanner.Scan() {
			line := scanner.Text()
			output.WriteString(line + "\n")
			if strings.Contains(line, "ready") {
				select {
				case readyCh <- struct{}{}:
				default:
				}
			}
		}
	}()

	select {
	case <-readyCh:
	case <-time.After(15 * time.Second):
		cmd.Process.Kill()
		cmd.Wait()
		t.Fatal("timed out waiting for ready message")
	}

	// Stop the service and wait for all stderr to be captured.
	cmd.Process.Signal(syscall.SIGTERM)
	cmd.Wait()
	<-scanDone

	log := output.String()

	portStr := fmt.Sprintf("%d", port)
	if !strings.Contains(log, portStr) {
		t.Errorf("startup log does not contain port %s:\n%s", portStr, log)
	}
	if !strings.Contains(log, "nats://") {
		t.Errorf("startup log does not contain 'nats://':\n%s", log)
	}
	if !strings.Contains(log, "token") {
		t.Errorf("startup log does not contain 'token':\n%s", log)
	}
}

// ---------------------------------------------------------------------------
// TS-06-14: Graceful Shutdown
// Requirement: 06-REQ-8.2
// Description: On SIGTERM, the service drains NATS and exits with code 0.
// ---------------------------------------------------------------------------

func TestGracefulShutdown(t *testing.T) {
	if !natsAvailable(t, "localhost:4222") {
		t.Skip("NATS not available at localhost:4222, skipping graceful shutdown test")
	}

	binPath := buildBinary(t)
	port := getFreePort(t)
	configPath := writeTestConfig(t, port, "nats://localhost:4222", 30)

	cmd := exec.Command(binPath)
	cmd.Env = append(os.Environ(), "CONFIG_PATH="+configPath)

	stderrPipe, err := cmd.StderrPipe()
	if err != nil {
		t.Fatal(err)
	}

	if err := cmd.Start(); err != nil {
		t.Fatal(err)
	}

	// Drain stderr and wait for the ready message.
	readyCh := make(chan struct{}, 1)
	go func() {
		scanner := bufio.NewScanner(stderrPipe)
		for scanner.Scan() {
			if strings.Contains(scanner.Text(), "ready") {
				select {
				case readyCh <- struct{}{}:
				default:
				}
			}
		}
	}()

	select {
	case <-readyCh:
	case <-time.After(15 * time.Second):
		cmd.Process.Kill()
		cmd.Wait()
		t.Fatal("timed out waiting for ready message")
	}

	// Send SIGTERM for graceful shutdown.
	if err := cmd.Process.Signal(syscall.SIGTERM); err != nil {
		cmd.Process.Kill()
		t.Fatalf("failed to send SIGTERM: %v", err)
	}

	// Wait for the process to exit and check the exit code.
	exitCh := make(chan error, 1)
	go func() {
		exitCh <- cmd.Wait()
	}()

	select {
	case err := <-exitCh:
		if err != nil {
			t.Errorf("expected exit code 0 after SIGTERM, got: %v", err)
		}
	case <-time.After(15 * time.Second):
		cmd.Process.Kill()
		t.Fatal("timed out waiting for graceful shutdown")
	}
}
