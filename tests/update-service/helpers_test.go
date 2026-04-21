// Package updateservice contains integration tests for the UPDATE_SERVICE.
//
// Tests that do not require external infrastructure (no real podman, no real OCI
// registry) always run when cargo is available:
//   - TestStartupLogging (TS-07-17)
//   - TestGracefulShutdown (TS-07-18)
//   - TestListAdaptersEmpty, TestGetAdapterStatusNotFound, TestRemoveAdapterNotFound,
//     TestInstallAdapterInvalidArgument (gRPC basic error paths)
//
// Smoke tests that require a real podman environment and a reachable OCI registry
// are skipped gracefully when those are not available.
package updateservice

import (
	"bytes"
	"fmt"
	"net"
	"os/exec"
	"path/filepath"
	"strings"
	"sync"
	"testing"
	"time"
)

const (
	// updateServiceAddr is the default gRPC address for the UPDATE_SERVICE.
	updateServiceAddr = "localhost:50052"

	// updateServiceGRPCService is the fully-qualified gRPC service name.
	// The proto package is "update" and the service is "UpdateService".
	updateServiceGRPCService = "update.UpdateService"
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

// buildUpdateServiceBinary builds the update-service Rust binary and returns its path.
// Skips the test if cargo is not available in PATH.
func buildUpdateServiceBinary(t *testing.T) string {
	t.Helper()
	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("cargo not in PATH; skipping binary-dependent test")
	}
	root := findRepoRoot(t)
	rhivosDir := filepath.Join(root, "rhivos")
	cmd := exec.Command("cargo", "build", "-p", "update-service")
	cmd.Dir = rhivosDir
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("cargo build -p update-service failed:\n%s\nerror: %v", out, err)
	}
	return filepath.Join(rhivosDir, "target", "debug", "update-service")
}

// protoPath returns the path to the update_service.proto file.
func protoPath(t *testing.T) string {
	t.Helper()
	root := findRepoRoot(t)
	return filepath.Join(root, "proto", "update", "update_service.proto")
}

// updateServiceProcess manages a running update-service subprocess.
type updateServiceProcess struct {
	cmd    *exec.Cmd
	output *safeBuffer
}

// startUpdateService starts the update-service binary.
// Registers a cleanup that kills the process when the test ends.
func startUpdateService(t *testing.T, binPath string) *updateServiceProcess {
	t.Helper()
	buf := &safeBuffer{}
	cmd := exec.Command(binPath, "serve")
	cmd.Stdout = buf
	cmd.Stderr = buf

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start update-service: %v", err)
	}

	proc := &updateServiceProcess{cmd: cmd, output: buf}

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
func waitForLog(proc *updateServiceProcess, substr string, timeout time.Duration) bool {
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		if strings.Contains(proc.output.String(), substr) {
			return true
		}
		time.Sleep(50 * time.Millisecond)
	}
	return strings.Contains(proc.output.String(), substr)
}

// waitForPort polls until the given TCP address is reachable or timeout expires.
// Returns true if the port became reachable.
func waitForPort(addr string, timeout time.Duration) bool {
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		conn, err := net.DialTimeout("tcp", addr, 200*time.Millisecond)
		if err == nil {
			conn.Close()
			return true
		}
		time.Sleep(100 * time.Millisecond)
	}
	return false
}

// grpcurlCall runs grpcurl against the update-service with the given method and request JSON.
// Uses the proto file for service description since the server does not expose reflection.
// Returns (stdout+stderr, error).
func grpcurlCall(t *testing.T, method, reqJSON string) (string, error) {
	t.Helper()
	root := findRepoRoot(t)
	protoDir := filepath.Join(root, "proto", "update")
	args := []string{
		"-plaintext",
		"-import-path", protoDir,
		"-proto", "update_service.proto",
		"-d", reqJSON,
		updateServiceAddr,
		updateServiceGRPCService + "/" + method,
	}
	cmd := exec.Command("grpcurl", args...)
	out, err := cmd.CombinedOutput()
	return string(out), err
}

// grpcurlCallNoBody runs grpcurl against the update-service with an empty request body.
func grpcurlCallNoBody(t *testing.T, method string) (string, error) {
	t.Helper()
	return grpcurlCall(t, method, "{}")
}

// ensureServiceReady waits for the update-service gRPC port to accept connections.
// Fails the test if the port does not become available within 10 seconds.
func ensureServiceReady(t *testing.T) {
	t.Helper()
	if !waitForPort(updateServiceAddr, 10*time.Second) {
		t.Fatal("update-service gRPC port did not become available within 10s")
	}
}

// grpcurlInstallAdapter calls InstallAdapter with the given image_ref and checksum.
func grpcurlInstallAdapter(t *testing.T, imageRef, checksum string) (string, error) {
	t.Helper()
	reqJSON := fmt.Sprintf(`{"image_ref":%q,"checksum_sha256":%q}`, imageRef, checksum)
	return grpcurlCall(t, "InstallAdapter", reqJSON)
}

// grpcurlWatch starts a background grpcurl process for the streaming
// WatchAdapterStates RPC. Returns the command and a thread-safe output buffer.
// The caller must kill the process when done.
func grpcurlWatch(t *testing.T) (*exec.Cmd, *safeBuffer) {
	t.Helper()
	root := findRepoRoot(t)
	protoDir := filepath.Join(root, "proto", "update")
	cmd := exec.Command("grpcurl",
		"-plaintext",
		"-import-path", protoDir,
		"-proto", "update_service.proto",
		"-d", "{}",
		updateServiceAddr,
		updateServiceGRPCService+"/WatchAdapterStates",
	)
	buf := &safeBuffer{}
	cmd.Stdout = buf
	cmd.Stderr = buf
	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start WatchAdapterStates grpcurl: %v", err)
	}
	return cmd, buf
}

// podmanPullAndDigest pulls an OCI image and returns its sha256 digest.
// Returns ("", error) if podman pull or inspect fails.
func podmanPullAndDigest(image string) (string, error) {
	pullCmd := exec.Command("podman", "pull", image)
	if out, err := pullCmd.CombinedOutput(); err != nil {
		return "", fmt.Errorf("podman pull failed: %v\n%s", err, out)
	}
	digestCmd := exec.Command("podman", "image", "inspect", "--format", "{{.Digest}}", image)
	out, err := digestCmd.CombinedOutput()
	if err != nil {
		return "", fmt.Errorf("podman inspect failed: %v\n%s", err, out)
	}
	return strings.TrimSpace(string(out)), nil
}
