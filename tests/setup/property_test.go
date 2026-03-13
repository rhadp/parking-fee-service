package setup

import (
	"context"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

// TS-01-P1: Rust Workspace Completeness
// Property 1: For every Rust component, it is a workspace member and builds.
// Validates: 01-REQ-2.1, 01-REQ-2.2, 01-REQ-4.3
func TestPropertyRustCompleteness(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping property test in short mode")
	}

	root := repoRoot(t)

	cargoContent, err := os.ReadFile(filepath.Join(root, "rhivos", "Cargo.toml"))
	if err != nil {
		t.Fatalf("failed to read rhivos/Cargo.toml: %v", err)
	}

	components := []string{"locking-service", "cloud-gateway-client", "update-service", "parking-operator-adaptor", "mock-sensors"}
	for _, component := range components {
		t.Run(component, func(t *testing.T) {
			if !strings.Contains(string(cargoContent), component) {
				t.Errorf("component %s not listed as workspace member", component)
			}

			cmd := exec.Command("cargo", "build", "-p", component)
			cmd.Dir = filepath.Join(root, "rhivos")
			output, err := cmd.CombinedOutput()
			if err != nil {
				t.Errorf("cargo build -p %s failed: %v\n%s", component, err, string(output))
			}
		})
	}
}

// TS-01-P2: Go Workspace Completeness
// Property 2: For every Go component, it builds.
// Validates: 01-REQ-3.1, 01-REQ-3.2, 01-REQ-4.4
func TestPropertyGoCompleteness(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping property test in short mode")
	}

	root := repoRoot(t)

	paths := []string{
		"backend/parking-fee-service",
		"backend/cloud-gateway",
		"mock/parking-app-cli",
		"mock/companion-app-cli",
		"mock/parking-operator",
	}

	for _, pkg := range paths {
		t.Run(pkg, func(t *testing.T) {
			cmd := exec.Command("go", "build", "./"+pkg)
			cmd.Dir = root
			output, err := cmd.CombinedOutput()
			if err != nil {
				t.Errorf("go build ./%s failed: %v\n%s", pkg, err, string(output))
			}
		})
	}
}

// TS-01-P3: Skeleton Exit Behavior
// Property 3: For any skeleton binary with any arguments, exit code is 0.
// Validates: 01-REQ-4.1, 01-REQ-4.2, 01-REQ-4.E1
func TestPropertySkeletonExit(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping property test in short mode")
	}

	root := repoRoot(t)

	// Build Rust binaries
	buildCmd := exec.Command("cargo", "build")
	buildCmd.Dir = filepath.Join(root, "rhivos")
	if output, err := buildCmd.CombinedOutput(); err != nil {
		t.Fatalf("cargo build failed: %v\n%s", err, string(output))
	}

	rustBinaries := []string{
		"locking-service",
		"cloud-gateway-client",
		"update-service",
		"parking-operator-adaptor",
		"location-sensor",
		"speed-sensor",
		"door-sensor",
	}

	argSets := [][]string{
		{},
		{"--help"},
		{"--unknown"},
		{"foo", "bar"},
	}

	for _, bin := range rustBinaries {
		binPath := filepath.Join(root, "rhivos", "target", "debug", bin)
		for _, args := range argSets {
			testName := bin + "/" + strings.Join(append([]string{"no-args"}, args...), "_")
			t.Run(testName, func(t *testing.T) {
				ctx, cancel := context.WithTimeout(context.Background(), skeletonTimeout)
				defer cancel()
				cmd := exec.CommandContext(ctx, binPath, args...)
				_, err := cmd.CombinedOutput()
				if err != nil {
					t.Errorf("binary %s with args %v did not exit 0: %v", bin, args, err)
				}
			})
		}
	}

	// Test Go binaries
	goBinaries := map[string]string{
		"parking-fee-service": filepath.Join(root, "backend", "parking-fee-service"),
		"cloud-gateway":      filepath.Join(root, "backend", "cloud-gateway"),
		"parking-app-cli":    filepath.Join(root, "mock", "parking-app-cli"),
		"companion-app-cli":  filepath.Join(root, "mock", "companion-app-cli"),
		"parking-operator":   filepath.Join(root, "mock", "parking-operator"),
	}

	for name, dir := range goBinaries {
		goBuild := exec.Command("go", "build", "-o", name, ".")
		goBuild.Dir = dir
		if output, err := goBuild.CombinedOutput(); err != nil {
			t.Fatalf("go build %s failed: %v\n%s", name, err, string(output))
		}
		defer func(d, n string) {
			_ = os.Remove(filepath.Join(d, n))
		}(dir, name)

		for _, args := range argSets {
			testName := name + "/" + strings.Join(append([]string{"no-args"}, args...), "_")
			t.Run(testName, func(t *testing.T) {
				ctx, cancel := context.WithTimeout(context.Background(), skeletonTimeout)
				defer cancel()
				cmd := exec.CommandContext(ctx, filepath.Join(dir, name), args...)
				_, err := cmd.CombinedOutput()
				if err != nil {
					t.Errorf("binary %s with args %v did not exit 0: %v", name, args, err)
				}
			})
		}
	}
}

// TS-01-P4: Proto Generation Idempotency
// Property 4: Running make proto twice produces identical output.
// Validates: 01-REQ-5.5, 01-REQ-5.6
func TestPropertyProtoIdempotency(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping property test in short mode")
	}

	if _, err := exec.LookPath("protoc"); err != nil {
		t.Skip("skipping: protoc not installed")
	}

	root := repoRoot(t)

	// First run
	cmd1 := exec.Command("make", "proto")
	cmd1.Dir = root
	if output, err := cmd1.CombinedOutput(); err != nil {
		t.Fatalf("first make proto failed: %v\n%s", err, string(output))
	}

	checksums1 := checksumDir(t, filepath.Join(root, "gen", "go"))

	// Second run
	cmd2 := exec.Command("make", "proto")
	cmd2.Dir = root
	if output, err := cmd2.CombinedOutput(); err != nil {
		t.Fatalf("second make proto failed: %v\n%s", err, string(output))
	}

	checksums2 := checksumDir(t, filepath.Join(root, "gen", "go"))

	// Compare
	for path, hash1 := range checksums1 {
		hash2, ok := checksums2[path]
		if !ok {
			t.Errorf("file %s exists after first run but not second", path)
			continue
		}
		if hash1 != hash2 {
			t.Errorf("file %s changed between make proto runs", path)
		}
	}
	for path := range checksums2 {
		if _, ok := checksums1[path]; !ok {
			t.Errorf("file %s exists after second run but not first", path)
		}
	}
}

// checksumDir computes a map of relative path -> file content hash for all files in a directory.
func checksumDir(t *testing.T, dir string) map[string]string {
	t.Helper()
	result := make(map[string]string)

	if !isDir(dir) {
		return result
	}

	err := filepath.Walk(dir, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return err
		}
		if info.IsDir() {
			return nil
		}
		rel, _ := filepath.Rel(dir, path)
		content, err := os.ReadFile(path)
		if err != nil {
			return err
		}
		result[rel] = string(content)
		return nil
	})
	if err != nil {
		t.Fatalf("failed to walk directory %s: %v", dir, err)
	}
	return result
}

// TS-01-P5: Infrastructure Idempotency
// Property 5: Multiple infra-up calls result in exactly one container per service.
// Validates: 01-REQ-6.2, 01-REQ-6.E2
func TestPropertyInfraIdempotency(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping infrastructure property test in short mode")
	}

	if _, err := exec.LookPath("podman"); err != nil {
		t.Skip("skipping: podman not available")
	}

	root := repoRoot(t)

	t.Cleanup(func() {
		cmd := exec.Command("make", "infra-down")
		cmd.Dir = root
		_ = cmd.Run()
	})

	for _, n := range []int{1, 2, 3} {
		t.Run(strings.Repeat("up", n), func(t *testing.T) {
			// Clean slate
			downCmd := exec.Command("make", "infra-down")
			downCmd.Dir = root
			_ = downCmd.Run()

			for i := 0; i < n; i++ {
				cmd := exec.Command("make", "infra-up")
				cmd.Dir = root
				if output, err := cmd.CombinedOutput(); err != nil {
					t.Fatalf("make infra-up (call %d) failed: %v\n%s", i+1, err, string(output))
				}
			}

			psCmd := exec.Command("podman", "ps", "--format", "{{.Names}}")
			psOutput, err := psCmd.CombinedOutput()
			if err != nil {
				t.Fatalf("podman ps failed: %v", err)
			}

			lines := strings.Split(strings.TrimSpace(string(psOutput)), "\n")
			natsCount := 0
			dbCount := 0
			for _, line := range lines {
				if strings.Contains(line, "nats") {
					natsCount++
				}
				if strings.Contains(line, "databroker") || strings.Contains(line, "kuksa") {
					dbCount++
				}
			}

			if natsCount != 1 {
				t.Errorf("after %d infra-up calls: expected 1 nats, got %d", n, natsCount)
			}
			if dbCount != 1 {
				t.Errorf("after %d infra-up calls: expected 1 databroker, got %d", n, dbCount)
			}
		})
	}
}

// TS-01-P6: Directory Structure Completeness
// Property 6: Every directory in the PRD exists and contains at least one non-.gitkeep file.
// Validates: 01-REQ-1.1 through 01-REQ-1.6
func TestPropertyDirCompleteness(t *testing.T) {
	root := repoRoot(t)

	allDirs := []string{
		"rhivos",
		"rhivos/locking-service",
		"rhivos/cloud-gateway-client",
		"rhivos/update-service",
		"rhivos/parking-operator-adaptor",
		"rhivos/mock-sensors",
		"backend",
		"backend/parking-fee-service",
		"backend/cloud-gateway",
		"android",
		"mobile",
		"mock",
		"mock/parking-app-cli",
		"mock/companion-app-cli",
		"mock/parking-operator",
		"proto",
		"deployments",
		"tests",
		"tests/setup",
	}

	for _, dir := range allDirs {
		t.Run(dir, func(t *testing.T) {
			path := filepath.Join(root, dir)
			if !isDir(path) {
				t.Fatalf("directory %s does not exist", dir)
			}

			entries, err := os.ReadDir(path)
			if err != nil {
				t.Fatalf("failed to read directory %s: %v", dir, err)
			}

			nonGitkeep := 0
			for _, entry := range entries {
				if entry.Name() != ".gitkeep" {
					nonGitkeep++
				}
			}
			if nonGitkeep == 0 {
				t.Errorf("directory %s contains no files (excluding .gitkeep)", dir)
			}
		})
	}
}

// TS-01-P7: Test Runner Discovery
// Property 7: Every component has at least one test.
// Validates: 01-REQ-8.1, 01-REQ-8.2, 01-REQ-8.4
func TestPropertyTestDiscovery(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping property test in short mode")
	}

	root := repoRoot(t)

	// Rust crates
	t.Run("rust", func(t *testing.T) {
		cmd := exec.Command("cargo", "test")
		cmd.Dir = filepath.Join(root, "rhivos")
		output, err := cmd.CombinedOutput()
		if err != nil {
			t.Fatalf("cargo test failed: %v\n%s", err, string(output))
		}
		if !strings.Contains(string(output), "test result: ok") {
			t.Error("cargo test did not produce 'test result: ok'")
		}
	})

	// Go modules
	t.Run("go", func(t *testing.T) {
		ctx, cancel := context.WithTimeout(context.Background(), 2*time.Minute)
		defer cancel()
		cmd := exec.CommandContext(ctx, "go", "test", "./...")
		cmd.Dir = root
		output, err := cmd.CombinedOutput()
		if err != nil {
			t.Fatalf("go test ./... failed: %v\n%s", err, string(output))
		}
		if !strings.Contains(string(output), "ok") {
			t.Error("go test did not produce 'ok'")
		}
	})
}

// TS-01-P8: Proto Service Completeness
// Property 8: Every gRPC service and its RPCs are defined in proto files.
// Validates: 01-REQ-5.2, 01-REQ-5.3, 01-REQ-5.4
func TestPropertyProtoServiceCompleteness(t *testing.T) {
	root := repoRoot(t)

	expected := map[string]struct {
		file string
		rpcs []string
	}{
		"UpdateService": {
			file: "update_service.proto",
			rpcs: []string{"InstallAdapter", "WatchAdapterStates", "ListAdapters", "RemoveAdapter", "GetAdapterStatus"},
		},
		"ParkingAdaptor": {
			file: "parking_adaptor.proto",
			rpcs: []string{"StartSession", "StopSession", "GetStatus", "GetRate"},
		},
	}

	for service, spec := range expected {
		t.Run(service, func(t *testing.T) {
			path := filepath.Join(root, "proto", spec.file)
			content, err := os.ReadFile(path)
			if err != nil {
				t.Fatalf("failed to read proto/%s: %v", spec.file, err)
			}

			text := string(content)
			if !strings.Contains(text, "service "+service) {
				t.Errorf("proto file %s missing 'service %s'", spec.file, service)
			}

			for _, rpc := range spec.rpcs {
				if !strings.Contains(text, "rpc "+rpc) {
					t.Errorf("proto file %s missing 'rpc %s'", spec.file, rpc)
				}
			}
		})
	}
}
