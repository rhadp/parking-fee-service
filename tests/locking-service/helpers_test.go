// Package lockingservice contains integration tests for the LOCKING_SERVICE.
//
// Tests that require a live DATA_BROKER and a running locking-service skip
// gracefully when the infrastructure is unavailable. Because the locking-service
// uses the kuksa.val.v1 API and the current DATA_BROKER (kuksa-databroker 0.5.0)
// only exposes kuksa.val.v2, tests that need end-to-end processing will skip in
// practice. See docs/errata/03_locking_service_proto_compat.md for details.
//
// TestConnectionRetryFailure (TS-03-E1) and TestStartupLogging run without any
// infrastructure and always execute when cargo is available.
package lockingservice

import (
	"bytes"
	"fmt"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"sync"
	"testing"
	"time"
)

const (
	// tcpAddr is the host:port of the DATA_BROKER TCP listener.
	tcpAddr = "localhost:55556"

	// grpcService is the kuksa.val.v2 gRPC service name used by grpcurl.
	grpcService = "kuksa.val.v2.VAL"

	// Signal paths used by the locking-service.
	signalIsLocked = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked"
	signalIsOpen   = "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen"
	signalSpeed    = "Vehicle.Speed"
	signalCommand  = "Vehicle.Command.Door.Lock"
	signalResponse = "Vehicle.Command.Door.Response"
)

// safeBuffer is a thread-safe byte buffer for capturing subprocess output.
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

// findRepoRoot locates the repository root by running `git rev-parse --show-toplevel`.
func findRepoRoot(t *testing.T) string {
	t.Helper()
	cmd := exec.Command("git", "rev-parse", "--show-toplevel")
	out, err := cmd.Output()
	if err != nil {
		t.Fatalf("failed to find repo root via git: %v", err)
	}
	return strings.TrimSpace(string(out))
}

// skipIfGrpcurlMissing skips the test if grpcurl is not available in PATH.
func skipIfGrpcurlMissing(t *testing.T) {
	t.Helper()
	if _, err := exec.LookPath("grpcurl"); err != nil {
		t.Skip("grpcurl not in PATH; skipping live gRPC test")
	}
}

// skipIfPodmanMissing skips the test if podman is not in PATH.
func skipIfPodmanMissing(t *testing.T) {
	t.Helper()
	if _, err := exec.LookPath("podman"); err != nil {
		t.Skip("podman not in PATH; skipping infrastructure lifecycle test")
	}
}

// skipIfDataBrokerUnreachable skips the test if the DATA_BROKER TCP port is not reachable.
func skipIfDataBrokerUnreachable(t *testing.T) {
	t.Helper()
	conn, err := net.DialTimeout("tcp", tcpAddr, 2*time.Second)
	if err != nil {
		t.Skipf("DATA_BROKER TCP not reachable at %s: %v", tcpAddr, err)
	}
	conn.Close()
}

// buildLockingServiceBinary builds the locking-service Rust binary and returns its path.
// Skips the test if cargo is not available in PATH.
func buildLockingServiceBinary(t *testing.T) string {
	t.Helper()
	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("cargo not in PATH; skipping binary-dependent test")
	}
	root := findRepoRoot(t)
	rhivosDir := filepath.Join(root, "rhivos")
	cmd := exec.Command("cargo", "build", "-p", "locking-service")
	cmd.Dir = rhivosDir
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("cargo build -p locking-service failed:\n%s\nerror: %v", out, err)
	}
	return filepath.Join(rhivosDir, "target", "debug", "locking-service")
}

// lockingServiceProcess manages a running locking-service subprocess.
type lockingServiceProcess struct {
	cmd    *exec.Cmd
	output *safeBuffer
}

// startLockingService starts the locking-service binary with the given DATABROKER_ADDR.
// Registers a cleanup that sends SIGKILL to the process when the test ends.
func startLockingService(t *testing.T, binPath, dataBrokerAddr string) *lockingServiceProcess {
	t.Helper()
	buf := &safeBuffer{}
	cmd := exec.Command(binPath, "serve")
	cmd.Env = append(os.Environ(),
		"DATABROKER_ADDR="+dataBrokerAddr,
		"RUST_LOG=info",
	)
	cmd.Stdout = buf
	cmd.Stderr = buf

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start locking-service: %v", err)
	}

	proc := &lockingServiceProcess{cmd: cmd, output: buf}

	t.Cleanup(func() {
		if cmd.Process != nil {
			_ = cmd.Process.Kill()
			_ = cmd.Wait()
		}
	})

	return proc
}

// waitForLog polls the process output until the given substring appears or the
// timeout is reached. Returns true if the substring was found.
func waitForLog(proc *lockingServiceProcess, substr string, timeout time.Duration) bool {
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		if strings.Contains(proc.output.String(), substr) {
			return true
		}
		time.Sleep(50 * time.Millisecond)
	}
	// Final check after timeout.
	return strings.Contains(proc.output.String(), substr)
}

// grpcurlPublishValue sends a PublishValue request to the DATA_BROKER via grpcurl.
// Logs (but does not fail) on grpcurl errors — callers assert outcomes separately.
func grpcurlPublishValue(t *testing.T, reqJSON string) string {
	t.Helper()
	args := []string{"-plaintext", "-d", reqJSON, tcpAddr, grpcService + "/PublishValue"}
	cmd := exec.Command("grpcurl", args...)
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Logf("grpcurl PublishValue warning: %v\noutput: %s", err, out)
	}
	return string(out)
}

// grpcurlGetValue sends a GetValue request to the DATA_BROKER via grpcurl.
// Logs (but does not fail) on grpcurl errors — callers assert outcomes separately.
func grpcurlGetValue(t *testing.T, signalPath string) string {
	t.Helper()
	reqJSON := fmt.Sprintf(`{"signal_id": {"path": %q}}`, signalPath)
	args := []string{"-plaintext", "-d", reqJSON, tcpAddr, grpcService + "/GetValue"}
	cmd := exec.Command("grpcurl", args...)
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Logf("grpcurl GetValue warning for %s: %v\noutput: %s", signalPath, err, out)
	}
	return string(out)
}

// grpcurlPublishBool publishes a boolean value to the named signal via grpcurl.
func grpcurlPublishBool(t *testing.T, signalPath string, value bool) {
	t.Helper()
	reqJSON := fmt.Sprintf(`{"signal_id": {"path": %q}, "value": {"bool_value": %v}}`,
		signalPath, value)
	grpcurlPublishValue(t, reqJSON)
}

// grpcurlPublishFloat publishes a float value to the named signal via grpcurl.
func grpcurlPublishFloat(t *testing.T, signalPath string, value float64) {
	t.Helper()
	reqJSON := fmt.Sprintf(`{"signal_id": {"path": %q}, "value": {"float_value": %v}}`,
		signalPath, value)
	grpcurlPublishValue(t, reqJSON)
}

// grpcurlPublishString publishes a string value to the named signal via grpcurl.
func grpcurlPublishString(t *testing.T, signalPath string, value string) {
	t.Helper()
	reqJSON := fmt.Sprintf(`{"signal_id": {"path": %q}, "value": {"string_value": %q}}`,
		signalPath, value)
	grpcurlPublishValue(t, reqJSON)
}

// pollSignalForContent polls GetValue on the named signal until the output contains
// the given substring or the timeout is reached. Returns the last output.
func pollSignalForContent(t *testing.T, signalPath, substr string, timeout time.Duration) (string, bool) {
	t.Helper()
	deadline := time.Now().Add(timeout)
	var lastOut string
	for time.Now().Before(deadline) {
		lastOut = grpcurlGetValue(t, signalPath)
		if strings.Contains(lastOut, substr) {
			return lastOut, true
		}
		time.Sleep(100 * time.Millisecond)
	}
	return lastOut, false
}
