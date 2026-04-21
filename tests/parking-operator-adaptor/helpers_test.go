// Package parkingoperatoradaptortests contains integration tests for the
// PARKING_OPERATOR_ADAPTOR service.
//
// Tests that require a live DATA_BROKER skip gracefully when unavailable.
// Tests that use a mock DATA_BROKER are self-contained and always run when
// cargo is available.
package parkingoperatoradaptortests

import (
	"bytes"
	"context"
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

	kuksapb "github.com/rhadp/parking-fee-service/tests/parking-operator-adaptor/pb/kuksa"
	parkingpb "github.com/rhadp/parking-fee-service/tests/parking-operator-adaptor/pb/parking"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

// ── Signal path constants ─────────────────────────────────────────────────────

const (
	signalIsLocked      = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked"
	signalSessionActive = "Vehicle.Parking.SessionActive"
)

// ── safeBuffer: thread-safe capture of subprocess output ─────────────────────

type safeBuffer struct {
	mu  sync.Mutex
	buf bytes.Buffer
}

func (b *safeBuffer) Write(p []byte) (n int, err error) {
	b.mu.Lock()
	defer b.mu.Unlock()
	return b.buf.Write(p)
}

func (b *safeBuffer) String() string {
	b.mu.Lock()
	defer b.mu.Unlock()
	return b.buf.String()
}

// ── Repo root ─────────────────────────────────────────────────────────────────

// findRepoRoot walks up from the test working directory to find the repo root
// (identified by go.work).
func findRepoRoot(t *testing.T) string {
	t.Helper()
	dir, err := os.Getwd()
	if err != nil {
		t.Fatal(err)
	}
	for {
		if _, err := os.Stat(filepath.Join(dir, "go.work")); err == nil {
			return dir
		}
		parent := filepath.Dir(dir)
		if parent == dir {
			t.Fatal("could not find project root (go.work not found)")
		}
		dir = parent
	}
}

// ── Binary builder ────────────────────────────────────────────────────────────

// buildAdaptorBinary builds the parking-operator-adaptor Rust binary and
// returns its path. Skips the test if cargo is not in PATH.
func buildAdaptorBinary(t *testing.T) string {
	t.Helper()
	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("cargo not in PATH; skipping binary-dependent test")
	}
	root := findRepoRoot(t)
	rhivosDir := filepath.Join(root, "rhivos")
	cmd := exec.Command("cargo", "build", "-p", "parking-operator-adaptor")
	cmd.Dir = rhivosDir
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("cargo build -p parking-operator-adaptor failed:\n%s\nerror: %v", out, err)
	}
	return filepath.Join(rhivosDir, "target", "debug", "parking-operator-adaptor")
}

// ── Adaptor process ───────────────────────────────────────────────────────────

// adaptorProcess manages a running parking-operator-adaptor subprocess.
type adaptorProcess struct {
	cmd    *exec.Cmd
	output *safeBuffer
}

// startAdaptor starts the adaptor binary with the given environment variables.
// Registers a cleanup that kills the process when the test ends.
func startAdaptor(t *testing.T, binPath string, env map[string]string) *adaptorProcess {
	t.Helper()
	buf := &safeBuffer{}
	cmd := exec.Command(binPath, "serve")

	// Inherit the parent environment and add/override with provided values.
	baseEnv := os.Environ()
	for k, v := range env {
		baseEnv = append(baseEnv, k+"="+v)
	}
	cmd.Env = baseEnv
	cmd.Stdout = buf
	cmd.Stderr = buf

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start parking-operator-adaptor: %v", err)
	}

	proc := &adaptorProcess{cmd: cmd, output: buf}

	t.Cleanup(func() {
		if cmd.Process != nil {
			_ = cmd.Process.Kill()
			_ = cmd.Wait()
		}
	})

	return proc
}

// waitForLog polls the process output until the given substring appears or
// timeout is reached. Returns true if found.
func waitForLog(proc *adaptorProcess, substr string, timeout time.Duration) bool {
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		if strings.Contains(proc.output.String(), substr) {
			return true
		}
		time.Sleep(50 * time.Millisecond)
	}
	return strings.Contains(proc.output.String(), substr)
}

// ── Port wait ─────────────────────────────────────────────────────────────────

// waitForPort polls until the TCP port is listening or timeout is reached.
func waitForPort(addr string, timeout time.Duration) error {
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		conn, err := net.DialTimeout("tcp", addr, 100*time.Millisecond)
		if err == nil {
			conn.Close()
			return nil
		}
		time.Sleep(50 * time.Millisecond)
	}
	return fmt.Errorf("timeout waiting for %s to listen", addr)
}

// ── Mock PARKING_OPERATOR HTTP server ─────────────────────────────────────────

// mockParkingOperator is an HTTP test server that simulates the PARKING_OPERATOR
// REST API.
type mockParkingOperator struct {
	ts        *httptest.Server
	mu        sync.Mutex
	startCnt  int
	stopCnt   int
	sessionID string
}

// newMockParkingOperator creates a mock PARKING_OPERATOR HTTP server with
// configurable responses. Returns the mock and registers cleanup.
func newMockParkingOperator(t *testing.T) *mockParkingOperator {
	t.Helper()
	m := &mockParkingOperator{
		sessionID: "mock-session-001",
	}
	m.ts = httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		switch {
		case r.Method == http.MethodPost && r.URL.Path == "/parking/start":
			m.mu.Lock()
			m.startCnt++
			m.mu.Unlock()
			w.WriteHeader(http.StatusOK)
			json.NewEncoder(w).Encode(map[string]any{ //nolint
				"session_id": m.sessionID,
				"status":     "active",
				"rate": map[string]any{
					"type":     "per_hour",
					"amount":   2.50,
					"currency": "EUR",
				},
			})
		case r.Method == http.MethodPost && r.URL.Path == "/parking/stop":
			m.mu.Lock()
			m.stopCnt++
			m.mu.Unlock()
			w.WriteHeader(http.StatusOK)
			json.NewEncoder(w).Encode(map[string]any{ //nolint
				"session_id":       m.sessionID,
				"status":           "completed",
				"duration_seconds": 3600,
				"total_amount":     2.50,
				"currency":         "EUR",
			})
		default:
			w.WriteHeader(http.StatusNotFound)
			fmt.Fprint(w, `{"error":"not found"}`)
		}
	}))
	t.Cleanup(m.ts.Close)
	return m
}

// URL returns the base URL of the mock PARKING_OPERATOR.
func (m *mockParkingOperator) URL() string { return m.ts.URL }

// StartCount returns the number of /parking/start calls received.
func (m *mockParkingOperator) StartCount() int {
	m.mu.Lock()
	defer m.mu.Unlock()
	return m.startCnt
}

// StopCount returns the number of /parking/stop calls received.
func (m *mockParkingOperator) StopCount() int {
	m.mu.Lock()
	defer m.mu.Unlock()
	return m.stopCnt
}

// WaitForStart polls until the start count reaches expected or timeout.
func (m *mockParkingOperator) WaitForStart(expected int, timeout time.Duration) bool {
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		if m.StartCount() >= expected {
			return true
		}
		time.Sleep(50 * time.Millisecond)
	}
	return m.StartCount() >= expected
}

// WaitForStop polls until the stop count reaches expected or timeout.
func (m *mockParkingOperator) WaitForStop(expected int, timeout time.Duration) bool {
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		if m.StopCount() >= expected {
			return true
		}
		time.Sleep(50 * time.Millisecond)
	}
	return m.StopCount() >= expected
}

// ── Mock DATA_BROKER gRPC server ──────────────────────────────────────────────

// mockDataBroker implements the kuksa.val.v1.VALService gRPC service.
// It provides:
//   - Set: records Vehicle.Parking.SessionActive values
//   - Subscribe: streams IsLocked updates on demand via SendIsLocked
//   - Get: returns empty (not used by the adaptor)
type mockDataBroker struct {
	kuksapb.UnimplementedVALServiceServer

	mu                  sync.Mutex
	sessionActiveValues []bool // values received via Set for SessionActive

	// isLockedCh is used to push IsLocked updates to all active subscriptions.
	isLockedCh chan bool
	// subConnectedCh is closed when the first Subscribe call arrives.
	subConnectedCh chan struct{}
	subOnce        sync.Once
}

// newMockDataBroker creates a new mockDataBroker and registers a gRPC server
// on a random port. Returns the broker and its listen address (host:port).
// The server is stopped when the test ends.
func newMockDataBroker(t *testing.T) (*mockDataBroker, string) {
	t.Helper()
	lis, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("listen for mock DATA_BROKER: %v", err)
	}

	broker := &mockDataBroker{
		isLockedCh:     make(chan bool, 16),
		subConnectedCh: make(chan struct{}),
	}

	srv := grpc.NewServer()
	kuksapb.RegisterVALServiceServer(srv, broker)

	go srv.Serve(lis) //nolint
	t.Cleanup(func() { srv.Stop() })

	addr := lis.Addr().String()
	return broker, addr
}

// Set records Vehicle.Parking.SessionActive values from adaptor set calls.
func (b *mockDataBroker) Set(_ context.Context, req *kuksapb.SetRequest) (*kuksapb.SetResponse, error) {
	b.mu.Lock()
	defer b.mu.Unlock()
	for _, update := range req.Updates {
		if update.Entry != nil && update.Entry.Path == signalSessionActive {
			if update.Entry.Value != nil {
				b.sessionActiveValues = append(b.sessionActiveValues, update.Entry.Value.GetBool())
			}
		}
	}
	return &kuksapb.SetResponse{}, nil
}

// Subscribe streams IsLocked updates to the adaptor.
// It sends an empty initial response immediately (to unblock the tonic client's
// streaming .await), then forwards updates from isLockedCh.
func (b *mockDataBroker) Subscribe(_ *kuksapb.SubscribeRequest, stream kuksapb.VALService_SubscribeServer) error {
	// Signal that a subscription has been established.
	b.subOnce.Do(func() { close(b.subConnectedCh) })

	// Send an empty initial response immediately to unblock the tonic client.
	if err := stream.Send(&kuksapb.SubscribeResponse{}); err != nil {
		return err
	}

	// Stream IsLocked updates until the client disconnects.
	for {
		select {
		case isLocked, ok := <-b.isLockedCh:
			if !ok {
				return nil
			}
			resp := &kuksapb.SubscribeResponse{
				Updates: []*kuksapb.EntryUpdate{
					{
						Entry: &kuksapb.DataEntry{
							Path: signalIsLocked,
							Value: &kuksapb.Datapoint{
								Value: &kuksapb.Datapoint_Bool{Bool: isLocked},
							},
						},
						Fields: []kuksapb.Field{kuksapb.Field_FIELD_VALUE},
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

// SendIsLocked pushes an IsLocked update to all active subscriptions.
func (b *mockDataBroker) SendIsLocked(isLocked bool) {
	b.isLockedCh <- isLocked
}

// WaitForSubscription blocks until the first Subscribe call arrives or timeout.
func (b *mockDataBroker) WaitForSubscription(timeout time.Duration) bool {
	select {
	case <-b.subConnectedCh:
		return true
	case <-time.After(timeout):
		return false
	}
}

// LastSessionActive returns the most recently received Vehicle.Parking.SessionActive
// value, or (false, false) if no values have been received yet.
func (b *mockDataBroker) LastSessionActive() (value bool, ok bool) {
	b.mu.Lock()
	defer b.mu.Unlock()
	if len(b.sessionActiveValues) == 0 {
		return false, false
	}
	return b.sessionActiveValues[len(b.sessionActiveValues)-1], true
}

// WaitForSessionActive polls until Vehicle.Parking.SessionActive equals expected
// or timeout is reached.
func (b *mockDataBroker) WaitForSessionActive(expected bool, timeout time.Duration) bool {
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		if v, ok := b.LastSessionActive(); ok && v == expected {
			return true
		}
		time.Sleep(50 * time.Millisecond)
	}
	v, ok := b.LastSessionActive()
	return ok && v == expected
}

// ── gRPC client for ParkingAdaptor ───────────────────────────────────────────

// newParkingAdaptorClient connects to the adaptor's gRPC server and returns a
// client. The connection is closed when the test ends.
func newParkingAdaptorClient(t *testing.T, addr string) parkingpb.ParkingAdaptorClient {
	t.Helper()
	conn, err := grpc.NewClient(addr, grpc.WithTransportCredentials(insecure.NewCredentials()))
	if err != nil {
		t.Fatalf("dial parking-operator-adaptor at %s: %v", addr, err)
	}
	t.Cleanup(func() { conn.Close() })
	return parkingpb.NewParkingAdaptorClient(conn)
}
