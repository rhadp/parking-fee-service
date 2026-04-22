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
	"sync/atomic"
	"testing"
	"time"

	pb "github.com/rhadp/parking-fee-service/gen/kuksa/val/v1"
	pa "github.com/rhadp/parking-fee-service/gen/parking_adaptor/v1"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

// tcpTarget is the host:port for the DATA_BROKER TCP listener.
const tcpTarget = "localhost:55556"

// VSS signal path constants.
const (
	signalIsLocked      = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked"
	signalSessionActive = "Vehicle.Parking.SessionActive"
)

// repoRoot returns the absolute path to the repository root directory.
func repoRoot(t *testing.T) string {
	t.Helper()
	dir, err := os.Getwd()
	if err != nil {
		t.Fatalf("failed to get working directory: %v", err)
	}
	for {
		if _, err := os.Stat(filepath.Join(dir, "Makefile")); err == nil {
			return dir
		}
		parent := filepath.Dir(dir)
		if parent == dir {
			t.Fatal("could not find repository root (no Makefile found)")
		}
		dir = parent
	}
}

// skipIfTCPUnreachable skips the test if the DATA_BROKER TCP port is not
// reachable within 2 seconds.
func skipIfTCPUnreachable(t *testing.T) {
	t.Helper()
	conn, err := net.DialTimeout("tcp", tcpTarget, 2*time.Second)
	if err != nil {
		t.Skipf("DATA_BROKER TCP port %s not reachable: %v (container not running?)", tcpTarget, err)
	}
	conn.Close()
}

// connectTCP creates a gRPC client connection to the DATA_BROKER via TCP.
// The connection is closed automatically when the test completes.
func connectTCP(t *testing.T) *grpc.ClientConn {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	conn, err := grpc.DialContext(ctx, tcpTarget,
		grpc.WithTransportCredentials(insecure.NewCredentials()),
		grpc.WithBlock(),
	)
	if err != nil {
		t.Fatalf("failed to connect to DATA_BROKER via TCP at %s: %v", tcpTarget, err)
	}
	t.Cleanup(func() { conn.Close() })
	return conn
}

// newVALClient creates a VAL gRPC client from a connection.
func newVALClient(conn *grpc.ClientConn) pb.VALClient {
	return pb.NewVALClient(conn)
}

// connectAdaptorGRPC creates a gRPC client connection to the parking-operator-adaptor
// service on the given port. The connection is closed automatically when the test completes.
func connectAdaptorGRPC(t *testing.T, port int) pa.ParkingAdaptorClient {
	t.Helper()
	target := fmt.Sprintf("localhost:%d", port)
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	conn, err := grpc.DialContext(ctx, target,
		grpc.WithTransportCredentials(insecure.NewCredentials()),
		grpc.WithBlock(),
	)
	if err != nil {
		t.Fatalf("failed to connect to parking-operator-adaptor gRPC at %s: %v", target, err)
	}
	t.Cleanup(func() { conn.Close() })
	return pa.NewParkingAdaptorClient(conn)
}

// setBool sets a boolean signal value on the DATA_BROKER.
func setBool(t *testing.T, client pb.VALClient, path string, value bool) {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	_, err := client.Set(ctx, &pb.SetRequest{
		Updates: []*pb.EntryUpdate{
			{
				Entry: &pb.DataEntry{
					Path: path,
					Value: &pb.Datapoint{
						Value: &pb.Datapoint_BoolValue{BoolValue: value},
					},
				},
				Fields: []pb.Field{pb.Field_FIELD_VALUE},
			},
		},
	})
	if err != nil {
		t.Fatalf("Set bool(%q, %v) failed: %v", path, value, err)
	}
}

// getBool reads a boolean signal value from the DATA_BROKER.
func getBool(t *testing.T, client pb.VALClient, path string) *bool {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	resp, err := client.Get(ctx, &pb.GetRequest{
		Entries: []*pb.EntryRequest{
			{
				Path:   path,
				View:   pb.View_VIEW_CURRENT_VALUE,
				Fields: []pb.Field{pb.Field_FIELD_VALUE},
			},
		},
	})
	if err != nil {
		t.Fatalf("Get(%q) failed: %v", path, err)
	}
	if len(resp.Entries) == 0 {
		t.Fatalf("Get(%q) returned no entries", path)
	}
	entry := resp.Entries[0]
	if entry.Value == nil {
		return nil
	}
	switch v := entry.Value.Value.(type) {
	case *pb.Datapoint_BoolValue:
		return &v.BoolValue
	default:
		return nil
	}
}

// waitForBool polls the given signal until it matches the expected value or
// the timeout expires.
func waitForBool(t *testing.T, client pb.VALClient, path string, expected bool, timeout time.Duration) {
	t.Helper()
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		val := getBool(t, client, path)
		if val != nil && *val == expected {
			return
		}
		time.Sleep(200 * time.Millisecond)
	}
	t.Fatalf("timed out waiting for %q to become %v", path, expected)
}

// getFreePort finds an available TCP port on localhost.
func getFreePort(t *testing.T) int {
	t.Helper()
	l, err := net.Listen("tcp", "localhost:0")
	if err != nil {
		t.Fatalf("failed to find free port: %v", err)
	}
	port := l.Addr().(*net.TCPAddr).Port
	l.Close()
	return port
}

// buildAdaptor builds the parking-operator-adaptor Rust binary and returns the
// path to the compiled executable.
func buildAdaptor(t *testing.T) string {
	t.Helper()
	root := repoRoot(t)
	serviceDir := filepath.Join(root, "rhivos", "parking-operator-adaptor")

	cmd := exec.Command("cargo", "build", "--release")
	cmd.Dir = serviceDir
	output, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("failed to build parking-operator-adaptor: %v\n%s", err, output)
	}

	return filepath.Join(root, "rhivos", "target", "release", "parking-operator-adaptor")
}

// logCapture is a thread-safe buffer that captures stderr output.
type logCapture struct {
	mu   sync.Mutex
	buf  strings.Builder
	done chan struct{}
}

// newLogCapture creates a logCapture that drains r in a background goroutine.
func newLogCapture(r io.Reader) *logCapture {
	lc := &logCapture{done: make(chan struct{})}
	go func() {
		defer close(lc.done)
		scanner := bufio.NewScanner(r)
		// Use a larger buffer for potentially long log lines.
		scanner.Buffer(make([]byte, 0, 64*1024), 256*1024)
		for scanner.Scan() {
			line := scanner.Text()
			lc.mu.Lock()
			lc.buf.WriteString(line)
			lc.buf.WriteString("\n")
			lc.mu.Unlock()
		}
	}()
	return lc
}

// contents returns the captured output so far.
func (lc *logCapture) contents() string {
	lc.mu.Lock()
	defer lc.mu.Unlock()
	return lc.buf.String()
}

// startAdaptor starts the parking-operator-adaptor binary and waits for the
// "parking-operator-adaptor ready" log line. Returns the exec.Cmd and the
// logCapture for stdout (where tracing output goes). The process is killed
// on test cleanup.
func startAdaptor(t *testing.T, binary string, env ...string) (*exec.Cmd, *logCapture) {
	t.Helper()

	cmd := exec.Command(binary)
	cmd.Env = append(os.Environ(), env...)

	// Tracing output goes to stdout via tracing_subscriber::fmt::init().
	stdoutPipe, err := cmd.StdoutPipe()
	if err != nil {
		t.Fatalf("failed to create stdout pipe: %v", err)
	}

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start parking-operator-adaptor: %v", err)
	}

	t.Cleanup(func() {
		if cmd.Process != nil {
			_ = cmd.Process.Kill()
			_ = cmd.Wait()
		}
	})

	// Wait for the ready log line with timeout.
	readyCh := make(chan struct{})
	// Create a pipe-through reader: scan for readiness, then continue draining.
	pr, pw := io.Pipe()
	go func() {
		defer pw.Close()
		scanner := bufio.NewScanner(stdoutPipe)
		scanner.Buffer(make([]byte, 0, 64*1024), 256*1024)
		readyClosed := false
		for scanner.Scan() {
			line := scanner.Text()
			// Write to the pipe for logCapture.
			_, _ = pw.Write([]byte(line + "\n"))
			if !readyClosed && strings.Contains(line, "parking-operator-adaptor ready") {
				close(readyCh)
				readyClosed = true
			}
		}
	}()

	lc := newLogCapture(pr)

	select {
	case <-readyCh:
		// Service is ready.
	case <-time.After(60 * time.Second):
		t.Fatal("parking-operator-adaptor did not become ready within 60 seconds")
	}

	return cmd, lc
}

// startAdaptorNoWait starts the parking-operator-adaptor binary without
// waiting for the ready signal. Useful for tests that expect startup failure.
// Returns the exec.Cmd and a safeBuffer that captures stdout (where tracing
// output goes). The caller must call cmd.Wait() to wait for the process to exit.
func startAdaptorNoWait(t *testing.T, binary string, env ...string) (*exec.Cmd, *safeBuffer) {
	t.Helper()

	cmd := exec.Command(binary)
	cmd.Env = append(os.Environ(), env...)

	// Use a safeBuffer for stdout instead of a pipe, so that cmd.Wait()
	// can complete without racing with pipe reads.
	// Tracing output goes to stdout via tracing_subscriber::fmt::init().
	buf := &safeBuffer{}
	cmd.Stdout = buf

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start parking-operator-adaptor: %v", err)
	}

	t.Cleanup(func() {
		if cmd.Process != nil {
			_ = cmd.Process.Kill()
			_ = cmd.Wait()
		}
	})

	return cmd, buf
}

// safeBuffer is a thread-safe buffer that implements io.Writer.
type safeBuffer struct {
	mu  sync.Mutex
	buf strings.Builder
}

// Write implements io.Writer.
func (sb *safeBuffer) Write(p []byte) (n int, err error) {
	sb.mu.Lock()
	defer sb.mu.Unlock()
	return sb.buf.Write(p)
}

// contents returns the captured output so far.
func (sb *safeBuffer) contents() string {
	sb.mu.Lock()
	defer sb.mu.Unlock()
	return sb.buf.String()
}

// adaptorEnv builds the environment variable array for starting the adaptor.
func adaptorEnv(grpcPort int, operatorURL string) []string {
	return []string{
		fmt.Sprintf("GRPC_PORT=%d", grpcPort),
		fmt.Sprintf("PARKING_OPERATOR_URL=%s", operatorURL),
		fmt.Sprintf("DATA_BROKER_ADDR=http://%s", tcpTarget),
		"VEHICLE_ID=DEMO-VIN-001",
		"ZONE_ID=zone-demo-1",
		"RUST_LOG=info",
	}
}

// mockOperator is a mock PARKING_OPERATOR HTTP server that tracks call counts
// and returns configurable responses.
type mockOperator struct {
	server     *httptest.Server
	startCount atomic.Int32
	stopCount  atomic.Int32
	mu         sync.Mutex
	// Configurable response overrides. If nil, defaults are used.
	startResp *mockStartResponse
	stopResp  *mockStopResponse
}

type mockStartResponse struct {
	SessionID string  `json:"session_id"`
	Status    string  `json:"status"`
	Rate      mockRate `json:"rate"`
}

type mockRate struct {
	Type     string  `json:"type"`
	Amount   float64 `json:"amount"`
	Currency string  `json:"currency"`
}

type mockStopResponse struct {
	SessionID       string  `json:"session_id"`
	Status          string  `json:"status"`
	DurationSeconds int     `json:"duration_seconds"`
	TotalAmount     float64 `json:"total_amount"`
	Currency        string  `json:"currency"`
}

type mockStartRequest struct {
	VehicleID string `json:"vehicle_id"`
	ZoneID    string `json:"zone_id"`
	Timestamp int64  `json:"timestamp"`
}

// startMockOperator creates and starts a mock PARKING_OPERATOR HTTP server.
// The server is automatically shut down when the test completes.
func startMockOperator(t *testing.T) *mockOperator {
	t.Helper()

	mo := &mockOperator{
		startResp: &mockStartResponse{
			SessionID: "test-session-1",
			Status:    "active",
			Rate: mockRate{
				Type:     "per_hour",
				Amount:   2.50,
				Currency: "EUR",
			},
		},
		stopResp: &mockStopResponse{
			SessionID:       "test-session-1",
			Status:          "completed",
			DurationSeconds: 3600,
			TotalAmount:     2.50,
			Currency:        "EUR",
		},
	}

	mux := http.NewServeMux()
	mux.HandleFunc("/parking/start", func(w http.ResponseWriter, r *http.Request) {
		mo.startCount.Add(1)
		if r.Method != http.MethodPost {
			http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
			return
		}
		// Parse request body for validation.
		var req mockStartRequest
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			http.Error(w, "bad request", http.StatusBadRequest)
			return
		}

		mo.mu.Lock()
		resp := mo.startResp
		// Update session_id in stop response to match what was started.
		if mo.stopResp != nil {
			mo.stopResp.SessionID = resp.SessionID
		}
		mo.mu.Unlock()

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(resp)
	})
	mux.HandleFunc("/parking/stop", func(w http.ResponseWriter, r *http.Request) {
		mo.stopCount.Add(1)
		if r.Method != http.MethodPost {
			http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
			return
		}

		mo.mu.Lock()
		resp := mo.stopResp
		mo.mu.Unlock()

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(resp)
	})

	mo.server = httptest.NewServer(mux)
	t.Cleanup(func() { mo.server.Close() })

	return mo
}

// url returns the base URL of the mock operator server.
func (mo *mockOperator) url() string {
	return mo.server.URL
}

// waitForStartCalled polls until the start endpoint has been called at least
// the expected number of times, or the timeout expires.
func (mo *mockOperator) waitForStartCalled(t *testing.T, count int32, timeout time.Duration) {
	t.Helper()
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		if mo.startCount.Load() >= count {
			return
		}
		time.Sleep(200 * time.Millisecond)
	}
	t.Fatalf("timed out waiting for start call count >= %d (got %d)", count, mo.startCount.Load())
}

// waitForStopCalled polls until the stop endpoint has been called at least
// the expected number of times, or the timeout expires.
func (mo *mockOperator) waitForStopCalled(t *testing.T, count int32, timeout time.Duration) {
	t.Helper()
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		if mo.stopCount.Load() >= count {
			return
		}
		time.Sleep(200 * time.Millisecond)
	}
	t.Fatalf("timed out waiting for stop call count >= %d (got %d)", count, mo.stopCount.Load())
}

