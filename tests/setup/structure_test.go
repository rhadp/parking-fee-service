package setup_test

import (
	"testing"
)

// TS-01-1: Rust component directory exists (01-REQ-1.1)
func TestStructure_RustDir(t *testing.T) {
	root := repoRoot(t)
	assertDirExists(t, root, "rhivos")
	assertFileExists(t, root, "rhivos/Cargo.toml")
}

// TS-01-2: Go component directory exists (01-REQ-1.2)
func TestStructure_GoBackendDir(t *testing.T) {
	root := repoRoot(t)
	assertDirExists(t, root, "backend")
	assertDirExists(t, root, "backend/parking-fee-service")
	assertDirExists(t, root, "backend/cloud-gateway")
}

// TS-01-3: Proto directory exists (01-REQ-1.3)
func TestStructure_ProtoDir(t *testing.T) {
	root := repoRoot(t)
	assertDirExists(t, root, "proto")
	files := globFiles(t, root, "proto/*.proto")
	if len(files) < 3 {
		t.Errorf("expected at least 3 proto files in proto/, found %d", len(files))
	}
}

// TS-01-4: Mock directory exists (01-REQ-1.4)
func TestStructure_MockDir(t *testing.T) {
	root := repoRoot(t)
	assertDirExists(t, root, "mock")
	assertDirExists(t, root, "mock/parking-app-cli")
	assertDirExists(t, root, "mock/companion-app-cli")
}

// TS-01-5: Android placeholder directories exist (01-REQ-1.5)
func TestStructure_PlaceholderDirs(t *testing.T) {
	root := repoRoot(t)
	assertDirExists(t, root, "aaos/parking-app")
	assertFileExists(t, root, "aaos/parking-app/README.md")
	assertDirExists(t, root, "android/companion-app")
	assertFileExists(t, root, "android/companion-app/README.md")
}

// TS-01-6: Infrastructure directory exists (01-REQ-1.6)
func TestStructure_InfraDir(t *testing.T) {
	root := repoRoot(t)
	assertDirExists(t, root, "infra")
	assertFileExists(t, root, "infra/docker-compose.yml")
	assertFileExists(t, root, "infra/mosquitto/mosquitto.conf")
}

// TS-01-42: Integration test directory structure (01-REQ-8.4)
func TestStructure_IntegrationTestDir(t *testing.T) {
	root := repoRoot(t)
	assertDirExists(t, root, "tests")
	assertDirExists(t, root, "tests/integration")
}
