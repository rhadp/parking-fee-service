package updateservice_test

import (
	"bufio"
	"context"
	"fmt"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"sync"
	"testing"
	"time"

	pb "github.com/rhadp/parking-fee-service/gen/update_service/v1"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

// defaultGRPCPort is the default port the UPDATE_SERVICE listens on.
const defaultGRPCPort = 50052

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

// buildUpdateService builds the update-service Rust binary and returns the
// path to the compiled executable.
func buildUpdateService(t *testing.T) string {
	t.Helper()
	root := repoRoot(t)
	serviceDir := filepath.Join(root, "rhivos", "update-service")

	cmd := exec.Command("cargo", "build", "--release", "-p", "update-service")
	cmd.Dir = serviceDir
	output, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("failed to build update-service: %v\n%s", err, output)
	}

	return filepath.Join(root, "rhivos", "target", "release", "update-service")
}

// serviceInstance holds the running service process and its log capture.
type serviceInstance struct {
	cmd     *exec.Cmd
	getLogs func() []string
}

// startUpdateService starts the update-service binary with a given config
// and waits for the "ready" log line. Returns a serviceInstance.
// The process is NOT automatically killed; the caller manages its lifecycle.
func startUpdateService(t *testing.T, binary string, port int, configPath string) *serviceInstance {
	t.Helper()

	cmd := exec.Command(binary)
	env := os.Environ()
	if configPath != "" {
		env = append(env, fmt.Sprintf("CONFIG_PATH=%s", configPath))
	} else {
		// Use a non-existent config path so the service starts with defaults.
		env = append(env, "CONFIG_PATH=/nonexistent/config.json")
	}
	cmd.Env = env

	// tracing_subscriber::fmt writes to stdout by default.
	stdoutPipe, err := cmd.StdoutPipe()
	if err != nil {
		t.Fatalf("failed to create stdout pipe: %v", err)
	}

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start update-service: %v", err)
	}

	var mu sync.Mutex
	var logLines []string
	readyCh := make(chan struct{})

	go func() {
		scanner := bufio.NewScanner(stdoutPipe)
		readyNotified := false
		for scanner.Scan() {
			line := scanner.Text()
			mu.Lock()
			logLines = append(logLines, line)
			mu.Unlock()
			if !readyNotified && strings.Contains(strings.ToLower(line), "ready") {
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
		t.Fatal("update-service did not become ready within 30 seconds")
	}

	getLogs := func() []string {
		mu.Lock()
		defer mu.Unlock()
		result := make([]string, len(logLines))
		copy(result, logLines)
		return result
	}

	return &serviceInstance{cmd: cmd, getLogs: getLogs}
}

// startUpdateServiceWithCleanup starts the service and registers a test
// cleanup handler to kill it.
func startUpdateServiceWithCleanup(t *testing.T, binary string, port int, configPath string) *serviceInstance {
	t.Helper()
	si := startUpdateService(t, binary, port, configPath)
	t.Cleanup(func() {
		if si.cmd.Process != nil {
			_ = si.cmd.Process.Kill()
			_ = si.cmd.Wait()
		}
	})
	return si
}

// connectGRPC creates a gRPC client connection to the UPDATE_SERVICE.
// The connection is closed automatically when the test completes.
func connectGRPC(t *testing.T, port int) *grpc.ClientConn {
	t.Helper()
	target := fmt.Sprintf("localhost:%d", port)
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	conn, err := grpc.DialContext(ctx, target,
		grpc.WithTransportCredentials(insecure.NewCredentials()),
		grpc.WithBlock(),
	)
	if err != nil {
		t.Fatalf("failed to connect to UPDATE_SERVICE at %s: %v", target, err)
	}
	t.Cleanup(func() { conn.Close() })
	return conn
}

// newUpdateServiceClient creates an UpdateService gRPC client from a connection.
func newUpdateServiceClient(conn *grpc.ClientConn) pb.UpdateServiceClient {
	return pb.NewUpdateServiceClient(conn)
}

// findFreePort finds a free TCP port by binding to :0 and returning the
// assigned port. This avoids port conflicts between parallel tests.
func findFreePort(t *testing.T) int {
	t.Helper()
	l, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to find free port: %v", err)
	}
	port := l.Addr().(*net.TCPAddr).Port
	l.Close()
	return port
}

// writeConfigFile creates a temporary JSON config file with the given port
// and returns its path. The file is removed on test cleanup.
func writeConfigFile(t *testing.T, port int) string {
	t.Helper()
	dir := t.TempDir()
	path := filepath.Join(dir, "config.json")
	content := fmt.Sprintf(`{"grpc_port":%d,"inactivity_timeout_secs":86400}`, port)
	if err := os.WriteFile(path, []byte(content), 0644); err != nil {
		t.Fatalf("failed to write config file: %v", err)
	}
	return path
}
