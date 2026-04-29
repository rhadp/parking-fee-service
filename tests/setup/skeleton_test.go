package setup_test

import (
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
)

// TS-01-13: Rust skeleton prints version and exits 0
// Requirements: 01-REQ-4.1, 01-REQ-4.4
func TestRustSkeletonBinaries(t *testing.T) {
	root := repoRoot(t)

	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("cargo not found on PATH; skipping Rust skeleton test")
	}

	// Build first to ensure binaries exist
	buildCmd := exec.Command("cargo", "build", "--workspace")
	buildCmd.Dir = filepath.Join(root, "rhivos")
	buildOutput, err := buildCmd.CombinedOutput()
	if err != nil {
		t.Fatalf("cargo build failed (prerequisite): %v\noutput:\n%s", err, string(buildOutput))
	}

	binaries := map[string]string{
		"locking-service":        "locking-service",
		"cloud-gateway-client":   "cloud-gateway-client",
		"update-service":         "update-service",
		"parking-operator-adaptor": "parking-operator-adaptor",
	}

	for binName, expectedName := range binaries {
		t.Run(binName, func(t *testing.T) {
			binPath := filepath.Join(root, "rhivos", "target", "debug", binName)
			cmd := exec.Command(binPath)
			output, err := cmd.CombinedOutput()
			if err != nil {
				t.Errorf("%s exited with error: %v\noutput:\n%s", binName, err, string(output))
				return
			}
			if !strings.Contains(string(output), expectedName) {
				t.Errorf("%s output does not contain component name %q: %s", binName, expectedName, string(output))
			}
		})
	}
}

// TS-01-14: Go skeleton prints version and exits 0
// Requirements: 01-REQ-4.2, 01-REQ-4.4
func TestGoSkeletonBinaries(t *testing.T) {
	root := repoRoot(t)

	if _, err := exec.LookPath("go"); err != nil {
		t.Skip("go not found on PATH; skipping Go skeleton test")
	}

	modules := map[string]string{
		"backend/parking-fee-service": "parking-fee-service",
		"backend/cloud-gateway":       "cloud-gateway",
		"mock/parking-app-cli":        "parking-app-cli",
		"mock/companion-app-cli":      "companion-app-cli",
		"mock/parking-operator":       "parking-operator",
	}

	for modPath, expectedName := range modules {
		t.Run(expectedName, func(t *testing.T) {
			cmd := exec.Command("go", "run", ".")
			cmd.Dir = filepath.Join(root, modPath)
			output, err := cmd.CombinedOutput()
			if err != nil {
				t.Errorf("go run %s failed: %v\noutput:\n%s", modPath, err, string(output))
				return
			}
			if !strings.Contains(string(output), expectedName) {
				t.Errorf("go run %s output does not contain component name %q: %s", modPath, expectedName, string(output))
			}
		})
	}
}

// TS-01-15: Mock sensor binaries print name and version
// Requirement: 01-REQ-4.3
func TestMockSensorBinaries(t *testing.T) {
	root := repoRoot(t)

	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("cargo not found on PATH; skipping mock sensor test")
	}

	// Build first
	buildCmd := exec.Command("cargo", "build", "--workspace")
	buildCmd.Dir = filepath.Join(root, "rhivos")
	buildOutput, err := buildCmd.CombinedOutput()
	if err != nil {
		t.Fatalf("cargo build failed (prerequisite): %v\noutput:\n%s", err, string(buildOutput))
	}

	sensors := []string{"location-sensor", "speed-sensor", "door-sensor"}

	for _, sensor := range sensors {
		t.Run(sensor, func(t *testing.T) {
			binPath := filepath.Join(root, "rhivos", "target", "debug", sensor)
			cmd := exec.Command(binPath)
			output, err := cmd.CombinedOutput()
			if err != nil {
				t.Errorf("%s exited with error: %v\noutput:\n%s", sensor, err, string(output))
				return
			}
			if !strings.Contains(string(output), sensor) {
				t.Errorf("%s output does not contain sensor name %q: %s", sensor, sensor, string(output))
			}
		})
	}
}
