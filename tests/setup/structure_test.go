package setup

import (
	"testing"
)

// TestRhivosDirectoryStructure verifies rhivos/ contains all required subdirectories.
// Test Spec: TS-01-1
// Requirement: 01-REQ-1.1
func TestRhivosDirectoryStructure(t *testing.T) {
	root := findRepoRoot(t)
	subdirs := []string{
		"locking-service",
		"cloud-gateway-client",
		"update-service",
		"parking-operator-adaptor",
		"mock-sensors",
	}
	for _, dir := range subdirs {
		t.Run(dir, func(t *testing.T) {
			assertDirExists(t, repoPath(root, "rhivos", dir))
		})
	}
}

// TestBackendDirectoryStructure verifies backend/ contains required subdirectories.
// Test Spec: TS-01-2
// Requirement: 01-REQ-1.2
func TestBackendDirectoryStructure(t *testing.T) {
	root := findRepoRoot(t)
	subdirs := []string{
		"parking-fee-service",
		"cloud-gateway",
	}
	for _, dir := range subdirs {
		t.Run(dir, func(t *testing.T) {
			assertDirExists(t, repoPath(root, "backend", dir))
		})
	}
}

// TestPlaceholderDirectories verifies android/ and mobile/ exist with README files
// that mention the respective apps.
// Test Spec: TS-01-3
// Requirements: 01-REQ-1.3, 01-REQ-1.4
func TestPlaceholderDirectories(t *testing.T) {
	root := findRepoRoot(t)

	t.Run("android/README.md exists", func(t *testing.T) {
		assertFileExists(t, repoPath(root, "android", "README.md"))
	})

	t.Run("android/README.md mentions PARKING_APP", func(t *testing.T) {
		assertFileContains(t, repoPath(root, "android", "README.md"), "PARKING_APP")
	})

	t.Run("mobile/README.md exists", func(t *testing.T) {
		assertFileExists(t, repoPath(root, "mobile", "README.md"))
	})

	t.Run("mobile/README.md mentions COMPANION_APP", func(t *testing.T) {
		assertFileContains(t, repoPath(root, "mobile", "README.md"), "COMPANION_APP")
	})
}

// TestMockDirectoryStructure verifies mock/ contains all required subdirectories.
// Test Spec: TS-01-4
// Requirement: 01-REQ-1.5
func TestMockDirectoryStructure(t *testing.T) {
	root := findRepoRoot(t)
	subdirs := []string{
		"parking-app-cli",
		"companion-app-cli",
		"parking-operator",
	}
	for _, dir := range subdirs {
		t.Run(dir, func(t *testing.T) {
			assertDirExists(t, repoPath(root, "mock", dir))
		})
	}
}

// TestProtoAndDeploymentDirectories verifies proto/ and deployments/ directories exist.
// Test Spec: TS-01-5
// Requirements: 01-REQ-1.6, 01-REQ-1.7
func TestProtoAndDeploymentDirectories(t *testing.T) {
	root := findRepoRoot(t)

	t.Run("proto", func(t *testing.T) {
		assertDirExists(t, repoPath(root, "proto"))
	})

	t.Run("deployments", func(t *testing.T) {
		assertDirExists(t, repoPath(root, "deployments"))
	})
}

// TestSetupDirectoryExists verifies tests/setup/ exists and contains a go.mod.
// Test Spec: TS-01-6
// Requirements: 01-REQ-1.8, 01-REQ-9.1
func TestSetupDirectoryExists(t *testing.T) {
	root := findRepoRoot(t)

	t.Run("tests/setup dir", func(t *testing.T) {
		assertDirExists(t, repoPath(root, "tests", "setup"))
	})

	t.Run("tests/setup/go.mod", func(t *testing.T) {
		assertFileExists(t, repoPath(root, "tests", "setup", "go.mod"))
	})
}
