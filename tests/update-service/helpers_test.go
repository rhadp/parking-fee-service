package update_service_test

import (
	"context"
	"fmt"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"testing"
	"time"

	update "github.com/rhadp/parking-fee-service/gen/update"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

const (
	// defaultGRPCPort is the default port the update-service listens on.
	defaultGRPCPort = 50052

	// testGRPCPort is the port used for integration tests to avoid
	// conflicting with a running instance.
	testGRPCPort = 50152

	// testGRPCTarget is the gRPC dial target for integration tests.
	testGRPCTarget = "localhost:50152"
)

// repoRoot returns the absolute path to the repository root.
// tests/update-service/ is two levels deep from the repo root.
func repoRoot(t *testing.T) string {
	t.Helper()
	root, err := filepath.Abs("../..")
	if err != nil {
		t.Fatalf("failed to resolve repo root: %v", err)
	}
	return root
}

// updateServiceBinaryPath returns the path to the update-service binary.
// It builds the binary if needed.
func updateServiceBinaryPath(t *testing.T) string {
	t.Helper()
	root := repoRoot(t)
	binaryPath := filepath.Join(root, "rhivos", "target", "debug", "update-service")

	// Build the binary.
	cmd := exec.Command("cargo", "build", "-p", "update-service")
	cmd.Dir = filepath.Join(root, "rhivos")
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("failed to build update-service: %v\n%s", err, string(out))
	}

	return binaryPath
}

// skipIfCargoUnavailable skips the test if cargo is not available.
func skipIfCargoUnavailable(t *testing.T) {
	t.Helper()
	_, err := exec.LookPath("cargo")
	if err != nil {
		t.Skip("cargo not found, skipping integration test")
	}
}

// skipIfPodmanUnavailable skips the test if podman is not available.
func skipIfPodmanUnavailable(t *testing.T) {
	t.Helper()
	_, err := exec.LookPath("podman")
	if err != nil {
		t.Skip("podman not found, skipping integration test")
	}
}

// startUpdateService starts the update-service binary as a subprocess
// and returns the process. It registers cleanup to kill the process
// when the test completes.
func startUpdateService(t *testing.T) *exec.Cmd {
	t.Helper()
	skipIfCargoUnavailable(t)

	binaryPath := updateServiceBinaryPath(t)

	// Write a temporary config with the test port.
	configDir := t.TempDir()
	configPath := filepath.Join(configDir, "config.json")
	configContent := fmt.Sprintf(
		`{"grpc_port":%d,"registry_url":"","inactivity_timeout_secs":86400,"container_storage_path":"/tmp/test-adapters/"}`,
		testGRPCPort,
	)
	if err := os.WriteFile(configPath, []byte(configContent), 0o644); err != nil {
		t.Fatalf("failed to write test config: %v", err)
	}

	cmd := exec.Command(binaryPath)
	cmd.Env = append(os.Environ(), "CONFIG_PATH="+configPath)
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start update-service: %v", err)
	}

	t.Cleanup(func() {
		_ = cmd.Process.Signal(os.Interrupt)
		done := make(chan error, 1)
		go func() { done <- cmd.Wait() }()
		select {
		case <-done:
		case <-time.After(15 * time.Second):
			_ = cmd.Process.Kill()
		}
	})

	// Wait for the gRPC port to become reachable.
	deadline := time.Now().Add(10 * time.Second)
	for time.Now().Before(deadline) {
		conn, err := net.DialTimeout("tcp", testGRPCTarget, 500*time.Millisecond)
		if err == nil {
			conn.Close()
			return cmd
		}
		time.Sleep(200 * time.Millisecond)
	}
	t.Fatalf("update-service gRPC port %s not reachable after 10s", testGRPCTarget)
	return nil
}

// dialUpdateService establishes a gRPC connection to the update-service.
func dialUpdateService(t *testing.T) (*grpc.ClientConn, update.UpdateServiceClient) {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	conn, err := grpc.DialContext(ctx, testGRPCTarget, //nolint:staticcheck
		grpc.WithTransportCredentials(insecure.NewCredentials()),
		grpc.WithBlock(), //nolint:staticcheck
	)
	if err != nil {
		t.Fatalf("failed to dial update-service at %s: %v", testGRPCTarget, err)
	}
	t.Cleanup(func() { conn.Close() })
	return conn, update.NewUpdateServiceClient(conn)
}
