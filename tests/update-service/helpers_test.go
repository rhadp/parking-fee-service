// Package updateservice_test contains integration tests for the UPDATE_SERVICE
// component (spec 07_update_service).
//
// These tests start the update-service binary and verify end-to-end gRPC
// behaviours. Tests that require podman or a real OCI registry skip
// automatically when those prerequisites are unavailable.
//
// Run integration tests:
//
//	cd tests/update-service && go test -v ./...
//
// Test Specs: TS-07-17, TS-07-18, TS-07-SMOKE-1, TS-07-SMOKE-2
package updateservice_test

import (
	"encoding/json"
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
// Navigates two levels up from this file: tests/update-service/ -> tests/ -> root.
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

// requirePodman skips if podman is not installed or its daemon is not reachable.
func requirePodman(t *testing.T) {
	t.Helper()
	if _, err := exec.LookPath("podman"); err != nil {
		t.Skip("podman not available; skipping container test")
	}
	cmd := exec.Command("podman", "info", "--format", "{{.Host.Arch}}")
	if err := cmd.Run(); err != nil {
		t.Skipf("podman socket not reachable (Podman machine may be stopped): %v", err)
	}
}

// ── Binary build ───────────────────────────────────────────────────────────

// buildUpdateService builds the update-service Rust binary and returns its path.
func buildUpdateService(t *testing.T) string {
	t.Helper()
	root := repoRoot(t)
	workspaceDir := filepath.Join(root, "rhivos")

	cmd := exec.Command("cargo", "build", "--bin", "update-service")
	cmd.Dir = workspaceDir
	if out, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("cargo build update-service: %v\n%s", err, out)
	}

	binPath := filepath.Join(workspaceDir, "target", "debug", "update-service")
	if _, err := os.Stat(binPath); err != nil {
		t.Fatalf("update-service binary not found at %s after build: %v", binPath, err)
	}
	return binPath
}

// ── Config helpers ─────────────────────────────────────────────────────────

// serviceConfig holds the configuration for a test service instance.
type serviceConfig struct {
	GRPCPort              int    `json:"grpc_port"`
	RegistryURL           string `json:"registry_url"`
	InactivityTimeoutSecs int    `json:"inactivity_timeout_secs"`
	ContainerStoragePath  string `json:"container_storage_path"`
}

// writeConfig writes a JSON config file and returns its path.
// The file is cleaned up automatically when the test ends.
func writeConfig(t *testing.T, cfg serviceConfig) string {
	t.Helper()
	f, err := os.CreateTemp("", "update-service-test-config-*.json")
	if err != nil {
		t.Fatalf("create temp config: %v", err)
	}
	defer f.Close()

	enc := json.NewEncoder(f)
	if err := enc.Encode(cfg); err != nil {
		t.Fatalf("encode config: %v", err)
	}

	t.Cleanup(func() { os.Remove(f.Name()) })
	return f.Name()
}

// freePort returns a free TCP port on localhost.
func freePort(t *testing.T) int {
	t.Helper()
	l, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("freePort: %v", err)
	}
	port := l.Addr().(*net.TCPAddr).Port
	l.Close()
	return port
}

// ── Service process helpers ────────────────────────────────────────────────

// serviceProcess wraps a running update-service process and captures its output.
type serviceProcess struct {
	cmd    *exec.Cmd
	output *safeBuffer
	done   chan struct{}
	port   int
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

// startUpdateService starts the update-service binary on the given port.
// It waits for the gRPC port to become reachable before returning.
// The process is killed during t.Cleanup.
func startUpdateService(t *testing.T, binPath string, port int, extraEnv map[string]string) *serviceProcess {
	t.Helper()

	cfg := writeConfig(t, serviceConfig{
		GRPCPort:              port,
		RegistryURL:           "",
		InactivityTimeoutSecs: 86400,
		ContainerStoragePath:  "/tmp/update-service-test/",
	})

	cmd := exec.Command(binPath, "serve")

	// Build env from current env + overrides.
	envMap := make(map[string]string)
	for _, kv := range os.Environ() {
		parts := strings.SplitN(kv, "=", 2)
		if len(parts) == 2 {
			envMap[parts[0]] = parts[1]
		}
	}
	envMap["CONFIG_PATH"] = cfg
	for k, v := range extraEnv {
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
		t.Fatalf("start update-service: %v", err)
	}

	sp := &serviceProcess{cmd: cmd, output: buf, done: make(chan struct{}), port: port}

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

	// Wait for the gRPC port to become reachable (up to 15 s).
	addr := fmt.Sprintf("localhost:%d", port)
	if err := waitForPort(addr, 15*time.Second); err != nil {
		t.Fatalf("update-service gRPC port %d not reachable: %v\noutput:\n%s", port, err, buf.String())
	}

	return sp
}

// waitForLog polls sp.output until a line containing substr appears or timeout expires.
func waitForLog(t *testing.T, sp *serviceProcess, substr string, timeout time.Duration) {
	t.Helper()
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		select {
		case <-sp.done:
			t.Fatalf("update-service exited before log %q appeared; output:\n%s", substr, sp.output.String())
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
// Returns -1 and timedOut=true if the process did not exit within the timeout.
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

// grpcurlUpdateService runs grpcurl against the update-service instance.
// It loads the proto from the repository's proto/update/ directory.
func grpcurlUpdateService(t *testing.T, port int, method, body string) (string, error) {
	t.Helper()
	root := repoRoot(t)
	protoDir := filepath.Join(root, "proto", "update")

	args := []string{
		"-plaintext",
		"-import-path", root + "/proto",
		"-import-path", protoDir,
		"-proto", "update_service.proto",
	}
	if body != "" {
		args = append(args, "-d", body)
	}
	args = append(args, fmt.Sprintf("localhost:%d", port), method)

	cmd := exec.Command("grpcurl", args...)
	out, err := cmd.CombinedOutput()
	return string(out), err
}

// grpcurlOK calls grpcurlUpdateService and fails the test if the call returns an error.
func grpcurlOK(t *testing.T, port int, method, body string) string {
	t.Helper()
	requireGrpcurl(t)
	out, err := grpcurlUpdateService(t, port, method, body)
	if err != nil {
		t.Fatalf("grpcurl %s failed: %v\noutput: %s", method, err, out)
	}
	return out
}
