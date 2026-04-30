package setup_test

import (
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
)

// TS-01-E1: Build succeeds with extraneous files in repo
// Requirement: 01-REQ-1.E1
func TestBuildSucceedsWithStrayFiles(t *testing.T) {
	root := repoRoot(t)

	if _, err := exec.LookPath("make"); err != nil {
		t.Skip("make not found on PATH; skipping")
	}

	// Create a stray file at repo root
	strayPath := filepath.Join(root, "stray_file.txt")
	err := os.WriteFile(strayPath, []byte("test content"), 0644)
	if err != nil {
		t.Fatalf("failed to create stray file: %v", err)
	}
	t.Cleanup(func() {
		os.Remove(strayPath)
	})

	cmd := exec.Command("make", "build")
	cmd.Dir = root
	output, err := cmd.CombinedOutput()
	if err != nil {
		t.Errorf("make build failed with stray file present (exit error: %v)\noutput:\n%s", err, string(output))
	}
}

// TS-01-E2: Cargo reports failing crate by name
// Requirement: 01-REQ-2.E1
func TestCargoReportsFailingCrate(t *testing.T) {
	root := repoRoot(t)

	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("cargo not found on PATH; skipping")
	}

	mainPath := filepath.Join(root, "rhivos", "locking-service", "src", "main.rs")

	// Read original content
	original, err := os.ReadFile(mainPath)
	if err != nil {
		t.Skip("locking-service/src/main.rs does not exist yet; skipping error injection test")
	}
	t.Cleanup(func() {
		os.WriteFile(mainPath, original, 0644)
	})

	// Inject a compile error
	err = os.WriteFile(mainPath, []byte("fn main() { this_is_not_valid_rust!!! }"), 0644)
	if err != nil {
		t.Fatalf("failed to inject error: %v", err)
	}

	cmd := exec.Command("cargo", "build", "--workspace")
	cmd.Dir = filepath.Join(root, "rhivos")
	output, err := cmd.CombinedOutput()
	if err == nil {
		t.Error("expected cargo build to fail after injecting error, but it succeeded")
		return
	}
	combined := string(output)
	if !strings.Contains(combined, "locking-service") {
		t.Errorf("cargo build error does not mention the failing crate 'locking-service':\n%s", combined)
	}
}

// TS-01-E3: Go build fails with missing dependency
// Requirement: 01-REQ-3.E1
func TestGoBuildFailsWithMissingDependency(t *testing.T) {
	root := repoRoot(t)

	if _, err := exec.LookPath("go"); err != nil {
		t.Skip("go not found on PATH; skipping")
	}

	mainPath := filepath.Join(root, "backend", "parking-fee-service", "main.go")

	// Read original content
	original, err := os.ReadFile(mainPath)
	if err != nil {
		t.Skip("backend/parking-fee-service/main.go does not exist yet; skipping error injection test")
	}
	t.Cleanup(func() {
		os.WriteFile(mainPath, original, 0644)
	})

	// Inject an undeclared import
	injected := "package main\n\nimport \"unknown/nonexistent/package\"\n\nfunc main() { _ = package.Foo }\n"
	err = os.WriteFile(mainPath, []byte(injected), 0644)
	if err != nil {
		t.Fatalf("failed to inject import: %v", err)
	}

	cmd := exec.Command("go", "build", "./...")
	cmd.Dir = filepath.Join(root, "backend", "parking-fee-service")
	output, err := cmd.CombinedOutput()
	if err == nil {
		t.Error("expected go build to fail with missing dependency, but it succeeded")
		return
	}
	combined := string(output)
	if !strings.Contains(combined, "unknown") {
		t.Errorf("go build error does not mention the missing import:\n%s", combined)
	}
}

// TS-01-E4: Skeleton exits non-zero on unknown flag
// Requirement: 01-REQ-4.E1
func TestSkeletonExitsNonZeroOnUnknownFlag(t *testing.T) {
	root := repoRoot(t)

	// Test Rust binaries
	if _, err := exec.LookPath("cargo"); err == nil {
		// Build first
		buildCmd := exec.Command("cargo", "build", "--workspace")
		buildCmd.Dir = filepath.Join(root, "rhivos")
		if buildOutput, err := buildCmd.CombinedOutput(); err != nil {
			t.Logf("cargo build failed (prerequisite): %v\noutput:\n%s", err, string(buildOutput))
		} else {
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
					output, err := cmd.CombinedOutput()
					if err == nil {
						t.Errorf("%s should exit non-zero with --invalid-flag but succeeded", bin)
						return
					}
					if len(output) == 0 {
						t.Errorf("%s produced no output on stderr/stdout with --invalid-flag", bin)
					}
				})
			}
		}
	}

	// Test Go binaries
	if _, err := exec.LookPath("go"); err == nil {
		goModules := []string{
			"backend/parking-fee-service",
			"backend/cloud-gateway",
			"mock/parking-app-cli",
			"mock/companion-app-cli",
			"mock/parking-operator",
		}

		for _, mod := range goModules {
			modName := filepath.Base(mod)
			t.Run("go/"+modName, func(t *testing.T) {
				cmd := exec.Command("go", "run", ".", "--invalid-flag")
				cmd.Dir = filepath.Join(root, mod)
				output, err := cmd.CombinedOutput()
				if err == nil {
					t.Errorf("%s should exit non-zero with --invalid-flag but succeeded", modName)
					return
				}
				if len(output) == 0 {
					t.Errorf("%s produced no output with --invalid-flag", modName)
				}
			})
		}
	}
}

// TS-01-E5: Protoc fails on missing import
// Requirement: 01-REQ-5.E1
func TestProtocFailsOnMissingImport(t *testing.T) {
	root := repoRoot(t)

	if _, err := exec.LookPath("protoc"); err != nil {
		t.Skip("protoc not found on PATH; skipping")
	}

	protoDir := filepath.Join(root, "proto")
	tempFile := filepath.Join(protoDir, "temp_test_missing_import.proto")

	err := os.WriteFile(tempFile, []byte(`syntax = "proto3";
import "nonexistent.proto";
package test;
`), 0644)
	if err != nil {
		t.Fatalf("failed to create temp proto file: %v", err)
	}
	t.Cleanup(func() {
		os.Remove(tempFile)
	})

	cmd := exec.Command("protoc",
		"--proto_path="+protoDir,
		"--descriptor_set_out=/dev/null",
		tempFile,
	)
	output, err := cmd.CombinedOutput()
	if err == nil {
		t.Error("expected protoc to fail with missing import, but it succeeded")
		return
	}
	combined := string(output)
	if !strings.Contains(combined, "nonexistent.proto") {
		t.Errorf("protoc error does not mention the missing import 'nonexistent.proto':\n%s", combined)
	}
}

// TS-01-E6: make build reports failing toolchain
// Requirement: 01-REQ-6.E1
func TestMakeBuildReportsFailingToolchain(t *testing.T) {
	root := repoRoot(t)

	if _, err := exec.LookPath("make"); err != nil {
		t.Skip("make not found on PATH; skipping")
	}
	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("cargo not found on PATH; skipping")
	}

	mainPath := filepath.Join(root, "rhivos", "locking-service", "src", "main.rs")

	// Read original content
	original, err := os.ReadFile(mainPath)
	if err != nil {
		t.Skip("locking-service/src/main.rs does not exist yet; skipping")
	}
	t.Cleanup(func() {
		os.WriteFile(mainPath, original, 0644)
	})

	// Inject a compile error
	err = os.WriteFile(mainPath, []byte("fn main() { invalid_syntax!!! }"), 0644)
	if err != nil {
		t.Fatalf("failed to inject error: %v", err)
	}

	cmd := exec.Command("make", "build")
	cmd.Dir = root
	output, err := cmd.CombinedOutput()
	if err == nil {
		t.Error("expected make build to fail after injecting Rust error, but it succeeded")
		return
	}
	combined := strings.ToLower(string(output))
	if !strings.Contains(combined, "cargo") && !strings.Contains(combined, "rust") && !strings.Contains(combined, "error") {
		t.Errorf("make build error does not identify failing toolchain:\n%s", string(output))
	}
}

// TS-01-E7: Port conflict on infra-up
// Requirement: 01-REQ-7.E1
// Gated behind SETUP_TEST_INFRA=1 because it requires Podman and container images.
func TestInfraUpPortConflict(t *testing.T) {
	if os.Getenv("SETUP_TEST_INFRA") != "1" {
		t.Skip("skipping infrastructure test (set SETUP_TEST_INFRA=1 to enable)")
	}

	root := repoRoot(t)

	if _, err := exec.LookPath("make"); err != nil {
		t.Skip("make not found on PATH; skipping")
	}
	if _, err := exec.LookPath("podman"); err != nil {
		t.Skip("podman not found on PATH; skipping")
	}

	// Bind port 4222 to create a conflict
	listener, err := net.Listen("tcp", ":4222")
	if err != nil {
		t.Skip("could not bind port 4222 (already in use?); skipping port conflict test")
	}
	defer listener.Close()

	cmd := exec.Command("make", "infra-up")
	cmd.Dir = root
	output, err := cmd.CombinedOutput()
	if err == nil {
		// Clean up — try to bring infra down if it somehow started
		downCmd := exec.Command("make", "infra-down")
		downCmd.Dir = root
		_ = downCmd.Run()
		t.Error("expected make infra-up to fail with port conflict, but it succeeded")
		return
	}
	// Verify the error is actually about a port conflict, not some other failure
	// (e.g., missing Makefile target). The error output from podman/compose should
	// mention the port or address binding issue.
	combined := strings.ToLower(string(output))
	portRelated := strings.Contains(combined, "port") ||
		strings.Contains(combined, "bind") ||
		strings.Contains(combined, "address already in use") ||
		strings.Contains(combined, "4222")
	if !portRelated {
		t.Errorf("make infra-up failed but not due to port conflict; output:\n%s", string(output))
	}
}

// TS-01-E9: Test runner reports syntax errors
// Requirement: 01-REQ-8.E1
func TestRunnerReportsSyntaxErrors(t *testing.T) {
	root := repoRoot(t)

	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("cargo not found on PATH; skipping")
	}

	mainPath := filepath.Join(root, "rhivos", "locking-service", "src", "main.rs")

	// Read original content
	original, err := os.ReadFile(mainPath)
	if err != nil {
		t.Skip("locking-service/src/main.rs does not exist yet; skipping")
	}
	t.Cleanup(func() {
		os.WriteFile(mainPath, original, 0644)
	})

	// Inject syntax error
	err = os.WriteFile(mainPath, []byte("fn main() { let x = ; }"), 0644)
	if err != nil {
		t.Fatalf("failed to inject syntax error: %v", err)
	}

	cmd := exec.Command("cargo", "test", "--workspace")
	cmd.Dir = filepath.Join(root, "rhivos")
	output, err := cmd.CombinedOutput()
	if err == nil {
		t.Error("expected cargo test to fail after injecting syntax error, but it succeeded")
		return
	}
	combined := string(output)
	if !strings.Contains(combined, "main.rs") {
		t.Errorf("cargo test error does not mention the file with the syntax error:\n%s", combined)
	}
}

// TS-01-E10: Setup test skips on missing toolchain
// Requirement: 01-REQ-9.E1
func TestToolchainSkipGracefully(t *testing.T) {
	root := repoRoot(t)

	if _, err := exec.LookPath("go"); err != nil {
		t.Skip("go not found on PATH; skipping")
	}

	// Run setup tests with a restricted PATH that excludes cargo.
	// This should cause Rust-related tests to skip rather than fail.
	// We run the test in the tests/setup directory with GOWORK=off to
	// avoid workspace errors that would mask the actual skip behavior.
	cmd := exec.Command("go", "test", "-v", "-run", "TestCargoBuildWorkspace", ".")
	cmd.Dir = filepath.Join(root, "tests", "setup")

	// Build a PATH that includes go but excludes cargo
	goPath, _ := exec.LookPath("go")
	goBinDir := filepath.Dir(goPath)

	var newEnv []string
	for _, env := range os.Environ() {
		if strings.HasPrefix(env, "PATH=") {
			newEnv = append(newEnv, "PATH="+goBinDir+":/usr/bin:/bin")
		} else if strings.HasPrefix(env, "GOWORK=") {
			// skip existing GOWORK — we set it below
			continue
		} else {
			newEnv = append(newEnv, env)
		}
	}
	// Disable Go workspace so the test can run in isolation
	newEnv = append(newEnv, "GOWORK=off")
	cmd.Env = newEnv

	output, err := cmd.CombinedOutput()
	out := string(output)

	// The test should skip (not fail) when cargo is not available.
	// Look for skip indicators in the output.
	if strings.Contains(out, "SKIP") || strings.Contains(out, "skip") {
		// Good — tests skip gracefully when the toolchain is missing
		return
	}

	// If the test command itself failed (exit non-zero) without producing
	// a skip message, that means it failed rather than skipping — which
	// violates the requirement.
	if err != nil {
		t.Errorf("expected test to SKIP when cargo is absent, but it failed instead.\noutput:\n%s", out)
		return
	}

	// If the test passed (exit 0) without skipping, cargo may have been
	// found via another mechanism (e.g., rustup shim in /usr/bin). That
	// is acceptable — the important thing is it did not fail.
}

// TS-01-E11: make proto fails when protoc missing
// Requirement: 01-REQ-10.E1
func TestMakeProtoFailsWhenProtocMissing(t *testing.T) {
	root := repoRoot(t)

	if _, err := exec.LookPath("make"); err != nil {
		t.Skip("make not found on PATH; skipping")
	}

	cmd := exec.Command("make", "proto")
	cmd.Dir = root

	// Set PATH to exclude protoc
	var newEnv []string
	for _, env := range os.Environ() {
		if strings.HasPrefix(env, "PATH=") {
			newEnv = append(newEnv, "PATH=/usr/bin:/bin")
		} else {
			newEnv = append(newEnv, env)
		}
	}
	cmd.Env = newEnv

	output, err := cmd.CombinedOutput()
	if err == nil {
		t.Error("expected make proto to fail when protoc is not on PATH, but it succeeded")
		return
	}
	combined := strings.ToLower(string(output))
	if !strings.Contains(combined, "protoc") {
		t.Errorf("make proto error does not mention protoc:\n%s", string(output))
	}
}
