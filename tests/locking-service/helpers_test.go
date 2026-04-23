package lockingservice_test

import (
	"bufio"
	"context"
	"encoding/json"
	"fmt"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"sync"
	"testing"
	"time"

	pb "github.com/rhadp/parking-fee-service/gen/kuksa/val/v1"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

// tcpTarget is the host:port for the DATA_BROKER TCP listener.
const tcpTarget = "localhost:55556"

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

// skipIfPodmanUnavailable skips the test if the podman command is not available.
func skipIfPodmanUnavailable(t *testing.T) {
	t.Helper()
	if _, err := exec.LookPath("podman"); err != nil {
		t.Skipf("podman not available: %v", err)
	}
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

// setFloat sets a float signal value on the DATA_BROKER.
func setFloat(t *testing.T, client pb.VALClient, path string, value float32) {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	_, err := client.Set(ctx, &pb.SetRequest{
		Updates: []*pb.EntryUpdate{
			{
				Entry: &pb.DataEntry{
					Path: path,
					Value: &pb.Datapoint{
						Value: &pb.Datapoint_FloatValue{FloatValue: value},
					},
				},
				Fields: []pb.Field{pb.Field_FIELD_VALUE},
			},
		},
	})
	if err != nil {
		t.Fatalf("Set float(%q, %f) failed: %v", path, value, err)
	}
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

// setString sets a string signal value on the DATA_BROKER.
func setString(t *testing.T, client pb.VALClient, path string, value string) {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	_, err := client.Set(ctx, &pb.SetRequest{
		Updates: []*pb.EntryUpdate{
			{
				Entry: &pb.DataEntry{
					Path: path,
					Value: &pb.Datapoint{
						Value: &pb.Datapoint_StringValue{StringValue: value},
					},
				},
				Fields: []pb.Field{pb.Field_FIELD_VALUE},
			},
		},
	})
	if err != nil {
		t.Fatalf("Set string(%q, %q) failed: %v", path, value, err)
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

// getString reads a string signal value from the DATA_BROKER.
func getString(t *testing.T, client pb.VALClient, path string) *string {
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
	case *pb.Datapoint_StringValue:
		return &v.StringValue
	default:
		return nil
	}
}

// buildLockingService builds the locking-service Rust binary and returns the
// path to the compiled executable.
func buildLockingService(t *testing.T) string {
	t.Helper()
	root := repoRoot(t)
	serviceDir := filepath.Join(root, "rhivos", "locking-service")

	cmd := exec.Command("cargo", "build", "--release")
	cmd.Dir = serviceDir
	output, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("failed to build locking-service: %v\n%s", err, output)
	}

	return filepath.Join(root, "rhivos", "target", "release", "locking-service")
}

// startLockingService starts the locking-service binary with the given
// DATABROKER_ADDR and waits for the "locking-service ready" log line.
// Returns the exec.Cmd. The process is killed on test cleanup.
func startLockingService(t *testing.T, binary string, addr string) *exec.Cmd {
	t.Helper()

	cmd := exec.Command(binary, "serve")
	cmd.Env = append(os.Environ(), fmt.Sprintf("DATABROKER_ADDR=%s", addr))

	// Capture stderr for log scanning.
	stderrPipe, err := cmd.StderrPipe()
	if err != nil {
		t.Fatalf("failed to create stderr pipe: %v", err)
	}

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start locking-service: %v", err)
	}

	// Ensure cleanup on test completion.
	t.Cleanup(func() {
		if cmd.Process != nil {
			_ = cmd.Process.Kill()
			_ = cmd.Wait()
		}
	})

	// Wait for the "locking-service ready" log line with timeout.
	readyCh := make(chan struct{})
	go func() {
		scanner := bufio.NewScanner(stderrPipe)
		for scanner.Scan() {
			line := scanner.Text()
			if strings.Contains(line, "locking-service ready") {
				close(readyCh)
				// Keep draining stderr to prevent pipe blocking.
				for scanner.Scan() {
				}
				return
			}
		}
	}()

	select {
	case <-readyCh:
		// Service is ready.
	case <-time.After(30 * time.Second):
		t.Fatal("locking-service did not become ready within 30 seconds")
	}

	return cmd
}

// makeLockJSON creates a lock command JSON payload.
func makeLockJSON(commandID string) string {
	cmd := map[string]interface{}{
		"command_id": commandID,
		"action":     "lock",
		"doors":      []string{"driver"},
	}
	data, _ := json.Marshal(cmd)
	return string(data)
}

// makeUnlockJSON creates an unlock command JSON payload.
func makeUnlockJSON(commandID string) string {
	cmd := map[string]interface{}{
		"command_id": commandID,
		"action":     "unlock",
		"doors":      []string{"driver"},
	}
	data, _ := json.Marshal(cmd)
	return string(data)
}

// waitForResponse polls the Vehicle.Command.Door.Response signal until it
// contains the expected command_id or the timeout expires. Returns the parsed
// response JSON.
func waitForResponse(t *testing.T, client pb.VALClient, expectedCmdID string, timeout time.Duration) map[string]interface{} {
	t.Helper()
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		val := getString(t, client, "Vehicle.Command.Door.Response")
		if val != nil && *val != "" {
			var resp map[string]interface{}
			if err := json.Unmarshal([]byte(*val), &resp); err == nil {
				if id, ok := resp["command_id"].(string); ok && id == expectedCmdID {
					return resp
				}
			}
		}
		time.Sleep(200 * time.Millisecond)
	}
	t.Fatalf("timed out waiting for response with command_id %q", expectedCmdID)
	return nil
}

// startLockingServiceManual starts the locking-service binary and waits for
// readiness. Unlike startLockingService, it does NOT register a cleanup handler.
// The caller must manage the process lifecycle (kill/wait). Returns the command
// and a function to retrieve captured log lines (from stderr).
func startLockingServiceManual(t *testing.T, binary string, addr string) (*exec.Cmd, func() []string) {
	t.Helper()

	cmd := exec.Command(binary, "serve")
	cmd.Env = append(os.Environ(), fmt.Sprintf("DATABROKER_ADDR=%s", addr))

	stderrPipe, err := cmd.StderrPipe()
	if err != nil {
		t.Fatalf("failed to create stderr pipe: %v", err)
	}

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start locking-service: %v", err)
	}

	var mu sync.Mutex
	var logLines []string
	readyCh := make(chan struct{})

	go func() {
		scanner := bufio.NewScanner(stderrPipe)
		readyNotified := false
		for scanner.Scan() {
			line := scanner.Text()
			mu.Lock()
			logLines = append(logLines, line)
			mu.Unlock()
			if !readyNotified && strings.Contains(line, "locking-service ready") {
				close(readyCh)
				readyNotified = true
			}
		}
	}()

	select {
	case <-readyCh:
		// Service is ready.
	case <-time.After(30 * time.Second):
		_ = cmd.Process.Kill()
		_ = cmd.Wait()
		t.Fatal("locking-service did not become ready within 30 seconds")
	}

	getLogs := func() []string {
		mu.Lock()
		defer mu.Unlock()
		result := make([]string, len(logLines))
		copy(result, logLines)
		return result
	}

	return cmd, getLogs
}

// makeInvalidDoorJSON creates a lock command JSON payload with an unsupported
// door value.
func makeInvalidDoorJSON(commandID string) string {
	cmd := map[string]interface{}{
		"command_id": commandID,
		"action":     "lock",
		"doors":      []string{"passenger"},
	}
	data, _ := json.Marshal(cmd)
	return string(data)
}

// VSS signal path constants used across tests.
const (
	signalSpeed    = "Vehicle.Speed"
	signalDoorOpen = "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen"
	signalIsLocked = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked"
	signalCommand  = "Vehicle.Command.Door.Lock"
	signalResponse = "Vehicle.Command.Door.Response"
)
