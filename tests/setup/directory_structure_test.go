package setup_test

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// repoRoot returns the absolute path to the repository root directory.
// It walks up from the test file's directory until it finds the Makefile.
func repoRoot(t *testing.T) string {
	t.Helper()

	// Start from the current working directory (tests/setup/)
	dir, err := os.Getwd()
	if err != nil {
		t.Fatalf("failed to get working directory: %v", err)
	}

	// Walk up to find repository root (contains Makefile)
	for {
		if _, err := os.Stat(filepath.Join(dir, "Makefile")); err == nil {
			return dir
		}
		parent := filepath.Dir(dir)
		if parent == dir {
			t.Fatal("could not find repository root (no Makefile found)")
		}
		dir = parent
	}
}

// TS-01-1: Repository contains rhivos directory structure
// Requirement: 01-REQ-1.1
func TestRhivosDirectoryStructure(t *testing.T) {
	root := repoRoot(t)

	subdirs := []string{
		"locking-service",
		"cloud-gateway-client",
		"update-service",
		"parking-operator-adaptor",
		"mock-sensors",
	}

	for _, dir := range subdirs {
		path := filepath.Join(root, "rhivos", dir)
		t.Run(dir, func(t *testing.T) {
			info, err := os.Stat(path)
			if err != nil {
				t.Fatalf("expected directory %s to exist: %v", path, err)
			}
			if !info.IsDir() {
				t.Fatalf("expected %s to be a directory", path)
			}
		})
	}
}

// TS-01-2: Repository contains backend directory structure
// Requirement: 01-REQ-1.2
func TestBackendDirectoryStructure(t *testing.T) {
	root := repoRoot(t)

	subdirs := []string{
		"parking-fee-service",
		"cloud-gateway",
	}

	for _, dir := range subdirs {
		path := filepath.Join(root, "backend", dir)
		t.Run(dir, func(t *testing.T) {
			info, err := os.Stat(path)
			if err != nil {
				t.Fatalf("expected directory %s to exist: %v", path, err)
			}
			if !info.IsDir() {
				t.Fatalf("expected %s to be a directory", path)
			}
		})
	}
}

// TS-01-3: Android and mobile placeholder directories exist
// Requirements: 01-REQ-1.3, 01-REQ-1.4
func TestPlaceholderDirectories(t *testing.T) {
	root := repoRoot(t)

	tests := []struct {
		dir          string
		contentMatch string
	}{
		{"android", "PARKING_APP"},
		{"mobile", "COMPANION_APP"},
	}

	for _, tc := range tests {
		t.Run(tc.dir, func(t *testing.T) {
			readmePath := filepath.Join(root, tc.dir, "README.md")
			data, err := os.ReadFile(readmePath)
			if err != nil {
				t.Fatalf("expected %s to exist: %v", readmePath, err)
			}
			if !strings.Contains(string(data), tc.contentMatch) {
				t.Errorf("expected %s to contain %q", readmePath, tc.contentMatch)
			}
		})
	}
}

// TS-01-4: Mock directory structure exists
// Requirement: 01-REQ-1.5
func TestMockDirectoryStructure(t *testing.T) {
	root := repoRoot(t)

	subdirs := []string{
		"parking-app-cli",
		"companion-app-cli",
		"parking-operator",
	}

	for _, dir := range subdirs {
		path := filepath.Join(root, "mock", dir)
		t.Run(dir, func(t *testing.T) {
			info, err := os.Stat(path)
			if err != nil {
				t.Fatalf("expected directory %s to exist: %v", path, err)
			}
			if !info.IsDir() {
				t.Fatalf("expected %s to be a directory", path)
			}
		})
	}
}

// TS-01-5: Proto and deployments directories exist
// Requirements: 01-REQ-1.6, 01-REQ-1.7
func TestProtoAndDeploymentsDirectories(t *testing.T) {
	root := repoRoot(t)

	dirs := []string{"proto", "deployments"}

	for _, dir := range dirs {
		path := filepath.Join(root, dir)
		t.Run(dir, func(t *testing.T) {
			info, err := os.Stat(path)
			if err != nil {
				t.Fatalf("expected directory %s to exist: %v", path, err)
			}
			if !info.IsDir() {
				t.Fatalf("expected %s to be a directory", path)
			}
		})
	}
}

// TS-01-6: Tests setup directory exists
// Requirements: 01-REQ-1.8
func TestSetupDirectoryExists(t *testing.T) {
	root := repoRoot(t)

	// Verify go.mod exists in tests/setup/
	goModPath := filepath.Join(root, "tests", "setup", "go.mod")
	if _, err := os.Stat(goModPath); err != nil {
		t.Fatalf("expected %s to exist: %v", goModPath, err)
	}
}
