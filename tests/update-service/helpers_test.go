// Package updateservice contains integration tests for the UPDATE_SERVICE
// component (spec 07_update_service). Tests verify gRPC port binding, startup
// logging, graceful shutdown, and basic RPC behaviour (TS-07-17, TS-07-18,
// TS-07-SMOKE-1, TS-07-SMOKE-2).
//
// All tests that require the compiled binary skip gracefully when cargo is not
// available. Tests that call gRPC endpoints skip when grpcurl is not installed.
// Smoke tests that require podman skip when podman is absent or the registry
// is unreachable.
package updateservice

import (
	"bytes"
	"encoding/json"
	"fmt"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"sync"
	"testing"
	"time"
)

// ── Constants ─────────────────────────────────────────────────────────────────

// testPort is the gRPC port used for integration tests. A non-default port
// avoids conflicts with any production update-service that may be running.
const testPort = 50099

// serviceReadyTimeout is the maximum time we wait for the gRPC port to start
// accepting connections after the process is launched.
const serviceReadyTimeout = 15 * time.Second

// ── Thread-safe log buffer ────────────────────────────────────────────────────

type safeBuffer struct {
	mu  sync.Mutex
	buf bytes.Buffer
}

func (sb *safeBuffer) Write(p []byte) (n int, err error) {
	sb.mu.Lock()
	defer sb.mu.Unlock()
	return sb.buf.Write(p)
}

func (sb *safeBuffer) String() string {
	sb.mu.Lock()
	defer sb.mu.Unlock()
	return sb.buf.String()
}

// ── Repository helpers ────────────────────────────────────────────────────────

// repoRoot returns the absolute path to the repository root.
// Tests live in tests/update-service/, two levels above the root.
func repoRoot(t *testing.T) string {
	t.Helper()
	wd, err := os.Getwd()
	if err != nil {
		t.Fatalf("failed to get working directory: %v", err)
	}
	root, err := filepath.Abs(filepath.Join(wd, "..", ".."))
	if err != nil {
		t.Fatalf("failed to resolve repo root: %v", err)
	}
	return root
}

// protoImportPath returns the path to the repository-level proto/ directory.
func protoImportPath(t *testing.T) string {
	t.Helper()
	return filepath.Join(repoRoot(t), "proto")
}

// ── Skip guards ───────────────────────────────────────────────────────────────

// requireCargo skips the test if cargo is not available.
func requireCargo(t *testing.T) {
	t.Helper()
	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("skipping: cargo not available on PATH")
	}
}

// requireGrpcurl skips the test if grpcurl is not installed.
func requireGrpcurl(t *testing.T) {
	t.Helper()
	if _, err := exec.LookPath("grpcurl"); err != nil {
		t.Skip("skipping: grpcurl not installed (https://github.com/fullstorydev/grpcurl)")
	}
}

// ── Binary management ─────────────────────────────────────────────────────────

var (
	buildOnce sync.Once
	builtBin  string
	buildErr  error
)

// ensureBinary builds the update-service binary once per test run (via
// `cargo build -p update-service`) and returns the path to the executable.
func ensureBinary(t *testing.T) string {
	t.Helper()
	requireCargo(t)

	root := repoRoot(t)
	rhivosDir := filepath.Join(root, "rhivos")

	buildOnce.Do(func() {
		cmd := exec.Command("cargo", "build", "-p", "update-service")
		cmd.Dir = rhivosDir
		out, err := cmd.CombinedOutput()
		if err != nil {
			buildErr = fmt.Errorf("cargo build -p update-service failed: %v\n%s", err, string(out))
			return
		}
		builtBin = filepath.Join(rhivosDir, "target", "debug", "update-service")
	})

	if buildErr != nil {
		t.Fatalf("failed to build update-service: %v", buildErr)
	}
	return builtBin
}

// ── Config file helpers ───────────────────────────────────────────────────────

// writeTestConfig writes a minimal JSON config for the update-service with the
// given gRPC port and returns the path to the config file. The file is
// automatically removed when the test ends.
func writeTestConfig(t *testing.T, port int) string {
	t.Helper()

	cfg := map[string]interface{}{
		"grpc_port":               port,
		"registry_url":            "",
		"inactivity_timeout_secs": 86400,
		"container_storage_path":  "/tmp/adapters-test/",
	}
	data, err := json.Marshal(cfg)
	if err != nil {
		t.Fatalf("failed to marshal test config: %v", err)
	}

	f, err := os.CreateTemp("", "update-service-test-config-*.json")
	if err != nil {
		t.Fatalf("failed to create temp config file: %v", err)
	}
	if _, err := f.Write(data); err != nil {
		f.Close()
		t.Fatalf("failed to write temp config: %v", err)
	}
	f.Close()

	t.Cleanup(func() { os.Remove(f.Name()) })
	return f.Name()
}

// ── Service process management ────────────────────────────────────────────────

// updateServiceProcess wraps a running update-service process.
type updateServiceProcess struct {
	cmd      *exec.Cmd
	outBuf   *safeBuffer
	done     chan struct{}
	exitCode int
}

// startUpdateService starts the update-service binary with the given port and
// waits until the gRPC port is accepting connections. If the port does not open
// within serviceReadyTimeout, the test is fatally failed.
// A cleanup hook ensures the process is killed when the test ends.
func startUpdateService(t *testing.T) *updateServiceProcess {
	t.Helper()
	bin := ensureBinary(t)
	cfgPath := writeTestConfig(t, testPort)

	buf := &safeBuffer{}
	cmd := exec.Command(bin)
	cmd.Env = append(os.Environ(),
		fmt.Sprintf("CONFIG_PATH=%s", cfgPath),
		"RUST_LOG=info",
	)
	cmd.Stdout = buf
	cmd.Stderr = buf

	usp := &updateServiceProcess{
		cmd:    cmd,
		outBuf: buf,
		done:   make(chan struct{}),
	}

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start update-service: %v", err)
	}

	go func() {
		err := cmd.Wait()
		if err != nil {
			if ee, ok := err.(*exec.ExitError); ok {
				usp.exitCode = ee.ExitCode()
			} else {
				usp.exitCode = -1
			}
		} else {
			usp.exitCode = 0
		}
		close(usp.done)
	}()

	t.Cleanup(func() {
		usp.kill()
		usp.waitForExit(5 * time.Second)
	})

	// Wait until the gRPC port is open (or process dies).
	endpoint := fmt.Sprintf("127.0.0.1:%d", testPort)
	deadline := time.Now().Add(serviceReadyTimeout)
	for time.Now().Before(deadline) {
		conn, err := net.DialTimeout("tcp", endpoint, 200*time.Millisecond)
		if err == nil {
			conn.Close()
			return usp
		}
		// Check for early exit.
		select {
		case <-usp.done:
			t.Fatalf("update-service exited before port %d was ready; logs:\n%s",
				testPort, usp.logs())
		default:
		}
		time.Sleep(100 * time.Millisecond)
	}

	t.Fatalf("update-service did not open port %d within %s; logs:\n%s",
		testPort, serviceReadyTimeout, usp.logs())
	return nil // unreachable
}

// kill sends SIGKILL to the process (best-effort).
func (usp *updateServiceProcess) kill() {
	if usp.cmd.Process != nil {
		_ = usp.cmd.Process.Kill()
	}
}

// sendSIGTERM sends SIGTERM to the process.
func (usp *updateServiceProcess) sendSIGTERM() {
	if usp.cmd.Process != nil {
		_ = usp.cmd.Process.Signal(sigTERM())
	}
}

// waitForExit waits for the process to exit within the given timeout.
// Returns (exitCode, true) on exit, or (-1, false) on timeout.
func (usp *updateServiceProcess) waitForExit(timeout time.Duration) (int, bool) {
	select {
	case <-usp.done:
		return usp.exitCode, true
	case <-time.After(timeout):
		return -1, false
	}
}

// logContains reports whether the captured log output contains s.
func (usp *updateServiceProcess) logContains(s string) bool {
	return strings.Contains(usp.outBuf.String(), s)
}

// logs returns all captured log output.
func (usp *updateServiceProcess) logs() string {
	return usp.outBuf.String()
}

// ── gRPC helpers (via grpcurl) ────────────────────────────────────────────────

// grpcCall invokes a gRPC method via grpcurl (plaintext, using the proto file).
// It returns (stdout+stderr output, nil) on success or (output, error) on failure.
func grpcCall(t *testing.T, method, body string) (string, error) {
	t.Helper()

	protoPath := protoImportPath(t)
	endpoint := fmt.Sprintf("127.0.0.1:%d", testPort)

	args := []string{
		"-plaintext",
		"-import-path", protoPath,
		"-proto", "update/update_service.proto",
	}
	if body != "" {
		args = append(args, "-d", body)
	}
	args = append(args, endpoint, "update.UpdateService/"+method)

	out, err := exec.Command("grpcurl", args...).CombinedOutput()
	return string(out), err
}
