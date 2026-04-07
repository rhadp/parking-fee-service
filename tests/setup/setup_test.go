// Package setup contains verification tests for the parking-fee-service monorepo
// project setup. These tests validate directory structure, workspace configurations,
// proto definitions, infrastructure config, and Makefile targets.
//
// Tests are designed to run from the tests/setup/ directory and reference the
// repository root via repoRoot(), which walks up from the test file location.
package setup

import (
	"encoding/json"
	"os"
	"path/filepath"
	"regexp"
	"strings"
	"testing"
)

// repoRoot returns the absolute path to the repository root by walking up
// from the current working directory until it finds the .git file/directory.
func repoRoot(t *testing.T) string {
	t.Helper()
	dir, err := os.Getwd()
	if err != nil {
		t.Fatalf("failed to get working directory: %v", err)
	}
	for {
		if _, err := os.Stat(filepath.Join(dir, ".git")); err == nil {
			return dir
		}
		parent := filepath.Dir(dir)
		if parent == dir {
			t.Fatal("could not find repository root (.git not found in any parent directory)")
		}
		dir = parent
	}
}

// pathExists returns true if the given path exists on the filesystem.
func pathExists(path string) bool {
	_, err := os.Stat(path)
	return err == nil
}

// readFileContent reads a file and returns its contents as a string.
// Returns an error if the file cannot be read.
func readFileContent(path string) (string, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return "", err
	}
	return string(data), nil
}

// --- TS-01-1: Repository contains rhivos directory structure ---
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
		if !pathExists(path) {
			t.Errorf("expected directory %s to exist", path)
		}
	}
}

// --- TS-01-2: Repository contains backend directory structure ---
// Requirement: 01-REQ-1.2
func TestBackendDirectoryStructure(t *testing.T) {
	root := repoRoot(t)
	subdirs := []string{
		"parking-fee-service",
		"cloud-gateway",
	}
	for _, dir := range subdirs {
		path := filepath.Join(root, "backend", dir)
		if !pathExists(path) {
			t.Errorf("expected directory %s to exist", path)
		}
	}
}

// --- TS-01-3: Android and mobile placeholder directories exist ---
// Requirements: 01-REQ-1.3, 01-REQ-1.4
func TestPlaceholderDirectories(t *testing.T) {
	root := repoRoot(t)

	// Check android/README.md exists and mentions PARKING_APP
	androidReadme := filepath.Join(root, "android", "README.md")
	if !pathExists(androidReadme) {
		t.Error("expected android/README.md to exist")
	} else {
		content, err := readFileContent(androidReadme)
		if err != nil {
			t.Errorf("failed to read android/README.md: %v", err)
		} else if !strings.Contains(content, "PARKING_APP") {
			t.Error("android/README.md should mention PARKING_APP")
		}
	}

	// Check mobile/README.md exists and mentions COMPANION_APP
	mobileReadme := filepath.Join(root, "mobile", "README.md")
	if !pathExists(mobileReadme) {
		t.Error("expected mobile/README.md to exist")
	} else {
		content, err := readFileContent(mobileReadme)
		if err != nil {
			t.Errorf("failed to read mobile/README.md: %v", err)
		} else if !strings.Contains(content, "COMPANION_APP") {
			t.Error("mobile/README.md should mention COMPANION_APP")
		}
	}
}

// --- TS-01-4: Mock directory structure exists ---
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
		if !pathExists(path) {
			t.Errorf("expected directory %s to exist", path)
		}
	}
}

// --- TS-01-5: Proto and deployments directories exist ---
// Requirements: 01-REQ-1.6, 01-REQ-1.7
func TestProtoAndDeploymentsDirectories(t *testing.T) {
	root := repoRoot(t)

	if !pathExists(filepath.Join(root, "proto")) {
		t.Error("expected proto/ directory to exist")
	}
	if !pathExists(filepath.Join(root, "deployments")) {
		t.Error("expected deployments/ directory to exist")
	}
}

// --- TS-01-6: Tests setup directory exists ---
// Requirements: 01-REQ-1.8, 01-REQ-9.1
func TestSetupDirectoryExists(t *testing.T) {
	root := repoRoot(t)

	if !pathExists(filepath.Join(root, "tests", "setup")) {
		t.Error("expected tests/setup/ directory to exist")
	}
	if !pathExists(filepath.Join(root, "tests", "setup", "go.mod")) {
		t.Error("expected tests/setup/go.mod to exist")
	}
}

// --- TS-01-7: Cargo workspace is correctly configured ---
// Requirements: 01-REQ-2.1, 01-REQ-2.2
func TestCargoWorkspaceConfiguration(t *testing.T) {
	root := repoRoot(t)

	cargoTomlPath := filepath.Join(root, "rhivos", "Cargo.toml")
	if !pathExists(cargoTomlPath) {
		t.Fatal("expected rhivos/Cargo.toml to exist")
	}

	content, err := readFileContent(cargoTomlPath)
	if err != nil {
		t.Fatalf("failed to read rhivos/Cargo.toml: %v", err)
	}

	// Check workspace members are declared
	expectedMembers := []string{
		"locking-service",
		"cloud-gateway-client",
		"update-service",
		"parking-operator-adaptor",
		"mock-sensors",
	}
	for _, member := range expectedMembers {
		if !strings.Contains(content, member) {
			t.Errorf("rhivos/Cargo.toml should contain workspace member %q", member)
		}
	}

	// Check each member has Cargo.toml and src/main.rs
	for _, member := range expectedMembers {
		memberCargoToml := filepath.Join(root, "rhivos", member, "Cargo.toml")
		if !pathExists(memberCargoToml) {
			t.Errorf("expected %s to exist", memberCargoToml)
		}
		memberMainRs := filepath.Join(root, "rhivos", member, "src", "main.rs")
		if !pathExists(memberMainRs) {
			t.Errorf("expected %s to exist", memberMainRs)
		}
	}
}

// --- TS-01-8: Mock sensors declares three binary targets ---
// Requirement: 01-REQ-2.3
func TestMockSensorsBinaryTargets(t *testing.T) {
	root := repoRoot(t)

	cargoTomlPath := filepath.Join(root, "rhivos", "mock-sensors", "Cargo.toml")
	if !pathExists(cargoTomlPath) {
		t.Fatal("expected rhivos/mock-sensors/Cargo.toml to exist")
	}

	content, err := readFileContent(cargoTomlPath)
	if err != nil {
		t.Fatalf("failed to read mock-sensors Cargo.toml: %v", err)
	}

	// Check for [[bin]] entries for each sensor
	expectedBins := []string{
		"location-sensor",
		"speed-sensor",
		"door-sensor",
	}
	for _, bin := range expectedBins {
		if !strings.Contains(content, bin) {
			t.Errorf("mock-sensors Cargo.toml should declare binary target %q", bin)
		}
	}
}

// --- TS-01-10: Go workspace file references all modules ---
// Requirement: 01-REQ-3.1
func TestGoWorkspaceReferences(t *testing.T) {
	root := repoRoot(t)

	goWorkPath := filepath.Join(root, "go.work")
	if !pathExists(goWorkPath) {
		t.Fatal("expected go.work to exist at repository root")
	}

	content, err := readFileContent(goWorkPath)
	if err != nil {
		t.Fatalf("failed to read go.work: %v", err)
	}

	expectedModules := []string{
		"backend/parking-fee-service",
		"backend/cloud-gateway",
		"mock/parking-app-cli",
		"mock/companion-app-cli",
		"mock/parking-operator",
		"tests/setup",
	}
	for _, mod := range expectedModules {
		if !strings.Contains(content, mod) {
			t.Errorf("go.work should reference module %q", mod)
		}
	}
}

// --- TS-01-11: Each Go module has go.mod and main.go ---
// Requirements: 01-REQ-3.2, 01-REQ-3.3
func TestGoModuleStructure(t *testing.T) {
	root := repoRoot(t)

	// Modules that should have both go.mod and main.go
	modulesWithMain := []string{
		"backend/parking-fee-service",
		"backend/cloud-gateway",
		"mock/parking-app-cli",
		"mock/companion-app-cli",
		"mock/parking-operator",
	}
	for _, mod := range modulesWithMain {
		goModPath := filepath.Join(root, mod, "go.mod")
		if !pathExists(goModPath) {
			t.Errorf("expected %s/go.mod to exist", mod)
		}
		mainGoPath := filepath.Join(root, mod, "main.go")
		if !pathExists(mainGoPath) {
			t.Errorf("expected %s/main.go to exist", mod)
		}
	}

	// tests/setup should have go.mod but NOT main.go
	if !pathExists(filepath.Join(root, "tests", "setup", "go.mod")) {
		t.Error("expected tests/setup/go.mod to exist")
	}
}

// --- TS-01-18: Makefile has all required targets ---
// Requirement: 01-REQ-6.1
func TestMakefileTargets(t *testing.T) {
	root := repoRoot(t)

	makefilePath := filepath.Join(root, "Makefile")
	if !pathExists(makefilePath) {
		t.Fatal("expected Makefile to exist at repository root")
	}

	content, err := readFileContent(makefilePath)
	if err != nil {
		t.Fatalf("failed to read Makefile: %v", err)
	}

	// Required targets per 01-REQ-6.1
	requiredTargets := []string{
		"build",
		"test",
		"clean",
		"proto",
		"infra-up",
		"infra-down",
		"check",
	}
	for _, target := range requiredTargets {
		// Match target definition at start of line (e.g., "build:" or "build: deps")
		pattern := regexp.MustCompile(`(?m)^` + regexp.QuoteMeta(target) + `\s*:`)
		if !pattern.MatchString(content) {
			t.Errorf("Makefile should define target %q", target)
		}
	}
}

// --- TS-01-23: compose.yml defines NATS and Kuksa services ---
// Requirement: 01-REQ-7.1
func TestComposeServices(t *testing.T) {
	root := repoRoot(t)

	composePath := filepath.Join(root, "deployments", "compose.yml")
	if !pathExists(composePath) {
		t.Fatal("expected deployments/compose.yml to exist")
	}

	content, err := readFileContent(composePath)
	if err != nil {
		t.Fatalf("failed to read compose.yml: %v", err)
	}

	// Check for NATS service with port 4222
	if !strings.Contains(content, "nats") {
		t.Error("compose.yml should define a nats service")
	}
	if !strings.Contains(content, "4222") {
		t.Error("compose.yml should expose port 4222 for NATS")
	}

	// Check for Kuksa Databroker service with port 55556
	if !strings.Contains(content, "55556") {
		t.Error("compose.yml should expose port 55556 for Kuksa Databroker")
	}
}

// --- TS-01-24: NATS configuration file exists ---
// Requirement: 01-REQ-7.2
func TestNATSConfigExists(t *testing.T) {
	root := repoRoot(t)

	natsConfPath := filepath.Join(root, "deployments", "nats", "nats-server.conf")
	if !pathExists(natsConfPath) {
		t.Fatal("expected deployments/nats/nats-server.conf to exist")
	}

	info, err := os.Stat(natsConfPath)
	if err != nil {
		t.Fatalf("failed to stat nats-server.conf: %v", err)
	}
	if info.Size() == 0 {
		t.Error("deployments/nats/nats-server.conf should not be empty")
	}
}

// --- TS-01-25: VSS overlay defines custom signals ---
// Requirement: 01-REQ-7.3
func TestVSSOverlaySignals(t *testing.T) {
	root := repoRoot(t)

	overlayPath := filepath.Join(root, "deployments", "vss-overlay.json")
	if !pathExists(overlayPath) {
		t.Fatal("expected deployments/vss-overlay.json to exist")
	}

	content, err := readFileContent(overlayPath)
	if err != nil {
		t.Fatalf("failed to read vss-overlay.json: %v", err)
	}

	// Verify it is valid JSON
	var parsed any
	if err := json.Unmarshal([]byte(content), &parsed); err != nil {
		t.Errorf("vss-overlay.json should be valid JSON: %v", err)
	}

	// Check for required custom signals
	requiredSignals := []string{
		"Vehicle.Parking.SessionActive",
		"Vehicle.Command.Door.Lock",
		"Vehicle.Command.Door.Response",
	}
	for _, signal := range requiredSignals {
		if !strings.Contains(content, signal) {
			t.Errorf("vss-overlay.json should define signal %q", signal)
		}
	}
}

// --- TS-01-16: Proto files are valid proto3 ---
// Requirements: 01-REQ-5.1, 01-REQ-5.2, 01-REQ-5.3
func TestProtoFilesValid(t *testing.T) {
	root := repoRoot(t)

	// Expected proto subdirectories per design
	expectedProtoDirs := []struct {
		dir  string
		desc string
	}{
		{"kuksa", "Kuksa Databroker value types"},
		{"update", "UPDATE_SERVICE interface"},
		{"adapter", "PARKING_OPERATOR_ADAPTOR interface"},
		{"gateway", "CLOUD_GATEWAY relay types"},
	}

	protoRoot := filepath.Join(root, "proto")
	if !pathExists(protoRoot) {
		t.Fatal("expected proto/ directory to exist")
	}

	for _, pd := range expectedProtoDirs {
		dirPath := filepath.Join(protoRoot, pd.dir)
		if !pathExists(dirPath) {
			t.Errorf("expected proto/%s/ directory to exist for %s", pd.dir, pd.desc)
			continue
		}

		// Find .proto files in this directory
		entries, err := os.ReadDir(dirPath)
		if err != nil {
			t.Errorf("failed to read proto/%s/: %v", pd.dir, err)
			continue
		}

		protoFound := false
		for _, entry := range entries {
			if strings.HasSuffix(entry.Name(), ".proto") {
				protoFound = true
				protoPath := filepath.Join(dirPath, entry.Name())
				content, err := readFileContent(protoPath)
				if err != nil {
					t.Errorf("failed to read %s: %v", protoPath, err)
					continue
				}

				// Check proto3 syntax
				if !strings.Contains(content, `syntax = "proto3"`) {
					t.Errorf("%s should use syntax = \"proto3\"", protoPath)
				}

				// Check package declaration
				packagePattern := regexp.MustCompile(`(?m)^package\s+\w+`)
				if !packagePattern.MatchString(content) {
					t.Errorf("%s should contain a package declaration", protoPath)
				}

				// Check go_package option
				goPackagePattern := regexp.MustCompile(`option\s+go_package\s*=`)
				if !goPackagePattern.MatchString(content) {
					t.Errorf("%s should contain a go_package option", protoPath)
				}
			}
		}

		if !protoFound {
			t.Errorf("proto/%s/ should contain at least one .proto file", pd.dir)
		}
	}
}

// --- TS-01-26: Rust crates have placeholder tests ---
// Requirement: 01-REQ-8.1
func TestRustCratesHavePlaceholderTests(t *testing.T) {
	root := repoRoot(t)

	crates := []string{
		"locking-service",
		"cloud-gateway-client",
		"update-service",
		"parking-operator-adaptor",
		"mock-sensors",
	}

	for _, crate := range crates {
		crateDir := filepath.Join(root, "rhivos", crate, "src")
		if !pathExists(crateDir) {
			t.Errorf("expected rhivos/%s/src/ directory to exist", crate)
			continue
		}

		// Walk the src directory looking for #[test]
		testFound := false
		err := filepath.Walk(crateDir, func(path string, info os.FileInfo, err error) error {
			if err != nil {
				return err
			}
			if !info.IsDir() && strings.HasSuffix(path, ".rs") {
				content, readErr := readFileContent(path)
				if readErr != nil {
					return nil
				}
				if strings.Contains(content, "#[test]") {
					testFound = true
				}
			}
			return nil
		})
		if err != nil {
			t.Errorf("failed to walk rhivos/%s/src/: %v", crate, err)
			continue
		}
		if !testFound {
			t.Errorf("rhivos/%s/ should contain at least one #[test] function", crate)
		}
	}
}

// --- TS-01-27: Go modules have placeholder tests ---
// Requirement: 01-REQ-8.2
func TestGoModulesHavePlaceholderTests(t *testing.T) {
	root := repoRoot(t)

	modules := []string{
		"backend/parking-fee-service",
		"backend/cloud-gateway",
		"mock/parking-app-cli",
		"mock/companion-app-cli",
		"mock/parking-operator",
	}

	for _, mod := range modules {
		modDir := filepath.Join(root, mod)
		if !pathExists(modDir) {
			t.Errorf("expected %s/ directory to exist", mod)
			continue
		}

		// Look for *_test.go files containing "func Test"
		entries, err := os.ReadDir(modDir)
		if err != nil {
			t.Errorf("failed to read %s/: %v", mod, err)
			continue
		}

		testFileFound := false
		testFuncFound := false
		for _, entry := range entries {
			if strings.HasSuffix(entry.Name(), "_test.go") {
				testFileFound = true
				content, readErr := readFileContent(filepath.Join(modDir, entry.Name()))
				if readErr != nil {
					continue
				}
				if strings.Contains(content, "func Test") {
					testFuncFound = true
				}
			}
		}

		if !testFileFound {
			t.Errorf("%s/ should contain at least one _test.go file", mod)
		}
		if !testFuncFound {
			t.Errorf("%s/ should contain at least one func Test* function", mod)
		}
	}
}
