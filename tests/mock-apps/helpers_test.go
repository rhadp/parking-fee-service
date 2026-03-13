// Package mockapps contains integration tests for the mock apps
// (mock sensors, parking-operator, companion-app-cli, parking-app-cli).
//
// Tests that require DATA_BROKER skip automatically when the broker is not
// reachable. Tests that require built binaries build them at test time.
package mockapps

import (
	"context"
	"fmt"
	"net"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"testing"
	"time"

	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

const (
	defaultBrokerAddr = "localhost:55556"
	brokerHTTPAddr    = "http://localhost:55556"
)

// repoRoot returns the absolute path to the repository root.
// The tests live in tests/mock-apps/, so the root is two levels up.
func repoRoot(t *testing.T) string {
	t.Helper()
	_, filename, _, ok := runtime.Caller(0)
	if !ok {
		t.Fatal("runtime.Caller failed")
	}
	// helpers_test.go is in tests/mock-apps/ → root is ../../
	root := filepath.Join(filepath.Dir(filename), "..", "..")
	abs, err := filepath.Abs(root)
	if err != nil {
		t.Fatalf("failed to resolve repo root: %v", err)
	}
	return abs
}

// isBrokerAvailable returns true if the Kuksa DATA_BROKER is reachable via TCP.
func isBrokerAvailable() bool {
	conn, err := grpc.NewClient(
		defaultBrokerAddr,
		grpc.WithTransportCredentials(insecure.NewCredentials()),
	)
	if err != nil {
		return false
	}
	defer conn.Close()
	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer cancel()
	conn.Connect()
	// Try a TCP dial to confirm reachability
	tcpConn, err := (&net.Dialer{}).DialContext(ctx, "tcp", defaultBrokerAddr)
	if err != nil {
		return false
	}
	tcpConn.Close()
	return true
}

// skipIfBrokerUnavailable skips the test if DATA_BROKER is not reachable.
func skipIfBrokerUnavailable(t *testing.T) {
	t.Helper()
	if !isBrokerAvailable() {
		t.Skip("DATA_BROKER not available at " + defaultBrokerAddr + "; skipping integration test")
	}
}

// findFreePort returns an available TCP port on localhost.
func findFreePort(t *testing.T) int {
	t.Helper()
	ln, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to find free port: %v", err)
	}
	port := ln.Addr().(*net.TCPAddr).Port
	ln.Close()
	return port
}

// buildGoBinary builds the Go package at srcDir and returns the path to the
// compiled binary in a temp directory. The binary is registered for cleanup.
func buildGoBinary(t *testing.T, srcDir string) string {
	t.Helper()
	tmpDir := t.TempDir()
	binName := filepath.Base(srcDir)
	binPath := filepath.Join(tmpDir, binName)
	cmd := exec.Command("go", "build", "-o", binPath, ".")
	cmd.Dir = srcDir
	// Propagate GOWORK so workspace replace directives are honoured.
	cmd.Env = os.Environ()
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("go build failed in %s: %v\n%s", srcDir, err, string(out))
	}
	return binPath
}

// buildRustBinary builds the Rust binary with the given package name from the
// rhivos Cargo workspace and returns its path. The binary lives in
// rhivos/target/debug/<name>.
func buildRustBinary(t *testing.T, packageName string) string {
	t.Helper()
	root := repoRoot(t)
	rhivosDir := filepath.Join(root, "rhivos")
	cmd := exec.Command("cargo", "build", "-p", packageName)
	cmd.Dir = rhivosDir
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("cargo build -p %s failed: %v\n%s", packageName, err, string(out))
	}
	binPath := filepath.Join(rhivosDir, "target", "debug", packageName)
	if _, err := os.Stat(binPath); err != nil {
		t.Fatalf("expected binary at %s but stat failed: %v", binPath, err)
	}
	return binPath
}

// waitForHTTPReady polls the given URL until it returns any HTTP response (not
// a connection error) or the timeout elapses.
func waitForHTTPReady(t *testing.T, url string, timeout time.Duration) {
	t.Helper()
	deadline := time.Now().Add(timeout)
	client := &http.Client{Timeout: 500 * time.Millisecond}
	for time.Now().Before(deadline) {
		resp, err := client.Get(url)
		if err == nil {
			resp.Body.Close()
			return
		}
		time.Sleep(100 * time.Millisecond)
	}
	t.Fatalf("service at %s did not become ready within %v", url, timeout)
}

// startProcess starts a background process with the given args.
// The process is automatically terminated (SIGKILL) when the test ends unless
// it has already exited.
func startProcess(t *testing.T, args ...string) *exec.Cmd {
	t.Helper()
	if len(args) == 0 {
		t.Fatal("startProcess requires at least one argument")
	}
	cmd := exec.Command(args[0], args[1:]...)
	cmd.Stdout = nil
	cmd.Stderr = nil
	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start process %v: %v", args, err)
	}
	t.Cleanup(func() {
		if cmd.ProcessState == nil || !cmd.ProcessState.Exited() {
			_ = cmd.Process.Kill()
		}
	})
	return cmd
}

// newGRPCConn returns an insecure gRPC connection to addr.
func newGRPCConn(t *testing.T, addr string) *grpc.ClientConn {
	t.Helper()
	conn, err := grpc.NewClient(addr, grpc.WithTransportCredentials(insecure.NewCredentials()))
	if err != nil {
		t.Fatalf("grpc.NewClient(%s) failed: %v", addr, err)
	}
	t.Cleanup(func() { conn.Close() })
	return conn
}

// brokerGRPCAddr returns the DATA_BROKER gRPC address from the env or default.
func brokerGRPCAddr() string {
	if addr := os.Getenv("DATA_BROKER_ADDR"); addr != "" {
		// Strip http:// prefix if present for raw gRPC dialing
		if len(addr) > 7 && addr[:7] == "http://" {
			return addr[7:]
		}
		return addr
	}
	return defaultBrokerAddr
}

// runSensor executes a sensor binary with the given args and environment.
// Returns the exit code. It does NOT fail the test on non-zero exit codes;
// callers must check the return value.
func runSensor(t *testing.T, binPath string, env []string, args ...string) (int, string, string) {
	t.Helper()
	cmd := exec.Command(binPath, args...)
	cmd.Env = env
	var stdout, stderr []byte
	outPipe, _ := cmd.StdoutPipe()
	errPipe, _ := cmd.StderrPipe()
	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start %s: %v", binPath, err)
	}
	if outPipe != nil {
		buf := make([]byte, 4096)
		n, _ := outPipe.Read(buf)
		stdout = buf[:n]
	}
	if errPipe != nil {
		buf := make([]byte, 4096)
		n, _ := errPipe.Read(buf)
		stderr = buf[:n]
	}
	err := cmd.Wait()
	code := 0
	if err != nil {
		if exitErr, ok := err.(*exec.ExitError); ok {
			code = exitErr.ExitCode()
		} else {
			code = -1
		}
	}
	return code, string(stdout), string(stderr)
}

// parkingOperatorSrcDir returns the absolute path to the parking-operator Go source.
func parkingOperatorSrcDir(t *testing.T) string {
	t.Helper()
	root := repoRoot(t)
	return filepath.Join(root, "mock", "parking-operator")
}

// statusURL returns the /parking/status/<id> URL for the given base and sessionID.
func statusURL(base, sessionID string) string {
	return fmt.Sprintf("%s/parking/status/%s", base, sessionID)
}
