// Package main contains tests for the parking-fee-service entry point.
package main

import (
	"bufio"
	"fmt"
	"io"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"syscall"
	"testing"
	"time"
)

// TestCompiles verifies the package compiles.
// Requirement: 01-REQ-8.2
func TestCompiles(t *testing.T) {
	// placeholder: verifies this package compiles successfully
}

// buildBinary compiles the main package to a temporary binary.
func buildBinary(t *testing.T) string {
	t.Helper()
	bin := filepath.Join(t.TempDir(), "parking-fee-service")
	cmd := exec.Command("go", "build", "-o", bin, ".")
	if out, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("go build failed: %v\n%s", err, out)
	}
	return bin
}

// freePort finds an available TCP port on localhost.
func freePort(t *testing.T) int {
	t.Helper()
	l, err := net.Listen("tcp", ":0")
	if err != nil {
		t.Fatal(err)
	}
	defer l.Close()
	return l.Addr().(*net.TCPAddr).Port
}

// testConfigJSON returns a minimal valid config JSON string with the given port.
func testConfigJSON(port int) string {
	return fmt.Sprintf(`{
		"port": %d,
		"proximity_threshold_meters": 500,
		"zones": [
			{
				"id": "munich-central",
				"name": "Munich Central Station Area",
				"polygon": [
					{"lat": 48.14,  "lon": 11.555},
					{"lat": 48.14,  "lon": 11.565},
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
				"rate": {"type": "per-hour", "amount": 2.50, "currency": "EUR"},
				"adapter": {
					"image_ref": "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0",
					"checksum_sha256": "sha256:abc123",
					"version": "1.0.0"
				}
			}
		]
	}`, port)
}

// writeTestConfig creates a temporary config file and returns its path.
func writeTestConfig(t *testing.T, port int) string {
	t.Helper()
	path := filepath.Join(t.TempDir(), "config.json")
	if err := os.WriteFile(path, []byte(testConfigJSON(port)), 0o644); err != nil {
		t.Fatal(err)
	}
	return path
}

// readUntilReady reads lines from r into a buffer until a line containing "ready"
// is found or the timeout expires. Returns collected output and any error.
// The goroutine this spawns exits when r is closed.
func readUntilReady(r io.Reader, timeout time.Duration) (string, error) {
	type result struct {
		output string
		err    error
	}
	ch := make(chan result, 1)
	go func() {
		var buf strings.Builder
		scanner := bufio.NewScanner(r)
		for scanner.Scan() {
			line := scanner.Text()
			buf.WriteString(line)
			buf.WriteByte('\n')
			if strings.Contains(line, "ready") {
				ch <- result{output: buf.String()}
				return
			}
		}
		ch <- result{output: buf.String(), err: fmt.Errorf("reader closed before ready")}
	}()

	select {
	case res := <-ch:
		return res.output, res.err
	case <-time.After(timeout):
		return "", fmt.Errorf("timed out waiting for ready message")
	}
}

// TestStartupLogging verifies that the service logs port, zone count, and
// operator count at startup.
// TS-05-15
func TestStartupLogging(t *testing.T) {
	bin := buildBinary(t)
	port := freePort(t)
	cfgPath := writeTestConfig(t, port)

	pr, pw, err := os.Pipe()
	if err != nil {
		t.Fatal(err)
	}

	cmd := exec.Command(bin)
	cmd.Env = append(os.Environ(), "CONFIG_PATH="+cfgPath)
	cmd.Stdout = pw
	cmd.Stderr = pw

	if err := cmd.Start(); err != nil {
		pw.Close()
		pr.Close()
		t.Fatal(err)
	}
	pw.Close() // parent does not write; close so child's EOF propagates.

	output, readErr := readUntilReady(pr, 5*time.Second)
	pr.Close() // stop background reader goroutine.

	// Clean up the process regardless of test outcome.
	cmd.Process.Signal(syscall.SIGTERM) //nolint:errcheck
	cmd.Wait()                          //nolint:errcheck

	if readErr != nil {
		t.Fatalf("service did not reach ready state: %v\noutput:\n%s", readErr, output)
	}

	portStr := fmt.Sprintf("%d", port)
	for _, want := range []string{portStr, "zones", "operators"} {
		if !strings.Contains(output, want) {
			t.Errorf("expected %q in startup output\nfull output:\n%s", want, output)
		}
	}
}

// TestGracefulShutdown verifies that the service exits cleanly (code 0) on SIGTERM.
// TS-05-16
func TestGracefulShutdown(t *testing.T) {
	bin := buildBinary(t)
	port := freePort(t)
	cfgPath := writeTestConfig(t, port)

	pr, pw, err := os.Pipe()
	if err != nil {
		t.Fatal(err)
	}

	cmd := exec.Command(bin)
	cmd.Env = append(os.Environ(), "CONFIG_PATH="+cfgPath)
	cmd.Stdout = pw
	cmd.Stderr = pw

	if err := cmd.Start(); err != nil {
		pw.Close()
		pr.Close()
		t.Fatal(err)
	}
	pw.Close() // parent does not write; close parent copy so child EOF propagates.

	output, readErr := readUntilReady(pr, 5*time.Second)
	// NOTE: do NOT close pr here — child still writes to its stdout/stderr (the
	// write end it inherited). Closing the read end before the child exits causes
	// SIGPIPE when the child next writes (e.g. "shutting down" log line).

	if readErr != nil {
		cmd.Process.Kill() //nolint:errcheck
		cmd.Wait()         //nolint:errcheck
		pr.Close()
		t.Fatalf("service did not reach ready state: %v\noutput:\n%s", readErr, output)
	}

	// Send SIGTERM and wait for a clean exit.
	if err := cmd.Process.Signal(syscall.SIGTERM); err != nil {
		pr.Close()
		t.Fatalf("failed to send SIGTERM: %v", err)
	}

	exitCh := make(chan error, 1)
	go func() { exitCh <- cmd.Wait() }()

	select {
	case exitErr := <-exitCh:
		pr.Close() // child done writing; now safe to close read end.
		if exitErr != nil {
			t.Errorf("expected exit code 0 after SIGTERM, got: %v", exitErr)
		}
	case <-time.After(10 * time.Second):
		cmd.Process.Kill() //nolint:errcheck
		cmd.Wait()         //nolint:errcheck
		pr.Close()
		t.Fatal("timed out waiting for graceful shutdown")
	}
}
