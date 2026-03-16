// Package updateservice contains integration tests for the UPDATE_SERVICE
// component (spec 07_update_service). Tests verify gRPC API behaviour,
// startup logging, and graceful shutdown. Live tests require a compiled
// update-service binary (built from rhivos/update-service/ via cargo) and
// optionally grpcurl for protocol-level assertions. Both prerequisites are
// skipped gracefully when unavailable.
package updateservice

import (
	"bytes"
	"encoding/json"
	"fmt"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"sync"
	"sync/atomic"
	"testing"
	"time"
)

// ── Port allocation ────────────────────────────────────────────────────────────

// portCounter provides unique ports for each test service instance so that
// parallel tests do not clash.
var portCounter int32 = 50060

// nextPort returns the next available port for a test service instance.
func nextPort() int {
	return int(atomic.AddInt32(&portCounter, 1))
}

// ── Thread-safe buffer ────────────────────────────────────────────────────────

// safeBuffer is a bytes.Buffer protected by a mutex so it can be written from
// the process output goroutine while being read from the test goroutine.
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
// Tests live in tests/update-service/, two levels up from the repo root.
func repoRoot(t *testing.T) string {
	t.Helper()
	wd, err := os.Getwd()
	if err != nil {
		t.Fatalf("failed to get working directory: %v", err)
	}
	root := filepath.Join(wd, "..", "..")
	abs, err := filepath.Abs(root)
	if err != nil {
		t.Fatalf("failed to resolve repo root: %v", err)
	}
	return abs
}

// protoDir returns the absolute path to the update-service proto directory.
// Both proto files (update_service.proto and common.proto) live here.
// grpcurl must pass -import-path protoDir so it can resolve the common.proto
// import referenced by update_service.proto.
func protoDir(t *testing.T) string {
	t.Helper()
	return filepath.Join(repoRoot(t), "rhivos", "update-service", "proto")
}

// ── Skip conditions ───────────────────────────────────────────────────────────

// requireCargo skips the test if cargo is not available on PATH.
func requireCargo(t *testing.T) {
	t.Helper()
	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("skipping: cargo not available")
	}
}

// requireGrpcurl skips the test if grpcurl is not available on PATH.
func requireGrpcurl(t *testing.T) {
	t.Helper()
	if _, err := exec.LookPath("grpcurl"); err != nil {
		t.Skip("skipping: grpcurl not available")
	}
}

// ── Binary build ──────────────────────────────────────────────────────────────

var (
	buildOnce sync.Once
	builtBin  string
	buildErr  error
)

// ensureBinary builds the update-service binary once per test run and returns
// the path to the compiled executable. The binary is built with
// `cargo build -p update-service` inside rhivos/.
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

// ── Config helpers ────────────────────────────────────────────────────────────

// serviceConfig mirrors the update-service Config struct for JSON marshalling.
type serviceConfig struct {
	GRPCPort              int    `json:"grpc_port"`
	RegistryURL           string `json:"registry_url"`
	InactivityTimeoutSecs uint64 `json:"inactivity_timeout_secs"`
	ContainerStoragePath  string `json:"container_storage_path"`
}

// defaultTestServiceConfig returns a serviceConfig suitable for integration
// tests, using the supplied gRPC port.
func defaultTestServiceConfig(port int) serviceConfig {
	return serviceConfig{
		GRPCPort:              port,
		RegistryURL:           "us-docker.pkg.dev/test-project/adapters",
		InactivityTimeoutSecs: 86400,
		ContainerStoragePath:  "/tmp/test-update-service-containers/",
	}
}

// createConfig writes a temporary config JSON file and returns its path.
func createConfig(t *testing.T, cfg serviceConfig) string {
	t.Helper()
	data, err := json.Marshal(cfg)
	if err != nil {
		t.Fatalf("failed to marshal service config: %v", err)
	}
	f, err := os.CreateTemp(t.TempDir(), "update-service-config-*.json")
	if err != nil {
		t.Fatalf("failed to create temp config file: %v", err)
	}
	defer f.Close()
	if _, err := f.Write(data); err != nil {
		t.Fatalf("failed to write temp config: %v", err)
	}
	return f.Name()
}

// ── Service process management ────────────────────────────────────────────────

// serviceProcess wraps a running update-service process with log capture and
// lifecycle management.
type serviceProcess struct {
	cmd      *exec.Cmd
	outBuf   *safeBuffer
	port     int
	done     chan struct{}
	exitCode int
}

// startService builds (if needed) and starts the update-service binary with
// the given config. It waits for the gRPC port to accept TCP connections before
// returning. A test cleanup hook kills the process on test completion.
func startService(t *testing.T, cfg serviceConfig) *serviceProcess {
	t.Helper()
	bin := ensureBinary(t)
	configPath := createConfig(t, cfg)

	buf := &safeBuffer{}
	cmd := exec.Command(bin, "serve")
	cmd.Env = append(os.Environ(),
		"CONFIG_PATH="+configPath,
		"RUST_LOG=info",
	)
	cmd.Stdout = buf
	cmd.Stderr = buf

	sp := &serviceProcess{
		cmd:    cmd,
		outBuf: buf,
		port:   cfg.GRPCPort,
		done:   make(chan struct{}),
	}

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start update-service: %v", err)
	}

	// Background goroutine: wait for the process to exit and record exit code.
	go func() {
		err := cmd.Wait()
		if err != nil {
			if ee, ok := err.(*exec.ExitError); ok {
				sp.exitCode = ee.ExitCode()
			} else {
				sp.exitCode = -1
			}
		} else {
			sp.exitCode = 0
		}
		close(sp.done)
	}()

	t.Cleanup(func() {
		sp.kill()
		sp.waitForExit(5 * time.Second)
	})

	// Wait for the gRPC port to be ready.
	if !sp.waitForPort(20 * time.Second) {
		logs := sp.logs()
		sp.kill()
		t.Fatalf("update-service did not start on port %d within timeout; logs:\n%s", cfg.GRPCPort, logs)
	}

	return sp
}

// kill sends SIGKILL to the process (best-effort, errors ignored).
func (sp *serviceProcess) kill() {
	if sp.cmd.Process != nil {
		_ = sp.cmd.Process.Kill()
	}
}

// sendSIGTERM sends SIGTERM to the process.
func (sp *serviceProcess) sendSIGTERM() {
	if sp.cmd.Process != nil {
		_ = sp.cmd.Process.Signal(sigTERM())
	}
}

// waitForExit waits for the process to exit within timeout.
// Returns (exitCode, true) if it exited, or (-1, false) on timeout.
func (sp *serviceProcess) waitForExit(timeout time.Duration) (int, bool) {
	select {
	case <-sp.done:
		return sp.exitCode, true
	case <-time.After(timeout):
		return -1, false
	}
}

// waitForPort polls the gRPC port until it accepts TCP connections or the
// timeout elapses.
func (sp *serviceProcess) waitForPort(timeout time.Duration) bool {
	addr := fmt.Sprintf("localhost:%d", sp.port)
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		conn, err := net.DialTimeout("tcp", addr, 300*time.Millisecond)
		if err == nil {
			conn.Close()
			return true
		}
		time.Sleep(200 * time.Millisecond)
	}
	return false
}

// logContains reports whether the captured log output contains s.
func (sp *serviceProcess) logContains(s string) bool {
	return bytes.Contains([]byte(sp.outBuf.String()), []byte(s))
}

// logs returns all captured log output.
func (sp *serviceProcess) logs() string {
	return sp.outBuf.String()
}

// waitForLog polls until the log contains s or the timeout elapses.
func (sp *serviceProcess) waitForLog(s string, timeout time.Duration) bool {
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		if sp.logContains(s) {
			return true
		}
		time.Sleep(100 * time.Millisecond)
	}
	return false
}

// ── gRPC helpers (grpcurl with explicit proto) ────────────────────────────────
//
// The update-service does not enable gRPC server reflection, so grpcurl must
// be invoked with -import-path/-proto flags pointing to the vendored proto
// definitions in rhivos/update-service/proto/.

// grpcCall runs a grpcurl command against the service's gRPC port using the
// update-service proto definitions. body may be empty for RPCs with no request.
// Returns the combined stdout+stderr and the command error.
func (sp *serviceProcess) grpcCall(t *testing.T, method, body string) (string, error) {
	t.Helper()
	pd := protoDir(t)
	args := []string{
		"-plaintext",
		"-import-path", pd,
		"-proto", "update_service.proto",
	}
	if body != "" {
		args = append(args, "-d", body)
	}
	args = append(args, fmt.Sprintf("localhost:%d", sp.port), method)
	out, err := exec.Command("grpcurl", args...).CombinedOutput()
	return string(out), err
}
