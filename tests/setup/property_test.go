package setup_test

import (
	"bufio"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

// TS-01-P1: Build Completeness (Property 1)
// For any component in the repository, building it produces zero errors.
func TestProperty_BuildCompleteness(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping property test in short mode")
	}
	root := repoRoot(t)

	// Rust workspace
	result := execCommand(t, root, "rhivos", "cargo", "build")
	if result.ExitCode != 0 {
		t.Errorf("cargo build failed (exit %d): %s", result.ExitCode, result.Combined)
	}

	// Go modules
	goModules := []string{
		"backend/parking-fee-service",
		"backend/cloud-gateway",
		"mock/parking-app-cli",
		"mock/companion-app-cli",
	}
	for _, mod := range goModules {
		result := execCommand(t, root, mod, "go", "build", "./...")
		if result.ExitCode != 0 {
			t.Errorf("go build failed for %s (exit %d): %s", mod, result.ExitCode, result.Combined)
		}
	}
}

// TS-01-P2: Proto-to-Code Consistency (Property 2)
// For any proto file defining a service, the set of RPC method names in the
// generated Rust code equals the set in the generated Go code.
func TestProperty_ProtoConsistency(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping property test in short mode")
	}
	root := repoRoot(t)

	// Map proto files to their expected RPCs and generated code locations
	type protoCheck struct {
		protoFile    string
		rpcs         []string
		goPackageDir string
		rustCrate    string
	}

	checks := []protoCheck{
		{
			protoFile:    "proto/update_service.proto",
			rpcs:         []string{"InstallAdapter", "WatchAdapterStates", "ListAdapters", "RemoveAdapter", "GetAdapterStatus"},
			goPackageDir: "gen/go/updateservicepb",
			rustCrate:    "rhivos/update-service",
		},
		{
			protoFile:    "proto/parking_adaptor.proto",
			rpcs:         []string{"StartSession", "StopSession", "GetStatus", "GetRate"},
			goPackageDir: "gen/go/parkingadaptorpb",
			rustCrate:    "rhivos/parking-operator-adaptor",
		},
	}

	for _, check := range checks {
		// Verify proto file defines the RPCs
		protoContent := readFile(t, root, check.protoFile)
		for _, rpc := range check.rpcs {
			if !strings.Contains(protoContent, "rpc "+rpc) {
				t.Errorf("proto file %s missing rpc %s", check.protoFile, rpc)
			}
		}

		// Verify Go generated code contains the RPCs
		goDir := filepath.Join(root, check.goPackageDir)
		goFiles, _ := filepath.Glob(filepath.Join(goDir, "*_grpc.pb.go"))
		if len(goFiles) == 0 {
			t.Errorf("no *_grpc.pb.go files found in %s", check.goPackageDir)
			continue
		}

		for _, goFile := range goFiles {
			data, err := os.ReadFile(goFile)
			if err != nil {
				t.Errorf("could not read %s: %v", goFile, err)
				continue
			}
			goContent := string(data)
			for _, rpc := range check.rpcs {
				if !strings.Contains(goContent, rpc) {
					t.Errorf("Go generated file %s missing RPC method %s", goFile, rpc)
				}
			}
		}

		// Verify Rust crate references the proto file
		buildRS := check.rustCrate + "/build.rs"
		assertFileExists(t, root, buildRS)
		assertFileContains(t, root, buildRS, ".proto")
	}
}

// TS-01-P3: Test Isolation (Property 3)
// For any unit test, running without infrastructure produces no failures.
func TestProperty_TestIsolation(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping property test in short mode")
	}
	root := repoRoot(t)

	// Ensure infrastructure is down
	execCommand(t, root, ".", "make", "infra-down")

	// Run Rust tests
	rustResult := execCommand(t, root, "rhivos", "cargo", "test")
	if rustResult.ExitCode != 0 {
		t.Errorf("cargo test failed without infrastructure (exit %d): %s", rustResult.ExitCode, rustResult.Combined)
	}

	// Run Go tests
	goModules := []string{
		"backend/parking-fee-service",
		"backend/cloud-gateway",
		"mock/parking-app-cli",
		"mock/companion-app-cli",
	}
	for _, mod := range goModules {
		result := execCommand(t, root, mod, "go", "test", "./...")
		if result.ExitCode != 0 {
			t.Errorf("go test failed for %s without infrastructure (exit %d): %s", mod, result.ExitCode, result.Combined)
		}
	}
}

// TS-01-P4: Mock CLI Usability (Property 4)
// For any mock CLI app, invoking with --help exits 0 with non-empty output.
func TestProperty_MockCLIUsability(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping property test in short mode")
	}
	root := repoRoot(t)

	for _, app := range []string{"parking-app-cli", "companion-app-cli"} {
		appDir := filepath.Join(root, "mock", app)
		// Build
		buildResult := execCommand(t, root, "mock/"+app, "go", "build", "-o", app, ".")
		if buildResult.ExitCode != 0 {
			t.Errorf("go build failed for %s: %s", app, buildResult.Combined)
			continue
		}
		defer os.Remove(filepath.Join(appDir, app))

		// Run with --help
		result := execCommand(t, root, "mock/"+app, "./"+app, "--help")
		if result.ExitCode != 0 {
			t.Errorf("expected %s --help to exit 0, got exit %d", app, result.ExitCode)
		}
		if len(result.Stdout) == 0 {
			t.Errorf("expected %s --help to produce non-empty stdout", app)
		}
	}
}

// TS-01-P5: Infrastructure Lifecycle Idempotency (Property 5)
// For any sequence of infra-up/down operations, the system reaches a consistent state.
func TestProperty_InfraIdempotency(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping infrastructure property test in short mode")
	}

	// Skip if no container runtime available
	if _, err := lookPath("podman"); err != nil {
		if _, err := lookPath("docker"); err != nil {
			t.Skip("no container runtime (podman/docker) available, skipping infra test")
		}
	}

	root := repoRoot(t)

	// Ensure clean starting state
	execCommand(t, root, ".", "make", "infra-down")
	time.Sleep(2 * time.Second)

	sequences := []struct {
		name string
		ops  []string
	}{
		{"up", []string{"up"}},
		{"up-up", []string{"up", "up"}},
		{"down-up", []string{"down", "up"}},
		{"up-down-up", []string{"up", "down", "up"}},
	}

	for _, seq := range sequences {
		t.Run(seq.name, func(t *testing.T) {
			for _, op := range seq.ops {
				execCommand(t, root, ".", "make", "infra-"+op)
				time.Sleep(3 * time.Second)
			}

			lastOp := seq.ops[len(seq.ops)-1]
			if lastOp == "up" {
				if !waitForPort(t, 1883, 30*time.Second) {
					t.Errorf("sequence %s: MQTT port 1883 not reachable", seq.name)
				}
				if !waitForPort(t, 55555, 30*time.Second) {
					t.Errorf("sequence %s: Kuksa port 55555 not reachable", seq.name)
				}
			} else {
				time.Sleep(2 * time.Second)
				if portIsOpen(t, 1883) {
					t.Errorf("sequence %s: MQTT port 1883 still open", seq.name)
				}
				if portIsOpen(t, 55555) {
					t.Errorf("sequence %s: Kuksa port 55555 still open", seq.name)
				}
			}
		})
	}

	// Final cleanup
	execCommand(t, root, ".", "make", "infra-down")
}

// TS-01-P6: Clean Build Reproducibility (Property 6)
// For any component, clean then build produces the same successful result.
func TestProperty_CleanBuildReproducibility(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping property test in short mode")
	}
	root := repoRoot(t)

	// Clean and build
	cleanResult := execCommand(t, root, ".", "make", "clean")
	if cleanResult.ExitCode != 0 {
		t.Fatalf("make clean failed (exit %d): %s", cleanResult.ExitCode, cleanResult.Combined)
	}

	buildResult := execCommand(t, root, ".", "make", "build")
	if buildResult.ExitCode != 0 {
		t.Errorf("make build after clean failed (exit %d): %s", buildResult.ExitCode, buildResult.Combined)
	}

	// Clean and build again to verify reproducibility
	cleanResult2 := execCommand(t, root, ".", "make", "clean")
	if cleanResult2.ExitCode != 0 {
		t.Fatalf("second make clean failed (exit %d): %s", cleanResult2.ExitCode, cleanResult2.Combined)
	}

	buildResult2 := execCommand(t, root, ".", "make", "build")
	if buildResult2.ExitCode != 0 {
		t.Errorf("second make build after clean failed (exit %d): %s", buildResult2.ExitCode, buildResult2.Combined)
	}
}

// TS-01-P7: Toolchain Detection (Property 7)
// For any missing required tool, the build system names it in the error message.
func TestProperty_ToolchainDetection(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping property test in short mode")
	}
	root := repoRoot(t)

	tools := []struct {
		name   string
		target string
	}{
		{"rustc", "build"},
		{"go", "build"},
		{"protoc", "proto"},
	}

	for _, tool := range tools {
		t.Run(tool.name, func(t *testing.T) {
			restrictedPath := pathWithout(t, tool.name)
			env := envWithPath(restrictedPath)

			result := execCommandWithEnv(t, root, ".", env, "make", tool.target)
			if result.ExitCode == 0 {
				t.Errorf("expected make %s to fail without %s, but it succeeded", tool.target, tool.name)
			}
			if !strings.Contains(result.Combined, tool.name) {
				t.Errorf("expected error output to contain '%s', got: %s", tool.name, result.Combined)
			}
		})
	}
}

// extractProtoRPCs extracts RPC method names from a proto file.
func extractProtoRPCs(t *testing.T, filePath string) []string {
	t.Helper()
	f, err := os.Open(filePath)
	if err != nil {
		t.Fatalf("could not open %s: %v", filePath, err)
	}
	defer f.Close()

	var rpcs []string
	scanner := bufio.NewScanner(f)
	for scanner.Scan() {
		line := strings.TrimSpace(scanner.Text())
		if strings.HasPrefix(line, "rpc ") {
			// Extract method name: "rpc MethodName(...)" -> "MethodName"
			parts := strings.Fields(line)
			if len(parts) >= 2 {
				name := strings.TrimSuffix(parts[1], "(")
				rpcs = append(rpcs, name)
			}
		}
	}
	return rpcs
}
