package setup_test

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// TestRustCratesHavePlaceholderTests verifies that each Rust crate has at least
// one unit test (identified by a #[test] annotation in the source files).
// Test Spec: TS-01-26
// Requirements: 01-REQ-8.1
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
		t.Run(crate, func(t *testing.T) {
			t.Helper()
			srcDir := filepath.Join(root, "rhivos", crate, "src")
			found := findInDir(t, srcDir, "#[test]")
			if !found {
				t.Errorf("no #[test] annotation found in rhivos/%s/src/**/*.rs", crate)
			}
		})
	}
}

// TestGoModulesHavePlaceholderTests verifies that each Go module (except tests/setup)
// has at least one test file with a func Test function.
// Test Spec: TS-01-27
// Requirements: 01-REQ-8.2
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
		t.Run(mod, func(t *testing.T) {
			t.Helper()
			modDir := filepath.Join(root, mod)
			found := findTestFuncInDir(t, modDir)
			if !found {
				t.Errorf("no func Test... found in %s/*_test.go", mod)
			}
		})
	}
}

// TestRustSkeletonsPrintVersion verifies that each Rust skeleton binary source
// contains the component name (version string) in its main.rs.
// Test Spec: TS-01-13
// Requirements: 01-REQ-4.1, 01-REQ-4.4
func TestRustSkeletonsPrintVersion(t *testing.T) {
	root := repoRoot(t)

	binaries := []struct {
		crate string
		name  string
		file  string
	}{
		{"locking-service", "locking-service", "src/main.rs"},
		{"cloud-gateway-client", "cloud-gateway-client", "src/main.rs"},
		{"update-service", "update-service", "src/main.rs"},
		{"parking-operator-adaptor", "parking-operator-adaptor", "src/main.rs"},
		{"mock-sensors", "location-sensor", "src/bin/location_sensor.rs"},
		{"mock-sensors", "speed-sensor", "src/bin/speed_sensor.rs"},
		{"mock-sensors", "door-sensor", "src/bin/door_sensor.rs"},
	}

	for _, b := range binaries {
		t.Run(b.name, func(t *testing.T) {
			path := filepath.Join(root, "rhivos", b.crate, b.file)
			data, err := os.ReadFile(path)
			if err != nil {
				t.Fatalf("cannot read %s: %v", path, err)
			}
			content := string(data)
			if !strings.Contains(content, b.name) {
				t.Errorf("file %s does not contain component name %q in version string", path, b.name)
			}
		})
	}
}

// TestRustSkeletonsHandleUnknownFlags verifies that each Rust skeleton source
// checks for flag arguments (starting with '-') and rejects them.
// Test Spec: TS-01-E4
// Requirements: 01-REQ-4.E1
func TestRustSkeletonsHandleUnknownFlags(t *testing.T) {
	root := repoRoot(t)

	sources := []struct {
		crate string
		file  string
	}{
		{"locking-service", "src/main.rs"},
		{"cloud-gateway-client", "src/main.rs"},
		{"update-service", "src/main.rs"},
		{"parking-operator-adaptor", "src/main.rs"},
		{"mock-sensors", "src/bin/location_sensor.rs"},
		{"mock-sensors", "src/bin/speed_sensor.rs"},
		{"mock-sensors", "src/bin/door_sensor.rs"},
	}

	for _, s := range sources {
		t.Run(s.crate+"/"+s.file, func(t *testing.T) {
			path := filepath.Join(root, "rhivos", s.crate, s.file)
			data, err := os.ReadFile(path)
			if err != nil {
				t.Fatalf("cannot read %s: %v", path, err)
			}
			content := string(data)
			// Source must check for flags and call eprintln! + process::exit
			if !strings.Contains(content, "starts_with('-')") {
				t.Errorf("file %s does not check for unknown flags (starts_with('-'))", path)
			}
			if !strings.Contains(content, "eprintln!") {
				t.Errorf("file %s does not print usage to stderr (eprintln!)", path)
			}
			if !strings.Contains(content, "process::exit(1)") {
				t.Errorf("file %s does not exit non-zero on unknown flag (process::exit(1))", path)
			}
		})
	}
}

// TestGoSkeletonsPrintVersion verifies that each Go skeleton's main.go contains
// the component name in its version string.
// Test Spec: TS-01-14
// Requirements: 01-REQ-4.2, 01-REQ-4.4
func TestGoSkeletonsPrintVersion(t *testing.T) {
	root := repoRoot(t)

	modules := []struct {
		path string
		name string
	}{
		{"backend/parking-fee-service", "parking-fee-service"},
		{"backend/cloud-gateway", "cloud-gateway"},
		{"mock/parking-app-cli", "parking-app-cli"},
		{"mock/companion-app-cli", "companion-app-cli"},
	}

	for _, m := range modules {
		t.Run(m.path, func(t *testing.T) {
			mainPath := filepath.Join(root, m.path, "main.go")
			data, err := os.ReadFile(mainPath)
			if err != nil {
				t.Fatalf("cannot read %s: %v", mainPath, err)
			}
			content := string(data)
			if !strings.Contains(content, m.name) {
				t.Errorf("main.go in %s does not contain component name %q", m.path, m.name)
			}
		})
	}
}

// findInDir recursively searches for a substring in all .rs files under dir.
func findInDir(t *testing.T, dir string, substring string) bool {
	t.Helper()
	entries, err := os.ReadDir(dir)
	if err != nil {
		t.Logf("cannot read dir %s: %v", dir, err)
		return false
	}
	for _, entry := range entries {
		path := filepath.Join(dir, entry.Name())
		if entry.IsDir() {
			if findInDir(t, path, substring) {
				return true
			}
		} else if strings.HasSuffix(entry.Name(), ".rs") {
			data, err := os.ReadFile(path)
			if err != nil {
				continue
			}
			if strings.Contains(string(data), substring) {
				return true
			}
		}
	}
	return false
}

// findTestFuncInDir searches for a func Test... declaration in *_test.go files under dir.
func findTestFuncInDir(t *testing.T, dir string) bool {
	t.Helper()
	entries, err := os.ReadDir(dir)
	if err != nil {
		t.Logf("cannot read dir %s: %v", dir, err)
		return false
	}
	for _, entry := range entries {
		if entry.IsDir() {
			continue
		}
		if !strings.HasSuffix(entry.Name(), "_test.go") {
			continue
		}
		path := filepath.Join(dir, entry.Name())
		data, err := os.ReadFile(path)
		if err != nil {
			continue
		}
		if strings.Contains(string(data), "func Test") {
			return true
		}
	}
	return false
}
