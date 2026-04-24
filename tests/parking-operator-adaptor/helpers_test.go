package parkingoperatoradaptor_test

import (
	"bufio"
	"context"
	"encoding/json"
	"fmt"
	"io"
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

	"github.com/rhadp/parking-fee-service/gen/adapter"
	"github.com/rhadp/parking-fee-service/gen/kuksa"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

const (
	// tcpTarget is the host address for the DATA_BROKER TCP listener.
	tcpTarget = "localhost:55556"

	// connectTimeout is the maximum time to wait for a gRPC connection.
	connectTimeout = 5 * time.Second

	// rpcTimeout is the maximum time to wait for a single gRPC call.
	rpcTimeout = 5 * time.Second

	// serviceReadyTimeout is the maximum time to wait for the
	// parking-operator-adaptor to become ready after startup.
	serviceReadyTimeout = 45 * time.Second

	// signalIsLocked is the VSS path for the driver door lock state.
	signalIsLocked = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked"

	// signalSessionActive is the custom VSS path for parking session state.
	signalSessionActive = "Vehicle.Parking.SessionActive"
)

// ---------------------------------------------------------------------------
// Repo / Build helpers
// ---------------------------------------------------------------------------

// findRepoRoot walks up from the current directory until it finds .git.
func findRepoRoot(t *testing.T) string {
	t.Helper()
	dir, err := os.Getwd()
	if err != nil {
		t.Fatalf("failed to get working directory: %v", err)
	}
	for {
		if _, err := os.Stat(filepath.Join(dir, ".git")); err == nil {
			return dir
		}
		// Also handle worktree case (.git is a file, not a directory)
		if info, err := os.Stat(filepath.Join(dir, ".git")); err == nil && !info.IsDir() {
			return dir
		}
		parent := filepath.Dir(dir)
		if parent == dir {
			t.Fatal("could not find repo root (.git)")
		}
		dir = parent
	}
}

// buildAdaptor builds the parking-operator-adaptor Rust binary using cargo
// and returns the path to the compiled binary.
func buildAdaptor(t *testing.T) string {
	t.Helper()
	root := findRepoRoot(t)
	rhivosDir := filepath.Join(root, "rhivos")

	cmd := exec.Command("cargo", "build", "-p", "parking-operator-adaptor")
	cmd.Dir = rhivosDir
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("failed to build parking-operator-adaptor: %v\n%s", err, string(out))
	}

	binaryPath := filepath.Join(rhivosDir, "target", "debug", "parking-operator-adaptor")
	if _, err := os.Stat(binaryPath); os.IsNotExist(err) {
		t.Fatalf("parking-operator-adaptor binary not found at %s after build", binaryPath)
	}

	return binaryPath
}

// ---------------------------------------------------------------------------
// DATA_BROKER connectivity
// ---------------------------------------------------------------------------

// skipIfTCPUnreachable skips the test if the DATA_BROKER TCP port is not
// reachable.
func skipIfTCPUnreachable(t *testing.T) {
	t.Helper()
	conn, err := net.DialTimeout("tcp", tcpTarget, 2*time.Second)
	if err != nil {
		t.Skipf("DATA_BROKER TCP port not reachable at %s: %v", tcpTarget, err)
	}
	conn.Close()
}

// dialTCP creates a gRPC client connection to the DATA_BROKER via TCP.
func dialTCP(t *testing.T) *grpc.ClientConn {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), connectTimeout)
	defer cancel()

	conn, err := grpc.DialContext(ctx, tcpTarget,
		grpc.WithTransportCredentials(insecure.NewCredentials()),
		grpc.WithBlock(),
	)
	if err != nil {
		t.Fatalf("failed to dial TCP %s: %v", tcpTarget, err)
	}
	t.Cleanup(func() { conn.Close() })
	return conn
}

// newBrokerClient creates a kuksa VAL gRPC client over TCP.
func newBrokerClient(t *testing.T) kuksa.VALClient {
	t.Helper()
	return kuksa.NewVALClient(dialTCP(t))
}

// dialAdaptor creates a gRPC client connection to the parking-operator-adaptor.
func dialAdaptor(t *testing.T, port int) *grpc.ClientConn {
	t.Helper()
	target := fmt.Sprintf("localhost:%d", port)
	ctx, cancel := context.WithTimeout(context.Background(), connectTimeout)
	defer cancel()

	conn, err := grpc.DialContext(ctx, target,
		grpc.WithTransportCredentials(insecure.NewCredentials()),
		grpc.WithBlock(),
	)
	if err != nil {
		t.Fatalf("failed to dial adaptor at %s: %v", target, err)
	}
	t.Cleanup(func() { conn.Close() })
	return conn
}

// newAdaptorClient creates a ParkingOperatorAdaptorService gRPC client.
func newAdaptorClient(t *testing.T, port int) adapter.ParkingOperatorAdaptorServiceClient {
	t.Helper()
	return adapter.NewParkingOperatorAdaptorServiceClient(dialAdaptor(t, port))
}

// ---------------------------------------------------------------------------
// DATA_BROKER signal helpers
// ---------------------------------------------------------------------------

// getSignalValue performs a Get RPC for the given signal path.
func getSignalValue(t *testing.T, client kuksa.VALClient, path string) (*kuksa.DataEntry, error) {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), rpcTimeout)
	defer cancel()

	resp, err := client.Get(ctx, &kuksa.GetRequest{
		Entries: []*kuksa.EntryRequest{
			{
				Path:   path,
				View:   kuksa.View_VIEW_CURRENT_VALUE,
				Fields: []kuksa.Field{kuksa.Field_FIELD_VALUE},
			},
		},
	})
	if err != nil {
		return nil, err
	}
	if len(resp.Entries) == 0 {
		return nil, fmt.Errorf("no entries returned for %s", path)
	}
	return resp.Entries[0], nil
}

// setSignalBool sets a boolean signal value in DATA_BROKER.
func setSignalBool(t *testing.T, client kuksa.VALClient, path string, val bool) {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), rpcTimeout)
	defer cancel()

	_, err := client.Set(ctx, &kuksa.SetRequest{
		Updates: []*kuksa.EntryUpdate{
			{
				Entry: &kuksa.DataEntry{
					Path: path,
					Value: &kuksa.Datapoint{
						Value: &kuksa.Datapoint_BoolValue{BoolValue: val},
					},
				},
				Fields: []kuksa.Field{kuksa.Field_FIELD_VALUE},
			},
		},
	})
	if err != nil {
		t.Fatalf("failed to set %s to %v: %v", path, val, err)
	}
}

// getBoolValue reads a boolean signal value from DATA_BROKER.
func getBoolValue(t *testing.T, client kuksa.VALClient, path string) (bool, bool) {
	t.Helper()
	entry, err := getSignalValue(t, client, path)
	if err != nil {
		t.Fatalf("failed to get %s: %v", path, err)
	}
	if entry.Value == nil {
		return false, false
	}
	return entry.Value.GetBoolValue(), true
}

// waitForBoolSignal polls a boolean signal until it matches the expected value
// or the timeout expires.
func waitForBoolSignal(t *testing.T, client kuksa.VALClient, path string, expected bool, timeout time.Duration) {
	t.Helper()
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		val, ok := getBoolValue(t, client, path)
		if ok && val == expected {
			return
		}
		time.Sleep(200 * time.Millisecond)
	}
	t.Fatalf("timed out waiting for %s to become %v", path, expected)
}

// ---------------------------------------------------------------------------
// Log capture
// ---------------------------------------------------------------------------

// logCapture is a thread-safe log buffer that captures process output
// line by line and supports waiting for specific log lines.
type logCapture struct {
	mu    sync.Mutex
	lines []string
	cond  *sync.Cond
}

func newLogCapture() *logCapture {
	lc := &logCapture{}
	lc.cond = sync.NewCond(&lc.mu)
	return lc
}

func (lc *logCapture) appendLine(line string) {
	lc.mu.Lock()
	lc.lines = append(lc.lines, line)
	lc.mu.Unlock()
	lc.cond.Broadcast()
}

// contains checks if any log line contains the given substring.
func (lc *logCapture) contains(substr string) bool {
	lc.mu.Lock()
	defer lc.mu.Unlock()
	for _, line := range lc.lines {
		if strings.Contains(line, substr) {
			return true
		}
	}
	return false
}

// waitFor blocks until a log line containing substr appears or the timeout
// expires. Returns true if found, false on timeout.
func (lc *logCapture) waitFor(substr string, timeout time.Duration) bool {
	deadline := time.Now().Add(timeout)
	lc.mu.Lock()
	defer lc.mu.Unlock()

	for {
		for _, line := range lc.lines {
			if strings.Contains(line, substr) {
				return true
			}
		}
		remaining := time.Until(deadline)
		if remaining <= 0 {
			return false
		}
		timer := time.AfterFunc(remaining, func() {
			lc.cond.Broadcast()
		})
		lc.cond.Wait()
		timer.Stop()
	}
}

// allLines returns a copy of all captured log lines.
func (lc *logCapture) allLines() []string {
	lc.mu.Lock()
	defer lc.mu.Unlock()
	cp := make([]string, len(lc.lines))
	copy(cp, lc.lines)
	return cp
}

// ---------------------------------------------------------------------------
// Adaptor process management
// ---------------------------------------------------------------------------

// adaptorProcess represents a running parking-operator-adaptor process.
type adaptorProcess struct {
	cmd  *exec.Cmd
	logs *logCapture
}

// startAdaptor starts the parking-operator-adaptor binary with the given
// configuration and waits for the "ready" log line. The process is
// automatically killed when the test completes.
func startAdaptor(t *testing.T, binaryPath string, env map[string]string) *adaptorProcess {
	t.Helper()

	cmd := exec.Command(binaryPath)
	envSlice := []string{
		fmt.Sprintf("PATH=%s", os.Getenv("PATH")),
		fmt.Sprintf("HOME=%s", os.Getenv("HOME")),
	}
	for k, v := range env {
		envSlice = append(envSlice, fmt.Sprintf("%s=%s", k, v))
	}
	cmd.Env = envSlice

	// Capture both stdout and stderr: tracing_subscriber::fmt writes
	// to stdout by default; error output may go to stderr.
	stdoutPipe, err := cmd.StdoutPipe()
	if err != nil {
		t.Fatalf("failed to create stdout pipe: %v", err)
	}
	stderrPipe, err := cmd.StderrPipe()
	if err != nil {
		t.Fatalf("failed to create stderr pipe: %v", err)
	}

	logs := newLogCapture()

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start parking-operator-adaptor: %v", err)
	}

	// Read stdout and stderr in background, capturing log lines.
	go func() {
		scanner := bufio.NewScanner(stdoutPipe)
		for scanner.Scan() {
			logs.appendLine(scanner.Text())
		}
	}()
	go func() {
		scanner := bufio.NewScanner(stderrPipe)
		for scanner.Scan() {
			logs.appendLine(scanner.Text())
		}
	}()

	// Wait for the service to become ready.
	if !logs.waitFor("ready", serviceReadyTimeout) {
		_ = cmd.Process.Kill()
		_ = cmd.Wait()
		allLogs := strings.Join(logs.allLines(), "\n")
		t.Fatalf("parking-operator-adaptor did not become ready within %v\nLogs:\n%s",
			serviceReadyTimeout, allLogs)
	}

	// Register cleanup to kill the process when the test finishes.
	t.Cleanup(func() {
		if cmd.Process != nil {
			_ = cmd.Process.Kill()
			_ = cmd.Wait()
		}
	})

	return &adaptorProcess{cmd: cmd, logs: logs}
}

// startAdaptorNoWait starts the adaptor without waiting for the "ready"
// log line. Used for tests that expect the process to fail on startup.
func startAdaptorNoWait(t *testing.T, binaryPath string, env map[string]string) *adaptorProcess {
	t.Helper()

	cmd := exec.Command(binaryPath)
	envSlice := []string{
		fmt.Sprintf("PATH=%s", os.Getenv("PATH")),
		fmt.Sprintf("HOME=%s", os.Getenv("HOME")),
	}
	for k, v := range env {
		envSlice = append(envSlice, fmt.Sprintf("%s=%s", k, v))
	}
	cmd.Env = envSlice

	stdoutPipe, err := cmd.StdoutPipe()
	if err != nil {
		t.Fatalf("failed to create stdout pipe: %v", err)
	}
	stderrPipe, err := cmd.StderrPipe()
	if err != nil {
		t.Fatalf("failed to create stderr pipe: %v", err)
	}

	logs := newLogCapture()

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start parking-operator-adaptor: %v", err)
	}

	go func() {
		scanner := bufio.NewScanner(stdoutPipe)
		for scanner.Scan() {
			logs.appendLine(scanner.Text())
		}
	}()
	go func() {
		scanner := bufio.NewScanner(stderrPipe)
		for scanner.Scan() {
			logs.appendLine(scanner.Text())
		}
	}()

	t.Cleanup(func() {
		if cmd.Process != nil {
			_ = cmd.Process.Kill()
			_ = cmd.Wait()
		}
	})

	return &adaptorProcess{cmd: cmd, logs: logs}
}

// ---------------------------------------------------------------------------
// Mock PARKING_OPERATOR HTTP server
// ---------------------------------------------------------------------------

// mockOperatorServer is a configurable mock HTTP server that simulates the
// PARKING_OPERATOR REST API. It captures requests and returns configurable
// responses for /parking/start and /parking/stop endpoints.
type mockOperatorServer struct {
	server   *httptest.Server
	mu       sync.Mutex
	requests []capturedRequest
	// startCallCount tracks the number of POST /parking/start calls.
	startCallCount int
	// stopCallCount tracks the number of POST /parking/stop calls.
	stopCallCount int
}

type capturedRequest struct {
	Method string
	Path   string
	Body   string
}

// newMockOperator creates a mock PARKING_OPERATOR that returns realistic
// responses for start and stop endpoints.
func newMockOperator(t *testing.T) *mockOperatorServer {
	t.Helper()

	mock := &mockOperatorServer{}

	mock.server = httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		body, _ := io.ReadAll(r.Body)
		mock.mu.Lock()
		mock.requests = append(mock.requests, capturedRequest{
			Method: r.Method,
			Path:   r.URL.Path,
			Body:   string(body),
		})

		w.Header().Set("Content-Type", "application/json")

		switch {
		case r.Method == "POST" && r.URL.Path == "/parking/start":
			mock.startCallCount++
			// Parse zone_id from request for a realistic session_id
			var reqBody map[string]any
			_ = json.Unmarshal(body, &reqBody)
			sessionID := fmt.Sprintf("sess-%d", mock.startCallCount)

			mock.mu.Unlock()

			w.WriteHeader(200)
			resp := map[string]any{
				"session_id": sessionID,
				"status":     "active",
				"rate": map[string]any{
					"type":     "per_hour",
					"amount":   2.50,
					"currency": "EUR",
				},
			}
			json.NewEncoder(w).Encode(resp)

		case r.Method == "POST" && r.URL.Path == "/parking/stop":
			mock.stopCallCount++
			var reqBody map[string]any
			_ = json.Unmarshal(body, &reqBody)
			sessionID, _ := reqBody["session_id"].(string)
			if sessionID == "" {
				sessionID = "unknown"
			}

			mock.mu.Unlock()

			w.WriteHeader(200)
			resp := map[string]any{
				"session_id":       sessionID,
				"status":           "completed",
				"duration_seconds": 3600,
				"total_amount":     2.50,
				"currency":         "EUR",
			}
			json.NewEncoder(w).Encode(resp)

		default:
			mock.mu.Unlock()
			w.WriteHeader(404)
			fmt.Fprintf(w, `{"error":"not found"}`)
		}
	}))

	t.Cleanup(func() { mock.server.Close() })
	return mock
}

func (m *mockOperatorServer) URL() string {
	return m.server.URL
}

func (m *mockOperatorServer) getStartCallCount() int {
	m.mu.Lock()
	defer m.mu.Unlock()
	return m.startCallCount
}

func (m *mockOperatorServer) getStopCallCount() int {
	m.mu.Lock()
	defer m.mu.Unlock()
	return m.stopCallCount
}

func (m *mockOperatorServer) getRequests() []capturedRequest {
	m.mu.Lock()
	defer m.mu.Unlock()
	cp := make([]capturedRequest, len(m.requests))
	copy(cp, m.requests)
	return cp
}

// waitForStartCalls waits until the mock has received at least `count` start
// calls or the timeout expires.
func (m *mockOperatorServer) waitForStartCalls(t *testing.T, count int, timeout time.Duration) {
	t.Helper()
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		if m.getStartCallCount() >= count {
			return
		}
		time.Sleep(200 * time.Millisecond)
	}
	t.Fatalf("timed out waiting for %d start calls (got %d)", count, m.getStartCallCount())
}

// waitForStopCalls waits until the mock has received at least `count` stop
// calls or the timeout expires.
func (m *mockOperatorServer) waitForStopCalls(t *testing.T, count int, timeout time.Duration) {
	t.Helper()
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		if m.getStopCallCount() >= count {
			return
		}
		time.Sleep(200 * time.Millisecond)
	}
	t.Fatalf("timed out waiting for %d stop calls (got %d)", count, m.getStopCallCount())
}

// ---------------------------------------------------------------------------
// Port helpers
// ---------------------------------------------------------------------------

// getFreePort finds an available TCP port.
func getFreePort(t *testing.T) int {
	t.Helper()
	lis, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to find free port: %v", err)
	}
	port := lis.Addr().(*net.TCPAddr).Port
	lis.Close()
	return port
}

// ---------------------------------------------------------------------------
// Common adaptor launch helper
// ---------------------------------------------------------------------------

// adaptorEnv builds the standard environment for launching the adaptor with
// the given mock operator URL, DATA_BROKER address, and gRPC port.
func adaptorEnv(operatorURL, dataBrokerAddr string, grpcPort int) map[string]string {
	return map[string]string{
		"PARKING_OPERATOR_URL": operatorURL,
		"DATA_BROKER_ADDR":    dataBrokerAddr,
		"GRPC_PORT":           fmt.Sprintf("%d", grpcPort),
		"VEHICLE_ID":          "DEMO-VIN-001",
		"ZONE_ID":             "zone-demo-1",
	}
}
