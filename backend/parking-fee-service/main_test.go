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

// buildBinary compiles the service binary and returns the path to it.
func buildBinary(t *testing.T) string {
	t.Helper()
	binPath := filepath.Join(t.TempDir(), "parking-fee-service")
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

// writeTestConfig writes a minimal config JSON file with the specified port
// and returns the file path.
func writeTestConfig(t *testing.T, port int) string {
	t.Helper()
	dir := t.TempDir()
	path := filepath.Join(dir, "config.json")
	data := fmt.Sprintf(`{
  "port": %d,
  "proximity_threshold_meters": 500,
  "zones": [
    {
      "id": "z1",
      "name": "Zone 1",
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
      "id": "op1",
      "name": "Operator 1",
      "zone_id": "z1",
      "rate": {"type": "per-hour", "amount": 1.0, "currency": "EUR"},
      "adapter": {"image_ref": "example.com/test:v1", "checksum_sha256": "sha256:abc", "version": "1.0.0"}
    }
  ]
}`, port)
	if err := os.WriteFile(path, []byte(data), 0644); err != nil {
		t.Fatalf("failed to write test config: %v", err)
	}
	return path
}

// ---------------------------------------------------------------------------
// TS-05-15: Startup Logging
// ---------------------------------------------------------------------------

func TestStartupLogging(t *testing.T) {
	binPath := buildBinary(t)
	port := getFreePort(t)
	configPath := writeTestConfig(t, port)

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
	if !strings.Contains(log, "zones") {
		t.Errorf("startup log does not contain 'zones':\n%s", log)
	}
	if !strings.Contains(log, "operators") {
		t.Errorf("startup log does not contain 'operators':\n%s", log)
	}
}

// ---------------------------------------------------------------------------
// TS-05-16: Graceful Shutdown
// ---------------------------------------------------------------------------

func TestGracefulShutdown(t *testing.T) {
	binPath := buildBinary(t)
	port := getFreePort(t)
	configPath := writeTestConfig(t, port)

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
