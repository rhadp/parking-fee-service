// Package databroker_test contains integration tests for the DATA_BROKER
// (Eclipse Kuksa Databroker) component as defined in spec 02_data_broker.
//
// Tests are organised into:
//   - compose_test.go  — static checks of deployments/compose.yml
//   - signal_test.go   — live gRPC connectivity, metadata, set/get
//   - pubsub_test.go   — live gRPC subscription notifications
//   - edge_test.go     — edge-case / error scenarios
//   - property_test.go — property / invariant tests
//
// Live tests skip automatically when:
//   - grpcurl is not installed (requireGrpcurl)
//   - TCP port 55556 is not reachable (requireDatabrokerTCP)
//   - UDS socket is not accessible from the host (requireDatabrokerUDS)
package databroker_test

import (
	"fmt"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
	"testing"
	"time"
)

// ── Repo root ──────────────────────────────────────────────────────────────

// repoRoot returns the absolute path to the repository root.
// Navigates three levels up from this file: tests/databroker/ → tests/ → root.
func repoRoot(t *testing.T) string {
	t.Helper()
	_, filename, _, ok := runtime.Caller(0)
	if !ok {
		t.Fatal("runtime.Caller failed")
	}
	// filename is .../tests/databroker/helpers_test.go
	root := filepath.Join(filepath.Dir(filename), "..", "..")
	abs, err := filepath.Abs(root)
	if err != nil {
		t.Fatalf("filepath.Abs failed: %v", err)
	}
	return abs
}

// ── Skip conditions ────────────────────────────────────────────────────────

// requireGrpcurl skips the test if the grpcurl binary is not on PATH.
func requireGrpcurl(t *testing.T) {
	t.Helper()
	if _, err := exec.LookPath("grpcurl"); err != nil {
		t.Skip("grpcurl not found on PATH; skipping live gRPC test")
	}
}

// requireDatabrokerTCP skips the test if port 55556 on localhost is not
// reachable, which means the DATA_BROKER container is not running.
func requireDatabrokerTCP(t *testing.T) {
	t.Helper()
	requireGrpcurl(t)
	conn, err := net.DialTimeout("tcp", "localhost:55556", 2*time.Second)
	if err != nil {
		t.Skipf("DATA_BROKER not reachable on localhost:55556 (%v); skipping live test", err)
	}
	conn.Close()
}

// requireDatabrokerUDS skips the test when the UDS socket is not accessible
// from the test host. On macOS the socket lives inside the Podman VM and
// cannot be dialled directly from the host OS.
func requireDatabrokerUDS(t *testing.T) {
	t.Helper()
	requireGrpcurl(t)
	sockPath := udsSocketPath()
	if _, err := os.Stat(sockPath); os.IsNotExist(err) {
		t.Skipf("UDS socket not found at %s (possibly inside Podman VM); skipping UDS test", sockPath)
	}
	// On macOS the socket may exist in the VM filesystem but not be reachable.
	if runtime.GOOS == "darwin" {
		t.Skip("UDS socket is inside the Podman VM on macOS; skipping UDS test")
	}
}

// udsSocketPath returns the expected path of the UDS socket accessible from
// the test host. The compose.yml bind-mounts /tmp/kuksa from the host into
// the container, so the socket is at /tmp/kuksa/kuksa-databroker.sock on the
// host (Linux) or inside the Podman VM on macOS.
func udsSocketPath() string {
	return "/tmp/kuksa/kuksa-databroker.sock"
}

// udsGrpcTarget returns the grpcurl target string for the UDS socket.
func udsGrpcTarget() string {
	return "unix://" + udsSocketPath()
}

// ── grpcurl wrappers ───────────────────────────────────────────────────────

// grpcurlTCP runs grpcurl against the DATA_BROKER TCP endpoint
// (localhost:55556, plaintext) and returns (stdout, stderr, error).
func grpcurlTCP(t *testing.T, method string, body string, extraArgs ...string) (string, string, error) {
	t.Helper()
	return runGrpcurl(t, "localhost:55556", method, body, extraArgs...)
}

// grpcurlUDS runs grpcurl against the DATA_BROKER UDS endpoint and returns
// (stdout, stderr, error).
func grpcurlUDS(t *testing.T, method string, body string, extraArgs ...string) (string, string, error) {
	t.Helper()
	return runGrpcurl(t, udsGrpcTarget(), method, body, extraArgs...)
}

// runGrpcurl executes a grpcurl invocation and returns stdout, stderr, and any
// process error.  body may be empty string to send no request body.
func runGrpcurl(t *testing.T, target, method, body string, extraArgs ...string) (string, string, error) {
	t.Helper()
	args := []string{"-plaintext"}
	args = append(args, extraArgs...)
	if body != "" {
		args = append(args, "-d", body)
	}
	args = append(args, target, method)

	cmd := exec.Command("grpcurl", args...)
	var stdout, stderr strings.Builder
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr

	err := cmd.Run()
	return stdout.String(), stderr.String(), err
}

// grpcurlOK calls grpcurlTCP and fails the test if the call returns an error.
// Returns combined stdout+stderr for assertion.
func grpcurlOK(t *testing.T, method, body string) string {
	t.Helper()
	stdout, stderr, err := grpcurlTCP(t, method, body)
	if err != nil {
		t.Fatalf("grpcurl %s failed: %v\nstdout: %s\nstderr: %s", method, err, stdout, stderr)
	}
	return stdout + stderr
}

// grpcurlUDSOK calls grpcurlUDS and fails the test if the call returns an error.
func grpcurlUDSOK(t *testing.T, method, body string) string {
	t.Helper()
	stdout, stderr, err := grpcurlUDS(t, method, body)
	if err != nil {
		t.Fatalf("grpcurl UDS %s failed: %v\nstdout: %s\nstderr: %s", method, err, stdout, stderr)
	}
	return stdout + stderr
}

// ── Compose YAML helpers ───────────────────────────────────────────────────

// readComposeYML returns the contents of deployments/compose.yml.
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

// ── Port helpers ───────────────────────────────────────────────────────────

// waitForPort polls addr until a TCP connection succeeds or the timeout expires.
func waitForPort(addr string, timeout time.Duration) error {
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		conn, err := net.DialTimeout("tcp", addr, 200*time.Millisecond)
		if err == nil {
			conn.Close()
			return nil
		}
		time.Sleep(100 * time.Millisecond)
	}
	return fmt.Errorf("timeout waiting for %s", addr)
}

// ── Container helpers ──────────────────────────────────────────────────────

// podmanCompose runs a podman compose command in the deployments directory and
// returns the combined output and any error.
func podmanCompose(t *testing.T, args ...string) (string, error) {
	t.Helper()
	root := repoRoot(t)
	deploymentsDir := filepath.Join(root, "deployments")

	// Try "podman compose" first, fall back to "podman-compose"
	var cmd *exec.Cmd
	if _, err := exec.LookPath("podman"); err == nil {
		fullArgs := append([]string{"compose"}, args...)
		cmd = exec.Command("podman", fullArgs...)
	} else if _, err := exec.LookPath("podman-compose"); err == nil {
		cmd = exec.Command("podman-compose", args...)
	} else {
		t.Skip("neither 'podman compose' nor 'podman-compose' found; skipping container test")
		return "", nil
	}

	cmd.Dir = deploymentsDir
	out, err := cmd.CombinedOutput()
	return string(out), err
}

// requirePodman skips the test if podman is not available or its socket is
// not reachable (e.g. Podman machine is stopped).
func requirePodman(t *testing.T) {
	t.Helper()
	if _, err := exec.LookPath("podman"); err != nil {
		if _, err2 := exec.LookPath("podman-compose"); err2 != nil {
			t.Skip("podman not available; skipping container test")
		}
	}
	// Verify the podman socket is actually reachable by running a simple info command.
	cmd := exec.Command("podman", "info", "--format", "{{.Host.Arch}}")
	if err := cmd.Run(); err != nil {
		t.Skipf("podman socket not reachable (Podman machine may be stopped): %v", err)
	}
}
