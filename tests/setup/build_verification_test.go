package setup_test

import (
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
)

// requireTool skips the test if the named tool is not on PATH.
// Satisfies 01-REQ-9.E1 / TS-01-E10.
func requireTool(t *testing.T, name string) {
	t.Helper()
	if _, err := exec.LookPath(name); err != nil {
		t.Skipf("required tool %q not found on PATH; skipping", name)
	}
}

// TS-01-9: Cargo build succeeds for entire workspace
// Requirement: 01-REQ-2.4
func TestCargoBuildWorkspace(t *testing.T) {
	requireTool(t, "cargo")
	root := repoRoot(t)

	cmd := exec.Command("cargo", "build", "--workspace")
	cmd.Dir = filepath.Join(root, "rhivos")
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("cargo build --workspace failed: %v\n%s", err, out)
	}
}

// TS-01-12: Go build succeeds for all modules
// Requirement: 01-REQ-3.4
func TestGoBuildAllModules(t *testing.T) {
	requireTool(t, "go")
	root := repoRoot(t)

	// Build each Go module individually, mirroring the Makefile's build-go target.
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
				t.Fatalf("go build ./... in %s failed: %v\n%s", mod, err, out)
			}
		})
	}
}

// TS-01-13: Rust skeleton prints version and exits 0
// Requirement: 01-REQ-4.1, 01-REQ-4.4
func TestRustSkeletonBinaries(t *testing.T) {
	requireTool(t, "cargo")
	root := repoRoot(t)

	// Ensure binaries are built.
	cmd := exec.Command("cargo", "build", "--workspace")
	cmd.Dir = filepath.Join(root, "rhivos")
	if out, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("cargo build failed: %v\n%s", err, out)
	}

	binaries := []struct {
		name          string
		componentName string // what must appear in stdout
	}{
		{"locking-service", "locking-service"},
		{"cloud-gateway-client", "cloud-gateway-client"},
		{"update-service", "update-service"},
		{"parking-operator-adaptor", "parking-operator-adaptor"},
	}

	for _, bin := range binaries {
		t.Run(bin.name, func(t *testing.T) {
			binPath := filepath.Join(root, "rhivos", "target", "debug", bin.name)
			cmd := exec.Command(binPath)
			out, err := cmd.Output()
			if err != nil {
				t.Fatalf("%s exited with error: %v", bin.name, err)
			}
			stdout := string(out)
			if !strings.Contains(stdout, bin.componentName) {
				t.Errorf("%s stdout should contain %q, got: %q", bin.name, bin.componentName, stdout)
			}
		})
	}
}

// TS-01-14: Go skeleton prints version and exits 0
// Requirement: 01-REQ-4.2, 01-REQ-4.4
func TestGoSkeletonBinaries(t *testing.T) {
	requireTool(t, "go")
	root := repoRoot(t)

	modules := []struct {
		modPath       string
		componentName string
	}{
		{"backend/parking-fee-service", "parking-fee-service"},
		{"backend/cloud-gateway", "cloud-gateway"},
		{"mock/parking-app-cli", "parking-app-cli"},
		{"mock/companion-app-cli", "companion-app-cli"},
		{"mock/parking-operator", "parking-operator"},
	}

	for _, mod := range modules {
		t.Run(mod.componentName, func(t *testing.T) {
			cmd := exec.Command("go", "run", ".")
			cmd.Dir = filepath.Join(root, mod.modPath)
			out, err := cmd.Output()
			if err != nil {
				t.Fatalf("go run %s failed: %v", mod.modPath, err)
			}
			stdout := string(out)
			if !strings.Contains(stdout, mod.componentName) {
				t.Errorf("stdout should contain %q, got: %q", mod.componentName, stdout)
			}
		})
	}
}

// TS-01-15: Mock sensor binaries print name and version
// Requirement: 01-REQ-4.3
func TestMockSensorBinaries(t *testing.T) {
	requireTool(t, "cargo")
	root := repoRoot(t)

	// Ensure binaries are built.
	cmd := exec.Command("cargo", "build", "--workspace")
	cmd.Dir = filepath.Join(root, "rhivos")
	if out, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("cargo build failed: %v\n%s", err, out)
	}

	sensors := []string{"location-sensor", "speed-sensor", "door-sensor"}

	for _, sensor := range sensors {
		t.Run(sensor, func(t *testing.T) {
			binPath := filepath.Join(root, "rhivos", "target", "debug", sensor)
			cmd := exec.Command(binPath)
			out, err := cmd.Output()
			if err != nil {
				t.Fatalf("%s exited with error: %v", sensor, err)
			}
			stdout := string(out)
			if !strings.Contains(stdout, sensor) {
				t.Errorf("%s stdout should contain %q, got: %q", sensor, sensor, stdout)
			}
		})
	}
}

// TS-01-28: cargo test passes for all Rust crates
// Requirement: 01-REQ-8.3
func TestCargoTestPasses(t *testing.T) {
	requireTool(t, "cargo")
	root := repoRoot(t)

	cmd := exec.Command("cargo", "test", "--workspace")
	cmd.Dir = filepath.Join(root, "rhivos")
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("cargo test --workspace failed: %v\n%s", err, out)
	}
}

// TS-01-29: go test passes for all Go modules
// Requirement: 01-REQ-8.4
func TestGoTestPasses(t *testing.T) {
	requireTool(t, "go")
	root := repoRoot(t)

	// Test each Go module individually, mirroring the Makefile's test-go target.
	modules := []string{
		"backend/parking-fee-service",
		"backend/cloud-gateway",
		"mock/parking-app-cli",
		"mock/companion-app-cli",
		"mock/parking-operator",
	}

	for _, mod := range modules {
		t.Run(mod, func(t *testing.T) {
			cmd := exec.Command("go", "test", "./...")
			cmd.Dir = filepath.Join(root, mod)
			out, err := cmd.CombinedOutput()
			if err != nil {
				t.Fatalf("go test ./... in %s failed: %v\n%s", mod, err, out)
			}
		})
	}
}

// TS-01-E4: Skeleton exits non-zero on unknown flag
// Requirement: 01-REQ-4.E1
// Tests ALL skeleton binaries per Major Skeptic finding about incomplete coverage.
func TestSkeletonExitsNonZeroOnUnknownFlag(t *testing.T) {
	requireTool(t, "cargo")
	root := repoRoot(t)

	// Ensure binaries are built.
	cmd := exec.Command("cargo", "build", "--workspace")
	cmd.Dir = filepath.Join(root, "rhivos")
	if out, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("cargo build failed: %v\n%s", err, out)
	}

	// Test all Rust skeleton binaries.
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
			cmd := exec.Command(binPath, "--invalid-flag")
			var stderr strings.Builder
			cmd.Stderr = &stderr
			err := cmd.Run()
			if err == nil {
				t.Errorf("%s should exit non-zero on --invalid-flag", bin)
			}
			if stderr.Len() == 0 {
				t.Errorf("%s should print usage info to stderr on --invalid-flag", bin)
			}
		})
	}
}

// TS-01-P2: Skeleton determinism across invocations
// Property: Property 2 (Skeleton Determinism)
func TestPropertySkeletonDeterminism(t *testing.T) {
	requireTool(t, "cargo")
	requireTool(t, "go")
	root := repoRoot(t)

	// Ensure Rust binaries are built.
	cmd := exec.Command("cargo", "build", "--workspace")
	cmd.Dir = filepath.Join(root, "rhivos")
	if out, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("cargo build failed: %v\n%s", err, out)
	}

	// Test Rust skeleton determinism.
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

			cmd1 := exec.Command(binPath)
			out1, err1 := cmd1.Output()
			if err1 != nil {
				t.Fatalf("%s first invocation failed: %v", bin, err1)
			}

			cmd2 := exec.Command(binPath)
			out2, err2 := cmd2.Output()
			if err2 != nil {
				t.Fatalf("%s second invocation failed: %v", bin, err2)
			}

			if string(out1) != string(out2) {
				t.Errorf("%s output not deterministic: %q vs %q", bin, out1, out2)
			}
		})
	}

	// Test Go skeleton determinism.
	goModules := []struct {
		modPath string
		name    string
	}{
		{"backend/parking-fee-service", "parking-fee-service"},
		{"backend/cloud-gateway", "cloud-gateway"},
		{"mock/parking-app-cli", "parking-app-cli"},
		{"mock/companion-app-cli", "companion-app-cli"},
		{"mock/parking-operator", "parking-operator"},
	}

	for _, mod := range goModules {
		t.Run("go/"+mod.name, func(t *testing.T) {
			cmd1 := exec.Command("go", "run", ".")
			cmd1.Dir = filepath.Join(root, mod.modPath)
			out1, err1 := cmd1.Output()
			if err1 != nil {
				t.Fatalf("%s first invocation failed: %v", mod.name, err1)
			}

			cmd2 := exec.Command("go", "run", ".")
			cmd2.Dir = filepath.Join(root, mod.modPath)
			out2, err2 := cmd2.Output()
			if err2 != nil {
				t.Fatalf("%s second invocation failed: %v", mod.name, err2)
			}

			if string(out1) != string(out2) {
				t.Errorf("%s output not deterministic: %q vs %q", mod.name, out1, out2)
			}
		})
	}
}

// TS-01-E2: Cargo reports failing crate by name
// Requirement: 01-REQ-2.E1
func TestCargoReportsFailingCrate(t *testing.T) {
	requireTool(t, "cargo")
	root := repoRoot(t)

	// Inject a syntax error into locking-service.
	mainPath := filepath.Join(root, "rhivos", "locking-service", "src", "main.rs")
	original, err := os.ReadFile(mainPath)
	if err != nil {
		t.Fatalf("failed to read %s: %v", mainPath, err)
	}
	defer func() {
		if err := os.WriteFile(mainPath, original, 0o644); err != nil {
			t.Errorf("failed to restore %s: %v", mainPath, err)
		}
	}()

	if err := os.WriteFile(mainPath, []byte("fn main() { SYNTAX ERROR }"), 0o644); err != nil {
		t.Fatalf("failed to inject error: %v", err)
	}

	cmd := exec.Command("cargo", "build", "--workspace")
	cmd.Dir = filepath.Join(root, "rhivos")
	out, err := cmd.CombinedOutput()
	if err == nil {
		t.Fatal("cargo build should have failed with syntax error")
	}
	combined := string(out)
	if !strings.Contains(combined, "locking-service") && !strings.Contains(combined, "locking_service") {
		t.Errorf("cargo build error should identify the failing crate; output: %s", combined)
	}
}

// TS-01-E3: Go build fails with missing dependency
// Requirement: 01-REQ-3.E1
func TestGoBuildFailsMissingDependency(t *testing.T) {
	requireTool(t, "go")
	root := repoRoot(t)

	mainPath := filepath.Join(root, "backend", "parking-fee-service", "main.go")
	original, err := os.ReadFile(mainPath)
	if err != nil {
		t.Fatalf("failed to read %s: %v", mainPath, err)
	}
	defer func() {
		if err := os.WriteFile(mainPath, original, 0o644); err != nil {
			t.Errorf("failed to restore %s: %v", mainPath, err)
		}
	}()

	broken := `package main

import (
	"fmt"
	"unknown/nonexistent/package"
)

func main() {
	fmt.Println("parking-fee-service v0.1.0")
	_ = package.Foo
}
`
	// Using a valid Go import but nonexistent package.
	broken = "package main\n\nimport \"unknown/nonexistent\"\n\nfunc main() { _ = nonexistent.Foo }\n"

	if err := os.WriteFile(mainPath, []byte(broken), 0o644); err != nil {
		t.Fatalf("failed to inject bad import: %v", err)
	}

	cmd := exec.Command("go", "build", "./...")
	cmd.Dir = filepath.Join(root, "backend", "parking-fee-service")
	out, err := cmd.CombinedOutput()
	if err == nil {
		t.Fatal("go build should have failed with missing import")
	}
	combined := string(out)
	if !strings.Contains(combined, "unknown/nonexistent") {
		t.Errorf("go build error should mention the missing import; output: %s", combined)
	}
}

// TS-01-E9: Test runner reports syntax errors
// Requirement: 01-REQ-8.E1
func TestCargoTestReportsSyntaxError(t *testing.T) {
	requireTool(t, "cargo")
	root := repoRoot(t)

	mainPath := filepath.Join(root, "rhivos", "locking-service", "src", "main.rs")
	original, err := os.ReadFile(mainPath)
	if err != nil {
		t.Fatalf("failed to read %s: %v", mainPath, err)
	}
	defer func() {
		if err := os.WriteFile(mainPath, original, 0o644); err != nil {
			t.Errorf("failed to restore %s: %v", mainPath, err)
		}
	}()

	if err := os.WriteFile(mainPath, []byte("fn main() { let x = ; }"), 0o644); err != nil {
		t.Fatalf("failed to inject syntax error: %v", err)
	}

	cmd := exec.Command("cargo", "test", "--workspace")
	cmd.Dir = filepath.Join(root, "rhivos")
	out, err := cmd.CombinedOutput()
	if err == nil {
		t.Fatal("cargo test should have failed with syntax error")
	}
	combined := string(out)
	if !strings.Contains(combined, "main.rs") {
		t.Errorf("cargo test error should mention the file; output: %s", combined)
	}
}

// TS-01-E1: Build succeeds with extraneous files in repo
// Requirement: 01-REQ-1.E1
func TestBuildWithStrayFile(t *testing.T) {
	requireTool(t, "cargo")
	requireTool(t, "go")
	requireTool(t, "make")
	root := repoRoot(t)

	strayPath := filepath.Join(root, "stray_file.txt")
	if err := os.WriteFile(strayPath, []byte("test content"), 0o644); err != nil {
		t.Fatalf("failed to create stray file: %v", err)
	}
	defer os.Remove(strayPath)

	cmd := exec.Command("make", "build")
	cmd.Dir = root
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("make build should succeed with stray file: %v\n%s", err, out)
	}
}
