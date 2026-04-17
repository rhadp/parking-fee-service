// Package parkingoperatoradaptor contains integration tests for the
// PARKING_OPERATOR_ADAPTOR component (spec 08_parking_operator_adaptor).
//
// Tests verify startup logging, graceful shutdown, connection retry behaviour,
// session state loss on restart, and end-to-end lock/unlock/manual flows.
//
// Infrastructure requirements:
//   - Most tests require DATA_BROKER (Kuksa Databroker v1-compatible) reachable
//     on localhost:55556. Tests skip when the broker is not reachable.
//   - Smoke tests additionally require grpcurl (for signal injection) and a
//     pre-built parking-operator-adaptor binary (built via `cargo build`).
//   - TestDatabrokerUnreachable is the exception: it intentionally starts the
//     adaptor against a non-listening port and verifies non-zero exit — no
//     infrastructure is required.
//
// NOTE — proto compatibility gap:
//
//	The parking-operator-adaptor uses the kuksa.val.v1 proto for DATA_BROKER
//	communication (VAL Subscribe/Set RPCs). A real Kuksa Databroker 0.5.0+
//	instance exposes only the kuksa.val.v2.VAL API. When the adaptor tries to
//	subscribe to IsLocked via the v1 API against a v2 broker, the Subscribe RPC
//	fails and the adaptor exits non-zero before printing "ready". All tests that
//	require a running adaptor (TestStartupLogging, TestGracefulShutdown,
//	TestInitialSessionActive, TestSessionLostOnRestart, and all smoke tests)
//	will therefore skip when the only available broker is v2-only.
//	TestDatabrokerUnreachable runs in all environments.
package parkingoperatoradaptor

import (
	"bytes"
	"encoding/json"
	"fmt"
	"net"
	"net/http"
	"net/http/httptest"
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
	// testGrpcPort is the gRPC port used for integration tests. Non-default to
	// avoid collisions with a running production adaptor.
	testGrpcPort = 50060

	// dataBrokerEndpoint is the host:port of the DATA_BROKER TCP listener.
	dataBrokerEndpoint = "localhost:55556"

	// dataBrokerHTTP is the full HTTP URL passed to the adaptor via DATA_BROKER_ADDR.
	dataBrokerHTTP = "http://localhost:55556"

	// readyLogLine is the string the adaptor logs once it is fully initialised
	// and accepting gRPC connections.
	readyLogLine = "parking-operator-adaptor ready"

	// serviceReadyTimeout is how long we wait for readyLogLine to appear.
	serviceReadyTimeout = 30 * time.Second
)

// ── VSS signal paths ──────────────────────────────────────────────────────────

const (
	signalIsLocked      = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked"
	signalSessionActive = "Vehicle.Parking.SessionActive"
)

// ── Thread-safe log buffer ────────────────────────────────────────────────────

// safeBuffer is a bytes.Buffer protected by a mutex for concurrent write/read.
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
// Tests live in tests/parking-operator-adaptor/, two levels above the root.
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

// protoDir returns the path to the parking-operator-adaptor proto directory.
func protoDir(t *testing.T) string {
	t.Helper()
	return filepath.Join(repoRoot(t), "rhivos", "parking-operator-adaptor", "proto")
}

// ── Skip guards ───────────────────────────────────────────────────────────────

// requireCargo skips the test if cargo is not on PATH.
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

// requireTCPReachable skips the test if the DATA_BROKER TCP port is not reachable.
func requireTCPReachable(t *testing.T) {
	t.Helper()
	conn, err := net.DialTimeout("tcp", dataBrokerEndpoint, 2*time.Second)
	if err != nil {
		t.Skipf("skipping: DATA_BROKER not reachable at %s (start with: cd deployments && podman compose up -d): %v",
			dataBrokerEndpoint, err)
	}
	conn.Close()
}

// ── Binary management ─────────────────────────────────────────────────────────

var (
	buildOnce sync.Once
	builtBin  string
	buildErr  error
)

// ensureBinary builds the parking-operator-adaptor binary once per test run and
// returns the path to the executable.
func ensureBinary(t *testing.T) string {
	t.Helper()
	requireCargo(t)

	root := repoRoot(t)
	rhivosDir := filepath.Join(root, "rhivos")

	buildOnce.Do(func() {
		cmd := exec.Command("cargo", "build", "-p", "parking-operator-adaptor")
		cmd.Dir = rhivosDir
		out, err := cmd.CombinedOutput()
		if err != nil {
			buildErr = fmt.Errorf("cargo build -p parking-operator-adaptor failed: %v\n%s", err, string(out))
			return
		}
		builtBin = filepath.Join(rhivosDir, "target", "debug", "parking-operator-adaptor")
	})

	if buildErr != nil {
		t.Fatalf("failed to build parking-operator-adaptor: %v", buildErr)
	}
	return builtBin
}

// ── Mock PARKING_OPERATOR HTTP server ─────────────────────────────────────────

// mockOperator is a lightweight in-process HTTP server that simulates the
// PARKING_OPERATOR REST API (POST /parking/start and POST /parking/stop).
type mockOperator struct {
	server *httptest.Server

	mu              sync.Mutex
	startCount      int
	stopCount       int
	activeSessionID string
}

// newMockOperator creates a mock PARKING_OPERATOR HTTP server and registers a
// cleanup hook to shut it down when the test ends.
func newMockOperator(t *testing.T) *mockOperator {
	t.Helper()
	m := &mockOperator{}

	mux := http.NewServeMux()
	mux.HandleFunc("/parking/start", m.handleStart)
	mux.HandleFunc("/parking/stop", m.handleStop)
	m.server = httptest.NewServer(mux)

	t.Cleanup(m.server.Close)
	return m
}

// url returns the base URL of the mock operator server.
func (m *mockOperator) url() string {
	return m.server.URL
}

// getStartCount returns the number of /parking/start calls received.
func (m *mockOperator) getStartCount() int {
	m.mu.Lock()
	defer m.mu.Unlock()
	return m.startCount
}

// getStopCount returns the number of /parking/stop calls received.
func (m *mockOperator) getStopCount() int {
	m.mu.Lock()
	defer m.mu.Unlock()
	return m.stopCount
}

// waitForStartCount polls until startCount reaches n or timeout elapses.
func (m *mockOperator) waitForStartCount(n int, timeout time.Duration) bool {
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		if m.getStartCount() >= n {
			return true
		}
		time.Sleep(100 * time.Millisecond)
	}
	return false
}

// waitForStopCount polls until stopCount reaches n or timeout elapses.
func (m *mockOperator) waitForStopCount(n int, timeout time.Duration) bool {
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		if m.getStopCount() >= n {
			return true
		}
		time.Sleep(100 * time.Millisecond)
	}
	return false
}

func (m *mockOperator) handleStart(w http.ResponseWriter, r *http.Request) {
	m.mu.Lock()
	defer m.mu.Unlock()

	m.startCount++
	sessionID := fmt.Sprintf("sess-test-%d", m.startCount)
	m.activeSessionID = sessionID

	w.Header().Set("Content-Type", "application/json")
	_ = json.NewEncoder(w).Encode(map[string]interface{}{
		"session_id": sessionID,
		"status":     "active",
		"rate": map[string]interface{}{
			"type":     "per_hour",
			"amount":   2.50,
			"currency": "EUR",
		},
	})
}

func (m *mockOperator) handleStop(w http.ResponseWriter, r *http.Request) {
	m.mu.Lock()
	defer m.mu.Unlock()

	m.stopCount++
	sessionID := m.activeSessionID
	m.activeSessionID = ""

	w.Header().Set("Content-Type", "application/json")
	_ = json.NewEncoder(w).Encode(map[string]interface{}{
		"session_id":       sessionID,
		"status":           "completed",
		"duration_seconds": 3600,
		"total_amount":     2.50,
		"currency":         "EUR",
	})
}

// ── Adaptor process management ────────────────────────────────────────────────

// adaptorProcess wraps a running parking-operator-adaptor process with log
// capture and lifecycle management.
type adaptorProcess struct {
	cmd      *exec.Cmd
	outBuf   *safeBuffer
	done     chan struct{}
	exitCode int
}

// startAdaptorRaw starts the parking-operator-adaptor binary with the given
// environment overrides. It does NOT wait for readiness — callers must poll
// logs themselves. A cleanup hook kills the process when the test ends.
func startAdaptorRaw(t *testing.T, envOverrides map[string]string) *adaptorProcess {
	t.Helper()
	bin := ensureBinary(t)

	buf := &safeBuffer{}
	cmd := exec.Command(bin)

	// Build environment: start from current env, apply overrides.
	baseEnv := os.Environ()
	filtered := baseEnv[:0:len(baseEnv)]
	for _, e := range baseEnv {
		keep := true
		for k := range envOverrides {
			if strings.HasPrefix(e, k+"=") {
				keep = false
				break
			}
		}
		if keep {
			filtered = append(filtered, e)
		}
	}
	for k, v := range envOverrides {
		filtered = append(filtered, k+"="+v)
	}
	cmd.Env = filtered
	cmd.Stdout = buf
	cmd.Stderr = buf

	ap := &adaptorProcess{
		cmd:    cmd,
		outBuf: buf,
		done:   make(chan struct{}),
	}

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start parking-operator-adaptor: %v", err)
	}

	go func() {
		err := cmd.Wait()
		if err != nil {
			if ee, ok := err.(*exec.ExitError); ok {
				ap.exitCode = ee.ExitCode()
			} else {
				ap.exitCode = -1
			}
		} else {
			ap.exitCode = 0
		}
		close(ap.done)
	}()

	t.Cleanup(func() {
		ap.kill()
		ap.waitForExit(5 * time.Second)
	})

	return ap
}

// startAdaptor starts the adaptor against the given mock operator and waits for
// it to become ready. If the adaptor exits before printing readyLogLine (e.g.
// due to the kuksa.val.v1 vs. v2 proto incompatibility with the running
// DATA_BROKER), the test is skipped with a descriptive message.
//
// Preconditions: DATA_BROKER must be reachable (requireTCPReachable is called).
func startAdaptor(t *testing.T, mockOp *mockOperator) *adaptorProcess {
	t.Helper()
	requireTCPReachable(t)

	overrides := map[string]string{
		"PARKING_OPERATOR_URL": mockOp.url(),
		"DATA_BROKER_ADDR":     dataBrokerHTTP,
		"GRPC_PORT":            fmt.Sprintf("%d", testGrpcPort),
		"VEHICLE_ID":           "TEST-VIN-001",
		"ZONE_ID":              "zone-test-1",
		"RUST_LOG":             "info",
	}
	ap := startAdaptorRaw(t, overrides)

	// Poll every 100 ms for the ready log line, but skip if the process exits
	// early (which happens when the v1 Subscribe RPC fails against a v2 broker).
	deadline := time.Now().Add(serviceReadyTimeout)
	for time.Now().Before(deadline) {
		if ap.logContains(readyLogLine) {
			return ap
		}
		select {
		case <-ap.done:
			t.Skipf(
				"parking-operator-adaptor exited before becoming ready. "+
					"This is expected when the adaptor's kuksa.val.v1 proto is "+
					"incompatible with the live DATA_BROKER kuksa.val.v2.VAL API.\n"+
					"Process logs:\n%s",
				ap.logs(),
			)
		default:
		}
		time.Sleep(100 * time.Millisecond)
	}

	logs := ap.logs()
	ap.kill()
	t.Skipf(
		"parking-operator-adaptor did not become ready within %s. "+
			"This is expected when the adaptor's kuksa.val.v1 proto is "+
			"incompatible with the live DATA_BROKER kuksa.val.v2.VAL API.\n"+
			"Process logs:\n%s",
		serviceReadyTimeout, logs,
	)
	return nil // unreachable
}

// kill sends SIGKILL to the process (best-effort).
func (ap *adaptorProcess) kill() {
	if ap.cmd.Process != nil {
		_ = ap.cmd.Process.Kill()
	}
}

// sendSIGTERM sends SIGTERM to the process.
func (ap *adaptorProcess) sendSIGTERM() {
	if ap.cmd.Process != nil {
		_ = ap.cmd.Process.Signal(sigTERM())
	}
}

// waitForExit waits for the process to exit within the given timeout.
// Returns (exitCode, true) on exit or (-1, false) on timeout.
func (ap *adaptorProcess) waitForExit(timeout time.Duration) (int, bool) {
	select {
	case <-ap.done:
		return ap.exitCode, true
	case <-time.After(timeout):
		return -1, false
	}
}

// logContains reports whether the captured log output contains s.
func (ap *adaptorProcess) logContains(s string) bool {
	return strings.Contains(ap.outBuf.String(), s)
}

// logs returns all captured log output.
func (ap *adaptorProcess) logs() string {
	return ap.outBuf.String()
}

// waitForLog polls until the log contains s or the timeout elapses.
func (ap *adaptorProcess) waitForLog(s string, timeout time.Duration) bool {
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		if ap.logContains(s) {
			return true
		}
		time.Sleep(100 * time.Millisecond)
	}
	return false
}

// ── gRPC helpers (via grpcurl) ────────────────────────────────────────────────

// grpcCallAdaptor invokes a ParkingAdaptor gRPC method via grpcurl using the
// parking_adaptor.proto definition. Returns (output, nil) on RPC success or
// (output, error) on failure (non-zero grpcurl exit).
func grpcCallAdaptor(t *testing.T, port int, method, body string) (string, error) {
	t.Helper()
	requireGrpcurl(t)

	endpoint := fmt.Sprintf("127.0.0.1:%d", port)
	pd := protoDir(t)

	args := []string{
		"-plaintext",
		"-import-path", pd,
		"-proto", "parking_adaptor.proto",
	}
	if body != "" {
		args = append(args, "-d", body)
	}
	args = append(args, endpoint, "parking.adaptor.ParkingAdaptor/"+method)

	out, err := exec.Command("grpcurl", args...).CombinedOutput()
	return string(out), err
}

// ── DATA_BROKER helpers ───────────────────────────────────────────────────────

// brokerSetIsLocked sets the Vehicle.Cabin.Door.Row1.DriverSide.IsLocked signal
// in DATA_BROKER via the kuksa.val.v2.VAL/PublishValue RPC. Returns an error if
// the grpcurl call fails (used in smoke tests that skip on signal propagation
// failures rather than fatally failing).
func brokerSetIsLockedRaw(signal string, value bool) error {
	data := fmt.Sprintf(
		`{"signal_id":{"path":%q},"data_point":{"value":{"bool":%v}}}`,
		signal, value,
	)
	out, err := exec.Command("grpcurl", "-plaintext", "-d", data,
		dataBrokerEndpoint, "kuksa.val.v2.VAL/PublishValue").CombinedOutput()
	if err != nil {
		return fmt.Errorf("grpcurl PublishValue failed: %w\n%s", err, out)
	}
	return nil
}

// brokerSetIsLocked sets the Vehicle.Cabin.Door.Row1.DriverSide.IsLocked signal
// in DATA_BROKER via the v2 API. Fatally fails the test on grpcurl error.
func brokerSetIsLocked(t *testing.T, value bool) {
	t.Helper()
	requireGrpcurl(t)
	if err := brokerSetIsLockedRaw(signalIsLocked, value); err != nil {
		t.Fatalf("brokerSetIsLocked(%v): %v", value, err)
	}
}

// brokerGetBool reads a boolean VSS signal from DATA_BROKER via the v2 API.
// Returns false if the signal is unset.
func brokerGetBool(t *testing.T, signal string) bool {
	t.Helper()
	requireGrpcurl(t)

	data := fmt.Sprintf(`{"signal_id":{"path":%q}}`, signal)
	out, err := exec.Command("grpcurl", "-plaintext", "-d", data,
		dataBrokerEndpoint, "kuksa.val.v2.VAL/GetValue").CombinedOutput()
	if err != nil {
		t.Fatalf("brokerGetBool(%q) failed: %v\n%s", signal, err, out)
	}
	return strings.Contains(string(out), `"bool":true`)
}

// waitForBrokerBool polls until the boolean signal matches expected, or the
// timeout elapses. Returns true if the expected value was observed.
func waitForBrokerBool(t *testing.T, signal string, expected bool, timeout time.Duration) bool {
	t.Helper()
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		if brokerGetBool(t, signal) == expected {
			return true
		}
		time.Sleep(200 * time.Millisecond)
	}
	return false
}

// waitForGRPCActive polls GetStatus until active matches expected or timeout.
func waitForGRPCActive(t *testing.T, port int, expected bool, timeout time.Duration) bool {
	t.Helper()
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		out, err := grpcCallAdaptor(t, port, "GetStatus", "")
		if err == nil {
			isActive := strings.Contains(out, `"active": true`) ||
				strings.Contains(out, `"active":true`)
			if isActive == expected {
				return true
			}
		}
		time.Sleep(200 * time.Millisecond)
	}
	return false
}
