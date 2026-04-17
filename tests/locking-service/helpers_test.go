// Package lockingservice_test contains integration tests for the LOCKING_SERVICE
// component (spec 03_locking_service).
//
// These tests start the locking-service binary against a real DATA_BROKER
// container and verify end-to-end command processing behaviours via grpcurl.
//
// All live tests skip automatically when:
//   - podman (or podman-compose) is not installed
//   - grpcurl is not on PATH
//   - port 55556 is not reachable (DATA_BROKER not running)
//
// Run integration tests only (requires Podman + grpcurl):
//
//	cd tests/locking-service && go test -v ./...
//
// Test Specs: TS-03-1, TS-03-13, TS-03-E1, TS-03-SMOKE-1, TS-03-SMOKE-2,
//
//	TS-03-SMOKE-3
package lockingservice_test

import (
	"bufio"
	"fmt"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
	"sync"
	"testing"
	"time"
)

// ── Repo root ──────────────────────────────────────────────────────────────

// repoRoot returns the absolute path to the repository root.
// Navigates three levels up from this file: tests/locking-service/ -> tests/ -> root.
func repoRoot(t *testing.T) string {
	t.Helper()
	_, filename, _, ok := runtime.Caller(0)
	if !ok {
		t.Fatal("runtime.Caller failed")
	}
	root := filepath.Join(filepath.Dir(filename), "..", "..")
	abs, err := filepath.Abs(root)
	if err != nil {
		t.Fatalf("filepath.Abs failed: %v", err)
	}
	return abs
}

// ── Skip conditions ────────────────────────────────────────────────────────

// requireGrpcurl skips if grpcurl is not on PATH.
func requireGrpcurl(t *testing.T) {
	t.Helper()
	if _, err := exec.LookPath("grpcurl"); err != nil {
		t.Skip("grpcurl not found on PATH; skipping live gRPC test")
	}
}

// requirePodman skips if podman is not installed or its socket is not reachable.
func requirePodman(t *testing.T) {
	t.Helper()
	if _, err := exec.LookPath("podman"); err != nil {
		if _, err2 := exec.LookPath("podman-compose"); err2 != nil {
			t.Skip("podman not available; skipping container test")
		}
	}
	cmd := exec.Command("podman", "info", "--format", "{{.Host.Arch}}")
	if err := cmd.Run(); err != nil {
		t.Skipf("podman socket not reachable (Podman machine may be stopped): %v", err)
	}
}

// requireDatabrokerTCP skips if the DATA_BROKER is not reachable on port 55556.
func requireDatabrokerTCP(t *testing.T) {
	t.Helper()
	requireGrpcurl(t)
	conn, err := net.DialTimeout("tcp", "localhost:55556", 2*time.Second)
	if err != nil {
		t.Skipf("DATA_BROKER not reachable on localhost:55556 (%v); skipping live test", err)
	}
	conn.Close()
}

// ── Binary build ───────────────────────────────────────────────────────────

// buildLockingService builds the locking-service Rust binary and returns its path.
// The binary is placed in the shared Rust workspace target directory
// (rhivos/target/debug/locking-service) since all crates share one target dir.
func buildLockingService(t *testing.T) string {
	t.Helper()
	root := repoRoot(t)
	// The workspace root for Rust is rhivos/; cargo build runs from there.
	workspaceDir := filepath.Join(root, "rhivos")

	cmd := exec.Command("cargo", "build", "--bin", "locking-service")
	cmd.Dir = workspaceDir
	if out, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("cargo build locking-service: %v\n%s", err, out)
	}

	// Shared workspace target: rhivos/target/debug/locking-service
	binPath := filepath.Join(workspaceDir, "target", "debug", "locking-service")
	if _, err := os.Stat(binPath); err != nil {
		t.Fatalf("locking-service binary not found at %s after build: %v", binPath, err)
	}
	return binPath
}

// ── Container lifecycle ────────────────────────────────────────────────────

// podmanCompose runs a podman compose command in the deployments directory
// and returns combined output and any error.
func podmanCompose(t *testing.T, args ...string) (string, error) {
	t.Helper()
	root := repoRoot(t)
	deploymentsDir := filepath.Join(root, "deployments")

	var cmd *exec.Cmd
	if _, err := exec.LookPath("podman"); err == nil {
		fullArgs := append([]string{"compose"}, args...)
		cmd = exec.Command("podman", fullArgs...)
	} else if _, err := exec.LookPath("podman-compose"); err == nil {
		cmd = exec.Command("podman-compose", args...)
	} else {
		t.Skip("neither 'podman compose' nor 'podman-compose' found; skipping container test")
		return "", nil
	}

	cmd.Dir = deploymentsDir
	out, err := cmd.CombinedOutput()
	return string(out), err
}

// startDatabroker starts the kuksa-databroker container and waits for it to
// be reachable on port 55556.  Cleanup stops the container.
func startDatabroker(t *testing.T) {
	t.Helper()
	requirePodman(t)

	out, err := podmanCompose(t, "up", "-d", "kuksa-databroker")
	if err != nil {
		t.Fatalf("podman compose up kuksa-databroker: %v\n%s", err, out)
	}

	t.Cleanup(func() {
		out, err := podmanCompose(t, "stop", "kuksa-databroker")
		if err != nil {
			t.Logf("podman compose stop kuksa-databroker (cleanup): %v\n%s", err, out)
		}
	})

	// Wait up to 20 seconds for the DATA_BROKER to be reachable.
	if err := waitForPort("localhost:55556", 20*time.Second); err != nil {
		t.Fatalf("DATA_BROKER did not become reachable within 20s: %v", err)
	}
}

// ── Service process helpers ────────────────────────────────────────────────

// serviceProcess wraps a running locking-service process and captures its output.
type serviceProcess struct {
	cmd    *exec.Cmd
	output *safeBuffer
	done   chan struct{}
}

// safeBuffer is a goroutine-safe strings.Builder.
type safeBuffer struct {
	mu  sync.Mutex
	buf strings.Builder
}

func (sb *safeBuffer) Write(p []byte) (int, error) {
	sb.mu.Lock()
	defer sb.mu.Unlock()
	return sb.buf.Write(p)
}

func (sb *safeBuffer) String() string {
	sb.mu.Lock()
	defer sb.mu.Unlock()
	return sb.buf.String()
}

// startLockingService starts the locking-service binary with the `serve` subcommand.
// DATABROKER_ADDR defaults to http://localhost:55556 unless overridden by env.
// The process is killed in t.Cleanup.
func startLockingService(t *testing.T, binPath string, env map[string]string) *serviceProcess {
	t.Helper()

	cmd := exec.Command(binPath, "serve")

	// Build environment: start from current env then apply overrides.
	baseEnv := os.Environ()
	envMap := make(map[string]string)
	for _, kv := range baseEnv {
		parts := strings.SplitN(kv, "=", 2)
		if len(parts) == 2 {
			envMap[parts[0]] = parts[1]
		}
	}
	for k, v := range env {
		envMap[k] = v
	}
	var envSlice []string
	for k, v := range envMap {
		envSlice = append(envSlice, k+"="+v)
	}
	cmd.Env = envSlice

	buf := &safeBuffer{}
	cmd.Stdout = buf
	cmd.Stderr = buf

	if err := cmd.Start(); err != nil {
		t.Fatalf("start locking-service: %v", err)
	}

	sp := &serviceProcess{cmd: cmd, output: buf, done: make(chan struct{})}

	go func() {
		cmd.Wait() //nolint:errcheck
		close(sp.done)
	}()

	t.Cleanup(func() {
		if cmd.Process != nil {
			cmd.Process.Kill() //nolint:errcheck
		}
		<-sp.done
	})

	return sp
}

// waitForLog polls sp.output until a line containing substr appears or timeout expires.
func waitForLog(t *testing.T, sp *serviceProcess, substr string, timeout time.Duration) {
	t.Helper()
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		select {
		case <-sp.done:
			t.Fatalf("locking-service exited before log %q appeared; output:\n%s", substr, sp.output.String())
		default:
		}
		if strings.Contains(sp.output.String(), substr) {
			return
		}
		time.Sleep(100 * time.Millisecond)
	}
	t.Fatalf("timeout waiting for log %q; output:\n%s", substr, sp.output.String())
}

// waitForExit waits for the process to exit within timeout and returns its exit code.
// Returns -1 if the process did not exit within the timeout.
func waitForExit(sp *serviceProcess, timeout time.Duration) (exitCode int, timedOut bool) {
	select {
	case <-sp.done:
		if sp.cmd.ProcessState != nil {
			return sp.cmd.ProcessState.ExitCode(), false
		}
		return 0, false
	case <-time.After(timeout):
		return -1, true
	}
}

// ── Port helpers ───────────────────────────────────────────────────────────

// waitForPort polls addr until a TCP connection succeeds or timeout expires.
func waitForPort(addr string, timeout time.Duration) error {
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		conn, err := net.DialTimeout("tcp", addr, 200*time.Millisecond)
		if err == nil {
			conn.Close()
			return nil
		}
		time.Sleep(100 * time.Millisecond)
	}
	return fmt.Errorf("timeout waiting for %s", addr)
}

// ── grpcurl helpers ────────────────────────────────────────────────────────

// grpcurl runs grpcurl against localhost:55556 (DATA_BROKER TCP endpoint).
func grpcurlTCP(t *testing.T, method, body string) (string, error) {
	t.Helper()
	args := []string{"-plaintext"}
	if body != "" {
		args = append(args, "-d", body)
	}
	args = append(args, "localhost:55556", method)
	cmd := exec.Command("grpcurl", args...)
	out, err := cmd.CombinedOutput()
	return string(out), err
}

// grpcurlOK calls grpcurlTCP and fails the test if the call returns an error.
func grpcurlOK(t *testing.T, method, body string) string {
	t.Helper()
	out, err := grpcurlTCP(t, method, body)
	if err != nil {
		t.Fatalf("grpcurl %s failed: %v\noutput: %s", method, err, out)
	}
	return out
}

// publishBool sets a boolean VSS signal via the DATA_BROKER v2 API.
func publishBool(t *testing.T, path string, value bool) {
	t.Helper()
	boolStr := "false"
	if value {
		boolStr = "true"
	}
	body := fmt.Sprintf(`{"signal_id":{"path":%q},"data_point":{"bool":%s}}`, path, boolStr)
	grpcurlOK(t, "kuksa.val.v2.VAL/PublishValue", body)
}

// publishFloat sets a float VSS signal via the DATA_BROKER v2 API.
func publishFloat(t *testing.T, path string, value float64) {
	t.Helper()
	body := fmt.Sprintf(`{"signal_id":{"path":%q},"data_point":{"float":%g}}`, path, value)
	grpcurlOK(t, "kuksa.val.v2.VAL/PublishValue", body)
}

// publishString sets a string VSS signal via the DATA_BROKER v2 API.
func publishString(t *testing.T, path, value string) {
	t.Helper()
	// Escape the value as a JSON string within JSON.
	body := fmt.Sprintf(`{"signal_id":{"path":%q},"data_point":{"string":%q}}`, path, value)
	grpcurlOK(t, "kuksa.val.v2.VAL/PublishValue", body)
}

// getValue reads a VSS signal via the DATA_BROKER v2 API and returns the raw JSON response.
func getValue(t *testing.T, path string) string {
	t.Helper()
	body := fmt.Sprintf(`{"signal_id":{"path":%q}}`, path)
	return grpcurlOK(t, "kuksa.val.v2.VAL/GetValue", body)
}

// waitForSignal polls the DATA_BROKER until the signal value contains substr or timeout.
func waitForSignal(t *testing.T, path, substr string, timeout time.Duration) string {
	t.Helper()
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		out, err := grpcurlTCP(t, "kuksa.val.v2.VAL/GetValue",
			fmt.Sprintf(`{"signal_id":{"path":%q}}`, path))
		if err == nil && strings.Contains(out, substr) {
			return out
		}
		time.Sleep(200 * time.Millisecond)
	}
	out := getValue(t, path)
	t.Fatalf("signal %q did not contain %q within %v; last value: %s", path, substr, timeout, out)
	return ""
}

// ── Command JSON helpers ───────────────────────────────────────────────────

// lockCommandJSON builds a lock command JSON payload.
func lockCommandJSON(commandID string) string {
	return fmt.Sprintf(`{"command_id":%q,"action":"lock","doors":["driver"]}`, commandID)
}

// unlockCommandJSON builds an unlock command JSON payload.
func unlockCommandJSON(commandID string) string {
	return fmt.Sprintf(`{"command_id":%q,"action":"unlock","doors":["driver"]}`, commandID)
}

// ── Log scanner ────────────────────────────────────────────────────────────

// scanLines reads lines from r until a line containing substr is found or timeout elapses.
// Returns true if the line was found.
func scanLines(output *safeBuffer, substr string, timeout time.Duration) bool {
	deadline := time.Now().Add(timeout)
	scanner := bufio.NewScanner(strings.NewReader(""))
	for time.Now().Before(deadline) {
		text := output.String()
		scanner = bufio.NewScanner(strings.NewReader(text))
		for scanner.Scan() {
			if strings.Contains(scanner.Text(), substr) {
				return true
			}
		}
		time.Sleep(50 * time.Millisecond)
	}
	return false
}
