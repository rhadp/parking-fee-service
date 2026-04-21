package setup

import (
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
)

// TestRustSkeletonBinaryPrintsVersion verifies each Rust skeleton binary prints
// a version string containing its name to stdout and exits with code 0.
// Test Spec: TS-01-13
// Requirements: 01-REQ-4.1, 01-REQ-4.4
func TestRustSkeletonBinaryPrintsVersion(t *testing.T) {
	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("cargo not found on PATH — skipping Rust binary tests")
	}

	root := findRepoRoot(t)

	// Ensure binaries are built.
	cmd := exec.Command("cargo", "build", "--workspace")
	cmd.Dir = filepath.Join(root, "rhivos")
	if out, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("cargo build failed:\n%s\n%v", out, err)
	}

	binaries := []struct {
		name      string
		component string // substring expected in stdout
	}{
		{"locking-service", "locking-service"},
		{"cloud-gateway-client", "cloud-gateway-client"},
		{"update-service", "update-service"},
		{"parking-operator-adaptor", "parking-operator-adaptor"},
		{"location-sensor", "location-sensor"},
		{"speed-sensor", "speed-sensor"},
		{"door-sensor", "door-sensor"},
	}

	for _, b := range binaries {
		t.Run(b.name, func(t *testing.T) {
			binPath := filepath.Join(root, "rhivos", "target", "debug", b.name)
			cmd := exec.Command(binPath)
			out, err := cmd.Output()
			if err != nil {
				t.Fatalf("%s exited with error: %v", b.name, err)
			}
			if !strings.Contains(string(out), b.component) {
				t.Fatalf("expected stdout to contain %q, got: %s", b.component, out)
			}
		})
	}
}

// TestGoSkeletonBinaryPrintsVersion verifies each Go skeleton binary prints
// a version string containing its name to stdout and exits with code 0.
// Test Spec: TS-01-14
// Requirements: 01-REQ-4.2, 01-REQ-4.4
func TestGoSkeletonBinaryPrintsVersion(t *testing.T) {
	if _, err := exec.LookPath("go"); err != nil {
		t.Skip("go not found on PATH — skipping Go binary tests")
	}

	root := findRepoRoot(t)

	modules := []struct {
		dir       string
		component string   // substring expected in stdout
		args      []string // args to pass (empty = no args, ["--version"] for servers)
	}{
		// Backend servers use --version flag (they start serving with no args).
		{"backend/parking-fee-service", "parking-fee-service", []string{"--version"}},
		{"backend/cloud-gateway", "cloud-gateway", []string{"--version"}},
		// Mock CLIs print version with no arguments.
		{"mock/parking-app-cli", "parking-app-cli", nil},
		{"mock/companion-app-cli", "companion-app-cli", nil},
		{"mock/parking-operator", "parking-operator", nil},
	}

	for _, m := range modules {
		t.Run(m.dir, func(t *testing.T) {
			goArgs := append([]string{"run", "."}, m.args...)
			cmd := exec.Command("go", goArgs...)
			cmd.Dir = filepath.Join(root, m.dir)
			out, err := cmd.Output()
			if err != nil {
				t.Fatalf("go run %s %v failed: %v", m.dir, m.args, err)
			}
			if !strings.Contains(string(out), m.component) {
				t.Fatalf("expected stdout to contain %q, got: %s", m.component, out)
			}
		})
	}
}

// TestMockSensorBinaryPrintsName verifies each mock sensor binary prints its
// specific name when executed with no arguments.
// Test Spec: TS-01-15
// Requirement: 01-REQ-4.3
func TestMockSensorBinaryPrintsName(t *testing.T) {
	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("cargo not found on PATH — skipping sensor binary tests")
	}

	root := findRepoRoot(t)

	// Build mock-sensors.
	cmd := exec.Command("cargo", "build", "-p", "mock-sensors")
	cmd.Dir = filepath.Join(root, "rhivos")
	if out, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("cargo build -p mock-sensors failed:\n%s\n%v", out, err)
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
			if !strings.Contains(string(out), sensor) {
				t.Fatalf("expected stdout to contain %q, got: %s", sensor, out)
			}
		})
	}
}

// TestSkeletonUnknownFlag verifies skeleton binaries exit non-zero with stderr
// output when invoked with an unrecognized flag.
// Test Spec: TS-01-E4
// Requirement: 01-REQ-4.E1
func TestSkeletonUnknownFlag(t *testing.T) {
	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("cargo not found on PATH — skipping unknown flag tests")
	}

	root := findRepoRoot(t)

	// Build workspace.
	cmd := exec.Command("cargo", "build", "--workspace")
	cmd.Dir = filepath.Join(root, "rhivos")
	if out, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("cargo build failed:\n%s\n%v", out, err)
	}

	// Test mock-sensors main binary — it rejects flags starting with '-'.
	t.Run("mock-sensors", func(t *testing.T) {
		binPath := filepath.Join(root, "rhivos", "target", "debug", "mock-sensors")
		cmd := exec.Command(binPath, "--invalid-flag")
		stderr, err := cmd.CombinedOutput()
		if err == nil {
			t.Fatal("expected non-zero exit code for --invalid-flag, but got 0")
		}
		if len(stderr) == 0 {
			t.Fatal("expected stderr output for unknown flag, but got nothing")
		}
	})
}

// TestSkeletonDeterminism verifies skeleton binaries produce identical output
// across multiple invocations.
// Test Spec: TS-01-P2
// Property: Property 2 (Skeleton Determinism)
func TestSkeletonDeterminism(t *testing.T) {
	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("cargo not found on PATH — skipping determinism tests")
	}

	root := findRepoRoot(t)

	// Build workspace.
	cmd := exec.Command("cargo", "build", "--workspace")
	cmd.Dir = filepath.Join(root, "rhivos")
	if out, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("cargo build failed:\n%s\n%v", out, err)
	}

	binaries := []string{
		"locking-service",
		"cloud-gateway-client",
		"update-service",
		"parking-operator-adaptor",
		"location-sensor",
		"speed-sensor",
		"door-sensor",
	}

	for _, bin := range binaries {
		t.Run(bin, func(t *testing.T) {
			binPath := filepath.Join(root, "rhivos", "target", "debug", bin)

			cmd1 := exec.Command(binPath)
			out1, err1 := cmd1.Output()
			if err1 != nil {
				t.Fatalf("first invocation of %s failed: %v", bin, err1)
			}

			cmd2 := exec.Command(binPath)
			out2, err2 := cmd2.Output()
			if err2 != nil {
				t.Fatalf("second invocation of %s failed: %v", bin, err2)
			}

			if string(out1) != string(out2) {
				t.Fatalf("non-deterministic output:\n  run1: %s\n  run2: %s", out1, out2)
			}
		})
	}
}
