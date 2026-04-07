// Package setup contains build-command-based verification tests that invoke
// build tools as subprocesses and assert success. These tests validate that
// Rust binaries compile, Go binaries compile, and proto files parse correctly.
//
// Test Spec: TS-01-30, TS-01-31
// Requirements: 01-REQ-9.1, 01-REQ-9.2, 01-REQ-9.4, 01-REQ-9.E1
package setup

import (
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
)

// toolAvailable checks whether the named tool is available on PATH.
func toolAvailable(name string) bool {
	_, err := exec.LookPath(name)
	return err == nil
}

// --- TS-01-30, TS-01-31: Setup verification tests ---
// Requirement: 01-REQ-9.1, 01-REQ-9.2, 01-REQ-9.4

// TestRustBuild verifies that all Rust workspace members compile successfully
// by invoking `cargo build --workspace` as a subprocess.
// Requirement: 01-REQ-9.E1 — skips if cargo is not installed.
func TestRustBuild(t *testing.T) {
	if !toolAvailable("cargo") {
		t.Skip("skipping: cargo is not installed or not on PATH")
	}

	root := repoRoot(t)
	rhivosDir := filepath.Join(root, "rhivos")

	cmd := exec.Command("cargo", "build", "--workspace")
	cmd.Dir = rhivosDir
	output, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("cargo build --workspace failed (exit code non-zero):\n%s", string(output))
	}
	t.Log("Rust workspace build succeeded")
}

// TestGoBuild verifies that all Go modules compile successfully by invoking
// `go build` for each module as a subprocess.
// Requirement: 01-REQ-9.E1 — skips if go is not installed.
func TestGoBuild(t *testing.T) {
	if !toolAvailable("go") {
		t.Skip("skipping: go is not installed or not on PATH")
	}

	root := repoRoot(t)

	// Build each Go module individually to get clear per-module results
	modules := []string{
		"backend/parking-fee-service",
		"backend/cloud-gateway",
		"mock/parking-app-cli",
		"mock/companion-app-cli",
		"mock/parking-operator",
	}

	for _, mod := range modules {
		t.Run(mod, func(t *testing.T) {
			modDir := filepath.Join(root, mod)
			cmd := exec.Command("go", "build", "./...")
			cmd.Dir = modDir
			output, err := cmd.CombinedOutput()
			if err != nil {
				t.Fatalf("go build ./... failed for %s:\n%s", mod, string(output))
			}
			t.Logf("Go module %s build succeeded", mod)
		})
	}
}

// TestProtoValidation verifies that all .proto files in the proto/ directory
// parse without errors by invoking `protoc` as a subprocess.
// Requirement: 01-REQ-9.E1 — skips if protoc is not installed.
func TestProtoValidation(t *testing.T) {
	if !toolAvailable("protoc") {
		t.Skip("skipping: protoc is not installed or not on PATH")
	}

	root := repoRoot(t)
	protoDir := filepath.Join(root, "proto")

	// Collect all .proto files
	protoFiles, err := collectProtoFiles(protoDir)
	if err != nil {
		t.Fatalf("failed to collect proto files: %v", err)
	}
	if len(protoFiles) == 0 {
		t.Fatal("no .proto files found in proto/ directory")
	}

	// Validate each proto file individually for clear per-file results
	for _, pf := range protoFiles {
		// Use relative path from proto dir for the test name
		relPath, _ := filepath.Rel(protoDir, pf)
		t.Run(relPath, func(t *testing.T) {
			cmd := exec.Command("protoc",
				"--proto_path="+protoDir,
				"--descriptor_set_out=/dev/null",
				pf,
			)
			output, err := cmd.CombinedOutput()
			if err != nil {
				t.Fatalf("protoc failed for %s:\n%s", relPath, string(output))
			}
			t.Logf("proto file %s parsed successfully", relPath)
		})
	}

	// Also validate all proto files together (cross-import resolution)
	// Test Spec: TS-01-17
	t.Run("all_protos_together", func(t *testing.T) {
		args := []string{
			"--proto_path=" + protoDir,
			"--descriptor_set_out=/dev/null",
		}
		args = append(args, protoFiles...)
		cmd := exec.Command("protoc", args...)
		output, err := cmd.CombinedOutput()
		if err != nil {
			t.Fatalf("protoc failed for all proto files together:\n%s", string(output))
		}
		t.Log("all proto files parsed together successfully")
	})
}

// collectProtoFiles walks the given directory and returns paths to all .proto files.
func collectProtoFiles(dir string) ([]string, error) {
	var files []string
	entries, err := filepath.Glob(filepath.Join(dir, "*", "*.proto"))
	if err != nil {
		return nil, err
	}
	files = append(files, entries...)

	// Also check for .proto files directly in the dir
	topLevel, err := filepath.Glob(filepath.Join(dir, "*.proto"))
	if err != nil {
		return nil, err
	}
	files = append(files, topLevel...)
	return files, nil
}

// --- TS-01-9: Cargo build succeeds for entire workspace ---
// Requirement: 01-REQ-2.4
func TestCargoBuildWorkspace(t *testing.T) {
	if !toolAvailable("cargo") {
		t.Skip("skipping: cargo is not installed or not on PATH")
	}

	root := repoRoot(t)
	cmd := exec.Command("cargo", "build", "--workspace")
	cmd.Dir = filepath.Join(root, "rhivos")
	output, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("cargo build --workspace failed:\n%s", string(output))
	}
}

// --- TS-01-12: Go build succeeds for all modules ---
// Requirement: 01-REQ-3.4
func TestGoBuildAll(t *testing.T) {
	if !toolAvailable("go") {
		t.Skip("skipping: go is not installed or not on PATH")
	}

	root := repoRoot(t)

	modules := []string{
		"backend/parking-fee-service",
		"backend/cloud-gateway",
		"mock/parking-app-cli",
		"mock/companion-app-cli",
		"mock/parking-operator",
	}

	for _, mod := range modules {
		modDir := filepath.Join(root, mod)
		cmd := exec.Command("go", "build", "./...")
		cmd.Dir = modDir
		output, err := cmd.CombinedOutput()
		if err != nil {
			t.Fatalf("go build ./... failed for %s:\n%s", mod, string(output))
		}
	}
}

// --- TS-01-13: Rust skeleton prints version and exits 0 ---
// Requirements: 01-REQ-4.1, 01-REQ-4.4
func TestRustSkeletonVersion(t *testing.T) {
	if !toolAvailable("cargo") {
		t.Skip("skipping: cargo is not installed or not on PATH")
	}

	root := repoRoot(t)

	// First build
	cmd := exec.Command("cargo", "build", "--workspace")
	cmd.Dir = filepath.Join(root, "rhivos")
	if output, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("cargo build failed:\n%s", string(output))
	}

	binaries := map[string]string{
		"locking-service":        "locking-service",
		"cloud-gateway-client":   "cloud-gateway-client",
		"update-service":         "update-service",
		"parking-operator-adaptor": "parking-operator-adaptor",
		"location-sensor":        "location-sensor",
		"speed-sensor":           "speed-sensor",
		"door-sensor":            "door-sensor",
	}

	for bin, name := range binaries {
		t.Run(bin, func(t *testing.T) {
			binPath := filepath.Join(root, "rhivos", "target", "debug", bin)
			cmd := exec.Command(binPath)
			output, err := cmd.CombinedOutput()
			if err != nil {
				t.Fatalf("%s exited with error: %v\noutput: %s", bin, err, string(output))
			}
			if !strings.Contains(string(output), name) {
				t.Errorf("%s output should contain %q, got: %s", bin, name, string(output))
			}
		})
	}
}

// --- TS-01-14: Go skeleton prints version and exits 0 ---
// Requirements: 01-REQ-4.2, 01-REQ-4.4
func TestGoSkeletonVersion(t *testing.T) {
	if !toolAvailable("go") {
		t.Skip("skipping: go is not installed or not on PATH")
	}

	root := repoRoot(t)

	modules := map[string]string{
		"backend/parking-fee-service": "parking-fee-service",
		"backend/cloud-gateway":      "cloud-gateway",
		"mock/parking-app-cli":       "parking-app-cli",
		"mock/companion-app-cli":     "companion-app-cli",
		"mock/parking-operator":      "parking-operator",
	}

	for mod, name := range modules {
		t.Run(name, func(t *testing.T) {
			cmd := exec.Command("go", "run", ".")
			cmd.Dir = filepath.Join(root, mod)
			output, err := cmd.CombinedOutput()
			if err != nil {
				t.Fatalf("go run %s failed: %v\noutput: %s", mod, err, string(output))
			}
			if !strings.Contains(string(output), name) {
				t.Errorf("output should contain %q, got: %s", name, string(output))
			}
		})
	}
}

// --- TS-01-P1: Build completeness across all components ---
// Property 1: Build Completeness
func TestBuildCompleteness(t *testing.T) {
	if !toolAvailable("cargo") {
		t.Skip("skipping: cargo is not installed or not on PATH")
	}

	root := repoRoot(t)

	// Build the Rust workspace
	cmd := exec.Command("cargo", "build", "--workspace")
	cmd.Dir = filepath.Join(root, "rhivos")
	if output, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("cargo build failed:\n%s", string(output))
	}

	// Verify all binaries were produced
	expectedBins := []string{
		"locking-service",
		"cloud-gateway-client",
		"update-service",
		"parking-operator-adaptor",
		"location-sensor",
		"speed-sensor",
		"door-sensor",
	}
	for _, bin := range expectedBins {
		binPath := filepath.Join(root, "rhivos", "target", "debug", bin)
		if !pathExists(binPath) {
			t.Errorf("expected binary %s to exist after build", binPath)
		}
	}
}

// --- TS-01-P2: Skeleton determinism across invocations ---
// Property 2: Skeleton Determinism
func TestSkeletonDeterminism(t *testing.T) {
	if !toolAvailable("cargo") {
		t.Skip("skipping: cargo is not installed or not on PATH")
	}

	root := repoRoot(t)

	// Build first
	cmd := exec.Command("cargo", "build", "--workspace")
	cmd.Dir = filepath.Join(root, "rhivos")
	if output, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("cargo build failed:\n%s", string(output))
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

			cmd1 := exec.Command(binPath)
			out1, err1 := cmd1.Output()
			if err1 != nil {
				t.Fatalf("first run of %s failed: %v", bin, err1)
			}

			cmd2 := exec.Command(binPath)
			out2, err2 := cmd2.Output()
			if err2 != nil {
				t.Fatalf("second run of %s failed: %v", bin, err2)
			}

			if string(out1) != string(out2) {
				t.Errorf("output differs between runs:\nrun1: %s\nrun2: %s", string(out1), string(out2))
			}
		})
	}
}
