// Package lockingservice_test contains integration tests for the
// LOCKING_SERVICE component. Tests require a running DATA_BROKER container
// accessible at localhost:55556 and the locking-service Rust binary.
//
// Start the databroker before running tests:
//
//	cd deployments && podman compose up -d kuksa-databroker
package lockingservice_test

import (
	"bufio"
	"context"
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"sync"
	"testing"
	"time"

	kuksapb "parking-fee-service/tests/locking-service/kuksa"

	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

// Endpoints and timeouts.
const (
	tcpAddr        = "localhost:55556"
	connectTimeout = 5 * time.Second
	opTimeout      = 5 * time.Second
	readyTimeout   = 30 * time.Second

	// VSS signal paths.
	signalCommand  = "Vehicle.Command.Door.Lock"
	signalResponse = "Vehicle.Command.Door.Response"
	signalIsLocked = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked"
	signalIsOpen   = "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen"
	signalSpeed    = "Vehicle.Speed"
)

// lockCommand is used to build command JSON payloads.
type lockCommand struct {
	CommandID string   `json:"command_id"`
	Action    string   `json:"action"`
	Doors     []string `json:"doors"`
	Source    string   `json:"source,omitempty"`
	VIN       string   `json:"vin,omitempty"`
	Timestamp int64    `json:"timestamp,omitempty"`
}

// commandResponse is used to parse response JSON.
type commandResponse struct {
	CommandID string `json:"command_id"`
	Status    string `json:"status"`
	Reason    string `json:"reason,omitempty"`
	Timestamp int64  `json:"timestamp"`
}

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

// dialTCP returns a gRPC ClientConn connected to the TCP endpoint.
func dialTCP(t *testing.T) *grpc.ClientConn {
	t.Helper()
	conn, err := grpc.NewClient(tcpAddr,
		grpc.WithTransportCredentials(insecure.NewCredentials()),
	)
	if err != nil {
		t.Fatalf("dialTCP: failed to create gRPC client for %s: %v", tcpAddr, err)
	}
	t.Cleanup(func() { conn.Close() })
	return conn
}

// valClient wraps a connection in a VAL gRPC client.
func valClient(conn *grpc.ClientConn) kuksapb.VALClient {
	return kuksapb.NewVALClient(conn)
}

// opCtx returns a context with the standard operation timeout.
func opCtx() (context.Context, context.CancelFunc) {
	return context.WithTimeout(context.Background(), opTimeout)
}

// setSignalBool writes a boolean value to the named VSS signal.
func setSignalBool(t *testing.T, client kuksapb.VALClient, path string, val bool) {
	t.Helper()
	ctx, cancel := opCtx()
	defer cancel()
	resp, err := client.Set(ctx, &kuksapb.SetRequest{
		Entries: []*kuksapb.DataEntry{
			{
				Path:  path,
				Value: &kuksapb.Datapoint{Value: &kuksapb.Datapoint_BoolValue{BoolValue: val}},
			},
		},
	})
	if err != nil {
		t.Fatalf("Set(%s, bool=%v): gRPC error: %v", path, val, err)
	}
	if !resp.Success {
		t.Fatalf("Set(%s, bool=%v): databroker error: %s", path, val, resp.Error)
	}
}

// setSignalFloat writes a float32 value to the named VSS signal.
func setSignalFloat(t *testing.T, client kuksapb.VALClient, path string, val float32) {
	t.Helper()
	ctx, cancel := opCtx()
	defer cancel()
	resp, err := client.Set(ctx, &kuksapb.SetRequest{
		Entries: []*kuksapb.DataEntry{
			{
				Path:  path,
				Value: &kuksapb.Datapoint{Value: &kuksapb.Datapoint_FloatValue{FloatValue: val}},
			},
		},
	})
	if err != nil {
		t.Fatalf("Set(%s, float=%v): gRPC error: %v", path, val, err)
	}
	if !resp.Success {
		t.Fatalf("Set(%s, float=%v): databroker error: %s", path, val, resp.Error)
	}
}

// setSignalString writes a string value to the named VSS signal.
func setSignalString(t *testing.T, client kuksapb.VALClient, path string, val string) {
	t.Helper()
	ctx, cancel := opCtx()
	defer cancel()
	resp, err := client.Set(ctx, &kuksapb.SetRequest{
		Entries: []*kuksapb.DataEntry{
			{
				Path:  path,
				Value: &kuksapb.Datapoint{Value: &kuksapb.Datapoint_StringValue{StringValue: val}},
			},
		},
	})
	if err != nil {
		t.Fatalf("Set(%s, string=%q): gRPC error: %v", path, val, err)
	}
	if !resp.Success {
		t.Fatalf("Set(%s, string=%q): databroker error: %s", path, val, resp.Error)
	}
}

// getSignalBool reads a boolean signal value. Returns (value, hasValue).
func getSignalBool(t *testing.T, client kuksapb.VALClient, path string) (bool, bool) {
	t.Helper()
	ctx, cancel := opCtx()
	defer cancel()
	resp, err := client.Get(ctx, &kuksapb.GetRequest{Paths: []string{path}})
	if err != nil {
		t.Fatalf("Get(%s): gRPC error: %v", path, err)
	}
	if len(resp.Entries) == 0 {
		return false, false
	}
	entry := resp.Entries[0]
	if entry.Value == nil {
		return false, false
	}
	bv, ok := entry.Value.Value.(*kuksapb.Datapoint_BoolValue)
	if !ok {
		return false, false
	}
	return bv.BoolValue, true
}

// getSignalString reads a string signal value. Returns (value, hasValue).
func getSignalString(t *testing.T, client kuksapb.VALClient, path string) (string, bool) {
	t.Helper()
	ctx, cancel := opCtx()
	defer cancel()
	resp, err := client.Get(ctx, &kuksapb.GetRequest{Paths: []string{path}})
	if err != nil {
		t.Fatalf("Get(%s): gRPC error: %v", path, err)
	}
	if len(resp.Entries) == 0 {
		return "", false
	}
	entry := resp.Entries[0]
	if entry.Value == nil {
		return "", false
	}
	sv, ok := entry.Value.Value.(*kuksapb.Datapoint_StringValue)
	if !ok {
		return "", false
	}
	return sv.StringValue, true
}

// buildLockingService builds the locking-service Rust binary using cargo and
// returns the path to the compiled binary.
func buildLockingService(t *testing.T) string {
	t.Helper()
	root := findRepoRoot(t)
	crateDir := filepath.Join(root, "rhivos", "locking-service")

	cmd := exec.Command("cargo", "build", "--quiet")
	cmd.Dir = crateDir
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("failed to build locking-service: %v\noutput: %s", err, string(out))
	}

	binary := filepath.Join(root, "rhivos", "target", "debug", "locking-service")
	if _, err := os.Stat(binary); err != nil {
		t.Fatalf("locking-service binary not found at %s", binary)
	}
	return binary
}

// lockingServiceProcess manages a running locking-service instance.
type lockingServiceProcess struct {
	cmd    *exec.Cmd
	mu     sync.Mutex
	logs   []string
	cancel context.CancelFunc
}

// startLockingService starts the locking-service binary with the given
// DATABROKER_ADDR and waits for the "locking-service ready" log line.
// Returns a handle to the process for cleanup.
func startLockingService(t *testing.T, binary string, addr string) *lockingServiceProcess {
	t.Helper()

	ctx, cancel := context.WithCancel(context.Background())
	cmd := exec.CommandContext(ctx, binary, "serve")
	cmd.Env = append(os.Environ(), fmt.Sprintf("DATABROKER_ADDR=%s", addr))

	stderr, err := cmd.StderrPipe()
	if err != nil {
		cancel()
		t.Fatalf("failed to pipe stderr: %v", err)
	}

	proc := &lockingServiceProcess{
		cmd:    cmd,
		cancel: cancel,
	}

	if err := cmd.Start(); err != nil {
		cancel()
		t.Fatalf("failed to start locking-service: %v", err)
	}

	// Read logs in background.
	ready := make(chan struct{})
	readyClosed := false
	go func() {
		scanner := bufio.NewScanner(stderr)
		for scanner.Scan() {
			line := scanner.Text()
			proc.mu.Lock()
			proc.logs = append(proc.logs, line)
			proc.mu.Unlock()
			if !readyClosed && strings.Contains(line, "locking-service ready") {
				readyClosed = true
				close(ready)
			}
		}
		// If we never got the ready signal and channel isn't closed, close it.
		if !readyClosed {
			readyClosed = true
			close(ready)
		}
	}()

	// Wait for ready or timeout.
	select {
	case <-ready:
		// Check if process is still running (ready signal might have been
		// sent because stderr closed without finding the line).
		proc.mu.Lock()
		found := false
		for _, l := range proc.logs {
			if strings.Contains(l, "locking-service ready") {
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
			t.Fatalf("locking-service exited before becoming ready. Logs:\n%s", allLogs)
		}
	case <-time.After(readyTimeout):
		cancel()
		_ = cmd.Wait()
		proc.mu.Lock()
		allLogs := strings.Join(proc.logs, "\n")
		proc.mu.Unlock()
		t.Fatalf("locking-service did not become ready within %v. Logs:\n%s", readyTimeout, allLogs)
	}

	t.Cleanup(func() {
		proc.stop()
	})

	return proc
}

// stop terminates the locking-service process.
func (p *lockingServiceProcess) stop() {
	p.cancel()
	_ = p.cmd.Wait()
}

// getLogs returns all captured log lines.
func (p *lockingServiceProcess) getLogs() []string {
	p.mu.Lock()
	defer p.mu.Unlock()
	cp := make([]string, len(p.logs))
	copy(cp, p.logs)
	return cp
}

// makeLockJSON creates a lock command JSON payload.
func makeLockJSON(commandID string) string {
	cmd := lockCommand{
		CommandID: commandID,
		Action:    "lock",
		Doors:     []string{"driver"},
	}
	b, _ := json.Marshal(cmd)
	return string(b)
}

// makeUnlockJSON creates an unlock command JSON payload.
func makeUnlockJSON(commandID string) string {
	cmd := lockCommand{
		CommandID: commandID,
		Action:    "unlock",
		Doors:     []string{"driver"},
	}
	b, _ := json.Marshal(cmd)
	return string(b)
}

// parseResponse parses a command response JSON string.
func parseResponse(t *testing.T, raw string) commandResponse {
	t.Helper()
	var resp commandResponse
	if err := json.Unmarshal([]byte(raw), &resp); err != nil {
		t.Fatalf("failed to parse response JSON %q: %v", raw, err)
	}
	return resp
}

// waitForResponse polls the Vehicle.Command.Door.Response signal until a
// response with the given command_id appears or the timeout expires.
func waitForResponse(t *testing.T, client kuksapb.VALClient, commandID string, timeout time.Duration) commandResponse {
	t.Helper()
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		raw, ok := getSignalString(t, client, signalResponse)
		if ok && raw != "" {
			resp := parseResponse(t, raw)
			if resp.CommandID == commandID {
				return resp
			}
		}
		time.Sleep(200 * time.Millisecond)
	}
	t.Fatalf("timed out waiting for response with command_id=%q", commandID)
	return commandResponse{} // unreachable
}

// ensureDatabrokerReachable verifies the databroker is reachable, skipping if not.
func ensureDatabrokerReachable(t *testing.T) kuksapb.VALClient {
	t.Helper()
	conn := dialTCP(t)
	client := valClient(conn)

	ctx, cancel := context.WithTimeout(context.Background(), connectTimeout)
	defer cancel()

	_, err := client.Get(ctx, &kuksapb.GetRequest{Paths: []string{signalSpeed}})
	if err != nil {
		t.Skipf("DATA_BROKER not reachable at %s: %v (skipping integration test)", tcpAddr, err)
	}
	return client
}

// resetSignals clears key signals to a known state before each test.
func resetSignals(t *testing.T, client kuksapb.VALClient) {
	t.Helper()
	setSignalFloat(t, client, signalSpeed, 0.0)
	setSignalBool(t, client, signalIsOpen, false)
	setSignalBool(t, client, signalIsLocked, false)
	setSignalString(t, client, signalResponse, "")
	setSignalString(t, client, signalCommand, "")
}
