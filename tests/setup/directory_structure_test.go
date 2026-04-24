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
			path := filepath.Join(root, "rhivos", dir)
			if !pathExists(path) {
				t.Errorf("expected directory %s to exist", path)
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
		t.Run(dir, func(t *testing.T) {
			path := filepath.Join(root, "backend", dir)
			if !pathExists(path) {
				t.Errorf("expected directory %s to exist", path)
			}
		})
	}
}

// TS-01-3: Android and mobile placeholder directories exist
// Requirement: 01-REQ-1.3, 01-REQ-1.4
func TestPlaceholderDirectories(t *testing.T) {
	root := repoRoot(t)

	t.Run("android/README.md exists", func(t *testing.T) {
		path := filepath.Join(root, "android", "README.md")
		if !pathExists(path) {
			t.Errorf("expected %s to exist", path)
		}
	})

	t.Run("android/README.md mentions PARKING_APP", func(t *testing.T) {
		path := filepath.Join(root, "android", "README.md")
		if !pathExists(path) {
			t.Skip("android/README.md does not exist")
		}
		if !fileContains(t, path, "PARKING_APP") {
			t.Errorf("android/README.md should mention PARKING_APP")
		}
	})

	t.Run("mobile/README.md exists", func(t *testing.T) {
		path := filepath.Join(root, "mobile", "README.md")
		if !pathExists(path) {
			t.Errorf("expected %s to exist", path)
		}
	})

	t.Run("mobile/README.md mentions COMPANION_APP", func(t *testing.T) {
		path := filepath.Join(root, "mobile", "README.md")
		if !pathExists(path) {
			t.Skip("mobile/README.md does not exist")
		}
		if !fileContains(t, path, "COMPANION_APP") {
			t.Errorf("mobile/README.md should mention COMPANION_APP")
		}
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
			path := filepath.Join(root, "mock", dir)
			if !pathExists(path) {
				t.Errorf("expected directory %s to exist", path)
			}
		})
	}
}

// TS-01-5: Proto and deployments directories exist
// Requirement: 01-REQ-1.6, 01-REQ-1.7
func TestProtoAndDeploymentsDirectories(t *testing.T) {
	root := repoRoot(t)

	t.Run("proto directory exists", func(t *testing.T) {
		path := filepath.Join(root, "proto")
		if !pathExists(path) {
			t.Errorf("expected directory %s to exist", path)
		}
	})

	t.Run("deployments directory exists", func(t *testing.T) {
		path := filepath.Join(root, "deployments")
		if !pathExists(path) {
			t.Errorf("expected directory %s to exist", path)
		}
	})
}

// TS-01-6: Tests setup directory exists
// Requirement: 01-REQ-1.8
func TestSetupDirectoryExists(t *testing.T) {
	root := repoRoot(t)

	t.Run("tests/setup directory exists", func(t *testing.T) {
		path := filepath.Join(root, "tests", "setup")
		if !pathExists(path) {
			t.Errorf("expected directory %s to exist", path)
		}
	})

	t.Run("tests/setup/go.mod exists", func(t *testing.T) {
		path := filepath.Join(root, "tests", "setup", "go.mod")
		if !pathExists(path) {
			t.Errorf("expected %s to exist", path)
		}
	})
}
