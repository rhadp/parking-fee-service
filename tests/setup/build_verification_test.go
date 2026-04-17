package setup_test

import (
	"os"
	"os/exec"
	"path/filepath"
	"testing"
)

// TestRustBuild verifies the Rust workspace compiles successfully by invoking
// cargo build --workspace as a subprocess. Skips when cargo is not on PATH.
// (TS-01-30, TS-01-31, 01-REQ-9.1, 01-REQ-9.2, 01-REQ-9.4, 01-REQ-9.E1)
func TestRustBuild(t *testing.T) {
	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("cargo not found on PATH; skipping Rust build verification")
	}

	root := repoRoot(t)
	rhivosDir := filepath.Join(root, "rhivos")

	cmd := exec.Command("cargo", "build", "--workspace")
	cmd.Dir = rhivosDir
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr

	if err := cmd.Run(); err != nil {
		t.Errorf("cargo build --workspace failed: %v", err)
	}
}

// TestGoBuild verifies all Go modules compile successfully by invoking
// go build ./... in each module directory. Skips when go is not on PATH.
// (TS-01-30, TS-01-31, 01-REQ-9.1, 01-REQ-9.2, 01-REQ-9.4, 01-REQ-9.E1)
func TestGoBuild(t *testing.T) {
	if _, err := exec.LookPath("go"); err != nil {
		t.Skip("go not found on PATH; skipping Go build verification")
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
		t.Run(mod, func(t *testing.T) {
			cmd := exec.Command("go", "build", "./...")
			cmd.Dir = filepath.Join(root, mod)
			cmd.Stdout = os.Stdout
			cmd.Stderr = os.Stderr
			if err := cmd.Run(); err != nil {
				t.Errorf("go build ./... failed in %s: %v", mod, err)
			}
		})
	}
}

// TestProtoFilesValidate verifies all .proto files parse without errors by
// invoking protoc --descriptor_set_out=/dev/null on each file. Skips when
// protoc is not on PATH.
// (TS-01-30, TS-01-31, 01-REQ-9.1, 01-REQ-9.2, 01-REQ-9.4, 01-REQ-9.E1)
func TestProtoFilesValidate(t *testing.T) {
	if _, err := exec.LookPath("protoc"); err != nil {
		t.Skip("protoc not found on PATH; skipping proto validation")
	}

	root := repoRoot(t)

	protoFiles := []string{
		"kuksa/val.proto",
		"update/update_service.proto",
		"adapter/adapter_service.proto",
		"gateway/gateway.proto",
	}

	for _, pf := range protoFiles {
		t.Run(pf, func(t *testing.T) {
			cmd := exec.Command(
				"protoc",
				"--proto_path=proto",
				"--descriptor_set_out=/dev/null",
				filepath.Join("proto", pf),
			)
			cmd.Dir = root
			out, err := cmd.CombinedOutput()
			if err != nil {
				t.Errorf("protoc failed for %s: %v\n%s", pf, err, string(out))
			}
		})
	}
}
