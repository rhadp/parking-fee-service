package setup

import (
	"context"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

const skeletonTimeout = 5 * time.Second

// runBinaryWithTimeout runs a binary with a timeout and returns its combined output and exit error.
func runBinaryWithTimeout(t *testing.T, binary string, args ...string) (string, error) {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), skeletonTimeout)
	defer cancel()

	cmd := exec.CommandContext(ctx, binary, args...)
	output, err := cmd.CombinedOutput()
	return string(output), err
}

// TS-01-14: Rust Skeleton Exit Behavior
// Requirement: 01-REQ-4.1
func TestRustSkeletonExit(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping skeleton tests in short mode")
	}

	root := repoRoot(t)

	// Build first
	cmd := exec.Command("cargo", "build")
	cmd.Dir = filepath.Join(root, "rhivos")
	if output, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("cargo build failed: %v\n%s", err, string(output))
	}

	binaries := []string{
		"locking-service",
		"cloud-gateway-client",
		"update-service",
		"parking-operator-adaptor",
		"location-sensor",
		"speed-sensor",
		"door-sensor",
	}

	for _, bin := range binaries {
		t.Run(bin, func(t *testing.T) {
			binPath := filepath.Join(root, "rhivos", "target", "debug", bin)
			output, err := runBinaryWithTimeout(t, binPath)
			if err != nil {
				t.Errorf("binary %s failed to exit cleanly: %v\noutput: %s", bin, err, output)
			}
			if !strings.Contains(strings.ToLower(output), strings.ToLower(bin)) {
				t.Errorf("binary %s output does not contain its name:\n%s", bin, output)
			}
		})
	}
}

// TS-01-15: Go Skeleton Exit Behavior
// Requirement: 01-REQ-4.2
func TestGoSkeletonExit(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping skeleton tests in short mode")
	}

	root := repoRoot(t)

	binaries := map[string]string{
		"parking-fee-service": filepath.Join(root, "backend", "parking-fee-service"),
		"cloud-gateway":      filepath.Join(root, "backend", "cloud-gateway"),
		"parking-app-cli":    filepath.Join(root, "mock", "parking-app-cli"),
		"companion-app-cli":  filepath.Join(root, "mock", "companion-app-cli"),
		"parking-operator":   filepath.Join(root, "mock", "parking-operator"),
	}

	for name, dir := range binaries {
		t.Run(name, func(t *testing.T) {
			// Build the binary
			buildCmd := exec.Command("go", "build", "-o", name, ".")
			buildCmd.Dir = dir
			if output, err := buildCmd.CombinedOutput(); err != nil {
				t.Fatalf("go build %s failed: %v\n%s", name, err, string(output))
			}
			defer func() {
				// Clean up built binary
				_ = exec.Command("rm", "-f", filepath.Join(dir, name)).Run()
			}()

			binPath := filepath.Join(dir, name)
			output, err := runBinaryWithTimeout(t, binPath)
			if err != nil {
				t.Errorf("binary %s failed to exit cleanly: %v\noutput: %s", name, err, output)
			}
			if !strings.Contains(strings.ToLower(output), strings.ToLower(name)) {
				t.Errorf("binary %s output does not contain its name:\n%s", name, output)
			}
		})
	}
}

// TS-01-16: Rust Binary List
// Requirement: 01-REQ-4.3
func TestRustBinaryList(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping binary list test in short mode")
	}

	root := repoRoot(t)

	// Build first
	cmd := exec.Command("cargo", "build")
	cmd.Dir = filepath.Join(root, "rhivos")
	if output, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("cargo build failed: %v\n%s", err, string(output))
	}

	binaries := []string{
		"locking-service",
		"cloud-gateway-client",
		"update-service",
		"parking-operator-adaptor",
		"location-sensor",
		"speed-sensor",
		"door-sensor",
	}

	for _, bin := range binaries {
		path := filepath.Join(root, "rhivos", "target", "debug", bin)
		if !fileExists(path) {
			t.Errorf("expected Rust binary %s to exist at %s", bin, path)
		}
	}
}

// TS-01-17: Go Binary List
// Requirement: 01-REQ-4.4
func TestGoBinaryList(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping binary list test in short mode")
	}

	root := repoRoot(t)

	packages := map[string]string{
		"parking-fee-service": filepath.Join(root, "backend", "parking-fee-service"),
		"cloud-gateway":      filepath.Join(root, "backend", "cloud-gateway"),
		"parking-app-cli":    filepath.Join(root, "mock", "parking-app-cli"),
		"companion-app-cli":  filepath.Join(root, "mock", "companion-app-cli"),
		"parking-operator":   filepath.Join(root, "mock", "parking-operator"),
	}

	for name, dir := range packages {
		t.Run(name, func(t *testing.T) {
			cmd := exec.Command("go", "build", "-o", "/dev/null", ".")
			cmd.Dir = dir
			output, err := cmd.CombinedOutput()
			if err != nil {
				t.Errorf("go build %s failed: %v\n%s", name, err, string(output))
			}
		})
	}
}
