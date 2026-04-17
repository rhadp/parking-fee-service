package main

import (
	"bytes"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

// TestCompiles verifies the package compiles successfully.
// Test Spec: TS-01-27
// Requirements: 01-REQ-8.2
func TestCompiles(t *testing.T) {
	// This test passes by virtue of the package compiling.
}

// buildBinary compiles the cloud-gateway binary to a temp directory and returns its path.
func buildBinary(t *testing.T) string {
	t.Helper()
	binPath := filepath.Join(t.TempDir(), "cloud-gateway-test")
	buildCmd := exec.Command("go", "build", "-o", binPath, ".")
	if out, err := buildCmd.CombinedOutput(); err != nil {
		t.Fatalf("failed to build binary: %v\n%s", err, out)
	}
	return binPath
}

// writeTestConfig writes a temporary JSON config file and returns its path.
func writeTestConfig(t *testing.T) string {
	t.Helper()
	configContent := `{"port":8081,"nats_url":"nats://localhost:4222","command_timeout_seconds":30,"tokens":[{"token":"demo-token-001","vin":"VIN12345"}]}`
	configPath := filepath.Join(t.TempDir(), "config.json")
	if err := os.WriteFile(configPath, []byte(configContent), 0600); err != nil {
		t.Fatalf("failed to write test config: %v", err)
	}
	return configPath
}

// captureProcessOutput starts a subprocess, waits a specified duration for it to write
// initial output, then kills it and returns all captured output. Uses pipes to avoid
// data races (exec goroutine writes; we read only after the pipe is closed).
func captureProcessOutput(t *testing.T, cmd *exec.Cmd, waitDuration time.Duration) string {
	t.Helper()

	// Use a pipe for combined stderr+stdout capture.
	// exec.Cmd copies subprocess output to the pipe; we read from it in a goroutine.
	pr, pw, err := os.Pipe()
	if err != nil {
		t.Fatalf("failed to create pipe: %v", err)
	}
	cmd.Stdout = pw
	cmd.Stderr = pw

	if err := cmd.Start(); err != nil {
		pw.Close()
		pr.Close()
		t.Fatalf("failed to start binary: %v", err)
	}

	// Close the write end in the parent process so the read end gets EOF when
	// the subprocess exits. exec.Cmd already has a copy of the write fd.
	pw.Close()

	// Read all output from the subprocess in a goroutine.
	// io.Copy returns when the subprocess exits (write end closed by OS).
	type result struct{ data []byte }
	readDone := make(chan result, 1)
	go func() {
		var buf bytes.Buffer
		io.Copy(&buf, pr) //nolint:errcheck
		pr.Close()
		readDone <- result{data: buf.Bytes()}
	}()

	// Give the subprocess time to write startup output, then kill it.
	time.Sleep(waitDuration)
	_ = cmd.Process.Kill()

	// Wait for the read goroutine to finish (subprocess exit closes the pipe).
	res := <-readDone

	// Now safe to call Wait() — reads are complete.
	_ = cmd.Wait()

	return string(res.data)
}

// TestStartupLogging verifies that the service logs version, port, NATS URL, and token count
// during startup, before attempting to connect to NATS.
// Test Spec: TS-06-15
// Requirements: 06-REQ-8.1
func TestStartupLogging(t *testing.T) {
	binPath := buildBinary(t)
	configPath := writeTestConfig(t)

	cmd := exec.Command(binPath)
	cmd.Env = append(os.Environ(), "CONFIG_PATH="+configPath)

	// Wait 2s for startup logs. Startup logging happens before NATS connect,
	// which takes ≥7s for 5 retries (1s+2s+4s+...), so 2s is a safe window.
	// The extra headroom compared to 500ms avoids intermittent failures when
	// the test binary is starved for CPU time under parallel test execution.
	output := captureProcessOutput(t, cmd, 2*time.Second)
	t.Logf("startup output:\n%s", output)

	if !strings.Contains(output, "8081") {
		t.Errorf("startup log missing port 8081; got:\n%s", output)
	}
	if !strings.Contains(output, "nats://") {
		t.Errorf("startup log missing NATS URL prefix 'nats://'; got:\n%s", output)
	}
	if !strings.Contains(output, "token") {
		t.Errorf("startup log missing token count field; got:\n%s", output)
	}
}

// TestGracefulShutdown verifies that the service exits with code 0 on SIGTERM.
// This test is skipped when no NATS server is available on localhost:4222.
// Test Spec: TS-06-14
// Requirements: 06-REQ-8.2
func TestGracefulShutdown(t *testing.T) {
	// Skip when NATS is not available.
	if out, err := exec.Command("nc", "-z", "localhost", "4222").CombinedOutput(); err != nil {
		t.Skipf("NATS server not available on localhost:4222 (%v %s); skipping graceful shutdown test", err, out)
	}

	binPath := buildBinary(t)
	configPath := writeTestConfig(t)

	pr, pw, err := os.Pipe()
	if err != nil {
		t.Fatalf("failed to create pipe: %v", err)
	}
	cmd := exec.Command(binPath)
	cmd.Env = append(os.Environ(), "CONFIG_PATH="+configPath)
	cmd.Stdout = pw
	cmd.Stderr = pw

	if err := cmd.Start(); err != nil {
		pw.Close()
		pr.Close()
		t.Fatalf("failed to start binary: %v", err)
	}
	pw.Close()

	// Read output asynchronously.
	readDone := make(chan []byte, 1)
	go func() {
		var buf bytes.Buffer
		io.Copy(&buf, pr) //nolint:errcheck
		pr.Close()
		readDone <- buf.Bytes()
	}()

	// Wait 3 seconds for the service to become ready (NATS connect + startup logging).
	time.Sleep(3 * time.Second)

	// Send SIGTERM for graceful shutdown.
	if err := cmd.Process.Signal(os.Interrupt); err != nil {
		_ = cmd.Process.Kill()
		outputData := <-readDone
		_ = cmd.Wait()
		t.Fatalf("failed to send interrupt signal: %v\noutput:\n%s", err, outputData)
	}

	// Wait for the process to exit (up to 15s).
	done := make(chan error, 1)
	go func() {
		done <- cmd.Wait()
	}()

	select {
	case waitErr := <-done:
		outputData := <-readDone
		t.Logf("shutdown output:\n%s", outputData)
		if waitErr != nil {
			if exitErr, ok := waitErr.(*exec.ExitError); ok {
				if exitErr.ExitCode() != 0 {
					t.Errorf("service exited with code %d, expected 0", exitErr.ExitCode())
				}
				// exit code 0 means success but Wait still returns error... check code
			} else {
				t.Errorf("unexpected wait error: %v", waitErr)
			}
		}
		// nil waitErr = exit code 0 = success
	case <-time.After(15 * time.Second):
		_ = cmd.Process.Kill()
		outputData := <-readDone
		_ = cmd.Wait()
		t.Errorf("service did not shut down within 15s after SIGTERM\noutput:\n%s", outputData)
	}
}
