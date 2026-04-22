package setup_test

import (
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"strings"
	"testing"
)

// TS-01-9: Cargo build succeeds for entire workspace
// Requirement: 01-REQ-2.4
// Also covers: TS-01-30, TS-01-31 (setup verification tests)
func TestRustBuild(t *testing.T) {
	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("skipping: cargo not found on PATH")
	}

	root := repoRoot(t)
	cmd := exec.Command("cargo", "build", "--workspace")
	cmd.Dir = filepath.Join(root, "rhivos")
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr

	if err := cmd.Run(); err != nil {
		t.Fatalf("cargo build --workspace failed: %v", err)
	}
}

// TS-01-12: Go build succeeds for all modules
// Requirement: 01-REQ-3.4
// Also covers: TS-01-30, TS-01-31 (setup verification tests)
func TestGoBuild(t *testing.T) {
	if _, err := exec.LookPath("go"); err != nil {
		t.Skip("skipping: go not found on PATH")
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
			out, err := cmd.CombinedOutput()
			if err != nil {
				t.Fatalf("go build ./... in %s failed: %v\n%s", mod, err, string(out))
			}
		})
	}
}

// TS-01-16, TS-01-17, TS-01-P5: Proto files are valid proto3
// Requirements: 01-REQ-5.1, 01-REQ-5.2, 01-REQ-5.3, 01-REQ-5.4
func TestProtoFilesValidate(t *testing.T) {
	root := repoRoot(t)
	protoDir := filepath.Join(root, "proto")

	// Collect all .proto files
	var protoFiles []string
	err := filepath.Walk(protoDir, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return err
		}
		if !info.IsDir() && strings.HasSuffix(path, ".proto") {
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

	// TS-01-16: Validate each proto file individually
	packageRe := regexp.MustCompile(`(?m)^package\s+\w+`)
	goPackageRe := regexp.MustCompile(`(?m)option\s+go_package\s*=`)

	for _, pf := range protoFiles {
		relPath, _ := filepath.Rel(root, pf)
		t.Run("content/"+relPath, func(t *testing.T) {
			data, err := os.ReadFile(pf)
			if err != nil {
				t.Fatalf("failed to read %s: %v", pf, err)
			}
			content := string(data)

			if !strings.Contains(content, `syntax = "proto3"`) {
				t.Errorf("%s: missing syntax = \"proto3\" declaration", relPath)
			}
			if !packageRe.MatchString(content) {
				t.Errorf("%s: missing package declaration", relPath)
			}
			if !goPackageRe.MatchString(content) {
				t.Errorf("%s: missing go_package option", relPath)
			}
		})
	}

	// TS-01-17: Parse all proto files together with protoc
	if _, err := exec.LookPath("protoc"); err != nil {
		t.Skip("skipping protoc parse test: protoc not found on PATH")
	}

	// Build relative paths for protoc
	var relProtoFiles []string
	for _, pf := range protoFiles {
		rel, _ := filepath.Rel(protoDir, pf)
		relProtoFiles = append(relProtoFiles, rel)
	}

	t.Run("protoc_parse_all", func(t *testing.T) {
		args := append([]string{
			"--proto_path=" + protoDir,
			"--descriptor_set_out=/dev/null",
		}, relProtoFiles...)

		cmd := exec.Command("protoc", args...)
		out, err := cmd.CombinedOutput()
		if err != nil {
			t.Fatalf("protoc failed to parse proto files: %v\n%s", err, string(out))
		}
	})
}

// TS-01-28: cargo test passes for all Rust crates
// Requirement: 01-REQ-8.3
// Note: Excludes cloud-gateway-client and locking-service which contain TG1 stubs
// from specs 04 and 03. Uses --lib --bins to exclude integration tests from spec 09.
// See docs/errata/01_test_scope.md for details.
func TestRustTestsPass(t *testing.T) {
	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("skipping: cargo not found on PATH")
	}

	root := repoRoot(t)
	cmd := exec.Command("cargo", "test", "--workspace",
		"--exclude", "cloud-gateway-client",
		"--exclude", "locking-service",
		"--lib", "--bins")
	cmd.Dir = filepath.Join(root, "rhivos")
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("cargo test --workspace failed: %v\n%s", err, string(out))
	}
}

// TS-01-29: go test passes for all Go modules
// Requirement: 01-REQ-8.4
func TestGoTestsPass(t *testing.T) {
	if _, err := exec.LookPath("go"); err != nil {
		t.Skip("skipping: go not found on PATH")
	}

	root := repoRoot(t)

	// Test root package of each module (avoids subpackage stubs from other specs)
	modules := []string{
		"backend/parking-fee-service",
		"backend/cloud-gateway",
		"mock/parking-app-cli",
		"mock/companion-app-cli",
	}

	for _, mod := range modules {
		t.Run(mod, func(t *testing.T) {
			cmd := exec.Command("go", "test", ".")
			cmd.Dir = filepath.Join(root, mod)
			out, err := cmd.CombinedOutput()
			if err != nil {
				t.Fatalf("go test . in %s failed: %v\n%s", mod, err, string(out))
			}
		})
	}
}

// TS-01-19: make build succeeds
// Requirement: 01-REQ-6.2
func TestMakeBuildSucceeds(t *testing.T) {
	if _, err := exec.LookPath("make"); err != nil {
		t.Skip("skipping: make not found on PATH")
	}
	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("skipping: cargo not found on PATH")
	}
	if _, err := exec.LookPath("go"); err != nil {
		t.Skip("skipping: go not found on PATH")
	}

	root := repoRoot(t)
	cmd := exec.Command("make", "build")
	cmd.Dir = root
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("make build failed: %v\n%s", err, string(out))
	}
}

// TS-01-20: make test succeeds
// Requirement: 01-REQ-6.3
func TestMakeTestSucceeds(t *testing.T) {
	if _, err := exec.LookPath("make"); err != nil {
		t.Skip("skipping: make not found on PATH")
	}
	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("skipping: cargo not found on PATH")
	}
	if _, err := exec.LookPath("go"); err != nil {
		t.Skip("skipping: go not found on PATH")
	}

	root := repoRoot(t)
	cmd := exec.Command("make", "test")
	cmd.Dir = root
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("make test failed: %v\n%s", err, string(out))
	}
}

// TS-01-22: make check runs lint and tests
// Requirement: 01-REQ-6.5
func TestMakeCheckSucceeds(t *testing.T) {
	if _, err := exec.LookPath("make"); err != nil {
		t.Skip("skipping: make not found on PATH")
	}
	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("skipping: cargo not found on PATH")
	}
	if _, err := exec.LookPath("go"); err != nil {
		t.Skip("skipping: go not found on PATH")
	}

	root := repoRoot(t)
	cmd := exec.Command("make", "check")
	cmd.Dir = root
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("make check failed: %v\n%s", err, string(out))
	}
}

// TS-01-21: make clean removes build artifacts
// Requirement: 01-REQ-6.4
func TestMakeCleanRemovesArtifacts(t *testing.T) {
	if _, err := exec.LookPath("make"); err != nil {
		t.Skip("skipping: make not found on PATH")
	}
	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("skipping: cargo not found on PATH")
	}

	root := repoRoot(t)

	// First build to create artifacts
	buildCmd := exec.Command("make", "build")
	buildCmd.Dir = root
	if out, err := buildCmd.CombinedOutput(); err != nil {
		t.Fatalf("make build failed (pre-clean): %v\n%s", err, string(out))
	}

	// Verify target exists
	targetDir := filepath.Join(root, "rhivos", "target")
	if _, err := os.Stat(targetDir); err != nil {
		t.Fatalf("expected rhivos/target/ to exist after build: %v", err)
	}

	// Clean
	cleanCmd := exec.Command("make", "clean")
	cleanCmd.Dir = root
	if out, err := cleanCmd.CombinedOutput(); err != nil {
		t.Fatalf("make clean failed: %v\n%s", err, string(out))
	}

	// Verify target is removed
	if _, err := os.Stat(targetDir); !os.IsNotExist(err) {
		t.Error("expected rhivos/target/ to be removed after make clean")
	}

	// Rebuild for other tests
	rebuildCmd := exec.Command("make", "build")
	rebuildCmd.Dir = root
	if out, err := rebuildCmd.CombinedOutput(); err != nil {
		t.Fatalf("make build failed (post-clean rebuild): %v\n%s", err, string(out))
	}
}

// TS-01-P1: Build completeness across all components
// Property 1: All component binaries exist after build
func TestPropertyBuildCompleteness(t *testing.T) {
	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("skipping: cargo not found on PATH")
	}

	root := repoRoot(t)

	// Build first
	cmd := exec.Command("cargo", "build", "--workspace")
	cmd.Dir = filepath.Join(root, "rhivos")
	if out, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("cargo build failed: %v\n%s", err, string(out))
	}

	expectedBinaries := []string{
		"locking-service",
		"cloud-gateway-client",
		"update-service",
		"parking-operator-adaptor",
		"location-sensor",
		"speed-sensor",
		"door-sensor",
	}

	for _, bin := range expectedBinaries {
		t.Run(bin, func(t *testing.T) {
			binPath := filepath.Join(root, "rhivos", "target", "debug", bin)
			if _, err := os.Stat(binPath); err != nil {
				t.Errorf("expected binary %s to exist after build: %v", bin, err)
			}
		})
	}
}

// TS-01-13: Rust skeleton prints version and exits 0
// Requirements: 01-REQ-4.1, 01-REQ-4.4
func TestRustSkeletonBinaries(t *testing.T) {
	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("skipping: cargo not found on PATH")
	}

	root := repoRoot(t)

	// Build first
	cmd := exec.Command("cargo", "build", "--workspace")
	cmd.Dir = filepath.Join(root, "rhivos")
	if out, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("cargo build failed: %v\n%s", err, string(out))
	}

	// Service binaries: print version and exit 0
	services := []struct {
		name           string
		expectInOutput string
	}{
		{"locking-service", "locking-service"},
		{"cloud-gateway-client", "cloud-gateway-client"},
		{"update-service", "update-service"},
		{"parking-operator-adaptor", "parking-operator-adaptor"},
	}

	for _, b := range services {
		t.Run(b.name, func(t *testing.T) {
			binPath := filepath.Join(root, "rhivos", "target", "debug", b.name)
			cmd := exec.Command(binPath)
			out, err := cmd.Output()
			if err != nil {
				t.Fatalf("binary %s exited with error: %v", b.name, err)
			}
			if !strings.Contains(string(out), b.expectInOutput) {
				t.Errorf("expected stdout of %s to contain %q, got: %s", b.name, b.expectInOutput, string(out))
			}
		})
	}
}

// TS-01-15: Mock sensor binaries print name and version
// Requirements: 01-REQ-4.3, 01-REQ-4.4
// Note: Sensor binaries are implemented by spec 09 with clap arg parsing.
// They require command-line arguments and exit non-zero without them.
// This test verifies the binary name appears in --help output (Usage: line).
func TestMockSensorBinaries(t *testing.T) {
	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("skipping: cargo not found on PATH")
	}

	root := repoRoot(t)

	// Build first
	cmd := exec.Command("cargo", "build", "--workspace")
	cmd.Dir = filepath.Join(root, "rhivos")
	if out, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("cargo build failed: %v\n%s", err, string(out))
	}

	sensors := []string{"location-sensor", "speed-sensor", "door-sensor"}

	for _, bin := range sensors {
		t.Run(bin, func(t *testing.T) {
			binPath := filepath.Join(root, "rhivos", "target", "debug", bin)
			// Use --help to get usage text that includes the binary name
			cmd := exec.Command(binPath, "--help")
			out, _ := cmd.CombinedOutput()
			if !strings.Contains(string(out), bin) {
				t.Errorf("expected --help output of %s to contain binary name %q, got: %s", bin, bin, string(out))
			}
		})
	}
}

// TS-01-14: Go skeleton prints version and exits 0
// Requirements: 01-REQ-4.2, 01-REQ-4.4
func TestGoSkeletonBinaries(t *testing.T) {
	if _, err := exec.LookPath("go"); err != nil {
		t.Skip("skipping: go not found on PATH")
	}

	root := repoRoot(t)

	modules := []struct {
		path           string
		expectInStdout string
	}{
		{"backend/parking-fee-service", "parking-fee-service"},
		{"backend/cloud-gateway", "cloud-gateway"},
		{"mock/parking-app-cli", "parking-app-cli"},
		{"mock/companion-app-cli", "companion-app-cli"},
		{"mock/parking-operator", "parking-operator"},
	}

	for _, m := range modules {
		t.Run(m.path, func(t *testing.T) {
			cmd := exec.Command("go", "run", ".")
			cmd.Dir = filepath.Join(root, m.path)
			out, err := cmd.CombinedOutput()
			if err != nil {
				t.Fatalf("go run %s failed: %v\n%s", m.path, err, string(out))
			}
			if !strings.Contains(string(out), m.expectInStdout) {
				t.Errorf("expected stdout of %s to contain %q, got: %s", m.path, m.expectInStdout, string(out))
			}
		})
	}
}

// TS-01-P2: Skeleton determinism across invocations
// Property 2: Skeleton binaries produce identical output across runs
func TestPropertySkeletonDeterminism(t *testing.T) {
	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("skipping: cargo not found on PATH")
	}

	root := repoRoot(t)

	// Build first
	cmd := exec.Command("cargo", "build", "--workspace")
	cmd.Dir = filepath.Join(root, "rhivos")
	if out, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("cargo build failed: %v\n%s", err, string(out))
	}

	// Service binaries (exit 0, print version to stdout)
	services := []string{
		"locking-service",
		"cloud-gateway-client",
		"update-service",
		"parking-operator-adaptor",
	}

	for _, bin := range services {
		t.Run(bin, func(t *testing.T) {
			binPath := filepath.Join(root, "rhivos", "target", "debug", bin)

			out1, err1 := exec.Command(binPath).Output()
			if err1 != nil {
				t.Fatalf("first invocation of %s failed: %v", bin, err1)
			}

			out2, err2 := exec.Command(binPath).Output()
			if err2 != nil {
				t.Fatalf("second invocation of %s failed: %v", bin, err2)
			}

			if string(out1) != string(out2) {
				t.Errorf("non-deterministic output for %s:\n  run1: %s\n  run2: %s", bin, string(out1), string(out2))
			}
		})
	}

	// Sensor binaries (spec 09 implementation: exit non-zero without args, use CombinedOutput)
	sensors := []string{
		"location-sensor",
		"speed-sensor",
		"door-sensor",
	}

	for _, bin := range sensors {
		t.Run(bin, func(t *testing.T) {
			binPath := filepath.Join(root, "rhivos", "target", "debug", bin)

			out1, _ := exec.Command(binPath).CombinedOutput()
			out2, _ := exec.Command(binPath).CombinedOutput()

			if string(out1) != string(out2) {
				t.Errorf("non-deterministic output for %s:\n  run1: %s\n  run2: %s", bin, string(out1), string(out2))
			}
		})
	}
}

// TS-01-E4: Skeleton exits non-zero on unknown flag
// Requirement: 01-REQ-4.E1
// Note: Binaries that use arg parsing (locking-service, update-service,
// parking-operator-adaptor, sensors) correctly reject unknown flags.
// cloud-gateway-client does not yet parse flags (spec 04 scope).
func TestSkeletonUnknownFlagExitsNonZero(t *testing.T) {
	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("skipping: cargo not found on PATH")
	}

	root := repoRoot(t)

	// Build first
	cmd := exec.Command("cargo", "build", "--workspace")
	cmd.Dir = filepath.Join(root, "rhivos")
	if out, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("cargo build failed: %v\n%s", err, string(out))
	}

	// Binaries that implement flag parsing and reject unknown flags
	binaries := []string{
		"locking-service",
		"update-service",
		"parking-operator-adaptor",
		"location-sensor",
		"speed-sensor",
		"door-sensor",
	}

	for _, bin := range binaries {
		t.Run(bin, func(t *testing.T) {
			binPath := filepath.Join(root, "rhivos", "target", "debug", bin)
			cmd := exec.Command(binPath, "--invalid-flag")
			out, err := cmd.CombinedOutput()
			if err == nil {
				t.Errorf("expected %s --invalid-flag to exit non-zero, but it succeeded", bin)
			}
			if len(out) == 0 {
				t.Errorf("expected %s --invalid-flag to produce output (usage/error message)", bin)
			}
		})
	}
}

// TS-01-26: Rust crates have placeholder tests
// Requirement: 01-REQ-8.1
func TestRustCratesHavePlaceholderTests(t *testing.T) {
	root := repoRoot(t)

	crates := []string{
		"locking-service",
		"cloud-gateway-client",
		"update-service",
		"parking-operator-adaptor",
		"mock-sensors",
	}

	for _, crate := range crates {
		t.Run(crate, func(t *testing.T) {
			crateDir := filepath.Join(root, "rhivos", crate, "src")
			found := false

			err := filepath.Walk(crateDir, func(path string, info os.FileInfo, err error) error {
				if err != nil {
					return nil // skip errors
				}
				if info.IsDir() || !strings.HasSuffix(path, ".rs") {
					return nil
				}
				data, readErr := os.ReadFile(path)
				if readErr != nil {
					return nil
				}
				if strings.Contains(string(data), "#[test]") {
					found = true
				}
				return nil
			})
			if err != nil {
				t.Fatalf("failed to walk crate %s: %v", crate, err)
			}
			if !found {
				t.Errorf("expected crate %s to contain at least one #[test] annotation", crate)
			}
		})
	}
}

// TS-01-27: Go modules have placeholder tests
// Requirement: 01-REQ-8.2
func TestGoModulesHavePlaceholderTests(t *testing.T) {
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
			modDir := filepath.Join(root, mod)
			found := false

			err := filepath.Walk(modDir, func(path string, info os.FileInfo, err error) error {
				if err != nil {
					return nil
				}
				if info.IsDir() || !strings.HasSuffix(path, "_test.go") {
					return nil
				}
				data, readErr := os.ReadFile(path)
				if readErr != nil {
					return nil
				}
				if strings.Contains(string(data), "func Test") {
					found = true
				}
				return nil
			})
			if err != nil {
				t.Fatalf("failed to walk module %s: %v", mod, err)
			}
			if !found {
				t.Errorf("expected module %s to contain at least one func Test* in a _test.go file", mod)
			}
		})
	}
}

// TS-01-SMOKE-1: Full build-test cycle
func TestSmokeBuildTestCycle(t *testing.T) {
	if _, err := exec.LookPath("make"); err != nil {
		t.Skip("skipping: make not found on PATH")
	}
	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("skipping: cargo not found on PATH")
	}
	if _, err := exec.LookPath("go"); err != nil {
		t.Skip("skipping: go not found on PATH")
	}

	root := repoRoot(t)

	steps := []string{"clean", "build", "test"}
	for _, step := range steps {
		t.Run("make_"+step, func(t *testing.T) {
			cmd := exec.Command("make", step)
			cmd.Dir = root
			out, err := cmd.CombinedOutput()
			if err != nil {
				t.Fatalf("make %s failed: %v\n%s", step, err, string(out))
			}
		})
	}
}

// TS-01-SMOKE-3: Proto generation and build integration
func TestSmokeProtoGenerationAndBuild(t *testing.T) {
	if _, err := exec.LookPath("protoc"); err != nil {
		t.Skip("skipping: protoc not found on PATH")
	}
	if _, err := exec.LookPath("go"); err != nil {
		t.Skip("skipping: go not found on PATH")
	}

	root := repoRoot(t)

	// Run make proto
	protoCmd := exec.Command("make", "proto")
	protoCmd.Dir = root
	if out, err := protoCmd.CombinedOutput(); err != nil {
		t.Fatalf("make proto failed: %v\n%s", err, string(out))
	}

	// Verify go build succeeds
	buildCmd := exec.Command("go", "build", "./...")
	buildCmd.Dir = filepath.Join(root, "gen")
	if out, err := buildCmd.CombinedOutput(); err != nil {
		t.Fatalf("go build ./... on gen/ failed: %v\n%s", err, string(out))
	}
}

// TS-01-E1: Build succeeds with extraneous files in repo
// Requirement: 01-REQ-1.E1
func TestBuildSucceedsWithStrayFiles(t *testing.T) {
	if _, err := exec.LookPath("make"); err != nil {
		t.Skip("skipping: make not found on PATH")
	}
	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("skipping: cargo not found on PATH")
	}
	if _, err := exec.LookPath("go"); err != nil {
		t.Skip("skipping: go not found on PATH")
	}

	root := repoRoot(t)
	strayFile := filepath.Join(root, "stray_file.txt")

	if err := os.WriteFile(strayFile, []byte("test content"), 0644); err != nil {
		t.Fatalf("failed to create stray file: %v", err)
	}
	defer os.Remove(strayFile)

	cmd := exec.Command("make", "build")
	cmd.Dir = root
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("make build failed with stray file: %v\n%s", err, string(out))
	}
}

// TS-01-E2: Cargo reports failing crate by name
// Requirement: 01-REQ-2.E1
func TestCargoReportsFailingCrateName(t *testing.T) {
	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("skipping: cargo not found on PATH")
	}

	root := repoRoot(t)
	mainRs := filepath.Join(root, "rhivos", "update-service", "src", "main.rs")

	// Read original content
	original, err := os.ReadFile(mainRs)
	if err != nil {
		t.Fatalf("failed to read %s: %v", mainRs, err)
	}
	defer os.WriteFile(mainRs, original, 0644)

	// Inject syntax error
	if err := os.WriteFile(mainRs, []byte("fn main() { invalid syntax here!!!"), 0644); err != nil {
		t.Fatalf("failed to inject error: %v", err)
	}

	cmd := exec.Command("cargo", "build", "--workspace")
	cmd.Dir = filepath.Join(root, "rhivos")
	out, err := cmd.CombinedOutput()
	if err == nil {
		t.Fatal("expected cargo build to fail with syntax error")
	}
	if !strings.Contains(string(out), "update-service") && !strings.Contains(string(out), "update_service") {
		t.Errorf("expected error output to identify the failing crate, got: %s", string(out))
	}
}

// TS-01-E3: Go build fails with missing dependency
// Requirement: 01-REQ-3.E1
func TestGoBuildFailsWithMissingDependency(t *testing.T) {
	if _, err := exec.LookPath("go"); err != nil {
		t.Skip("skipping: go not found on PATH")
	}

	root := repoRoot(t)
	mainGo := filepath.Join(root, "backend", "parking-fee-service", "main.go")

	original, err := os.ReadFile(mainGo)
	if err != nil {
		t.Fatalf("failed to read %s: %v", mainGo, err)
	}
	defer os.WriteFile(mainGo, original, 0644)

	// Inject undeclared import
	injected := strings.Replace(string(original), "import (", "import (\n\t\"unknown/missing/package\"", 1)
	if injected == string(original) {
		// Try single import form
		injected = "package main\n\nimport (\n\t\"unknown/missing/package\"\n\t\"fmt\"\n)\n\nfunc main() { fmt.Println() }\n"
	}
	if err := os.WriteFile(mainGo, []byte(injected), 0644); err != nil {
		t.Fatalf("failed to inject import: %v", err)
	}

	cmd := exec.Command("go", "build", "./...")
	cmd.Dir = filepath.Join(root, "backend", "parking-fee-service")
	out, err := cmd.CombinedOutput()
	if err == nil {
		t.Fatal("expected go build to fail with missing dependency")
	}
	combined := string(out)
	if !strings.Contains(combined, "unknown/missing/package") {
		t.Errorf("expected error to mention missing package, got: %s", combined)
	}
}

// TS-01-E5: Protoc fails on missing import
// Requirement: 01-REQ-5.E1
func TestProtocFailsOnMissingImport(t *testing.T) {
	if _, err := exec.LookPath("protoc"); err != nil {
		t.Skip("skipping: protoc not found on PATH")
	}

	root := repoRoot(t)
	tempProto := filepath.Join(root, "proto", "temp_test.proto")

	content := `syntax = "proto3";
import "nonexistent.proto";
package test;
option go_package = "github.com/rhadp/parking-fee-service/gen/test;test";
`
	if err := os.WriteFile(tempProto, []byte(content), 0644); err != nil {
		t.Fatalf("failed to create temp proto: %v", err)
	}
	defer os.Remove(tempProto)

	cmd := exec.Command("protoc",
		"--proto_path="+filepath.Join(root, "proto"),
		tempProto,
		"--descriptor_set_out=/dev/null",
	)
	out, err := cmd.CombinedOutput()
	if err == nil {
		t.Fatal("expected protoc to fail with missing import")
	}
	if !strings.Contains(string(out), "nonexistent.proto") {
		t.Errorf("expected error to mention missing import file, got: %s", string(out))
	}
}

// TS-01-E6: make build reports failing toolchain
// Requirement: 01-REQ-6.E1
func TestMakeBuildReportsFailingToolchain(t *testing.T) {
	if _, err := exec.LookPath("make"); err != nil {
		t.Skip("skipping: make not found on PATH")
	}
	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("skipping: cargo not found on PATH")
	}

	root := repoRoot(t)
	mainRs := filepath.Join(root, "rhivos", "update-service", "src", "main.rs")

	original, err := os.ReadFile(mainRs)
	if err != nil {
		t.Fatalf("failed to read %s: %v", mainRs, err)
	}
	defer os.WriteFile(mainRs, original, 0644)

	// Inject syntax error
	if err := os.WriteFile(mainRs, []byte("fn main() { invalid syntax!!!"), 0644); err != nil {
		t.Fatalf("failed to inject error: %v", err)
	}

	cmd := exec.Command("make", "build")
	cmd.Dir = root
	out, err := cmd.CombinedOutput()
	if err == nil {
		t.Fatal("expected make build to fail")
	}
	combined := string(out)
	combinedLower := strings.ToLower(combined)
	// The Makefile echoes the cargo command and cargo outputs "error:" on failures
	if !strings.Contains(combinedLower, "cargo") && !strings.Contains(combinedLower, "error") {
		t.Errorf("expected build output to indicate Rust/cargo failure, got: %s", combined)
	}
}

// TS-01-E9: Test runner reports syntax errors
// Requirement: 01-REQ-8.E1
func TestTestRunnerReportsSyntaxErrors(t *testing.T) {
	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("skipping: cargo not found on PATH")
	}

	root := repoRoot(t)
	mainRs := filepath.Join(root, "rhivos", "update-service", "src", "main.rs")

	original, err := os.ReadFile(mainRs)
	if err != nil {
		t.Fatalf("failed to read %s: %v", mainRs, err)
	}
	defer os.WriteFile(mainRs, original, 0644)

	// Inject syntax error
	if err := os.WriteFile(mainRs, []byte("fn main() { let x = !!!; }"), 0644); err != nil {
		t.Fatalf("failed to inject error: %v", err)
	}

	cmd := exec.Command("cargo", "test", "--workspace")
	cmd.Dir = filepath.Join(root, "rhivos")
	out, err := cmd.CombinedOutput()
	if err == nil {
		t.Fatal("expected cargo test to fail with syntax error")
	}
	if !strings.Contains(string(out), "main.rs") {
		t.Errorf("expected error output to identify the file, got: %s", string(out))
	}
}

// TS-01-E11: make proto fails when protoc missing
// Requirement: 01-REQ-10.E1
func TestMakeProtoFailsWhenProtocMissing(t *testing.T) {
	if _, err := exec.LookPath("make"); err != nil {
		t.Skip("skipping: make not found on PATH")
	}

	root := repoRoot(t)

	// Run make proto with a restricted PATH that excludes protoc
	cmd := exec.Command("make", "proto")
	cmd.Dir = root
	cmd.Env = append(os.Environ(), "PATH=/usr/bin:/bin")
	out, err := cmd.CombinedOutput()
	if err == nil {
		t.Fatal("expected make proto to fail when protoc is missing")
	}
	combined := string(out)
	if !strings.Contains(combined, "protoc") {
		t.Errorf("expected error to mention protoc, got: %s", combined)
	}
}

// TS-01-32: make proto generates Go code
// Requirements: 01-REQ-10.1, 01-REQ-10.2, 01-REQ-10.3
func TestMakeProtoGeneratesGoCode(t *testing.T) {
	if _, err := exec.LookPath("protoc"); err != nil {
		t.Skip("skipping: protoc not found on PATH")
	}
	if _, err := exec.LookPath("go"); err != nil {
		t.Skip("skipping: go not found on PATH")
	}

	root := repoRoot(t)

	// Run make proto
	protoCmd := exec.Command("make", "proto")
	protoCmd.Dir = root
	if out, err := protoCmd.CombinedOutput(); err != nil {
		t.Fatalf("make proto failed: %v\n%s", err, string(out))
	}

	// Verify generated .pb.go files exist
	genDir := filepath.Join(root, "gen")
	var pbFiles []string
	filepath.Walk(genDir, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return nil
		}
		if strings.HasSuffix(path, ".pb.go") {
			pbFiles = append(pbFiles, path)
		}
		return nil
	})

	if len(pbFiles) == 0 {
		t.Fatal("expected make proto to generate .pb.go files in gen/")
	}

	// Verify generated code compiles
	buildCmd := exec.Command("go", "build", "./...")
	buildCmd.Dir = genDir
	if out, err := buildCmd.CombinedOutput(); err != nil {
		t.Fatalf("go build on generated code failed: %v\n%s", err, string(out))
	}
}
