package setup_test

import (
	"path/filepath"
	"testing"
)

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
		t.Run(dir, func(t *testing.T) {
			assertPathExists(t, filepath.Join(root, "rhivos", dir))
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
		t.Run(dir, func(t *testing.T) {
			assertPathExists(t, filepath.Join(root, "backend", dir))
		})
	}
}

// TS-01-3: Android and mobile placeholder directories exist
// Requirements: 01-REQ-1.3, 01-REQ-1.4
func TestPlaceholderDirectories(t *testing.T) {
	root := repoRoot(t)

	t.Run("android/README.md exists", func(t *testing.T) {
		assertPathExists(t, filepath.Join(root, "android", "README.md"))
	})

	t.Run("mobile/README.md exists", func(t *testing.T) {
		assertPathExists(t, filepath.Join(root, "mobile", "README.md"))
	})

	t.Run("android README mentions PARKING_APP", func(t *testing.T) {
		assertFileContains(t, filepath.Join(root, "android", "README.md"), "PARKING_APP")
	})

	t.Run("mobile README mentions COMPANION_APP", func(t *testing.T) {
		assertFileContains(t, filepath.Join(root, "mobile", "README.md"), "COMPANION_APP")
	})
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
		t.Run(dir, func(t *testing.T) {
			assertPathExists(t, filepath.Join(root, "mock", dir))
		})
	}
}

// TS-01-5: Proto and deployments directories exist
// Requirements: 01-REQ-1.6, 01-REQ-1.7
func TestProtoAndDeploymentsDirectories(t *testing.T) {
	root := repoRoot(t)

	t.Run("proto", func(t *testing.T) {
		assertPathExists(t, filepath.Join(root, "proto"))
	})

	t.Run("deployments", func(t *testing.T) {
		assertPathExists(t, filepath.Join(root, "deployments"))
	})
}

// TS-01-6: Tests setup directory exists
// Requirements: 01-REQ-1.8
func TestSetupDirectoryExists(t *testing.T) {
	root := repoRoot(t)

	t.Run("tests/setup exists", func(t *testing.T) {
		assertPathExists(t, filepath.Join(root, "tests", "setup"))
	})

	t.Run("tests/setup/go.mod exists", func(t *testing.T) {
		assertPathExists(t, filepath.Join(root, "tests", "setup", "go.mod"))
	})
}
