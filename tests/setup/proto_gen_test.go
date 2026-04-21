package setup

import (
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
)

// TestMakeProtoFailsWithoutProtoc verifies `make proto` prints an error message
// mentioning protoc when protoc is not installed.
// Test Spec: TS-01-E11
// Requirement: 01-REQ-10.E1
func TestMakeProtoFailsWithoutProtoc(t *testing.T) {
	if _, err := exec.LookPath("make"); err != nil {
		t.Skip("make not found on PATH — skipping proto test")
	}

	root := findRepoRoot(t)

	// Run make proto with a PATH that excludes protoc.
	cmd := exec.Command("make", "proto")
	cmd.Dir = root
	// Set PATH to only include basic utilities, excluding protoc.
	cmd.Env = append(cmd.Environ(), "PATH=/usr/bin:/bin")
	out, err := cmd.CombinedOutput()
	if err == nil {
		t.Fatal("expected make proto to fail without protoc, but it succeeded")
	}
	combined := string(out)
	if !strings.Contains(strings.ToLower(combined), "protoc") {
		t.Fatalf("expected error to mention 'protoc', got:\n%s", combined)
	}
}

// TestMakeProtoGeneratesGoCode verifies `make proto` generates Go code and
// that the generated code compiles with go build.
// Test Spec: TS-01-32, TS-01-SMOKE-3
// Requirements: 01-REQ-10.1, 01-REQ-10.2, 01-REQ-10.3
func TestMakeProtoGeneratesGoCode(t *testing.T) {
	if _, err := exec.LookPath("protoc"); err != nil {
		t.Skip("protoc not found on PATH — skipping proto generation test")
	}
	if _, err := exec.LookPath("protoc-gen-go"); err != nil {
		t.Skip("protoc-gen-go not found on PATH — skipping proto generation test")
	}
	if _, err := exec.LookPath("protoc-gen-go-grpc"); err != nil {
		t.Skip("protoc-gen-go-grpc not found on PATH — skipping proto generation test")
	}

	root := findRepoRoot(t)

	// Run make proto.
	protoCmd := exec.Command("make", "proto")
	protoCmd.Dir = root
	out, err := protoCmd.CombinedOutput()
	if err != nil {
		t.Fatalf("make proto failed:\n%s\nerror: %v", out, err)
	}

	// Verify generated .pb.go files exist in gen/.
	genDir := filepath.Join(root, "gen")
	found := false
	_ = filepath.WalkDir(genDir, func(path string, d os.DirEntry, err error) error {
		if err != nil {
			return err
		}
		if !d.IsDir() && strings.HasSuffix(path, ".pb.go") {
			found = true
		}
		return nil
	})
	if !found {
		t.Fatal("make proto did not generate any .pb.go files in gen/")
	}

	// Verify go build succeeds for workspace modules (gen/ is not a module,
	// so we build individual modules rather than root-level ./...).
	modules := []string{
		"backend/parking-fee-service",
		"backend/cloud-gateway",
		"mock/parking-app-cli",
		"mock/companion-app-cli",
		"mock/parking-operator",
	}
	for _, mod := range modules {
		buildCmd := exec.Command("go", "build", "./...")
		buildCmd.Dir = filepath.Join(root, mod)
		out, err = buildCmd.CombinedOutput()
		if err != nil {
			t.Fatalf("go build failed in %s after make proto:\n%s\nerror: %v", mod, out, err)
		}
	}
}
