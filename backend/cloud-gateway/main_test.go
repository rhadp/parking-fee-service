package main

import (
	"bytes"
	"fmt"
	"log"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
	"sync"
	"syscall"
	"testing"
	"time"
)

// binaryPath is the path to the compiled cloud-gateway binary used by subprocess tests.
var binaryPath string

// TestMain builds the cloud-gateway binary once and runs all tests.
// This enables TS-06-14 and TS-06-15 to start the service as a real subprocess.
func TestMain(m *testing.M) {
	tmpDir, err := os.MkdirTemp("", "cloud-gateway-bin-*")
	if err != nil {
		log.Fatalf("create temp dir: %v", err)
	}
	defer os.RemoveAll(tmpDir)

	binaryPath = filepath.Join(tmpDir, "cloud-gateway")
	if runtime.GOOS == "windows" {
		binaryPath += ".exe"
	}

	build := exec.Command("go", "build", "-o", binaryPath, ".")
	build.Stdout = os.Stderr
	build.Stderr = os.Stderr
	if err := build.Run(); err != nil {
		log.Fatalf("build cloud-gateway binary: %v", err)
	}

	os.Exit(m.Run())
}

// safeBuf is a thread-safe bytes.Buffer suitable for capturing subprocess output
// while being read concurrently from the test goroutine.
type safeBuf struct {
	mu  sync.Mutex
	buf bytes.Buffer
}

// Write implements io.Writer in a thread-safe way.
func (s *safeBuf) Write(p []byte) (n int, err error) {
	s.mu.Lock()
	defer s.mu.Unlock()
	return s.buf.Write(p)
}

// String returns the accumulated output as a string in a thread-safe way.
func (s *safeBuf) String() string {
	s.mu.Lock()
	defer s.mu.Unlock()
	return s.buf.String()
}

// checkNATSAvailable tries a TCP dial to localhost:4222.
// Returns true if a NATS server is accepting connections.
func checkNATSAvailable() bool {
	conn, err := net.DialTimeout("tcp", "localhost:4222", 500*time.Millisecond)
	if err != nil {
		return false
	}
	conn.Close()
	return true
}

// writeTestConfig writes a minimal JSON config to a temporary file and returns the path.
// port is the HTTP listen port embedded in the config.
func writeTestConfig(t *testing.T, port int) string {
	t.Helper()
	content := fmt.Sprintf(`{
  "port": %d,
  "nats_url": "nats://localhost:4222",
  "command_timeout_seconds": 30,
  "tokens": [
    {"token": "test-token-001", "vin": "TESTVIN1"},
    {"token": "test-token-002", "vin": "TESTVIN2"}
  ]
}`, port)

	path := filepath.Join(t.TempDir(), "config.json")
	if err := os.WriteFile(path, []byte(content), 0644); err != nil {
		t.Fatalf("write test config: %v", err)
	}
	return path
}

// waitForOutput polls buf until it contains substr or the deadline is exceeded.
// Returns true if substr was found within the deadline.
func waitForOutput(buf *safeBuf, substr string, deadline time.Duration) bool {
	end := time.Now().Add(deadline)
	for time.Now().Before(end) {
		if strings.Contains(buf.String(), substr) {
			return true
		}
		time.Sleep(50 * time.Millisecond)
	}
	return false
}

// TestCompiles verifies the package compiles successfully (covered by TestMain build step).
func TestCompiles(t *testing.T) {
	// Compilation success is verified by the TestMain go build step.
}

// TestStartupLogging verifies that on startup the service logs its port, NATS URL,
// and token count before accepting requests.
// TS-06-15
func TestStartupLogging(t *testing.T) {
	if !checkNATSAvailable() {
		t.Skip("NATS not available on localhost:4222; skipping startup logging test")
	}

	cfgPath := writeTestConfig(t, 18081)

	var out safeBuf
	cmd := exec.Command(binaryPath)
	cmd.Env = append(os.Environ(), "CONFIG_PATH="+cfgPath)
	cmd.Stdout = &out
	cmd.Stderr = &out

	if err := cmd.Start(); err != nil {
		t.Fatalf("start service: %v", err)
	}
	defer cmd.Process.Kill() //nolint:errcheck

	// Wait for the "ready" message that appears once the HTTP server is listening.
	if !waitForOutput(&out, "ready", 10*time.Second) {
		t.Fatalf("service did not log 'ready' within timeout; output:\n%s", out.String())
	}

	output := out.String()

	// Verify port number appears in logs (06-REQ-8.1).
	if !strings.Contains(output, "18081") {
		t.Errorf("startup log missing port 18081; got:\n%s", output)
	}

	// Verify NATS URL appears in logs (06-REQ-8.1).
	if !strings.Contains(output, "nats://") {
		t.Errorf("startup log missing NATS URL; got:\n%s", output)
	}

	// Verify token count appears in logs (06-REQ-8.1).
	if !strings.Contains(output, "token") {
		t.Errorf("startup log missing token count; got:\n%s", output)
	}
}

// TestGracefulShutdown verifies that on SIGTERM the service drains NATS and exits
// with code 0.
// TS-06-14
func TestGracefulShutdown(t *testing.T) {
	if !checkNATSAvailable() {
		t.Skip("NATS not available on localhost:4222; skipping graceful shutdown test")
	}

	cfgPath := writeTestConfig(t, 18082)

	var out safeBuf
	cmd := exec.Command(binaryPath)
	cmd.Env = append(os.Environ(), "CONFIG_PATH="+cfgPath)
	cmd.Stdout = &out
	cmd.Stderr = &out

	if err := cmd.Start(); err != nil {
		t.Fatalf("start service: %v", err)
	}

	// Wait for the service to be fully started before sending SIGTERM.
	if !waitForOutput(&out, "ready", 10*time.Second) {
		cmd.Process.Kill() //nolint:errcheck
		t.Fatalf("service did not log 'ready' within timeout; output:\n%s", out.String())
	}

	// Send SIGTERM and wait for the process to exit.
	if err := cmd.Process.Signal(syscall.SIGTERM); err != nil {
		cmd.Process.Kill() //nolint:errcheck
		t.Fatalf("send SIGTERM: %v", err)
	}

	// Give the service up to 15 seconds to complete its graceful shutdown.
	done := make(chan error, 1)
	go func() {
		done <- cmd.Wait()
	}()

	select {
	case err := <-done:
		if err != nil {
			if exitErr, ok := err.(*exec.ExitError); ok {
				t.Errorf("exit code: want 0, got %d; output:\n%s", exitErr.ExitCode(), out.String())
			} else {
				t.Errorf("wait error: %v; output:\n%s", err, out.String())
			}
		}
		// err == nil means exit code 0: success.
	case <-time.After(15 * time.Second):
		cmd.Process.Kill() //nolint:errcheck
		t.Errorf("service did not exit within 15 seconds after SIGTERM; output:\n%s", out.String())
	}
}
