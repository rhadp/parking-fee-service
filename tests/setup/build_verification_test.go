package setup_test

import (
	"os"
	"os/exec"
	"path/filepath"
	"testing"
)

// TestRustBuild verifies that the Rust workspace compiles successfully by
// invoking `cargo build --workspace` in rhivos/ as a subprocess.
// If cargo is not on PATH the test is skipped with an explanatory message.
// Test Spec: TS-01-9, TS-01-30, TS-01-31
// Requirements: 01-REQ-2.4, 01-REQ-9.1, 01-REQ-9.2, 01-REQ-9.4, 01-REQ-9.E1
func TestRustBuild(t *testing.T) {
	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("cargo not found on PATH; skipping Rust build verification (install rustup to enable)")
	}

	root := repoRoot(t)
	rhivosDir := filepath.Join(root, "rhivos")

	cmd := exec.Command("cargo", "build", "--workspace")
	cmd.Dir = rhivosDir
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Errorf("cargo build --workspace failed in %s: %v\n%s", rhivosDir, err, string(out))
	}
}

// TestGoBuild verifies that all Go modules compile successfully by invoking
// `go build parking-fee-service/...` from the repository root as a subprocess.
// If go is not on PATH the test is skipped with an explanatory message.
// Test Spec: TS-01-12, TS-01-30, TS-01-31
// Requirements: 01-REQ-3.4, 01-REQ-9.1, 01-REQ-9.2, 01-REQ-9.4, 01-REQ-9.E1
func TestGoBuild(t *testing.T) {
	goExe, err := exec.LookPath("go")
	if err != nil {
		t.Skip("go not found on PATH; skipping Go build verification (install Go toolchain to enable)")
	}

	root := repoRoot(t)

	cmd := exec.Command(goExe, "build", "parking-fee-service/...")
	cmd.Dir = root
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Errorf("go build parking-fee-service/... failed: %v\n%s", err, string(out))
	}
}

// TestProtoFilesValidate verifies that all .proto files in the proto/ directory
// parse without errors by invoking protoc as a subprocess.
// If protoc is not on PATH the test is skipped with an explanatory message.
// Test Spec: TS-01-17, TS-01-30, TS-01-31
// Requirements: 01-REQ-5.4, 01-REQ-9.1, 01-REQ-9.2, 01-REQ-9.4, 01-REQ-9.E1
func TestProtoFilesValidate(t *testing.T) {
	if _, err := exec.LookPath("protoc"); err != nil {
		t.Skip("protoc not found on PATH; skipping proto validation (install protoc to enable)")
	}

	root := repoRoot(t)
	protoDir := filepath.Join(root, "proto")

	// Collect all .proto files under proto/.
	var protoFiles []string
	err := filepath.Walk(protoDir, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return err
		}
		if !info.IsDir() && filepath.Ext(path) == ".proto" {
			protoFiles = append(protoFiles, path)
		}
		return nil
	})
	if err != nil {
		t.Fatalf("cannot walk proto/ directory: %v", err)
	}
	if len(protoFiles) == 0 {
		t.Fatal("no .proto files found under proto/ — check directory structure")
	}

	// Validate all proto files together so cross-file imports are resolved.
	args := []string{"--proto_path=" + protoDir, "--descriptor_set_out=" + os.DevNull}
	args = append(args, protoFiles...)

	cmd := exec.Command("protoc", args...)
	cmd.Dir = root
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Errorf("protoc failed parsing proto files: %v\n%s", err, string(out))
	}
}
