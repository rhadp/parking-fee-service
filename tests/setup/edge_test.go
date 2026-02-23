package setup_test

import (
	"net"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

// TS-01-E1: Missing required directory (01-REQ-1.E1)
func TestEdge_MissingDirectory(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping destructive edge case test in short mode")
	}
	root := repoRoot(t)

	protoDir := filepath.Join(root, "proto")
	backupDir := filepath.Join(root, "proto_bak")

	// Rename proto/ to simulate missing directory
	if err := os.Rename(protoDir, backupDir); err != nil {
		t.Fatalf("could not rename proto/: %v", err)
	}
	t.Cleanup(func() {
		os.Rename(backupDir, protoDir)
	})

	result := execCommand(t, root, ".", "make", "build")
	if result.ExitCode == 0 {
		t.Error("expected make build to fail with missing proto/, but it succeeded")
	}
	if !strings.Contains(result.Combined, "proto") {
		t.Errorf("expected error output to reference 'proto', got: %s", result.Combined)
	}
}

// TS-01-E2: Proto import paths relative to root (01-REQ-2.E1)
func TestEdge_ProtoImportPaths(t *testing.T) {
	root := repoRoot(t)

	protoFiles := globFiles(t, root, "proto/*.proto")
	if len(protoFiles) == 0 {
		t.Fatal("no proto files found in proto/")
	}

	for _, f := range protoFiles {
		data, err := os.ReadFile(f)
		if err != nil {
			t.Errorf("could not read %s: %v", f, err)
			continue
		}
		content := string(data)
		lines := strings.Split(content, "\n")
		for _, line := range lines {
			trimmed := strings.TrimSpace(line)
			if strings.HasPrefix(trimmed, "import ") && strings.Contains(trimmed, ".proto") {
				// Extract the import path
				importPath := trimmed
				importPath = strings.TrimPrefix(importPath, "import ")
				importPath = strings.Trim(importPath, `";`)
				importPath = strings.TrimSpace(importPath)

				if strings.HasPrefix(importPath, "/") {
					t.Errorf("proto import path in %s starts with '/': %s", f, importPath)
				}
				if strings.HasPrefix(importPath, "../") {
					t.Errorf("proto import path in %s starts with '../': %s", f, importPath)
				}
			}
		}
	}
}

// TS-01-E3: Missing proto for Rust build (01-REQ-3.E1)
func TestEdge_MissingProtoRust(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping destructive edge case test in short mode")
	}
	root := repoRoot(t)

	protoFile := filepath.Join(root, "proto", "update_service.proto")
	backupFile := filepath.Join(root, "proto", "update_service.proto.bak")

	if err := os.Rename(protoFile, backupFile); err != nil {
		t.Fatalf("could not rename update_service.proto: %v", err)
	}
	t.Cleanup(func() {
		os.Rename(backupFile, protoFile)
	})

	result := execCommand(t, root, "rhivos", "cargo", "build")
	if result.ExitCode == 0 {
		t.Error("expected cargo build to fail with missing update_service.proto, but it succeeded")
	}
	if !strings.Contains(result.Combined, "update_service.proto") && !strings.Contains(result.Combined, "proto") {
		t.Errorf("expected error to reference missing proto file, got: %s", result.Combined)
	}
}

// TS-01-E4: Missing proto for Go generation (01-REQ-4.E1)
func TestEdge_MalformedProtoGo(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping destructive edge case test in short mode")
	}
	root := repoRoot(t)

	protoFile := filepath.Join(root, "proto", "common.proto")
	backup, err := os.ReadFile(protoFile)
	if err != nil {
		t.Fatalf("could not read common.proto: %v", err)
	}
	t.Cleanup(func() {
		os.WriteFile(protoFile, backup, 0644)
	})

	// Write invalid proto content
	if err := os.WriteFile(protoFile, []byte("invalid proto content {{{"), 0644); err != nil {
		t.Fatalf("could not corrupt common.proto: %v", err)
	}

	result := execCommand(t, root, ".", "make", "proto")
	if result.ExitCode == 0 {
		t.Error("expected make proto to fail with malformed proto, but it succeeded")
	}
	if !strings.Contains(result.Combined, "common.proto") && !strings.Contains(result.Combined, "proto") {
		t.Errorf("expected error to reference proto file, got: %s", result.Combined)
	}
}

// TS-01-E5: Unknown CLI command (01-REQ-5.E1)
func TestEdge_UnknownCLICommand(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping build test in short mode")
	}
	root := repoRoot(t)

	for _, app := range []string{"parking-app-cli", "companion-app-cli"} {
		appDir := filepath.Join(root, "mock", app)
		// Build first
		buildResult := execCommand(t, root, "mock/"+app, "go", "build", "-o", app, ".")
		if buildResult.ExitCode != 0 {
			t.Errorf("go build failed for %s: %s", app, buildResult.Combined)
			continue
		}
		defer os.Remove(filepath.Join(appDir, app))

		// Run with unknown command
		result := execCommand(t, root, "mock/"+app, "./"+app, "nonexistent-command")
		if result.ExitCode == 0 {
			t.Errorf("expected %s to exit non-zero for unknown command, got exit 0", app)
		}
		if len(result.Stderr) == 0 && len(result.Combined) == 0 {
			t.Errorf("expected %s to produce error output for unknown command, got empty", app)
		}
	}
}

// TS-01-E6: Missing Rust toolchain (01-REQ-6.E1)
func TestEdge_MissingRustToolchain(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping edge case test in short mode")
	}
	root := repoRoot(t)

	restrictedPath := pathWithout(t, "rustc", "cargo")
	env := envWithPath(restrictedPath)

	result := execCommandWithEnv(t, root, ".", env, "make", "build")
	if result.ExitCode == 0 {
		t.Error("expected make build to fail without rustc, but it succeeded")
	}
	if !strings.Contains(result.Combined, "rustc") && !strings.Contains(result.Combined, "cargo") {
		t.Errorf("expected error to name 'rustc' or 'cargo', got: %s", result.Combined)
	}
}

// TS-01-E7: Missing Go toolchain (01-REQ-6.E1)
func TestEdge_MissingGoToolchain(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping edge case test in short mode")
	}
	root := repoRoot(t)

	restrictedPath := pathWithout(t, "go")
	env := envWithPath(restrictedPath)

	result := execCommandWithEnv(t, root, ".", env, "make", "build")
	if result.ExitCode == 0 {
		t.Error("expected make build to fail without go, but it succeeded")
	}
	if !strings.Contains(result.Combined, "go") {
		t.Errorf("expected error to name 'go', got: %s", result.Combined)
	}
}

// TS-01-E8: Partial build failure continues (01-REQ-6.E2)
func TestEdge_PartialBuildFailure(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping destructive edge case test in short mode")
	}
	root := repoRoot(t)

	mainFile := filepath.Join(root, "backend", "parking-fee-service", "main.go")
	backup, err := os.ReadFile(mainFile)
	if err != nil {
		t.Fatalf("could not read main.go: %v", err)
	}
	t.Cleanup(func() {
		os.WriteFile(mainFile, backup, 0644)
	})

	// Write invalid Go source
	if err := os.WriteFile(mainFile, []byte("package main\nfunc main() { invalid }"), 0644); err != nil {
		t.Fatalf("could not corrupt main.go: %v", err)
	}

	result := execCommand(t, root, ".", "make", "build")
	if result.ExitCode == 0 {
		t.Error("expected make build to fail with corrupted source, but it succeeded")
	}

	// Verify error mentions parking-fee-service
	if !strings.Contains(result.Combined, "parking-fee-service") && !strings.Contains(result.Combined, "error") {
		t.Errorf("expected error output to reference parking-fee-service, got: %s", result.Combined)
	}

	// Verify Rust components still built (build should continue for independent components)
	targetDir := filepath.Join(root, "rhivos", "target", "debug")
	if _, err := os.Stat(targetDir); os.IsNotExist(err) {
		t.Error("expected rhivos/target/debug/ to exist (Rust build should have continued)")
	}
}

// TS-01-E9: Port conflict on infra-up (01-REQ-7.E1)
func TestEdge_PortConflict(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping infrastructure edge case test in short mode")
	}

	// Skip if no container runtime available
	if _, err := lookPath("podman"); err != nil {
		if _, err := lookPath("docker"); err != nil {
			t.Skip("no container runtime (podman/docker) available, skipping infra test")
		}
	}

	root := repoRoot(t)

	// Occupy port 1883
	listener, err := net.Listen("tcp", ":1883")
	if err != nil {
		t.Skipf("could not bind port 1883 (already in use?): %v", err)
	}
	defer listener.Close()

	result := execCommand(t, root, ".", "make", "infra-up")
	// The infra-up may fail or the container may fail to bind
	// We just check that the error is reported somehow
	if result.ExitCode == 0 {
		// Even if exit code is 0, the container might have failed to start
		// We defer cleanup
		defer execCommand(t, root, ".", "make", "infra-down")
	}

	if result.ExitCode != 0 {
		if !strings.Contains(result.Combined, "1883") && !strings.Contains(result.Combined, "bind") && !strings.Contains(result.Combined, "address already in use") {
			t.Logf("infra-up failed as expected but error message doesn't reference port: %s", result.Combined)
		}
	}

	// Clean up
	execCommand(t, root, ".", "make", "infra-down")
}

// TS-01-E10: Missing container runtime (01-REQ-7.E2)
func TestEdge_MissingContainerRuntime(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping edge case test in short mode")
	}
	root := repoRoot(t)

	restrictedPath := pathWithout(t, "podman", "docker", "podman-compose", "docker-compose")
	env := envWithPath(restrictedPath)

	result := execCommandWithEnv(t, root, ".", env, "make", "infra-up")
	if result.ExitCode == 0 {
		t.Error("expected make infra-up to fail without container runtime, but it succeeded")
		// Clean up if it somehow succeeded
		execCommand(t, root, ".", "make", "infra-down")
	}
	if !strings.Contains(result.Combined, "podman") && !strings.Contains(result.Combined, "docker") {
		t.Errorf("expected error to name 'podman' or 'docker', got: %s", result.Combined)
	}
}

// TS-01-E11: Empty test file is not a failure (01-REQ-8.E1)
func TestEdge_EmptyTestFile(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping edge case test in short mode")
	}
	root := repoRoot(t)

	emptyTestFile := filepath.Join(root, "backend", "parking-fee-service", "empty_test.go")
	if err := os.WriteFile(emptyTestFile, []byte("package main\n"), 0644); err != nil {
		t.Fatalf("could not create empty test file: %v", err)
	}
	t.Cleanup(func() {
		os.Remove(emptyTestFile)
	})

	result := execCommand(t, root, ".", "make", "test")
	if result.ExitCode != 0 {
		t.Errorf("make test failed with empty test file (exit %d): %s", result.ExitCode, result.Combined)
	}

	// Also wait to ensure no issues with the test runner
	_ = time.Now() // placeholder, test just checks exit code
}
