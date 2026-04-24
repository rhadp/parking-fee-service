package updateservice_test

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
	"syscall"
	"testing"
	"time"

	"github.com/rhadp/parking-fee-service/gen/update"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

const (
	// connectTimeout is the maximum time to wait for a gRPC connection.
	connectTimeout = 5 * time.Second

	// rpcTimeout is the maximum time to wait for a single gRPC call.
	rpcTimeout = 5 * time.Second

	// serviceReadyTimeout is the maximum time to wait for the
	// update-service to become ready after startup.
	serviceReadyTimeout = 15 * time.Second
)

// getFreePort finds a free TCP port by briefly binding to :0 and returning
// the assigned port. The port is released before returning, so the caller
// should bind to it quickly.
func getFreePort(t *testing.T) int {
	t.Helper()
	l, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to get free port: %v", err)
	}
	port := l.Addr().(*net.TCPAddr).Port
	l.Close()
	return port
}

// writeConfigFile creates a temporary JSON config file with the given port
// and returns its path. The file is automatically removed when the test
// completes.
func writeConfigFile(t *testing.T, port int) string {
	t.Helper()
	cfg := map[string]interface{}{
		"grpc_port":              port,
		"registry_url":           "",
		"inactivity_timeout_secs": 86400,
		"container_storage_path": "/var/lib/containers/adapters/",
	}
	data, err := json.Marshal(cfg)
	if err != nil {
		t.Fatalf("failed to marshal config: %v", err)
	}
	f, err := os.CreateTemp("", "update-service-config-*.json")
	if err != nil {
		t.Fatalf("failed to create temp config: %v", err)
	}
	if _, err := f.Write(data); err != nil {
		t.Fatalf("failed to write config: %v", err)
	}
	f.Close()
	t.Cleanup(func() { os.Remove(f.Name()) })
	return f.Name()
}

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

// buildUpdateService builds the update-service Rust binary using cargo
// and returns the path to the compiled binary.
func buildUpdateService(t *testing.T) string {
	t.Helper()
	root := findRepoRoot(t)
	rhivosDir := filepath.Join(root, "rhivos")

	cmd := exec.Command("cargo", "build", "-p", "update-service")
	cmd.Dir = rhivosDir
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("failed to build update-service: %v\n%s", err, string(out))
	}

	binaryPath := filepath.Join(rhivosDir, "target", "debug", "update-service")
	if _, err := os.Stat(binaryPath); os.IsNotExist(err) {
		t.Fatalf("update-service binary not found at %s after build", binaryPath)
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

// updateServiceProcess represents a running update-service process.
type updateServiceProcess struct {
	cmd  *exec.Cmd
	logs *logCapture
	port int
}

// startUpdateService starts the update-service binary on a dynamically
// allocated port, waits for the "UPDATE_SERVICE ready" log line, and
// registers a cleanup function that kills the process when the test
// completes.
func startUpdateService(t *testing.T, binaryPath string) *updateServiceProcess {
	t.Helper()

	port := getFreePort(t)
	configPath := writeConfigFile(t, port)

	cmd := exec.Command(binaryPath)
	cmd.Env = []string{
		fmt.Sprintf("CONFIG_PATH=%s", configPath),
		fmt.Sprintf("PATH=%s", os.Getenv("PATH")),
		fmt.Sprintf("HOME=%s", os.Getenv("HOME")),
	}
	// Put child in its own process group to avoid signal interference
	// with the test process.
	cmd.SysProcAttr = &syscall.SysProcAttr{Setpgid: true}

	// The tracing subscriber in update-service writes to stdout by default.
	stdoutPipe, err := cmd.StdoutPipe()
	if err != nil {
		t.Fatalf("failed to create stdout pipe: %v", err)
	}

	// Also capture stderr for completeness.
	stderrPipe, err := cmd.StderrPipe()
	if err != nil {
		t.Fatalf("failed to create stderr pipe: %v", err)
	}

	logs := newLogCapture()

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start update-service: %v", err)
	}

	// Read stdout in background, capturing log lines.
	go func() {
		scanner := bufio.NewScanner(stdoutPipe)
		for scanner.Scan() {
			logs.appendLine(scanner.Text())
		}
	}()

	// Read stderr in background, capturing log lines.
	go func() {
		scanner := bufio.NewScanner(stderrPipe)
		for scanner.Scan() {
			logs.appendLine(scanner.Text())
		}
	}()

	// Wait for the service to become ready.
	if !logs.waitFor("UPDATE_SERVICE ready", serviceReadyTimeout) {
		_ = cmd.Process.Kill()
		_ = cmd.Wait()
		allLogs := strings.Join(logs.allLines(), "\n")
		t.Fatalf("update-service did not become ready within %v\nLogs:\n%s", serviceReadyTimeout, allLogs)
	}

	// Register cleanup to kill the process when the test finishes.
	// Check ProcessState to avoid killing an already-exited process.
	t.Cleanup(func() {
		if cmd.Process != nil && cmd.ProcessState == nil {
			_ = cmd.Process.Kill()
			_ = cmd.Wait()
		}
	})

	return &updateServiceProcess{cmd: cmd, logs: logs, port: port}
}

// dialGRPC creates a gRPC client connection to the update-service at the
// given port.
func dialGRPC(t *testing.T, port int) *grpc.ClientConn {
	t.Helper()
	target := fmt.Sprintf("localhost:%d", port)
	ctx, cancel := context.WithTimeout(context.Background(), connectTimeout)
	defer cancel()

	conn, err := grpc.DialContext(ctx, target,
		grpc.WithTransportCredentials(insecure.NewCredentials()),
		grpc.WithBlock(),
	)
	if err != nil {
		t.Fatalf("failed to dial gRPC %s: %v", target, err)
	}
	t.Cleanup(func() { conn.Close() })
	return conn
}

// newUpdateServiceClient creates an UpdateService gRPC client connected
// to the service at the given port.
func newUpdateServiceClient(t *testing.T, port int) update.UpdateServiceClient {
	t.Helper()
	return update.NewUpdateServiceClient(dialGRPC(t, port))
}
