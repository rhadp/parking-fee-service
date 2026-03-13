package setup

import (
	"context"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

// TS-01-E1: Missing Directory Detection
// Requirement: 01-REQ-1.E1
func TestEdgeMissingDir(t *testing.T) {
	root := repoRoot(t)

	// Temporarily rename proto/ to simulate a missing directory
	src := filepath.Join(root, "proto")
	dst := filepath.Join(root, "proto_backup_test")

	if err := os.Rename(src, dst); err != nil {
		t.Fatalf("failed to rename proto/: %v", err)
	}
	t.Cleanup(func() {
		_ = os.Rename(dst, src)
	})

	// Run directory structure tests - they should detect the missing directory
	cmd := exec.Command("go", "test", "./tests/setup/", "-run", "TestTopLevelDirectories", "-count=1")
	cmd.Dir = root
	output, err := cmd.CombinedOutput()
	if err == nil {
		t.Error("expected test to fail when proto/ is missing, but it passed")
	}

	if !strings.Contains(string(output), "proto") {
		t.Error("test output does not mention the missing proto directory")
	}
}

// TS-01-E2: Missing Cargo Member
// Requirement: 01-REQ-2.E1
func TestEdgeMissingCargoMember(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping edge case test in short mode")
	}

	root := repoRoot(t)

	src := filepath.Join(root, "rhivos", "locking-service", "Cargo.toml")
	dst := filepath.Join(root, "rhivos", "locking-service", "Cargo.toml.bak")

	content, err := os.ReadFile(src)
	if err != nil {
		t.Fatalf("failed to read locking-service Cargo.toml: %v", err)
	}

	if err := os.Rename(src, dst); err != nil {
		t.Fatalf("failed to rename Cargo.toml: %v", err)
	}
	t.Cleanup(func() {
		_ = os.WriteFile(src, content, 0644)
		_ = os.Remove(dst)
	})

	cmd := exec.Command("cargo", "build")
	cmd.Dir = filepath.Join(root, "rhivos")
	_, buildErr := cmd.CombinedOutput()
	if buildErr == nil {
		t.Error("expected cargo build to fail with missing workspace member, but it succeeded")
	}
}

// TS-01-E3: Missing Go Module
// Requirement: 01-REQ-3.E1
func TestEdgeMissingGoMod(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping edge case test in short mode")
	}

	root := repoRoot(t)

	// Find the backend go.mod (may be at backend/go.mod or individual modules)
	goModPath := filepath.Join(root, "backend", "go.mod")
	if !fileExists(goModPath) {
		// Try individual module
		goModPath = filepath.Join(root, "backend", "parking-fee-service", "go.mod")
	}
	if !fileExists(goModPath) {
		t.Skip("cannot find backend go.mod to test")
	}

	content, err := os.ReadFile(goModPath)
	if err != nil {
		t.Fatalf("failed to read go.mod: %v", err)
	}

	dst := goModPath + ".bak"
	if err := os.Rename(goModPath, dst); err != nil {
		t.Fatalf("failed to rename go.mod: %v", err)
	}
	t.Cleanup(func() {
		_ = os.WriteFile(goModPath, content, 0644)
		_ = os.Remove(dst)
	})

	cmd := exec.Command("go", "build", "./...")
	cmd.Dir = root
	_, buildErr := cmd.CombinedOutput()
	if buildErr == nil {
		t.Error("expected go build to fail with missing go.mod, but it succeeded")
	}
}

// TS-01-E4: Skeleton Unrecognized Flag
// Requirement: 01-REQ-4.E1
func TestEdgeUnrecognizedFlag(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping edge case test in short mode")
	}

	root := repoRoot(t)

	// Build Rust binaries first
	buildCmd := exec.Command("cargo", "build")
	buildCmd.Dir = filepath.Join(root, "rhivos")
	if output, err := buildCmd.CombinedOutput(); err != nil {
		t.Fatalf("cargo build failed: %v\n%s", err, string(output))
	}

	rustBinaries := []string{
		"locking-service",
		"cloud-gateway-client",
		"update-service",
		"parking-operator-adaptor",
		"location-sensor",
		"speed-sensor",
		"door-sensor",
	}

	for _, bin := range rustBinaries {
		t.Run("rust/"+bin, func(t *testing.T) {
			binPath := filepath.Join(root, "rhivos", "target", "debug", bin)
			ctx, cancel := context.WithTimeout(context.Background(), skeletonTimeout)
			defer cancel()
			cmd := exec.CommandContext(ctx, binPath, "--unknown-flag")
			_, err := cmd.CombinedOutput()
			if err != nil {
				t.Errorf("binary %s did not exit 0 with --unknown-flag: %v", bin, err)
			}
		})
	}

	// Test Go binaries
	goBinaries := map[string]string{
		"parking-fee-service": filepath.Join(root, "backend", "parking-fee-service"),
		"cloud-gateway":      filepath.Join(root, "backend", "cloud-gateway"),
		"parking-app-cli":    filepath.Join(root, "mock", "parking-app-cli"),
		"companion-app-cli":  filepath.Join(root, "mock", "companion-app-cli"),
		"parking-operator":   filepath.Join(root, "mock", "parking-operator"),
	}

	for name, dir := range goBinaries {
		t.Run("go/"+name, func(t *testing.T) {
			goBuild := exec.Command("go", "build", "-o", name, ".")
			goBuild.Dir = dir
			if output, err := goBuild.CombinedOutput(); err != nil {
				t.Fatalf("go build %s failed: %v\n%s", name, err, string(output))
			}
			defer func() {
				_ = os.Remove(filepath.Join(dir, name))
			}()

			ctx, cancel := context.WithTimeout(context.Background(), skeletonTimeout)
			defer cancel()
			cmd := exec.CommandContext(ctx, filepath.Join(dir, name), "--unknown-flag")
			_, err := cmd.CombinedOutput()
			if err != nil {
				t.Errorf("binary %s did not exit 0 with --unknown-flag: %v", name, err)
			}
		})
	}
}

// TS-01-E5: Missing Protoc
// Requirement: 01-REQ-5.E1
func TestEdgeMissingProtoc(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping edge case test in short mode")
	}

	root := repoRoot(t)

	// Run make proto with empty PATH (no protoc)
	cmd := exec.Command("make", "proto")
	cmd.Dir = root
	cmd.Env = append(os.Environ(), "PATH=/nonexistent")
	output, err := cmd.CombinedOutput()
	if err == nil {
		t.Error("expected make proto to fail without protoc, but it succeeded")
	}

	text := strings.ToLower(string(output))
	if !strings.Contains(text, "protoc") {
		t.Errorf("error output does not mention protoc:\n%s", string(output))
	}
}

// TS-01-E6: Missing Podman
// Requirement: 01-REQ-6.E1
func TestEdgeMissingPodman(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping edge case test in short mode")
	}

	root := repoRoot(t)

	cmd := exec.Command("make", "infra-up")
	cmd.Dir = root
	cmd.Env = append(os.Environ(), "PATH=/nonexistent")
	output, err := cmd.CombinedOutput()
	if err == nil {
		t.Error("expected make infra-up to fail without podman, but it succeeded")
	}

	text := strings.ToLower(string(output))
	if !strings.Contains(text, "podman") {
		t.Errorf("error output does not mention podman:\n%s", string(output))
	}
}

// TS-01-E7: Idempotent Infrastructure Start
// Requirement: 01-REQ-6.E2
func TestEdgeIdempotentInfraUp(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping infrastructure idempotency test in short mode")
	}

	if _, err := exec.LookPath("podman"); err != nil {
		t.Skip("skipping: podman not available")
	}

	root := repoRoot(t)

	t.Cleanup(func() {
		cmd := exec.Command("make", "infra-down")
		cmd.Dir = root
		_ = cmd.Run()
	})

	// Run infra-up twice
	for i := 0; i < 2; i++ {
		cmd := exec.Command("make", "infra-up")
		cmd.Dir = root
		if output, err := cmd.CombinedOutput(); err != nil {
			t.Fatalf("make infra-up (attempt %d) failed: %v\n%s", i+1, err, string(output))
		}
	}

	// Count containers
	psCmd := exec.Command("podman", "ps", "--format", "{{.Names}}")
	psOutput, err := psCmd.CombinedOutput()
	if err != nil {
		t.Fatalf("podman ps failed: %v", err)
	}

	lines := strings.Split(strings.TrimSpace(string(psOutput)), "\n")
	natsCount := 0
	dbCount := 0
	for _, line := range lines {
		if strings.Contains(line, "nats") {
			natsCount++
		}
		if strings.Contains(line, "databroker") || strings.Contains(line, "kuksa") {
			dbCount++
		}
	}

	if natsCount != 1 {
		t.Errorf("expected 1 nats container, got %d", natsCount)
	}
	if dbCount != 1 {
		t.Errorf("expected 1 databroker container, got %d", dbCount)
	}
}

// TS-01-E8: Missing Toolchain
// Requirement: 01-REQ-7.E1
func TestEdgeMissingToolchain(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping edge case test in short mode")
	}

	root := repoRoot(t)

	// Run make build with no cargo in PATH
	cmd := exec.Command("make", "build")
	cmd.Dir = root
	cmd.Env = append(os.Environ(), "PATH=/nonexistent")
	_, err := cmd.CombinedOutput()
	if err == nil {
		t.Error("expected make build to fail without toolchains, but it succeeded")
	}
}

// TS-01-E9: No Tests Warning
// Requirement: 01-REQ-8.E1
func TestEdgeNoTestsWarning(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping edge case test in short mode")
	}

	root := repoRoot(t)

	// Find a Go test file to temporarily remove
	testFile := filepath.Join(root, "backend", "parking-fee-service", "main_test.go")
	if !fileExists(testFile) {
		t.Skip("backend/parking-fee-service/main_test.go does not exist")
	}

	content, err := os.ReadFile(testFile)
	if err != nil {
		t.Fatalf("failed to read test file: %v", err)
	}

	if err := os.Remove(testFile); err != nil {
		t.Fatalf("failed to remove test file: %v", err)
	}
	t.Cleanup(func() {
		_ = os.WriteFile(testFile, content, 0644)
	})

	// Run go test on the module
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()
	cmd := exec.CommandContext(ctx, "go", "test", "./backend/parking-fee-service/")
	cmd.Dir = root
	output, _ := cmd.CombinedOutput()

	if !strings.Contains(string(output), "no test files") {
		t.Errorf("expected 'no test files' warning, got:\n%s", string(output))
	}
}
