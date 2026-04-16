package setup_test

import (
	"path/filepath"
	"testing"
)

// TestRhivosDirectoryStructure verifies the rhivos/ directory contains all required
// subdirectories for RHIVOS Rust services.
// Test Spec: TS-01-1
// Requirements: 01-REQ-1.1
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
		t.Run(dir, func(t *testing.T) {
			assertDirExists(t, filepath.Join(root, "rhivos", dir))
		})
	}
}

// TestBackendDirectoryStructure verifies the backend/ directory contains all required
// subdirectories for Go backend services.
// Test Spec: TS-01-2
// Requirements: 01-REQ-1.2
func TestBackendDirectoryStructure(t *testing.T) {
	root := repoRoot(t)

	subdirs := []string{
		"parking-fee-service",
		"cloud-gateway",
	}

	for _, dir := range subdirs {
		t.Run(dir, func(t *testing.T) {
			assertDirExists(t, filepath.Join(root, "backend", dir))
		})
	}
}

// TestPlaceholderDirectories verifies that android/ and mobile/ placeholder directories
// exist with README.md files explaining their purpose.
// Test Spec: TS-01-3
// Requirements: 01-REQ-1.3, 01-REQ-1.4
func TestPlaceholderDirectories(t *testing.T) {
	root := repoRoot(t)

	t.Run("android_readme_exists", func(t *testing.T) {
		assertPathExists(t, filepath.Join(root, "android", "README.md"))
	})

	t.Run("android_readme_mentions_PARKING_APP", func(t *testing.T) {
		assertFileContains(t, filepath.Join(root, "android", "README.md"), "PARKING_APP")
	})

	t.Run("mobile_readme_exists", func(t *testing.T) {
		assertPathExists(t, filepath.Join(root, "mobile", "README.md"))
	})

	t.Run("mobile_readme_mentions_COMPANION_APP", func(t *testing.T) {
		assertFileContains(t, filepath.Join(root, "mobile", "README.md"), "COMPANION_APP")
	})
}

// TestMockDirectoryStructure verifies the mock/ directory contains all required
// subdirectories for mock CLI applications.
// Test Spec: TS-01-4
// Requirements: 01-REQ-1.5
func TestMockDirectoryStructure(t *testing.T) {
	root := repoRoot(t)

	subdirs := []string{
		"parking-app-cli",
		"companion-app-cli",
		"parking-operator",
	}

	for _, dir := range subdirs {
		t.Run(dir, func(t *testing.T) {
			assertDirExists(t, filepath.Join(root, "mock", dir))
		})
	}
}

// TestProtoAndDeploymentsDirectories verifies that proto/ and deployments/ directories
// exist as required by the project structure.
// Test Spec: TS-01-5
// Requirements: 01-REQ-1.6, 01-REQ-1.7
func TestProtoAndDeploymentsDirectories(t *testing.T) {
	root := repoRoot(t)

	t.Run("proto", func(t *testing.T) {
		assertDirExists(t, filepath.Join(root, "proto"))
	})

	t.Run("deployments", func(t *testing.T) {
		assertDirExists(t, filepath.Join(root, "deployments"))
	})
}

// TestSetupDirectoryExists verifies that tests/setup/ exists and contains a go.mod.
// Test Spec: TS-01-6
// Requirements: 01-REQ-1.8
func TestSetupDirectoryExists(t *testing.T) {
	root := repoRoot(t)

	t.Run("tests_setup_dir_exists", func(t *testing.T) {
		assertDirExists(t, filepath.Join(root, "tests", "setup"))
	})

	t.Run("tests_setup_go_mod_exists", func(t *testing.T) {
		assertPathExists(t, filepath.Join(root, "tests", "setup", "go.mod"))
	})
}
