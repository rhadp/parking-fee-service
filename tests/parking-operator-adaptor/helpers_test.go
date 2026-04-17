// Package parking_operator_adaptor_test provides integration tests for the
// PARKING_OPERATOR_ADAPTOR Rust binary.
//
// Tests use:
//   - A mock DATA_BROKER (Go gRPC server implementing kuksa.VALService)
//   - A mock PARKING_OPERATOR (Go HTTP server for /parking/start and /parking/stop)
//
// This makes all integration tests self-contained and independent of any
// running infrastructure.  TestDatabrokerUnreachable is the only test that
// explicitly tests the failure case when the broker is not available.
package parking_operator_adaptor_test

import (
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
	"runtime"
	"strings"
	"sync"
	"sync/atomic"
	"testing"
	"time"

	kuksapb "github.com/sdv-demo/tests/parking-operator-adaptor/pb/kuksa/kuksa"
	parkingpb "github.com/sdv-demo/tests/parking-operator-adaptor/pb/parking"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
	"google.golang.org/grpc/metadata"
)

// ── Repo root ──────────────────────────────────────────────────────────────

// repoRoot returns the absolute path to the repository root.
func repoRoot(t *testing.T) string {
	t.Helper()
	_, filename, _, ok := runtime.Caller(0)
	if !ok {
		t.Fatal("runtime.Caller failed")
	}
	// filename is .../tests/parking-operator-adaptor/helpers_test.go
	root := filepath.Join(filepath.Dir(filename), "..", "..")
	abs, err := filepath.Abs(root)
	if err != nil {
		t.Fatalf("filepath.Abs failed: %v", err)
	}
	return abs
}

// ── Binary management ──────────────────────────────────────────────────────

// buildAdaptorBinary compiles the parking-operator-adaptor Rust binary and
// returns the path to the resulting executable.  Build results are cached for
// the test run via a package-level sync.Once.
var (
	builtBinary     string
	buildBinaryOnce sync.Once
	buildBinaryErr  error
)

// getAdaptorBinary returns the path to the compiled adaptor binary, building
// it if necessary.
func getAdaptorBinary(t *testing.T) string {
	t.Helper()
	buildBinaryOnce.Do(func() {
		root := repoRoot(t)
		rhivosDir := filepath.Join(root, "rhivos")
		cmd := exec.Command("cargo", "build", "-p", "parking-operator-adaptor")
		cmd.Dir = rhivosDir
		cmd.Env = os.Environ()
		if data, err := cmd.CombinedOutput(); err != nil {
			buildBinaryErr = fmt.Errorf("cargo build failed: %v\n%s", err, data)
			return
		}
		builtBinary = filepath.Join(rhivosDir, "target", "debug", "parking-operator-adaptor")
	})
	if buildBinaryErr != nil {
		t.Fatalf("failed to build adaptor binary: %v", buildBinaryErr)
	}
	return builtBinary
}

// ── Free port helper ───────────────────────────────────────────────────────

// findFreePort returns an available TCP port on localhost.
func findFreePort(t *testing.T) int {
	t.Helper()
	l, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("findFreePort: %v", err)
	}
	port := l.Addr().(*net.TCPAddr).Port
	l.Close()
	return port
}

// ── Mock DATA_BROKER ───────────────────────────────────────────────────────

// mockDataBroker implements kuksa.VALService for integration testing.
// It records Set calls and allows the test to push lock events via PushLockEvent.
type mockDataBroker struct {
	kuksapb.UnimplementedVALServiceServer

	mu         sync.Mutex
	signals    map[string]bool // path → bool value
	subscribers []chan bool      // subscribers for IsLocked
	setBoolCalls []setBoolCall   // recorded Set calls
}

type setBoolCall struct {
	Signal string
	Value  bool
}

// newMockDataBroker creates a mock DATA_BROKER and starts it on a random port.
// The gRPC server is shut down at test cleanup.
// Returns the broker and its address ("http://localhost:PORT").
func newMockDataBroker(t *testing.T) (*mockDataBroker, string) {
	t.Helper()

	lis, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("newMockDataBroker: listen: %v", err)
	}

	broker := &mockDataBroker{
		signals: make(map[string]bool),
	}

	s := grpc.NewServer()
	kuksapb.RegisterVALServiceServer(s, broker)

	t.Cleanup(func() {
		s.GracefulStop()
		lis.Close()
	})

	go func() {
		_ = s.Serve(lis)
	}()

	port := lis.Addr().(*net.TCPAddr).Port
	addr := fmt.Sprintf("http://localhost:%d", port)
	return broker, addr
}

// Set handles SetRequest from the adaptor (e.g. setting SessionActive).
func (b *mockDataBroker) Set(_ context.Context, req *kuksapb.SetRequest) (*kuksapb.SetResponse, error) {
	b.mu.Lock()
	defer b.mu.Unlock()

	for _, entry := range req.Updates {
		if dp := entry.Value; dp != nil {
			if bv, ok := dp.Value.(*kuksapb.Datapoint_BoolValue); ok {
				b.signals[entry.Path] = bv.BoolValue
				b.setBoolCalls = append(b.setBoolCalls, setBoolCall{
					Signal: entry.Path,
					Value:  bv.BoolValue,
				})
			}
		}
	}
	return &kuksapb.SetResponse{}, nil
}

// Subscribe handles SubscribeRequest from the adaptor (for IsLocked signal).
// It sends an initial empty response to unblock the client's streaming call,
// then forwards lock events pushed via PushLockEvent.
func (b *mockDataBroker) Subscribe(req *kuksapb.SubscribeRequest, stream kuksapb.VALService_SubscribeServer) error {
	// Send initial headers immediately so the gRPC client's Subscribe().await
	// completes without waiting for the first data message.
	if err := stream.SendHeader(metadata.MD{}); err != nil {
		return err
	}

	// Create a channel for this subscriber.
	ch := make(chan bool, 64)
	b.mu.Lock()
	b.subscribers = append(b.subscribers, ch)
	b.mu.Unlock()

	// Clean up on exit.
	defer func() {
		b.mu.Lock()
		for i, s := range b.subscribers {
			if s == ch {
				b.subscribers = append(b.subscribers[:i], b.subscribers[i+1:]...)
				break
			}
		}
		b.mu.Unlock()
		close(ch)
	}()

	// Stream incoming events until context is cancelled.
	for {
		select {
		case val, ok := <-ch:
			if !ok {
				return nil
			}
			resp := &kuksapb.SubscribeResponse{
				Updates: []*kuksapb.DataEntry{
					{
						Path: "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked",
						Value: &kuksapb.Datapoint{
							Value: &kuksapb.Datapoint_BoolValue{BoolValue: val},
						},
					},
				},
			}
			if err := stream.Send(resp); err != nil {
				return err
			}
		case <-stream.Context().Done():
			return nil
		}
	}
}

// PushLockEvent sends an IsLocked value update to all subscribers.
func (b *mockDataBroker) PushLockEvent(locked bool) {
	b.mu.Lock()
	subs := make([]chan bool, len(b.subscribers))
	copy(subs, b.subscribers)
	b.mu.Unlock()

	for _, ch := range subs {
		select {
		case ch <- locked:
		default: // non-blocking; subscriber not keeping up
		}
	}
}

// getSignal returns the most recently Set value for the given signal path.
// Returns (false, false) if never set.
func (b *mockDataBroker) getSignal(path string) (value bool, exists bool) {
	b.mu.Lock()
	defer b.mu.Unlock()
	v, ok := b.signals[path]
	return v, ok
}

// hasSubscribers reports whether any subscribers are connected.
func (b *mockDataBroker) hasSubscribers() bool {
	b.mu.Lock()
	defer b.mu.Unlock()
	return len(b.subscribers) > 0
}

// ── Adaptor process management ─────────────────────────────────────────────

// adaptorProcess wraps a running adaptor process.
type adaptorProcess struct {
	cmd    *exec.Cmd
	port   int
	output strings.Builder
	mu     sync.Mutex
}

// startAdaptor starts the parking-operator-adaptor binary with the given
// environment variables.  The GRPC_PORT is auto-assigned if not provided.
// The process is killed at test cleanup.
func startAdaptor(t *testing.T, binary string, env ...string) *adaptorProcess {
	t.Helper()

	port := findFreePort(t)
	ap := &adaptorProcess{port: port}

	ap.cmd = exec.Command(binary, "serve")
	ap.cmd.Env = append(os.Environ(),
		fmt.Sprintf("GRPC_PORT=%d", port),
	)
	ap.cmd.Env = append(ap.cmd.Env, env...)

	// Capture combined stdout+stderr.
	pr, pw, err := os.Pipe()
	if err != nil {
		t.Fatalf("startAdaptor: create pipe: %v", err)
	}
	ap.cmd.Stdout = pw
	ap.cmd.Stderr = pw

	go func() {
		buf := make([]byte, 4096)
		for {
			n, readErr := pr.Read(buf)
			if n > 0 {
				ap.mu.Lock()
				ap.output.Write(buf[:n])
				ap.mu.Unlock()
			}
			if readErr != nil {
				return
			}
		}
	}()

	if err := ap.cmd.Start(); err != nil {
		pw.Close()
		pr.Close()
		t.Fatalf("startAdaptor: start process: %v", err)
	}
	pw.Close()

	t.Cleanup(func() {
		if ap.cmd.ProcessState == nil {
			ap.cmd.Process.Kill() //nolint
		}
		pr.Close()
	})

	return ap
}

// waitForGRPCReady polls the adaptor gRPC port until a connection can be
// established or the timeout expires.
func waitForGRPCReady(t *testing.T, ap *adaptorProcess, timeout time.Duration) {
	t.Helper()
	addr := fmt.Sprintf("localhost:%d", ap.port)
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		conn, err := net.DialTimeout("tcp", addr, 200*time.Millisecond)
		if err == nil {
			conn.Close()
			return
		}
		time.Sleep(100 * time.Millisecond)
	}
	t.Logf("adaptor output so far:\n%s", ap.getOutput())
	t.Fatalf("adaptor gRPC server not ready on %s within %s", addr, timeout)
}

// waitForExit waits for the adaptor process to exit and returns its exit code.
func waitForExit(ap *adaptorProcess, timeout time.Duration) (int, error) {
	done := make(chan error, 1)
	go func() { done <- ap.cmd.Wait() }()

	select {
	case err := <-done:
		if err == nil {
			return 0, nil
		}
		if exit, ok := err.(*exec.ExitError); ok {
			return exit.ExitCode(), nil
		}
		return -1, err
	case <-time.After(timeout):
		return -1, fmt.Errorf("timeout waiting for process to exit")
	}
}

// getOutput returns the captured output of the adaptor process.
func (ap *adaptorProcess) getOutput() string {
	ap.mu.Lock()
	defer ap.mu.Unlock()
	return ap.output.String()
}

// ── Mock PARKING_OPERATOR ──────────────────────────────────────────────────

// mockOperatorServer is a simple HTTP server that simulates the PARKING_OPERATOR
// REST API for /parking/start and /parking/stop endpoints.
type mockOperatorServer struct {
	*httptest.Server
	startCount atomic.Int32
	stopCount  atomic.Int32
	mu         sync.Mutex
	sessionID  string
}

// startMockOperator creates and starts a mock PARKING_OPERATOR HTTP server.
func startMockOperator(t *testing.T) *mockOperatorServer {
	t.Helper()
	ms := &mockOperatorServer{
		sessionID: "test-session-001",
	}

	ms.Server = httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		body, _ := io.ReadAll(r.Body)
		w.Header().Set("Content-Type", "application/json")

		switch r.URL.Path {
		case "/parking/start":
			ms.startCount.Add(1)
			ms.mu.Lock()
			sessID := ms.sessionID
			ms.mu.Unlock()
			json.NewEncoder(w).Encode(map[string]any{ //nolint
				"session_id": sessID,
				"status":     "active",
				"rate": map[string]any{
					"type":     "per_hour",
					"amount":   2.50,
					"currency": "EUR",
				},
			})
		case "/parking/stop":
			ms.stopCount.Add(1)
			var req map[string]any
			json.Unmarshal(body, &req) //nolint
			sessID, _ := req["session_id"].(string)
			if sessID == "" {
				ms.mu.Lock()
				sessID = ms.sessionID
				ms.mu.Unlock()
			}
			json.NewEncoder(w).Encode(map[string]any{ //nolint
				"session_id":       sessID,
				"status":           "completed",
				"duration_seconds": 3600,
				"total_amount":     2.50,
				"currency":         "EUR",
			})
		default:
			http.Error(w, "not found", http.StatusNotFound)
		}
	}))
	t.Cleanup(ms.Close)
	return ms
}

// ── gRPC client helpers ────────────────────────────────────────────────────

// newParkingClient creates a gRPC client connected to the adaptor.
func newParkingClient(t *testing.T, ap *adaptorProcess) parkingpb.ParkingAdaptorClient {
	t.Helper()
	addr := fmt.Sprintf("localhost:%d", ap.port)
	conn, err := grpc.NewClient(addr, grpc.WithTransportCredentials(insecure.NewCredentials()))
	if err != nil {
		t.Fatalf("newParkingClient: %v", err)
	}
	t.Cleanup(func() { conn.Close() })
	return parkingpb.NewParkingAdaptorClient(conn)
}

// grpcGetStatus calls GetStatus on the adaptor and returns the response.
func grpcGetStatus(t *testing.T, client parkingpb.ParkingAdaptorClient) *parkingpb.GetStatusResponse {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	resp, err := client.GetStatus(ctx, &parkingpb.GetStatusRequest{})
	if err != nil {
		t.Fatalf("GetStatus: %v", err)
	}
	return resp
}

// ── Utilities ─────────────────────────────────────────────────────────────

// waitForCondition polls condition() until it returns true or timeout expires.
func waitForCondition(t *testing.T, condition func() bool, timeout time.Duration, desc string) {
	t.Helper()
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		if condition() {
			return
		}
		time.Sleep(100 * time.Millisecond)
	}
	t.Fatalf("condition %q not met within %s", desc, timeout)
}

// waitForAdaptorSubscribed waits until the mock DATA_BROKER has at least one
// subscriber (meaning the adaptor has connected and subscribed to IsLocked).
func waitForAdaptorSubscribed(t *testing.T, broker *mockDataBroker, timeout time.Duration) {
	t.Helper()
	waitForCondition(t, broker.hasSubscribers, timeout, "adaptor subscribed to DATA_BROKER")
}
