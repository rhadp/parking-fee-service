// Package parkingoperatoradaptor_test contains integration tests for the
// PARKING_OPERATOR_ADAPTOR component. Tests require a running DATA_BROKER
// container accessible at localhost:55556 and the parking-operator-adaptor
// Rust binary.
//
// Start the databroker before running tests:
//
//	cd deployments && podman compose up -d kuksa-databroker
package parkingoperatoradaptor_test

import (
	"bufio"
	"context"
	"encoding/json"
	"fmt"
	"net"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"sync"
	"sync/atomic"
	"testing"
	"time"

	adaptorpb "parking-fee-service/tests/parking-operator-adaptor/adaptorpb"
	kuksapb "parking-fee-service/tests/parking-operator-adaptor/kuksa"

	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

// Endpoints and timeouts.
const (
	databrokerAddr = "localhost:55556"
	connectTimeout = 5 * time.Second
	opTimeout      = 5 * time.Second
	readyTimeout   = 30 * time.Second

	// VSS signal paths.
	signalIsLocked     = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked"
	signalSessionActive = "Vehicle.Parking.SessionActive"
)

// findRepoRoot walks up from the test working directory until it finds .git.
func findRepoRoot(t *testing.T) string {
	t.Helper()
	dir, err := os.Getwd()
	if err != nil {
		t.Fatalf("findRepoRoot: failed to get working directory: %v", err)
	}
	for {
		if _, err := os.Stat(filepath.Join(dir, ".git")); err == nil {
			return dir
		}
		parent := filepath.Dir(dir)
		if parent == dir {
			t.Fatal("findRepoRoot: .git not found; are tests run from within the repo?")
		}
		dir = parent
	}
}

// requirePodman skips the test if podman is not available.
func requirePodman(t *testing.T) {
	t.Helper()
	if _, err := exec.LookPath("podman"); err != nil {
		t.Skip("podman not available, skipping integration test")
	}
}

// dialDatabroker returns a gRPC ClientConn connected to the DATA_BROKER.
func dialDatabroker(t *testing.T) *grpc.ClientConn {
	t.Helper()
	conn, err := grpc.NewClient(databrokerAddr,
		grpc.WithTransportCredentials(insecure.NewCredentials()),
	)
	if err != nil {
		t.Fatalf("dialDatabroker: failed to create gRPC client for %s: %v", databrokerAddr, err)
	}
	t.Cleanup(func() { conn.Close() })
	return conn
}

// valClient wraps a connection in a Kuksa VAL gRPC client.
func valClient(conn *grpc.ClientConn) kuksapb.VALClient {
	return kuksapb.NewVALClient(conn)
}

// opCtx returns a context with the standard operation timeout.
func opCtx() (context.Context, context.CancelFunc) {
	return context.WithTimeout(context.Background(), opTimeout)
}

// ensureDatabrokerReachable verifies the databroker is reachable, skipping if not.
func ensureDatabrokerReachable(t *testing.T) kuksapb.VALClient {
	t.Helper()
	conn := dialDatabroker(t)
	client := valClient(conn)

	ctx, cancel := context.WithTimeout(context.Background(), connectTimeout)
	defer cancel()

	_, err := client.GetServerInfo(ctx, &kuksapb.GetServerInfoRequest{})
	if err != nil {
		t.Skipf("DATA_BROKER not reachable at %s: %v (skipping integration test)", databrokerAddr, err)
	}
	return client
}

// setSignalBool writes a boolean value to the named VSS signal via DATA_BROKER.
func setSignalBool(t *testing.T, client kuksapb.VALClient, path string, val bool) {
	t.Helper()
	ctx, cancel := opCtx()
	defer cancel()
	_, err := client.PublishValue(ctx, &kuksapb.PublishValueRequest{
		SignalId: &kuksapb.SignalID{Signal: &kuksapb.SignalID_Path{Path: path}},
		DataPoint: &kuksapb.Datapoint{
			Value: &kuksapb.Value{TypedValue: &kuksapb.Value_Bool{Bool: val}},
		},
	})
	if err != nil {
		t.Fatalf("PublishValue(%s, bool=%v): gRPC error: %v", path, val, err)
	}
}

// getSignalBool reads a boolean signal value from DATA_BROKER. Returns (value, hasValue).
func getSignalBool(t *testing.T, client kuksapb.VALClient, path string) (bool, bool) {
	t.Helper()
	ctx, cancel := opCtx()
	defer cancel()
	resp, err := client.GetValue(ctx, &kuksapb.GetValueRequest{
		SignalId: &kuksapb.SignalID{Signal: &kuksapb.SignalID_Path{Path: path}},
	})
	if err != nil {
		t.Fatalf("GetValue(%s): gRPC error: %v", path, err)
	}
	if resp.DataPoint == nil || resp.DataPoint.Value == nil {
		return false, false
	}
	bv, ok := resp.DataPoint.Value.TypedValue.(*kuksapb.Value_Bool)
	if !ok {
		return false, false
	}
	return bv.Bool, true
}

// waitForSignalBool polls a boolean signal until it matches the expected value or timeout.
func waitForSignalBool(t *testing.T, client kuksapb.VALClient, path string, expected bool, timeout time.Duration) {
	t.Helper()
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		val, ok := getSignalBool(t, client, path)
		if ok && val == expected {
			return
		}
		time.Sleep(200 * time.Millisecond)
	}
	t.Fatalf("timed out waiting for %s == %v", path, expected)
}

// resetSignals clears key signals to a known state.
func resetSignals(t *testing.T, client kuksapb.VALClient) {
	t.Helper()
	setSignalBool(t, client, signalIsLocked, false)
	setSignalBool(t, client, signalSessionActive, false)
}

// --- Adaptor gRPC client helpers ---

// dialAdaptor returns a gRPC ClientConn connected to the parking-operator-adaptor.
func dialAdaptor(t *testing.T, port int) *grpc.ClientConn {
	t.Helper()
	addr := fmt.Sprintf("localhost:%d", port)
	conn, err := grpc.NewClient(addr,
		grpc.WithTransportCredentials(insecure.NewCredentials()),
	)
	if err != nil {
		t.Fatalf("dialAdaptor: failed to create gRPC client for %s: %v", addr, err)
	}
	t.Cleanup(func() { conn.Close() })
	return conn
}

// adaptorClient wraps a connection in a ParkingAdaptorService gRPC client.
func adaptorClient(conn *grpc.ClientConn) adaptorpb.ParkingAdaptorServiceClient {
	return adaptorpb.NewParkingAdaptorServiceClient(conn)
}

// --- Binary build and process management ---

// buildAdaptor builds the parking-operator-adaptor Rust binary using cargo
// and returns the path to the compiled binary.
func buildAdaptor(t *testing.T) string {
	t.Helper()
	root := findRepoRoot(t)
	crateDir := filepath.Join(root, "rhivos", "parking-operator-adaptor")

	cmd := exec.Command("cargo", "build", "--quiet")
	cmd.Dir = crateDir
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("failed to build parking-operator-adaptor: %v\noutput: %s", err, string(out))
	}

	binary := filepath.Join(root, "rhivos", "target", "debug", "parking-operator-adaptor")
	if _, err := os.Stat(binary); err != nil {
		t.Fatalf("parking-operator-adaptor binary not found at %s", binary)
	}
	return binary
}

// adaptorProcess manages a running parking-operator-adaptor instance.
type adaptorProcess struct {
	cmd    *exec.Cmd
	mu     sync.Mutex
	logs   []string
	cancel context.CancelFunc
}

// freePort finds an available TCP port.
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

// startAdaptor starts the parking-operator-adaptor binary with the given
// configuration and waits for the "parking-operator-adaptor ready" log line.
func startAdaptor(t *testing.T, binary string, grpcPort int, operatorURL string) *adaptorProcess {
	t.Helper()

	ctx, cancel := context.WithCancel(context.Background())
	cmd := exec.CommandContext(ctx, binary)
	cmd.Env = append(os.Environ(),
		fmt.Sprintf("GRPC_PORT=%d", grpcPort),
		fmt.Sprintf("DATA_BROKER_ADDR=http://%s", databrokerAddr),
		fmt.Sprintf("PARKING_OPERATOR_URL=%s", operatorURL),
		"VEHICLE_ID=DEMO-VIN-001",
		"ZONE_ID=zone-demo-1",
		"RUST_LOG=info",
	)

	// tracing-subscriber fmt::init() writes to stdout by default.
	stdout, err := cmd.StdoutPipe()
	if err != nil {
		cancel()
		t.Fatalf("failed to pipe stdout: %v", err)
	}

	proc := &adaptorProcess{
		cmd:    cmd,
		cancel: cancel,
	}

	if err := cmd.Start(); err != nil {
		cancel()
		t.Fatalf("failed to start parking-operator-adaptor: %v", err)
	}

	// Read logs in background.
	ready := make(chan struct{})
	readyClosed := false
	go func() {
		scanner := bufio.NewScanner(stdout)
		for scanner.Scan() {
			line := scanner.Text()
			proc.mu.Lock()
			proc.logs = append(proc.logs, line)
			proc.mu.Unlock()
			if !readyClosed && strings.Contains(line, "parking-operator-adaptor ready") {
				readyClosed = true
				close(ready)
			}
		}
		if !readyClosed {
			readyClosed = true
			close(ready)
		}
	}()

	// Wait for ready or timeout.
	select {
	case <-ready:
		proc.mu.Lock()
		found := false
		for _, l := range proc.logs {
			if strings.Contains(l, "parking-operator-adaptor ready") {
				found = true
				break
			}
		}
		proc.mu.Unlock()
		if !found {
			cancel()
			_ = cmd.Wait()
			proc.mu.Lock()
			allLogs := strings.Join(proc.logs, "\n")
			proc.mu.Unlock()
			t.Fatalf("parking-operator-adaptor exited before becoming ready. Logs:\n%s", allLogs)
		}
	case <-time.After(readyTimeout):
		cancel()
		_ = cmd.Wait()
		proc.mu.Lock()
		allLogs := strings.Join(proc.logs, "\n")
		proc.mu.Unlock()
		t.Fatalf("parking-operator-adaptor did not become ready within %v. Logs:\n%s", readyTimeout, allLogs)
	}

	t.Cleanup(func() {
		proc.stop()
	})

	return proc
}

// stop terminates the parking-operator-adaptor process.
func (p *adaptorProcess) stop() {
	p.cancel()
	_ = p.cmd.Wait()
}

// getLogs returns all captured log lines.
func (p *adaptorProcess) getLogs() []string {
	p.mu.Lock()
	defer p.mu.Unlock()
	cp := make([]string, len(p.logs))
	copy(cp, p.logs)
	return cp
}

// --- Mock PARKING_OPERATOR HTTP server ---

// mockOperatorServer is a simple HTTP server that implements the
// PARKING_OPERATOR REST API for integration testing.
type mockOperatorServer struct {
	server       *http.Server
	listener     net.Listener
	mu           sync.Mutex
	startCount   int32
	stopCount    int32
	lastStartReq map[string]interface{}
	lastStopReq  map[string]interface{}
}

// startMockOperator starts a mock PARKING_OPERATOR HTTP server and returns it.
func startMockOperator(t *testing.T) *mockOperatorServer {
	t.Helper()

	listener, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to listen for mock operator: %v", err)
	}

	mock := &mockOperatorServer{
		listener: listener,
	}

	mux := http.NewServeMux()
	mux.HandleFunc("/parking/start", mock.handleStart)
	mux.HandleFunc("/parking/stop", mock.handleStop)

	mock.server = &http.Server{Handler: mux}

	go func() {
		_ = mock.server.Serve(listener)
	}()

	t.Cleanup(func() {
		mock.server.Close()
	})

	return mock
}

// URL returns the base URL of the mock operator server.
func (m *mockOperatorServer) URL() string {
	return fmt.Sprintf("http://%s", m.listener.Addr().String())
}

// StartCount returns the number of start requests received.
func (m *mockOperatorServer) StartCount() int {
	return int(atomic.LoadInt32(&m.startCount))
}

// StopCount returns the number of stop requests received.
func (m *mockOperatorServer) StopCount() int {
	return int(atomic.LoadInt32(&m.stopCount))
}

func (m *mockOperatorServer) handleStart(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}

	var req map[string]interface{}
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, "bad request", http.StatusBadRequest)
		return
	}

	m.mu.Lock()
	m.lastStartReq = req
	m.mu.Unlock()
	atomic.AddInt32(&m.startCount, 1)

	resp := map[string]interface{}{
		"session_id": fmt.Sprintf("sess-%d", atomic.LoadInt32(&m.startCount)),
		"status":     "active",
		"rate": map[string]interface{}{
			"type":     "per_hour",
			"amount":   2.50,
			"currency": "EUR",
		},
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(resp)
}

func (m *mockOperatorServer) handleStop(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}

	var req map[string]interface{}
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, "bad request", http.StatusBadRequest)
		return
	}

	m.mu.Lock()
	m.lastStopReq = req
	m.mu.Unlock()
	atomic.AddInt32(&m.stopCount, 1)

	resp := map[string]interface{}{
		"session_id":       req["session_id"],
		"status":           "completed",
		"duration_seconds": 3600,
		"total_amount":     2.50,
		"currency":         "EUR",
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(resp)
}

// waitForStartCount polls until the mock operator has received at least count start requests.
func (m *mockOperatorServer) waitForStartCount(t *testing.T, count int, timeout time.Duration) {
	t.Helper()
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		if m.StartCount() >= count {
			return
		}
		time.Sleep(200 * time.Millisecond)
	}
	t.Fatalf("timed out waiting for %d start requests (got %d)", count, m.StartCount())
}

// waitForStopCount polls until the mock operator has received at least count stop requests.
func (m *mockOperatorServer) waitForStopCount(t *testing.T, count int, timeout time.Duration) {
	t.Helper()
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		if m.StopCount() >= count {
			return
		}
		time.Sleep(200 * time.Millisecond)
	}
	t.Fatalf("timed out waiting for %d stop requests (got %d)", count, m.StopCount())
}
