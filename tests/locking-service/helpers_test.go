// Package lockingservice contains integration tests for the LOCKING_SERVICE
// component (spec 03_locking_service). Tests verify command subscription, safety
// constraint enforcement, lock state management, response publishing, and graceful
// lifecycle behaviour. Live tests require a running Kuksa Databroker container
// (via Podman) and a compiled locking-service binary. Both prerequisites are
// skipped gracefully when unavailable.
//
// API compatibility note: Kuksa Databroker v0.5.0 exposes both kuksa.val.v1.VAL
// (legacy) and kuksa.val.v2.VAL. The v1 API is not registered with gRPC
// reflection, so grpcurl must be invoked with explicit proto files. All gRPC
// helpers in this package include the -import-path/-proto flags pointing to the
// vendored proto definitions in rhivos/locking-service/proto/.
package lockingservice

import (
	"bytes"
	"context"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"sync"
	"testing"
	"time"
)

// ── Constants ─────────────────────────────────────────────────────────────────

const (
	tcpEndpoint      = "localhost:55556"
	lsDatabrokerAddr = "http://localhost:55556"

	signalCmdLock  = "Vehicle.Command.Door.Lock"
	signalCmdResp  = "Vehicle.Command.Door.Response"
	signalIsLocked = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked"
)

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
// Tests live in tests/locking-service/, so the root is two levels up.
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

// protoDir returns the absolute path to the kuksa.val.v1 proto root directory.
// Required because the v1 API is not registered with gRPC reflection in
// Kuksa Databroker v0.5.0, so grpcurl needs explicit proto file flags.
func protoDir(t *testing.T) string {
	t.Helper()
	return filepath.Join(repoRoot(t), "rhivos", "locking-service", "proto")
}

// ── Skip conditions ───────────────────────────────────────────────────────────

// requirePodman skips the test if podman is not available on PATH.
func requirePodman(t *testing.T) {
	t.Helper()
	if _, err := exec.LookPath("podman"); err != nil {
		t.Skip("skipping: podman not available")
	}
}

// requireGrpcurl skips the test if grpcurl is not available on PATH.
func requireGrpcurl(t *testing.T) {
	t.Helper()
	if _, err := exec.LookPath("grpcurl"); err != nil {
		t.Skip("skipping: grpcurl not available")
	}
}

// requireCargo skips the test if cargo is not available on PATH.
func requireCargo(t *testing.T) {
	t.Helper()
	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("skipping: cargo not available")
	}
}

// requireLockingServiceDeps skips if podman, grpcurl, or cargo are unavailable.
func requireLockingServiceDeps(t *testing.T) {
	t.Helper()
	requirePodman(t)
	requireGrpcurl(t)
	requireCargo(t)
}

// ── Databroker container lifecycle ────────────────────────────────────────────

// overlayPath returns the path to the VSS overlay JSON file.
func overlayPath(t *testing.T) string {
	t.Helper()
	return filepath.Join(repoRoot(t), "deployments", "vss-overlay.json")
}

// startDatabroker starts a Kuksa Databroker container with both the standard
// VSS 4.0 tree and the custom project overlay. The container is stopped and
// removed during test cleanup. TCP port 55556 is used.
func startDatabroker(t *testing.T) {
	t.Helper()

	// Remove any stale containers that might hold port 55556. This covers both
	// the named test container and any containers started by podman-compose.
	_ = exec.Command("podman", "rm", "-f", "ls-test-databroker").Run()
	// Kill any container using port 55556 (e.g. from a previous compose run).
	stopContainersOnPort(55556)

	overlay := overlayPath(t)
	out, err := exec.Command("podman", "run", "-d",
		"--name", "ls-test-databroker",
		"-p", "55556:55555",
		"-v", overlay+":/etc/kuksa/vss-overlay.json",
		"ghcr.io/eclipse-kuksa/kuksa-databroker:0.5.0",
		"--vss", "/vss_release_4.0.json,/etc/kuksa/vss-overlay.json",
		"--address", "0.0.0.0",
	).CombinedOutput()
	if err != nil {
		t.Fatalf("failed to start databroker container: %v\n%s", err, string(out))
	}

	t.Cleanup(func() {
		_ = exec.Command("podman", "rm", "-f", "ls-test-databroker").Run()
	})

	if !waitForDatabroker(t, 20*time.Second) {
		logs, _ := exec.Command("podman", "logs", "ls-test-databroker").CombinedOutput()
		t.Fatalf("databroker did not become healthy within timeout; logs:\n%s", string(logs))
	}
}

// stopContainersOnPort removes all running podman containers that bind the
// given host port. This prevents "address already in use" errors when tests
// fail to clean up or when compose-managed containers are still running.
func stopContainersOnPort(port int) {
	portStr := fmt.Sprintf("%d", port)
	// List all containers with port mappings.
	out, err := exec.Command("podman", "ps", "-a", "--format", "{{.Names}}\t{{.Ports}}").Output()
	if err != nil {
		return
	}
	for _, line := range strings.Split(string(out), "\n") {
		if strings.Contains(line, portStr) {
			parts := strings.SplitN(line, "\t", 2)
			if len(parts) > 0 && parts[0] != "" {
				_ = exec.Command("podman", "rm", "-f", parts[0]).Run()
			}
		}
	}
}

// waitForDatabroker polls the TCP endpoint until the databroker responds or the
// timeout elapses.
func waitForDatabroker(t *testing.T, timeout time.Duration) bool {
	t.Helper()
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		if databrokerHealthyTCP() {
			return true
		}
		time.Sleep(500 * time.Millisecond)
	}
	return false
}

// databrokerHealthyTCP returns true when the TCP endpoint responds to a v2
// GetServerInfo RPC. The v2 API is registered with gRPC reflection, so this
// check works without explicit proto files.
func databrokerHealthyTCP() bool {
	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer cancel()
	return exec.CommandContext(ctx,
		"grpcurl", "-plaintext",
		tcpEndpoint,
		"kuksa.val.v2.VAL/GetServerInfo",
	).Run() == nil
}

// ── locking-service binary build ─────────────────────────────────────────────

var (
	buildOnce sync.Once
	builtBin  string
	buildErr  error
)

// ensureBinary builds the locking-service binary (once per test run) and
// returns the path to the compiled executable.
func ensureBinary(t *testing.T) string {
	t.Helper()
	root := repoRoot(t)
	rhivosDir := filepath.Join(root, "rhivos")

	buildOnce.Do(func() {
		cmd := exec.Command("cargo", "build", "-p", "locking-service")
		cmd.Dir = rhivosDir
		out, err := cmd.CombinedOutput()
		if err != nil {
			buildErr = fmt.Errorf("cargo build -p locking-service failed: %v\n%s", err, string(out))
			return
		}
		builtBin = filepath.Join(rhivosDir, "target", "debug", "locking-service")
	})

	if buildErr != nil {
		t.Fatalf("failed to build locking-service: %v", buildErr)
	}
	return builtBin
}

// ── locking-service process lifecycle ────────────────────────────────────────

// lockingServiceProcess wraps a running locking-service process with log
// capture and lifecycle management.
type lockingServiceProcess struct {
	cmd      *exec.Cmd
	outBuf   *safeBuffer
	done     chan struct{}
	exitCode int
}

// startLockingService builds (if needed) and starts the locking-service binary
// with the given DATA_BROKER address. Stdout and stderr are captured together.
// The test cleanup kills the process if still running.
func startLockingService(t *testing.T, addr string) *lockingServiceProcess {
	t.Helper()
	bin := ensureBinary(t)

	buf := &safeBuffer{}
	cmd := exec.Command(bin)
	cmd.Env = append(os.Environ(),
		"DATABROKER_ADDR="+addr,
		"RUST_LOG=info",
	)
	cmd.Stdout = buf
	cmd.Stderr = buf

	ls := &lockingServiceProcess{
		cmd:    cmd,
		outBuf: buf,
		done:   make(chan struct{}),
	}

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start locking-service: %v", err)
	}

	// Background goroutine: wait for the process to exit and record the code.
	go func() {
		err := cmd.Wait()
		if err != nil {
			if ee, ok := err.(*exec.ExitError); ok {
				ls.exitCode = ee.ExitCode()
			} else {
				ls.exitCode = -1
			}
		} else {
			ls.exitCode = 0
		}
		close(ls.done)
	}()

	t.Cleanup(func() {
		ls.kill()
		ls.waitForExit(5 * time.Second)
	})

	return ls
}

// kill sends SIGKILL to the process (best-effort, errors ignored).
func (ls *lockingServiceProcess) kill() {
	if ls.cmd.Process != nil {
		_ = ls.cmd.Process.Kill()
	}
}

// sendSIGTERM sends SIGTERM to the process.
func (ls *lockingServiceProcess) sendSIGTERM() {
	if ls.cmd.Process != nil {
		_ = ls.cmd.Process.Signal(sigTERM())
	}
}

// waitForExit waits for the process to exit within timeout.
// Returns the exit code and true if the process exited, or (-1, false) on timeout.
func (ls *lockingServiceProcess) waitForExit(timeout time.Duration) (int, bool) {
	select {
	case <-ls.done:
		return ls.exitCode, true
	case <-time.After(timeout):
		return -1, false
	}
}

// logContains reports whether the captured log output contains s.
func (ls *lockingServiceProcess) logContains(s string) bool {
	return strings.Contains(ls.outBuf.String(), s)
}

// logs returns all captured log output.
func (ls *lockingServiceProcess) logs() string {
	return ls.outBuf.String()
}

// waitForLog polls until the captured log contains s or the timeout elapses.
func (ls *lockingServiceProcess) waitForLog(s string, timeout time.Duration) bool {
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		if ls.logContains(s) {
			return true
		}
		time.Sleep(100 * time.Millisecond)
	}
	return false
}

// ── gRPC helpers (grpcurl with explicit proto) ────────────────────────────────
//
// Kuksa Databroker v0.5.0 exposes kuksa.val.v1.VAL but does NOT register it
// with gRPC reflection. All grpcurl calls must include -import-path/-proto so
// grpcurl can marshal the request body into protobuf binary without reflection.

// grpcCall runs a grpcurl command against tcpEndpoint with the v1 proto file.
// body may be empty ("") for RPCs with no request body.
func grpcCall(t *testing.T, method, body string) (string, error) {
	t.Helper()
	pDir := protoDir(t)
	args := []string{
		"-plaintext",
		"-import-path", pDir,
		"-proto", "kuksa/val/v1/val.proto",
	}
	if body != "" {
		args = append(args, "-d", body)
	}
	args = append(args, tcpEndpoint, method)
	out, err := exec.Command("grpcurl", args...).CombinedOutput()
	return string(out), err
}

// grpcGet fetches the current value of a signal via VAL/Get with FIELD_VALUE.
func grpcGet(t *testing.T, path string) (string, error) {
	t.Helper()
	body := fmt.Sprintf(`{"entries": [{"path": %q, "view": "VIEW_CURRENT_VALUE", "fields": ["FIELD_VALUE"]}]}`, path)
	return grpcCall(t, "kuksa.val.v1.VAL/Get", body)
}

// grpcSetBool sets a boolean signal value via VAL/Set.
func grpcSetBool(t *testing.T, path string, value bool) (string, error) {
	t.Helper()
	v := "false"
	if value {
		v = "true"
	}
	body := fmt.Sprintf(
		`{"updates": [{"entry": {"path": %q, "value": {"bool": %s}}, "fields": ["FIELD_VALUE"]}]}`,
		path, v,
	)
	return grpcCall(t, "kuksa.val.v1.VAL/Set", body)
}

// grpcSetString sets a string signal value via VAL/Set.
func grpcSetString(t *testing.T, path, value string) (string, error) {
	t.Helper()
	body := fmt.Sprintf(
		`{"updates": [{"entry": {"path": %q, "value": {"string": %q}}, "fields": ["FIELD_VALUE"]}]}`,
		path, value,
	)
	return grpcCall(t, "kuksa.val.v1.VAL/Set", body)
}

// grpcSubscribeCapture starts a Subscribe stream for signalPath, runs action(),
// and returns what the stream emitted within timeout.
func grpcSubscribeCapture(t *testing.T, signalPath string, timeout time.Duration, action func()) string {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), timeout)
	defer cancel()

	pDir := protoDir(t)
	body := fmt.Sprintf(`{"entries": [{"path": %q, "view": "VIEW_CURRENT_VALUE", "fields": ["FIELD_VALUE"]}]}`, signalPath)
	cmd := exec.CommandContext(ctx,
		"grpcurl", "-plaintext",
		"-import-path", pDir,
		"-proto", "kuksa/val/v1/val.proto",
		"-d", body,
		tcpEndpoint,
		"kuksa.val.v1.VAL/Subscribe",
	)
	var buf bytes.Buffer
	cmd.Stdout = &buf
	cmd.Stderr = &buf

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start Subscribe stream: %v", err)
	}

	// Allow the subscription to be established before triggering the action.
	time.Sleep(400 * time.Millisecond)
	action()

	// Let the notification arrive then cancel.
	time.Sleep(1 * time.Second)
	cancel()
	_ = cmd.Wait()

	return buf.String()
}

// ── Signal polling helpers ────────────────────────────────────────────────────

// pollSignalContains polls the signal's value until the grpcGet output contains
// substr or the timeout elapses. Returns true on success.
func pollSignalContains(t *testing.T, signal, substr string, timeout time.Duration) bool {
	t.Helper()
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		out, err := grpcGet(t, signal)
		if err == nil && strings.Contains(out, substr) {
			return true
		}
		time.Sleep(200 * time.Millisecond)
	}
	return false
}

// ── Platform signal helper ────────────────────────────────────────────────────

// sigTERM is defined in helpers_unix_test.go (non-Windows).
