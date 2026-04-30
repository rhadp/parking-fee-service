package parkingoperatoradaptor_test

import (
	"bufio"
	"bytes"
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
	"syscall"
	"testing"
	"time"

	"github.com/rhadp/parking-fee-service/gen/adapter"
	kuksa "github.com/rhadp/parking-fee-service/gen/kuksa"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
	"google.golang.org/protobuf/types/known/timestamppb"
)

const (
	// tcpTarget is the host address for the DATA_BROKER TCP listener.
	// Compose maps container port 55555 to host port 55556.
	tcpTarget = "localhost:55556"

	// databrokerAddr is the full gRPC address for the DATA_BROKER.
	databrokerAddr = "http://localhost:55556"

	// VSS signal paths used by the parking-operator-adaptor.
	signalIsLocked      = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked"
	signalSessionActive = "Vehicle.Parking.SessionActive"
)

// repoRoot returns the absolute path to the repository root.
func repoRoot(t *testing.T) string {
	t.Helper()
	dir, err := os.Getwd()
	if err != nil {
		t.Fatalf("failed to get working directory: %v", err)
	}
	for {
		// Check for .git directory or file (worktree case).
		if info, err := os.Stat(filepath.Join(dir, ".git")); err == nil {
			_ = info
			return dir
		}
		parent := filepath.Dir(dir)
		if parent == dir {
			t.Fatal("could not find repo root (.git)")
		}
		dir = parent
	}
}

// skipIfTCPUnreachable skips the test if the DATA_BROKER TCP port is not reachable.
func skipIfTCPUnreachable(t *testing.T) {
	t.Helper()
	conn, err := net.DialTimeout("tcp", tcpTarget, 2*time.Second)
	if err != nil {
		t.Skipf("DATA_BROKER TCP port %s not reachable, skipping: %v", tcpTarget, err)
	}
	conn.Close()
}

// dialDatabroker establishes a gRPC connection to DATA_BROKER via TCP and
// returns the connection and a VALClient.
func dialDatabroker(t *testing.T) (*grpc.ClientConn, kuksa.VALClient) {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	conn, err := grpc.DialContext(ctx, tcpTarget, //nolint:staticcheck // DialContext is fine for tests
		grpc.WithTransportCredentials(insecure.NewCredentials()),
		grpc.WithBlock(), //nolint:staticcheck // WithBlock is fine for tests
	)
	if err != nil {
		t.Fatalf("failed to dial DATA_BROKER at %s: %v", tcpTarget, err)
	}
	t.Cleanup(func() { conn.Close() })
	return conn, kuksa.NewVALClient(conn)
}

// dialAdaptor establishes a gRPC connection to the parking-operator-adaptor
// and returns a ParkingOperatorAdaptorServiceClient.
func dialAdaptor(t *testing.T, port int) adapter.ParkingOperatorAdaptorServiceClient {
	t.Helper()
	target := fmt.Sprintf("localhost:%d", port)
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	conn, err := grpc.DialContext(ctx, target, //nolint:staticcheck // DialContext is fine for tests
		grpc.WithTransportCredentials(insecure.NewCredentials()),
		grpc.WithBlock(), //nolint:staticcheck // WithBlock is fine for tests
	)
	if err != nil {
		t.Fatalf("failed to dial adaptor at %s: %v", target, err)
	}
	t.Cleanup(func() { conn.Close() })
	return adapter.NewParkingOperatorAdaptorServiceClient(conn)
}

// publishValue sets a signal value on DATA_BROKER.
func publishValue(t *testing.T, client kuksa.VALClient, path string, value *kuksa.Value) {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	_, err := client.PublishValue(ctx, &kuksa.PublishValueRequest{
		SignalId:  &kuksa.SignalID{Signal: &kuksa.SignalID_Path{Path: path}},
		DataPoint: &kuksa.Datapoint{Timestamp: timestamppb.Now(), Value: value},
	})
	if err != nil {
		t.Fatalf("failed to publish %s: %v", path, err)
	}
}

// boolValue creates a Value with a bool typed value.
func boolValue(v bool) *kuksa.Value {
	return &kuksa.Value{TypedValue: &kuksa.Value_Bool{Bool: v}}
}

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

// buildAdaptor builds the parking-operator-adaptor binary via cargo and returns
// the path to the compiled binary.
func buildAdaptor(t *testing.T) string {
	t.Helper()
	root := repoRoot(t)

	cmd := exec.Command("cargo", "build", "-p", "parking-operator-adaptor")
	cmd.Dir = filepath.Join(root, "rhivos")
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("failed to build parking-operator-adaptor: %v\noutput: %s", err, string(out))
	}

	return filepath.Join(root, "rhivos", "target", "debug", "parking-operator-adaptor")
}

// adaptorProcess represents a running parking-operator-adaptor instance.
type adaptorProcess struct {
	cmd    *exec.Cmd
	mu     sync.Mutex
	stderr string
	done   chan struct{}
	err    error
}

// startAdaptor starts the parking-operator-adaptor binary as a background
// process. The process is killed on test cleanup.
func startAdaptor(t *testing.T, binary string, env []string) *adaptorProcess {
	t.Helper()

	cmd := exec.Command(binary)
	cmd.Env = env

	stderrPipe, err := cmd.StderrPipe()
	if err != nil {
		t.Fatalf("failed to create stderr pipe: %v", err)
	}

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start adaptor: %v", err)
	}

	proc := &adaptorProcess{
		cmd:  cmd,
		done: make(chan struct{}),
	}

	// Read stderr in a background goroutine.
	go func() {
		var buf bytes.Buffer
		_, _ = io.Copy(&buf, stderrPipe)
		proc.mu.Lock()
		proc.stderr = buf.String()
		proc.mu.Unlock()
		proc.err = cmd.Wait()
		close(proc.done)
	}()

	t.Cleanup(func() {
		select {
		case <-proc.done:
			// Already exited.
		default:
			_ = cmd.Process.Kill()
			<-proc.done
		}
	})

	return proc
}

// startAdaptorWithStderrScanner starts the adaptor and provides a line-by-line
// stderr scanner for log assertion. Returns the process and a channel that
// receives each stderr line.
func startAdaptorWithStderrScanner(t *testing.T, binary string, env []string) (*adaptorProcess, <-chan string) {
	t.Helper()

	cmd := exec.Command(binary)
	cmd.Env = env

	stderrPipe, err := cmd.StderrPipe()
	if err != nil {
		t.Fatalf("failed to create stderr pipe: %v", err)
	}

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start adaptor: %v", err)
	}

	proc := &adaptorProcess{
		cmd:  cmd,
		done: make(chan struct{}),
	}

	lines := make(chan string, 200)

	go func() {
		var buf bytes.Buffer
		scanner := bufio.NewScanner(io.TeeReader(stderrPipe, &buf))
		for scanner.Scan() {
			lines <- scanner.Text()
		}
		close(lines)
		proc.mu.Lock()
		proc.stderr = buf.String()
		proc.mu.Unlock()
		proc.err = cmd.Wait()
		close(proc.done)
	}()

	t.Cleanup(func() {
		select {
		case <-proc.done:
		default:
			_ = cmd.Process.Kill()
			<-proc.done
		}
	})

	return proc, lines
}

// waitReady waits for the "parking-operator-adaptor ready" log line in the
// stderr lines channel. Fails the test if the line doesn't appear within
// the timeout.
func waitReady(t *testing.T, lines <-chan string, timeout time.Duration) {
	t.Helper()
	deadline := time.After(timeout)
	for {
		select {
		case line, ok := <-lines:
			if !ok {
				t.Fatal("stderr stream closed before adaptor ready")
			}
			if strings.Contains(line, "parking-operator-adaptor ready") {
				return
			}
		case <-deadline:
			t.Fatal("timed out waiting for parking-operator-adaptor ready")
		}
	}
}

// collectLinesUntilReady reads lines from the stderr channel until the "ready"
// message is found or the timeout elapses. Returns all collected lines.
func collectLinesUntilReady(t *testing.T, lines <-chan string, timeout time.Duration) []string {
	t.Helper()
	var collected []string
	deadline := time.After(timeout)
	for {
		select {
		case line, ok := <-lines:
			if !ok {
				t.Fatal("stderr stream closed before adaptor ready")
				return collected
			}
			collected = append(collected, line)
			if strings.Contains(line, "parking-operator-adaptor ready") {
				return collected
			}
		case <-deadline:
			t.Fatal("timed out waiting for adaptor ready during log collection")
			return collected
		}
	}
}

// getStderr returns the accumulated stderr output.
func (p *adaptorProcess) getStderr() string {
	p.mu.Lock()
	defer p.mu.Unlock()
	return p.stderr
}

// waitExit waits for the process to exit within the given timeout and returns
// the exit code. Returns -1 if the timeout is reached.
func (p *adaptorProcess) waitExit(timeout time.Duration) int {
	select {
	case <-p.done:
		if p.err == nil {
			return 0
		}
		if exitErr, ok := p.err.(*exec.ExitError); ok {
			return exitErr.ExitCode()
		}
		return -1
	case <-time.After(timeout):
		return -1
	}
}

// sendSignal sends a signal to the adaptor process.
func (p *adaptorProcess) sendSignal(sig os.Signal) error {
	return p.cmd.Process.Signal(sig)
}

// adaptorEnv creates the environment for the adaptor process.
func adaptorEnv(grpcPort int, operatorURL string) []string {
	return []string{
		fmt.Sprintf("PATH=%s", os.Getenv("PATH")),
		fmt.Sprintf("HOME=%s", os.Getenv("HOME")),
		fmt.Sprintf("GRPC_PORT=%d", grpcPort),
		fmt.Sprintf("PARKING_OPERATOR_URL=%s", operatorURL),
		fmt.Sprintf("DATA_BROKER_ADDR=%s", databrokerAddr),
		"VEHICLE_ID=DEMO-VIN-001",
		"ZONE_ID=zone-demo-1",
	}
}

// mockOperator is a test HTTP server that simulates the PARKING_OPERATOR REST API.
type mockOperator struct {
	server     *httptest.Server
	mu         sync.Mutex
	startCalls []capturedRequest
	stopCalls  []capturedRequest
	sessionSeq int
}

type capturedRequest struct {
	Method  string
	Path    string
	Body    string
	Headers http.Header
}

func newMockOperator(t *testing.T) *mockOperator {
	t.Helper()

	m := &mockOperator{}

	m.server = httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		body, _ := io.ReadAll(r.Body)
		req := capturedRequest{
			Method:  r.Method,
			Path:    r.URL.Path,
			Body:    string(body),
			Headers: r.Header.Clone(),
		}

		w.Header().Set("Content-Type", "application/json")

		switch r.URL.Path {
		case "/parking/start":
			m.mu.Lock()
			m.startCalls = append(m.startCalls, req)
			m.sessionSeq++
			sessionID := fmt.Sprintf("sess-%d", m.sessionSeq)
			m.mu.Unlock()
			resp := map[string]any{
				"session_id": sessionID,
				"status":     "active",
				"rate": map[string]any{
					"type":     "per_hour",
					"amount":   2.50,
					"currency": "EUR",
				},
			}
			_ = json.NewEncoder(w).Encode(resp)

		case "/parking/stop":
			m.mu.Lock()
			m.stopCalls = append(m.stopCalls, req)
			m.mu.Unlock()
			var reqBody map[string]any
			_ = json.Unmarshal(body, &reqBody)
			sessionID := ""
			if sid, ok := reqBody["session_id"].(string); ok {
				sessionID = sid
			}
			resp := map[string]any{
				"session_id":       sessionID,
				"status":           "completed",
				"duration_seconds": 3600,
				"total_amount":     2.50,
				"currency":         "EUR",
			}
			_ = json.NewEncoder(w).Encode(resp)

		default:
			w.WriteHeader(http.StatusNotFound)
		}
	}))

	t.Cleanup(func() { m.server.Close() })
	return m
}

func (m *mockOperator) URL() string {
	return m.server.URL
}

func (m *mockOperator) getStartCalls() []capturedRequest {
	m.mu.Lock()
	defer m.mu.Unlock()
	cp := make([]capturedRequest, len(m.startCalls))
	copy(cp, m.startCalls)
	return cp
}

func (m *mockOperator) getStopCalls() []capturedRequest {
	m.mu.Lock()
	defer m.mu.Unlock()
	cp := make([]capturedRequest, len(m.stopCalls))
	copy(cp, m.stopCalls)
	return cp
}

func (m *mockOperator) waitForStartCalls(t *testing.T, count int, timeout time.Duration) {
	t.Helper()
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		if len(m.getStartCalls()) >= count {
			return
		}
		time.Sleep(100 * time.Millisecond)
	}
	t.Fatalf("timed out waiting for %d start calls (got %d)", count, len(m.getStartCalls()))
}

func (m *mockOperator) waitForStopCalls(t *testing.T, count int, timeout time.Duration) {
	t.Helper()
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		if len(m.getStopCalls()) >= count {
			return
		}
		time.Sleep(100 * time.Millisecond)
	}
	t.Fatalf("timed out waiting for %d stop calls (got %d)", count, len(m.getStopCalls()))
}

// waitForSessionActive polls DATA_BROKER for Vehicle.Parking.SessionActive
// and waits until it matches the expected value.
func waitForSessionActive(t *testing.T, client kuksa.VALClient, expected bool, timeout time.Duration) {
	t.Helper()
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
		resp, err := client.GetValue(ctx, &kuksa.GetValueRequest{
			SignalId: &kuksa.SignalID{Signal: &kuksa.SignalID_Path{Path: signalSessionActive}},
		})
		cancel()
		if err == nil {
			dp := resp.GetDataPoint()
			if dp != nil && dp.GetValue() != nil {
				val := dp.GetValue()
				if _, ok := val.GetTypedValue().(*kuksa.Value_Bool); ok {
					if val.GetBool() == expected {
						return
					}
				}
			}
		}
		time.Sleep(100 * time.Millisecond)
	}
	t.Fatalf("timed out waiting for SessionActive=%v", expected)
}

// Ensure all imported packages are used.
var (
	_ = syscall.SIGTERM
	_ adapter.ParkingOperatorAdaptorServiceClient
)
