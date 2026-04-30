package lockingsvc_test

import (
	"bufio"
	"bytes"
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

	// VSS signal paths used by the locking-service.
	signalSpeed    = "Vehicle.Speed"
	signalDoorOpen = "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen"
	signalIsLocked = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked"
	signalCommand  = "Vehicle.Command.Door.Lock"
	signalResponse = "Vehicle.Command.Door.Response"
)

// repoRoot returns the absolute path to the repository root.
// tests/locking-service/ is two levels deep from the repo root.
func repoRoot(t *testing.T) string {
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

// getValueOrFail reads a signal value from DATA_BROKER; fails the test on error.
func getValueOrFail(t *testing.T, client kuksa.VALClient, path string) *kuksa.Datapoint {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	resp, err := client.GetValue(ctx, &kuksa.GetValueRequest{
		SignalId: &kuksa.SignalID{Signal: &kuksa.SignalID_Path{Path: path}},
	})
	if err != nil {
		t.Fatalf("failed to get %s: %v", path, err)
	}
	return resp.GetDataPoint()
}

// boolValue creates a Value with a bool typed value.
func boolValue(v bool) *kuksa.Value {
	return &kuksa.Value{TypedValue: &kuksa.Value_Bool{Bool: v}}
}

// floatValue creates a Value with a float32 typed value.
func floatValue(v float32) *kuksa.Value {
	return &kuksa.Value{TypedValue: &kuksa.Value_Float{Float: v}}
}

// stringValue creates a Value with a string typed value.
func stringValue(v string) *kuksa.Value {
	return &kuksa.Value{TypedValue: &kuksa.Value_String_{String_: v}}
}

// lockCommandJSON returns a JSON-encoded lock command payload.
func lockCommandJSON(commandID string) string {
	cmd := map[string]any{
		"command_id": commandID,
		"action":     "lock",
		"doors":      []string{"driver"},
	}
	b, _ := json.Marshal(cmd)
	return string(b)
}

// unlockCommandJSON returns a JSON-encoded unlock command payload.
func unlockCommandJSON(commandID string) string {
	cmd := map[string]any{
		"command_id": commandID,
		"action":     "unlock",
		"doors":      []string{"driver"},
	}
	b, _ := json.Marshal(cmd)
	return string(b)
}

// buildLockingService builds the locking-service binary via cargo and returns
// the path to the compiled binary.
func buildLockingService(t *testing.T) string {
	t.Helper()
	root := repoRoot(t)

	cmd := exec.Command("cargo", "build", "-p", "locking-service")
	cmd.Dir = filepath.Join(root, "rhivos")
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("failed to build locking-service: %v\noutput: %s", err, string(out))
	}

	return filepath.Join(root, "rhivos", "target", "debug", "locking-service")
}

// lockingServiceProcess represents a running locking-service instance.
type lockingServiceProcess struct {
	cmd    *exec.Cmd
	mu     sync.Mutex
	stderr string
	done   chan struct{}
	err    error
}

// startLockingService starts the locking-service binary as a background process.
// The service is started with the "serve" subcommand and DATABROKER_ADDR set to
// the provided address. The process is killed on test cleanup.
func startLockingService(t *testing.T, binary, addr string) *lockingServiceProcess {
	t.Helper()

	cmd := exec.Command(binary, "serve")
	cmd.Env = []string{
		fmt.Sprintf("PATH=%s", os.Getenv("PATH")),
		fmt.Sprintf("HOME=%s", os.Getenv("HOME")),
		fmt.Sprintf("DATABROKER_ADDR=%s", addr),
	}

	stderrPipe, err := cmd.StderrPipe()
	if err != nil {
		t.Fatalf("failed to create stderr pipe: %v", err)
	}

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start locking-service: %v", err)
	}

	proc := &lockingServiceProcess{
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
		// Kill the process if it's still running.
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

// startLockingServiceWithStderrScanner starts the locking-service and provides
// a line-by-line stderr scanner for log assertion. Returns the process and a
// channel that receives each stderr line.
func startLockingServiceWithStderrScanner(t *testing.T, binary, addr string) (*lockingServiceProcess, <-chan string) {
	t.Helper()

	cmd := exec.Command(binary, "serve")
	cmd.Env = []string{
		fmt.Sprintf("PATH=%s", os.Getenv("PATH")),
		fmt.Sprintf("HOME=%s", os.Getenv("HOME")),
		fmt.Sprintf("DATABROKER_ADDR=%s", addr),
	}

	stderrPipe, err := cmd.StderrPipe()
	if err != nil {
		t.Fatalf("failed to create stderr pipe: %v", err)
	}

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start locking-service: %v", err)
	}

	proc := &lockingServiceProcess{
		cmd:  cmd,
		done: make(chan struct{}),
	}

	lines := make(chan string, 100)

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

// waitReady waits for the "locking-service ready" log line in the stderr lines
// channel. Fails the test if the line doesn't appear within the timeout.
func waitReady(t *testing.T, lines <-chan string, timeout time.Duration) {
	t.Helper()
	deadline := time.After(timeout)
	for {
		select {
		case line, ok := <-lines:
			if !ok {
				t.Fatal("stderr stream closed before locking-service ready")
			}
			if strings.Contains(line, "locking-service ready") {
				return
			}
		case <-deadline:
			t.Fatal("timed out waiting for locking-service ready")
		}
	}
}

// getStderr returns the accumulated stderr output.
func (p *lockingServiceProcess) getStderr() string {
	p.mu.Lock()
	defer p.mu.Unlock()
	return p.stderr
}

// waitExit waits for the process to exit within the given timeout and returns
// the exit code. Returns -1 if the timeout is reached.
func (p *lockingServiceProcess) waitExit(timeout time.Duration) int {
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

// waitForResponse subscribes to Vehicle.Command.Door.Response and waits for a
// response JSON to appear within the given timeout. Uses polling on the signal
// value since subscribing to a string signal and watching for changes requires
// careful coordination with the subscription stream.
func waitForResponse(t *testing.T, client kuksa.VALClient, timeout time.Duration) map[string]any {
	t.Helper()
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		dp := getValueOrFail(t, client, signalResponse)
		if dp != nil && dp.GetValue() != nil {
			if s := dp.GetValue().GetString_(); s != "" {
				var result map[string]any
				if err := json.Unmarshal([]byte(s), &result); err == nil {
					return result
				}
			}
		}
		time.Sleep(200 * time.Millisecond)
	}
	t.Fatal("timed out waiting for command response")
	return nil
}

// waitForNewResponse subscribes to Vehicle.Command.Door.Response and waits for
// a response with a specific command_id to appear.
func waitForNewResponse(t *testing.T, client kuksa.VALClient, commandID string, timeout time.Duration) map[string]any {
	t.Helper()
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		dp := getValueOrFail(t, client, signalResponse)
		if dp != nil && dp.GetValue() != nil {
			if s := dp.GetValue().GetString_(); s != "" {
				var result map[string]any
				if err := json.Unmarshal([]byte(s), &result); err == nil {
					if cid, ok := result["command_id"].(string); ok && cid == commandID {
						return result
					}
				}
			}
		}
		time.Sleep(200 * time.Millisecond)
	}
	t.Fatalf("timed out waiting for response with command_id=%q", commandID)
	return nil
}

// drainStream reads and discards one initial notification from a subscription
// stream. If no notification arrives within timeout the call returns gracefully.
func drainStream(stream kuksa.VAL_SubscribeClient, timeout time.Duration) {
	ch := make(chan struct{}, 1)
	go func() {
		_, _ = stream.Recv()
		ch <- struct{}{}
	}()
	select {
	case <-ch:
	case <-time.After(timeout):
	}
}
