package databroker_test

import (
	"context"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
	"testing"
	"time"
)

// ---- endpoint constants ----

const (
	tcpEndpoint = "localhost:55556"
	udsSocket   = "/tmp/kuksa-databroker.sock"
)

// ---- repo root ----

// repoRoot walks up from this test file's location until it finds the .git marker.
func repoRoot(t *testing.T) string {
	t.Helper()
	_, filename, _, ok := runtime.Caller(0)
	if !ok {
		t.Fatal("runtime.Caller failed")
	}
	dir := filepath.Dir(filename)
	for {
		if _, err := os.Stat(filepath.Join(dir, ".git")); err == nil {
			return dir
		}
		parent := filepath.Dir(dir)
		if parent == dir {
			t.Fatal("could not find repository root (no .git found)")
		}
		dir = parent
	}
}

// ---- skip guards ----

// requireTCPReachable skips the test if the databroker TCP port is not reachable.
func requireTCPReachable(t *testing.T) {
	t.Helper()
	conn, err := net.DialTimeout("tcp", tcpEndpoint, 2*time.Second)
	if err != nil {
		t.Skipf("databroker not reachable at %s (start with: cd deployments && podman compose up -d): %v",
			tcpEndpoint, err)
	}
	conn.Close()
}

// requireUDSSocket skips the test if the UDS socket is not accessible on this host.
// On macOS with Podman, the socket lives inside the Linux VM and is not reachable
// from the host, so these tests are skipped automatically.
func requireUDSSocket(t *testing.T) {
	t.Helper()
	if _, err := os.Stat(udsSocket); os.IsNotExist(err) {
		t.Skipf("UDS socket not found at %s (not host-accessible, e.g. macOS+Podman VM)", udsSocket)
	}
}

// requireGrpcurl skips the test if grpcurl is not in PATH.
func requireGrpcurl(t *testing.T) {
	t.Helper()
	if _, err := exec.LookPath("grpcurl"); err != nil {
		t.Skip("grpcurl not installed; get it from https://github.com/fullstorydev/grpcurl")
	}
}

// ---- grpcurl helpers ----

// grpcurlTCP runs grpcurl against the TCP endpoint with server reflection.
// It fails the test on grpcurl errors.
func grpcurlTCP(t *testing.T, method, data string) string {
	t.Helper()
	args := buildGrpcurlArgs(false, tcpEndpoint, method, data, nil)
	out, err := exec.Command("grpcurl", args...).CombinedOutput()
	if err != nil {
		t.Fatalf("grpcurl TCP %s failed: %v\noutput:\n%s", method, err, out)
	}
	return string(out)
}

// grpcurlTCPWithHeaders runs grpcurl against TCP with additional request headers.
func grpcurlTCPWithHeaders(t *testing.T, method, data string, headers map[string]string) string {
	t.Helper()
	args := buildGrpcurlArgs(false, tcpEndpoint, method, data, headers)
	out, err := exec.Command("grpcurl", args...).CombinedOutput()
	if err != nil {
		t.Fatalf("grpcurl TCP %s (with headers) failed: %v\noutput:\n%s", method, err, out)
	}
	return string(out)
}

// grpcurlTCPRaw runs grpcurl against TCP and returns output + error without failing.
func grpcurlTCPRaw(method, data string) (string, error) {
	args := buildGrpcurlArgs(false, tcpEndpoint, method, data, nil)
	out, err := exec.Command("grpcurl", args...).CombinedOutput()
	return string(out), err
}

// grpcurlUDS runs grpcurl against the UDS socket with server reflection.
// It fails the test on grpcurl errors.
func grpcurlUDS(t *testing.T, method, data string) string {
	t.Helper()
	args := buildGrpcurlArgs(true, udsSocket, method, data, nil)
	out, err := exec.Command("grpcurl", args...).CombinedOutput()
	if err != nil {
		t.Fatalf("grpcurl UDS %s failed: %v\noutput:\n%s", method, err, out)
	}
	return string(out)
}

// grpcurlSubscribeTCP subscribes to a signal via TCP, sets the signal using the
// provided setter function, and returns the subscription output (up to timeout).
//
// Subscribe request format (kuksa.val.v2): {"signal_paths": ["<path>"]}
// Response format: {"entries": {"<path>": {"value": {...}}}}
func grpcurlSubscribeTCP(t *testing.T, signal string, timeout time.Duration, setter func()) string {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), timeout)
	defer cancel()

	data := `{"signal_paths": ["` + signal + `"]}`
	args := buildGrpcurlArgs(false, tcpEndpoint, "kuksa.val.v2.VAL/Subscribe", data, nil)

	cmd := exec.CommandContext(ctx, "grpcurl", args...)
	outBuf := &strings.Builder{}
	cmd.Stdout = outBuf
	cmd.Stderr = outBuf

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start grpcurl subscriber: %v", err)
	}

	// Give the subscriber a moment to connect before the setter fires.
	time.Sleep(300 * time.Millisecond)
	setter()

	// Wait for output to accumulate, then cancel.
	time.Sleep(timeout - 500*time.Millisecond)
	cancel()
	_ = cmd.Wait()
	return outBuf.String()
}

// grpcurlSubscribeUDS subscribes to a signal via UDS socket (if accessible).
func grpcurlSubscribeUDS(t *testing.T, signal string, timeout time.Duration, setter func()) string {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), timeout)
	defer cancel()

	data := `{"signal_paths": ["` + signal + `"]}`
	args := buildGrpcurlArgs(true, udsSocket, "kuksa.val.v2.VAL/Subscribe", data, nil)

	cmd := exec.CommandContext(ctx, "grpcurl", args...)
	outBuf := &strings.Builder{}
	cmd.Stdout = outBuf
	cmd.Stderr = outBuf

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start grpcurl UDS subscriber: %v", err)
	}

	time.Sleep(300 * time.Millisecond)
	setter()

	time.Sleep(timeout - 500*time.Millisecond)
	cancel()
	_ = cmd.Wait()
	return outBuf.String()
}

// buildGrpcurlArgs constructs a grpcurl argument slice.
func buildGrpcurlArgs(uds bool, target, method, data string, headers map[string]string) []string {
	args := []string{"-plaintext"}
	if uds {
		args = append(args, "-unix")
	}
	for k, v := range headers {
		args = append(args, "-H", k+": "+v)
	}
	if data != "" {
		args = append(args, "-d", data)
	}
	args = append(args, target, method)
	return args
}

// ---- compose.yml helpers ----

// readComposeYML reads deployments/compose.yml and returns its content as a string.
func readComposeYML(t *testing.T) string {
	t.Helper()
	root := repoRoot(t)
	path := filepath.Join(root, "deployments", "compose.yml")
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("cannot read %s: %v", path, err)
	}
	return string(data)
}

// ---- assertion helpers ----

func assertContains(t *testing.T, haystack, needle, msg string) {
	t.Helper()
	if !strings.Contains(haystack, needle) {
		t.Errorf("%s: expected to find %q", msg, needle)
	}
}

func assertNotContains(t *testing.T, haystack, needle, msg string) {
	t.Helper()
	if strings.Contains(haystack, needle) {
		t.Errorf("%s: expected NOT to find %q", msg, needle)
	}
}
