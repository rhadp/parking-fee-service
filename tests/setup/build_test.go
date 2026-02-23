package setup_test

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

// TS-01-13: Rust workspace builds successfully (01-REQ-3.2)
func TestBuild_RustWorkspace(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping build test in short mode")
	}
	root := repoRoot(t)
	result := execCommand(t, root, "rhivos", "cargo", "build")
	if result.ExitCode != 0 {
		t.Errorf("cargo build failed (exit %d): %s", result.ExitCode, result.Combined)
	}
}

// TS-01-14: Rust workspace tests pass (01-REQ-3.3)
func TestBuild_RustTests(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping build test in short mode")
	}
	root := repoRoot(t)
	result := execCommand(t, root, "rhivos", "cargo", "test")
	if result.ExitCode != 0 {
		t.Errorf("cargo test failed (exit %d): %s", result.ExitCode, result.Combined)
	}
}

// TS-01-18: Go backend builds successfully (01-REQ-4.2)
func TestBuild_GoBackend(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping build test in short mode")
	}
	root := repoRoot(t)
	for _, svc := range []string{"parking-fee-service", "cloud-gateway"} {
		result := execCommand(t, root, "backend/"+svc, "go", "build", "./...")
		if result.ExitCode != 0 {
			t.Errorf("go build failed for %s (exit %d): %s", svc, result.ExitCode, result.Combined)
		}
	}
}

// TS-01-19: Go backend tests pass (01-REQ-4.3)
func TestBuild_GoTests(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping build test in short mode")
	}
	root := repoRoot(t)
	for _, svc := range []string{"parking-fee-service", "cloud-gateway"} {
		result := execCommand(t, root, "backend/"+svc, "go", "test", "./...")
		if result.ExitCode != 0 {
			t.Errorf("go test failed for %s (exit %d): %s", svc, result.ExitCode, result.Combined)
		}
	}
}

// TS-01-21: PARKING_FEE_SERVICE health endpoint (01-REQ-4.5)
func TestBuild_ParkingFeeServiceHealth(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping integration test in short mode")
	}
	root := repoRoot(t)

	// Build the binary
	svcDir := filepath.Join(root, "backend", "parking-fee-service")
	buildResult := execCommand(t, root, "backend/parking-fee-service", "go", "build", "-o", "parking-fee-service", ".")
	if buildResult.ExitCode != 0 {
		t.Fatalf("failed to build parking-fee-service: %s", buildResult.Combined)
	}
	defer os.Remove(filepath.Join(svcDir, "parking-fee-service"))

	// Start the server with a test port
	cmd := exec.Command(filepath.Join(svcDir, "parking-fee-service"))
	cmd.Env = append(os.Environ(), "PORT=18080")
	cmd.Dir = svcDir
	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start parking-fee-service: %v", err)
	}
	defer func() {
		cmd.Process.Kill()
		cmd.Wait()
	}()

	// Wait for the server to be ready
	if !waitForPort(t, 18080, 10*time.Second) {
		t.Fatal("parking-fee-service did not start on port 18080 within timeout")
	}

	// Check health endpoint
	statusCode, body, err := httpGet(t, "http://localhost:18080/health")
	if err != nil {
		t.Fatalf("health check request failed: %v", err)
	}
	if statusCode != 200 {
		t.Errorf("expected status 200, got %d", statusCode)
	}
	if !strings.Contains(body, `"status"`) || !strings.Contains(body, `"ok"`) {
		t.Errorf("expected body to contain status ok, got: %s", body)
	}
}

// TS-01-22: CLOUD_GATEWAY stub entry points (01-REQ-4.6)
func TestBuild_CloudGatewayStub(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping integration test in short mode")
	}
	root := repoRoot(t)

	// Build the binary
	svcDir := filepath.Join(root, "backend", "cloud-gateway")
	buildResult := execCommand(t, root, "backend/cloud-gateway", "go", "build", "-o", "cloud-gateway", ".")
	if buildResult.ExitCode != 0 {
		t.Fatalf("failed to build cloud-gateway: %s", buildResult.Combined)
	}
	defer os.Remove(filepath.Join(svcDir, "cloud-gateway"))

	// Start the server with a test port
	cmd := exec.Command(filepath.Join(svcDir, "cloud-gateway"))
	cmd.Env = append(os.Environ(), "PORT=18081")
	cmd.Dir = svcDir

	// Capture stdout to check for MQTT message
	var stdout strings.Builder
	cmd.Stdout = &stdout

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start cloud-gateway: %v", err)
	}
	defer func() {
		cmd.Process.Kill()
		cmd.Wait()
	}()

	// Wait for the server to be ready
	if !waitForPort(t, 18081, 10*time.Second) {
		t.Fatal("cloud-gateway did not start on port 18081 within timeout")
	}

	// Check health endpoint
	statusCode, _, err := httpGet(t, "http://localhost:18081/health")
	if err != nil {
		t.Fatalf("health check request failed: %v", err)
	}
	if statusCode != 200 {
		t.Errorf("expected status 200, got %d", statusCode)
	}

	// Check startup output mentions MQTT
	if !strings.Contains(stdout.String(), "MQTT") {
		t.Errorf("expected startup output to mention MQTT, got: %q", stdout.String())
	}
}

// TS-01-25: Mock CLI apps produce single binary (01-REQ-5.3)
func TestMockCLI_BuildBinary(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping build test in short mode")
	}
	root := repoRoot(t)

	for _, app := range []string{"parking-app-cli", "companion-app-cli"} {
		appDir := filepath.Join(root, "mock", app)
		result := execCommand(t, root, "mock/"+app, "go", "build", "-o", app, ".")
		if result.ExitCode != 0 {
			t.Errorf("go build failed for %s (exit %d): %s", app, result.ExitCode, result.Combined)
			continue
		}

		binaryPath := filepath.Join(appDir, app)
		info, err := os.Stat(binaryPath)
		if err != nil {
			t.Errorf("expected binary %s to exist: %v", binaryPath, err)
			continue
		}

		// Check it's executable
		if info.Mode()&0111 == 0 {
			t.Errorf("expected binary %s to be executable", binaryPath)
		}

		// Clean up
		os.Remove(binaryPath)
	}
}

// TS-01-26: Mock CLI apps show help (01-REQ-5.4)
func TestMockCLI_ShowHelp(t *testing.T) {
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

		// Run without arguments
		result := execCommand(t, root, "mock/"+app, "./"+app)
		if result.ExitCode != 0 {
			t.Errorf("expected %s to exit 0 with no args, got exit %d", app, result.ExitCode)
		}
		if len(result.Stdout) == 0 && len(result.Combined) == 0 {
			t.Errorf("expected %s to produce non-empty output, got empty", app)
		}
		// Check for usage/help text
		combined := result.Stdout + result.Stderr
		if !strings.Contains(combined, "Available Commands") && !strings.Contains(combined, "Usage") {
			t.Errorf("expected %s output to contain 'Available Commands' or 'Usage', got: %s", app, combined)
		}
	}
}

// TS-01-28: Top-level Makefile exists (01-REQ-6.1)
func TestMake_MakefileExists(t *testing.T) {
	root := repoRoot(t)
	assertFileExists(t, root, "Makefile")
}

// TS-01-29: make build succeeds (01-REQ-6.2)
func TestMake_Build(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping make build test in short mode")
	}
	root := repoRoot(t)
	result := execCommand(t, root, ".", "make", "build")
	if result.ExitCode != 0 {
		t.Errorf("make build failed (exit %d): %s", result.ExitCode, result.Combined)
	}
}

// TS-01-30: make test succeeds (01-REQ-6.3)
func TestMake_Test(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping make test in short mode")
	}
	root := repoRoot(t)
	result := execCommand(t, root, ".", "make", "test")
	if result.ExitCode != 0 {
		t.Errorf("make test failed (exit %d): %s", result.ExitCode, result.Combined)
	}
}

// TS-01-31: make lint succeeds (01-REQ-6.4)
func TestMake_Lint(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping make lint test in short mode")
	}
	root := repoRoot(t)
	result := execCommand(t, root, ".", "make", "lint")
	if result.ExitCode != 0 {
		t.Errorf("make lint failed (exit %d): %s", result.ExitCode, result.Combined)
	}
}

// TS-01-32: make proto generates Go code (01-REQ-6.5)
func TestMake_Proto(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping make proto test in short mode")
	}
	root := repoRoot(t)

	// Remove generated files first
	for _, pkg := range []string{"commonpb", "updateservicepb", "parkingadaptorpb"} {
		dir := filepath.Join(root, "gen", "go", pkg)
		files, _ := filepath.Glob(filepath.Join(dir, "*.pb.go"))
		for _, f := range files {
			os.Remove(f)
		}
	}

	// Run make proto
	result := execCommand(t, root, ".", "make", "proto")
	if result.ExitCode != 0 {
		t.Errorf("make proto failed (exit %d): %s", result.ExitCode, result.Combined)
	}

	// Verify generated files are restored
	for _, pkg := range []string{"commonpb", "updateservicepb", "parkingadaptorpb"} {
		files := globFiles(t, root, fmt.Sprintf("gen/go/%s/*.pb.go", pkg))
		if len(files) < 1 {
			t.Errorf("expected at least 1 .pb.go file in gen/go/%s/ after make proto, found %d", pkg, len(files))
		}
	}
}

// TS-01-33: make clean removes artifacts (01-REQ-6.6)
func TestMake_Clean(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping make clean test in short mode")
	}
	root := repoRoot(t)

	// Build first to create artifacts
	execCommand(t, root, ".", "make", "build")

	// Clean
	result := execCommand(t, root, ".", "make", "clean")
	if result.ExitCode != 0 {
		t.Errorf("make clean failed (exit %d): %s", result.ExitCode, result.Combined)
	}

	// Verify artifacts are gone
	targetDir := filepath.Join(root, "rhivos", "target")
	if _, err := os.Stat(targetDir); err == nil {
		t.Errorf("expected rhivos/target/ to be removed after make clean")
	}

	for _, app := range []string{"parking-app-cli", "companion-app-cli"} {
		binary := filepath.Join(root, "mock", app, app)
		if _, err := os.Stat(binary); err == nil {
			t.Errorf("expected mock/%s/%s binary to be removed after make clean", app, app)
		}
	}
}

// TS-01-39: Rust placeholder tests (01-REQ-8.1)
func TestBuild_RustPlaceholderTests(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping build test in short mode")
	}
	root := repoRoot(t)

	result := execCommand(t, root, "rhivos", "cargo", "test")
	if result.ExitCode != 0 {
		t.Fatalf("cargo test failed (exit %d): %s", result.ExitCode, result.Combined)
	}

	// Each crate should report at least one test
	crates := []string{"locking-service", "cloud-gateway-client", "update-service", "parking-operator-adaptor"}
	// cargo test output uses the crate name with hyphens replaced by underscores
	for _, crate := range crates {
		crateName := strings.ReplaceAll(crate, "-", "_")
		if !strings.Contains(result.Combined, crateName) {
			t.Errorf("cargo test output does not mention crate %s (looking for %s)", crate, crateName)
		}
	}
}

// TS-01-40: Go placeholder tests (01-REQ-8.2)
func TestBuild_GoPlaceholderTests(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping build test in short mode")
	}
	root := repoRoot(t)

	modules := []string{
		"backend/parking-fee-service",
		"backend/cloud-gateway",
		"mock/parking-app-cli",
		"mock/companion-app-cli",
	}

	for _, mod := range modules {
		result := execCommand(t, root, mod, "go", "test", "-v", "./...")
		if result.ExitCode != 0 {
			t.Errorf("go test failed for %s (exit %d): %s", mod, result.ExitCode, result.Combined)
			continue
		}
		if !strings.Contains(result.Combined, "PASS") {
			t.Errorf("expected go test output for %s to contain PASS, got: %s", mod, result.Combined)
		}
	}
}

// TS-01-41: Unit tests pass without infrastructure (01-REQ-8.3)
func TestBuild_TestsWithoutInfra(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping integration test in short mode")
	}
	root := repoRoot(t)

	// Ensure infra is down
	execCommand(t, root, ".", "make", "infra-down")

	// Run all tests
	result := execCommand(t, root, ".", "make", "test")
	if result.ExitCode != 0 {
		t.Errorf("make test failed without infrastructure (exit %d): %s", result.ExitCode, result.Combined)
	}
}
