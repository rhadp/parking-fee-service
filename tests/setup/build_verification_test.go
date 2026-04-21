package setup

import (
	"os"
	"os/exec"
	"path/filepath"
	"testing"
)

// TestRustBuild verifies that `cargo build --workspace` succeeds in rhivos/.
// It skips gracefully when cargo is not on PATH.
// Test Spec: TS-01-30, TS-01-31
// Requirements: 01-REQ-9.1, 01-REQ-9.2, 01-REQ-9.4
func TestRustBuild(t *testing.T) {
	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("cargo not found on PATH — skipping Rust build verification")
	}

	root := findRepoRoot(t)
	rhivosDir := filepath.Join(root, "rhivos")

	cmd := exec.Command("cargo", "build", "--workspace")
	cmd.Dir = rhivosDir
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("cargo build --workspace failed in rhivos/:\n%s\nerror: %v", out, err)
	}
}

// TestGoBuild verifies that `go build ./...` succeeds for all Go modules.
// It skips gracefully when go is not on PATH.
// Test Spec: TS-01-30, TS-01-31
// Requirements: 01-REQ-9.1, 01-REQ-9.2, 01-REQ-9.4
func TestGoBuild(t *testing.T) {
	if _, err := exec.LookPath("go"); err != nil {
		t.Skip("go not found on PATH — skipping Go build verification")
	}

	root := findRepoRoot(t)

	// Build each Go module explicitly to match the monorepo structure.
	modules := []string{
		"backend/parking-fee-service",
		"backend/cloud-gateway",
		"mock/parking-app-cli",
		"mock/companion-app-cli",
		"mock/parking-operator",
	}

	for _, mod := range modules {
		t.Run(mod, func(t *testing.T) {
			cmd := exec.Command("go", "build", "./...")
			cmd.Dir = filepath.Join(root, mod)
			out, err := cmd.CombinedOutput()
			if err != nil {
				t.Fatalf("go build ./... failed in %s:\n%s\nerror: %v", mod, out, err)
			}
		})
	}
}

// TestProtoFilesValidate verifies that protoc can parse all .proto files.
// It skips gracefully when protoc is not on PATH.
// Test Spec: TS-01-30, TS-01-31
// Requirements: 01-REQ-9.1, 01-REQ-9.2, 01-REQ-9.4
func TestProtoFilesValidate(t *testing.T) {
	if _, err := exec.LookPath("protoc"); err != nil {
		t.Skip("protoc not found on PATH — skipping proto validation")
	}

	root := findRepoRoot(t)
	protoDir := filepath.Join(root, "proto")

	// Collect all .proto files.
	var protoFiles []string
	err := filepath.WalkDir(protoDir, func(path string, d os.DirEntry, err error) error {
		if err != nil {
			return err
		}
		if !d.IsDir() && filepath.Ext(path) == ".proto" {
			protoFiles = append(protoFiles, path)
		}
		return nil
	})
	if err != nil {
		t.Fatalf("failed to walk proto directory: %v", err)
	}

	if len(protoFiles) == 0 {
		t.Fatal("no .proto files found in proto/ directory")
	}

	// Run protoc on all proto files together (validates cross-file imports).
	args := []string{"--proto_path=" + protoDir, "--descriptor_set_out=/dev/null"}
	args = append(args, protoFiles...)

	cmd := exec.Command("protoc", args...)
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("protoc failed to parse proto files:\n%s\nerror: %v", out, err)
	}
}
