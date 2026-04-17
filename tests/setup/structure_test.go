package setup_test

import (
	"path/filepath"
	"testing"
)

// TestRhivosDirectoryStructure verifies the rhivos/ directory contains all
// required subdirectories for Rust services (TS-01-1, 01-REQ-1.1).
func TestRhivosDirectoryStructure(t *testing.T) {
	root := repoRoot(t)
	dirs := []string{
		"locking-service",
		"cloud-gateway-client",
		"update-service",
		"parking-operator-adaptor",
		"mock-sensors",
	}
	for _, d := range dirs {
		assertPathExists(t, filepath.Join(root, "rhivos", d))
	}
}

// TestBackendDirectoryStructure verifies the backend/ directory contains all
// required subdirectories for Go services (TS-01-2, 01-REQ-1.2).
func TestBackendDirectoryStructure(t *testing.T) {
	root := repoRoot(t)
	dirs := []string{
		"parking-fee-service",
		"cloud-gateway",
	}
	for _, d := range dirs {
		assertPathExists(t, filepath.Join(root, "backend", d))
	}
}

// TestPlaceholderDirectories verifies android/ and mobile/ placeholder
// directories exist with README.md files containing expected content
// (TS-01-3, 01-REQ-1.3, 01-REQ-1.4).
func TestPlaceholderDirectories(t *testing.T) {
	root := repoRoot(t)

	androidReadme := filepath.Join(root, "android", "README.md")
	assertPathExists(t, androidReadme)
	assertFileContains(t, androidReadme, "PARKING_APP")

	mobileReadme := filepath.Join(root, "mobile", "README.md")
	assertPathExists(t, mobileReadme)
	assertFileContains(t, mobileReadme, "COMPANION_APP")
}

// TestMockDirectoryStructure verifies the mock/ directory contains all
// required subdirectories (TS-01-4, 01-REQ-1.5).
func TestMockDirectoryStructure(t *testing.T) {
	root := repoRoot(t)
	dirs := []string{
		"parking-app-cli",
		"companion-app-cli",
		"parking-operator",
	}
	for _, d := range dirs {
		assertPathExists(t, filepath.Join(root, "mock", d))
	}
}

// TestProtoAndDeploymentDirectories verifies the proto/ and deployments/
// directories exist (TS-01-5, 01-REQ-1.6, 01-REQ-1.7).
func TestProtoAndDeploymentDirectories(t *testing.T) {
	root := repoRoot(t)
	assertPathExists(t, filepath.Join(root, "proto"))
	assertPathExists(t, filepath.Join(root, "deployments"))
}

// TestSetupDirectoryExists verifies tests/setup/ exists and contains a Go
// module (TS-01-6, 01-REQ-1.8).
func TestSetupDirectoryExists(t *testing.T) {
	root := repoRoot(t)
	assertPathExists(t, filepath.Join(root, "tests", "setup"))
	assertPathExists(t, filepath.Join(root, "tests", "setup", "go.mod"))
}
