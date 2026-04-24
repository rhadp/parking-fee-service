package lockingservice_test

import (
	"bufio"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"sync"
	"testing"
	"time"

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

	// serviceReadyTimeout is the maximum time to wait for the locking-service
	// to become ready after startup.
	serviceReadyTimeout = 35 * time.Second

	// Signal path constants matching the locking-service Rust code.
	signalCommand  = "Vehicle.Command.Door.Lock"
	signalSpeed    = "Vehicle.Speed"
	signalIsOpen   = "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen"
	signalIsLocked = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked"
	signalResponse = "Vehicle.Command.Door.Response"
)

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
		parent := filepath.Dir(dir)
		if parent == dir {
			t.Fatal("could not find repo root (.git)")
		}
		dir = parent
	}
}

// skipIfTCPUnreachable skips the test if the DATA_BROKER is not reachable.
// It first checks raw TCP connectivity, then verifies that a real gRPC
// service (not just a network proxy like gvproxy) is listening by
// attempting a lightweight gRPC call.
func skipIfTCPUnreachable(t *testing.T) {
	t.Helper()
	conn, err := net.DialTimeout("tcp", tcpTarget, 2*time.Second)
	if err != nil {
		t.Skipf("DATA_BROKER TCP port not reachable at %s: %v", tcpTarget, err)
	}
	conn.Close()

	// Verify it's actually a DATA_BROKER by attempting a gRPC call.
	// This catches cases where another process (e.g. Podman's gvproxy)
	// is listening on the same port but is not a real DATA_BROKER.
	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()
	grpcConn, err := grpc.DialContext(ctx, tcpTarget,
		grpc.WithTransportCredentials(insecure.NewCredentials()),
		grpc.WithBlock(),
	)
	if err != nil {
		t.Skipf("DATA_BROKER gRPC connection failed at %s (port open but not a gRPC service): %v", tcpTarget, err)
	}
	client := kuksa.NewVALClient(grpcConn)
	_, err = client.Get(ctx, &kuksa.GetRequest{
		Entries: []*kuksa.EntryRequest{
			{
				Path:   "Vehicle.Speed",
				View:   kuksa.View_VIEW_CURRENT_VALUE,
				Fields: []kuksa.Field{kuksa.Field_FIELD_VALUE},
			},
		},
	})
	grpcConn.Close()
	if err != nil {
		t.Skipf("DATA_BROKER gRPC probe failed at %s (not a real DATA_BROKER): %v", tcpTarget, err)
	}
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

// newClient creates a kuksa VAL gRPC client over TCP.
func newClient(t *testing.T) kuksa.VALClient {
	t.Helper()
	return kuksa.NewVALClient(dialTCP(t))
}

// buildLockingService builds the locking-service Rust binary using cargo
// and returns the path to the compiled binary.
func buildLockingService(t *testing.T) string {
	t.Helper()
	root := findRepoRoot(t)
	rhivosDir := filepath.Join(root, "rhivos")

	cmd := exec.Command("cargo", "build", "-p", "locking-service")
	cmd.Dir = rhivosDir
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("failed to build locking-service: %v\n%s", err, string(out))
	}

	binaryPath := filepath.Join(rhivosDir, "target", "debug", "locking-service")
	if _, err := os.Stat(binaryPath); os.IsNotExist(err) {
		t.Fatalf("locking-service binary not found at %s after build", binaryPath)
	}

	return binaryPath
}

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
		// Use a timed wait to avoid missing broadcasts
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

// lockingServiceProcess represents a running locking-service process.
type lockingServiceProcess struct {
	cmd  *exec.Cmd
	logs *logCapture
}

// startLockingService starts the locking-service binary with the given
// DATA_BROKER address and waits for the "locking-service ready" log line.
// The process is automatically killed when the test completes.
func startLockingService(t *testing.T, binaryPath, databrokAddr string) *lockingServiceProcess {
	t.Helper()

	cmd := exec.Command(binaryPath, "serve")
	cmd.Env = []string{
		fmt.Sprintf("DATABROKER_ADDR=%s", databrokAddr),
		fmt.Sprintf("PATH=%s", os.Getenv("PATH")),
		fmt.Sprintf("HOME=%s", os.Getenv("HOME")),
	}

	stderrPipe, err := cmd.StderrPipe()
	if err != nil {
		t.Fatalf("failed to create stderr pipe: %v", err)
	}

	logs := newLogCapture()

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start locking-service: %v", err)
	}

	// Read stderr in background, capturing log lines.
	go func() {
		scanner := bufio.NewScanner(stderrPipe)
		for scanner.Scan() {
			logs.appendLine(scanner.Text())
		}
	}()

	// Wait for the service to become ready.
	if !logs.waitFor("locking-service ready", serviceReadyTimeout) {
		_ = cmd.Process.Kill()
		_ = cmd.Wait()
		allLogs := strings.Join(logs.allLines(), "\n")
		t.Fatalf("locking-service did not become ready within %v\nLogs:\n%s", serviceReadyTimeout, allLogs)
	}

	// Register cleanup to kill the process when the test finishes.
	t.Cleanup(func() {
		if cmd.Process != nil {
			_ = cmd.Process.Kill()
			_ = cmd.Wait()
		}
	})

	return &lockingServiceProcess{cmd: cmd, logs: logs}
}

// getSignalValue performs a Get RPC for the given signal path and returns
// the DataEntry.
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

// setSignalBool sets a boolean signal value.
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

// setSignalFloat sets a float signal value.
func setSignalFloat(t *testing.T, client kuksa.VALClient, path string, val float32) {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), rpcTimeout)
	defer cancel()

	_, err := client.Set(ctx, &kuksa.SetRequest{
		Updates: []*kuksa.EntryUpdate{
			{
				Entry: &kuksa.DataEntry{
					Path: path,
					Value: &kuksa.Datapoint{
						Value: &kuksa.Datapoint_FloatValue{FloatValue: val},
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

// setSignalString sets a string signal value.
func setSignalString(t *testing.T, client kuksa.VALClient, path string, val string) {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), rpcTimeout)
	defer cancel()

	_, err := client.Set(ctx, &kuksa.SetRequest{
		Updates: []*kuksa.EntryUpdate{
			{
				Entry: &kuksa.DataEntry{
					Path: path,
					Value: &kuksa.Datapoint{
						Value: &kuksa.Datapoint_StringValue{StringValue: val},
					},
				},
				Fields: []kuksa.Field{kuksa.Field_FIELD_VALUE},
			},
		},
	})
	if err != nil {
		t.Fatalf("failed to set %s to %q: %v", path, val, err)
	}
}

// getStringValue reads a string signal value from DATA_BROKER.
// Returns empty string if the signal has no value.
func getStringValue(t *testing.T, client kuksa.VALClient, path string) string {
	t.Helper()
	entry, err := getSignalValue(t, client, path)
	if err != nil {
		t.Fatalf("failed to get %s: %v", path, err)
	}
	if entry.Value == nil {
		return ""
	}
	return entry.Value.GetStringValue()
}

// getBoolValue reads a boolean signal value from DATA_BROKER.
// Returns the value and whether a value was present.
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

// makeLockCommandJSON creates a JSON lock command payload.
func makeLockCommandJSON(commandID string) string {
	cmd := map[string]interface{}{
		"command_id": commandID,
		"action":     "lock",
		"doors":      []string{"driver"},
	}
	b, _ := json.Marshal(cmd)
	return string(b)
}

// makeUnlockCommandJSON creates a JSON unlock command payload.
func makeUnlockCommandJSON(commandID string) string {
	cmd := map[string]interface{}{
		"command_id": commandID,
		"action":     "unlock",
		"doors":      []string{"driver"},
	}
	b, _ := json.Marshal(cmd)
	return string(b)
}

// waitForResponse polls Vehicle.Command.Door.Response until a response
// with the given command_id appears, or the timeout expires.
func waitForResponse(t *testing.T, client kuksa.VALClient, commandID string, timeout time.Duration) map[string]interface{} {
	t.Helper()
	deadline := time.Now().Add(timeout)

	for time.Now().Before(deadline) {
		raw := getStringValue(t, client, signalResponse)
		if raw != "" {
			var resp map[string]interface{}
			if err := json.Unmarshal([]byte(raw), &resp); err == nil {
				if resp["command_id"] == commandID {
					return resp
				}
			}
		}
		time.Sleep(200 * time.Millisecond)
	}

	t.Fatalf("timed out waiting for response with command_id=%q", commandID)
	return nil
}

// drainStderr reads and discards remaining stderr output. Used to avoid
// broken pipe errors when killing a process.
func drainStderr(r io.Reader) {
	if r != nil {
		_, _ = io.Copy(io.Discard, r)
	}
}
