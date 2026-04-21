// Package databroker contains integration tests for the DATA_BROKER (Kuksa Databroker)
// component. Tests use grpcurl subprocesses to interact with the gRPC API over TCP and UDS
// transports. Live gRPC tests skip when the DATA_BROKER container is unavailable or grpcurl
// is not installed. Static compose.yml config tests run without any container.
package databroker

import (
	"context"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

const (
	// tcpAddr is the host address for the DATA_BROKER TCP listener.
	tcpAddr = "localhost:55556"

	// udsSocketPrimary is the socket path expected when the container /tmp directory
	// is bind-mounted to /tmp/kuksa on the host (compose named-volume scenario).
	udsSocketPrimary = "/tmp/kuksa/kuksa-databroker.sock"

	// udsSocketFallback is the direct socket path used when the socket is at /tmp
	// on the host (e.g., during manual container runs or alternate compose configs).
	udsSocketFallback = "/tmp/kuksa-databroker.sock"

	// grpcService is the fully-qualified Kuksa VAL gRPC service name.
	grpcService = "kuksa.val.v2.VAL"
)

// findRepoRoot locates the repository root by running `git rev-parse --show-toplevel`.
// It fails the test if the root cannot be determined.
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

// skipIfTCPNotReachable skips the test if the DATA_BROKER TCP port is not reachable.
func skipIfTCPNotReachable(t *testing.T) {
	t.Helper()
	conn, err := net.DialTimeout("tcp", tcpAddr, 2*time.Second)
	if err != nil {
		t.Skipf("DATA_BROKER TCP not reachable at %s: %v", tcpAddr, err)
	}
	conn.Close()
}

// effectiveUDSSocket returns the host path to the UDS socket that currently exists,
// checking the primary (volume-mapped) path first, then the fallback.
// Skips the test if neither path exists.
func effectiveUDSSocket(t *testing.T) string {
	t.Helper()
	for _, path := range []string{udsSocketPrimary, udsSocketFallback} {
		if _, err := os.Stat(path); err == nil {
			return path
		}
	}
	t.Skipf("UDS socket not found (checked %q and %q); skipping", udsSocketPrimary, udsSocketFallback)
	return "" // unreachable
}

// grpcurlTCP runs grpcurl against the DATA_BROKER TCP endpoint and returns combined output.
// The test fails if grpcurl exits with a non-zero status.
func grpcurlTCP(t *testing.T, method, reqJSON string) string {
	t.Helper()
	args := []string{"-plaintext", "-d", reqJSON, tcpAddr, grpcService + "/" + method}
	cmd := exec.Command("grpcurl", args...)
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("grpcurl TCP %s failed: %v\noutput: %s", method, err, out)
	}
	return string(out)
}

// grpcurlTCPExpectError runs grpcurl against the TCP endpoint and expects a non-zero exit.
// Returns the combined output. The test fails if grpcurl unexpectedly succeeds.
func grpcurlTCPExpectError(t *testing.T, method, reqJSON string) string {
	t.Helper()
	args := []string{"-plaintext", "-d", reqJSON, tcpAddr, grpcService + "/" + method}
	cmd := exec.Command("grpcurl", args...)
	out, err := cmd.CombinedOutput()
	if err == nil {
		t.Fatalf("grpcurl TCP %s expected error but succeeded\noutput: %s", method, out)
	}
	return string(out)
}

// grpcurlUDS runs grpcurl against the DATA_BROKER UDS endpoint and returns combined output.
// sockPath must be the absolute host path to the socket file.
// The test fails if grpcurl exits with a non-zero status.
func grpcurlUDS(t *testing.T, sockPath, method, reqJSON string) string {
	t.Helper()
	target := "unix://" + sockPath
	args := []string{"-plaintext", "-d", reqJSON, target, grpcService + "/" + method}
	cmd := exec.Command("grpcurl", args...)
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("grpcurl UDS %s failed: %v\noutput: %s", method, err, out)
	}
	return string(out)
}

// readComposeYML reads deployments/compose.yml and returns its content as a string.
func readComposeYML(t *testing.T) string {
	t.Helper()
	root := findRepoRoot(t)
	path := filepath.Join(root, "deployments", "compose.yml")
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("failed to read compose.yml: %v", err)
	}
	return string(data)
}

// skipIfPodmanMissing skips the test if podman is not available in PATH.
func skipIfPodmanMissing(t *testing.T) {
	t.Helper()
	if _, err := exec.LookPath("podman"); err != nil {
		t.Skip("podman not in PATH; skipping container lifecycle test")
	}
}

// skipIfPodmanNotRunning skips the test if the Podman machine/daemon is not running.
// This is a stronger check than skipIfPodmanMissing: it verifies the socket is reachable.
func skipIfPodmanNotRunning(t *testing.T) {
	t.Helper()
	skipIfPodmanMissing(t)
	cmd := exec.Command("podman", "info")
	if err := cmd.Run(); err != nil {
		t.Skipf("Podman daemon not running (podman info failed): %v", err)
	}
}

// runPodmanCompose runs `podman compose <args>` in the deployments directory.
// Returns combined output and any error.
func runPodmanCompose(t *testing.T, args ...string) (string, error) {
	t.Helper()
	root := findRepoRoot(t)
	deploymentsDir := filepath.Join(root, "deployments")
	cmdArgs := append([]string{"compose"}, args...)
	cmd := exec.Command("podman", cmdArgs...)
	cmd.Dir = deploymentsDir
	out, err := cmd.CombinedOutput()
	return string(out), err
}

// runPodmanComposeCtx runs `podman compose <args>` in the deployments directory
// with a context for timeout control. Returns combined output and any error.
func runPodmanComposeCtx(ctx context.Context, t *testing.T, args ...string) (string, error) {
	t.Helper()
	root := findRepoRoot(t)
	deploymentsDir := filepath.Join(root, "deployments")
	cmdArgs := append([]string{"compose"}, args...)
	cmd := exec.CommandContext(ctx, "podman", cmdArgs...)
	cmd.Dir = deploymentsDir
	out, err := cmd.CombinedOutput()
	return string(out), err
}
