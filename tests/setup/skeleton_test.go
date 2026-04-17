package setup_test

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// goSkeletonModules maps directory path to the component name expected in version output.
var goSkeletonModules = map[string]string{
	"backend/parking-fee-service": "parking-fee-service",
	"backend/cloud-gateway":       "cloud-gateway",
	"mock/parking-app-cli":        "parking-app-cli",
	"mock/companion-app-cli":      "companion-app-cli",
	"mock/parking-operator":       "parking-operator",
}

// rustCrates lists all Rust crate directories under rhivos/.
var rustCrates = []string{
	"locking-service",
	"cloud-gateway-client",
	"update-service",
	"parking-operator-adaptor",
	"mock-sensors",
}

// TestRustSkeletonVersionStrings inspects each Rust crate's main.rs (or bin/*.rs)
// and verifies it contains the component name and "v0.1.0" version string.
// This covers TS-01-13, TS-01-15 (01-REQ-4.1, 01-REQ-4.3, 01-REQ-4.4).
func TestRustSkeletonVersionStrings(t *testing.T) {
	root := repoRoot(t)

	type rustBin struct {
		path string // relative path under repo root
		name string // expected name in version string
	}

	bins := []rustBin{
		{"rhivos/locking-service/src/main.rs", "locking-service"},
		{"rhivos/cloud-gateway-client/src/main.rs", "cloud-gateway-client"},
		{"rhivos/update-service/src/main.rs", "update-service"},
		{"rhivos/parking-operator-adaptor/src/main.rs", "parking-operator-adaptor"},
		{"rhivos/mock-sensors/src/bin/location-sensor.rs", "location-sensor"},
		{"rhivos/mock-sensors/src/bin/speed-sensor.rs", "speed-sensor"},
		{"rhivos/mock-sensors/src/bin/door-sensor.rs", "door-sensor"},
	}

	for _, b := range bins {
		t.Run(b.name, func(t *testing.T) {
			fullPath := filepath.Join(root, b.path)
			data, err := os.ReadFile(fullPath)
			if err != nil {
				t.Fatalf("cannot read %s: %v", b.path, err)
			}
			content := string(data)
			if !strings.Contains(content, b.name) {
				t.Errorf("%s: expected component name %q in source", b.path, b.name)
			}
			if !strings.Contains(content, "v0.1.0") {
				t.Errorf("%s: expected version string \"v0.1.0\" in source", b.path)
			}
			if !strings.Contains(content, "println!") {
				t.Errorf("%s: expected println! macro (stdout output) in source", b.path)
			}
		})
	}
}

// TestGoSkeletonVersionStrings inspects each Go module's main.go and verifies
// it contains the component name. Covers TS-01-14 (01-REQ-4.2, 01-REQ-4.4).
func TestGoSkeletonVersionStrings(t *testing.T) {
	root := repoRoot(t)
	for modPath, componentName := range goSkeletonModules {
		t.Run(componentName, func(t *testing.T) {
			mainGoPath := filepath.Join(root, modPath, "main.go")
			data, err := os.ReadFile(mainGoPath)
			if err != nil {
				t.Fatalf("cannot read %s/main.go: %v", modPath, err)
			}
			content := string(data)
			if !strings.Contains(content, componentName) {
				t.Errorf("%s/main.go: expected component name %q in source", modPath, componentName)
			}
		})
	}
}

// TestRustCratesHavePlaceholderTests verifies each Rust crate has at least one
// #[test] annotation in its source files. Covers TS-01-26 (01-REQ-8.1).
func TestRustCratesHavePlaceholderTests(t *testing.T) {
	root := repoRoot(t)
	for _, crate := range rustCrates {
		t.Run(crate, func(t *testing.T) {
			crateDir := filepath.Join(root, "rhivos", crate, "src")
			found := false
			err := filepath.Walk(crateDir, func(path string, info os.FileInfo, err error) error {
				if err != nil || found {
					return err
				}
				if !info.IsDir() && strings.HasSuffix(path, ".rs") {
					data, readErr := os.ReadFile(path)
					if readErr != nil {
						return readErr
					}
					if strings.Contains(string(data), "#[test]") {
						found = true
					}
				}
				return nil
			})
			if err != nil {
				t.Fatalf("walk error for crate %s: %v", crate, err)
			}
			if !found {
				t.Errorf("crate %s: no #[test] annotation found in src/", crate)
			}
		})
	}
}

// TestGoModulesHavePlaceholderTests verifies each Go module has at least one
// test function in its *_test.go files. Covers TS-01-27 (01-REQ-8.2).
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
			modDir := filepath.Join(root, mod)
			found := false
			err := filepath.Walk(modDir, func(path string, info os.FileInfo, err error) error {
				if err != nil || found {
					return err
				}
				if !info.IsDir() && strings.HasSuffix(path, "_test.go") {
					data, readErr := os.ReadFile(path)
					if readErr != nil {
						return readErr
					}
					if strings.Contains(string(data), "func Test") {
						found = true
					}
				}
				return nil
			})
			if err != nil {
				t.Fatalf("walk error for module %s: %v", mod, err)
			}
			if !found {
				t.Errorf("module %s: no func Test found in *_test.go files", mod)
			}
		})
	}
}

// TestRustSkeletonsHandleUnknownFlags verifies each Rust skeleton's source
// contains flag-rejection logic for args starting with '-'.
// Covers TS-01-E4 (01-REQ-4.E1).
func TestRustSkeletonsHandleUnknownFlags(t *testing.T) {
	root := repoRoot(t)

	type rustBin struct {
		path string
		name string
	}

	bins := []rustBin{
		{"rhivos/locking-service/src/main.rs", "locking-service"},
		{"rhivos/cloud-gateway-client/src/main.rs", "cloud-gateway-client"},
		{"rhivos/update-service/src/main.rs", "update-service"},
		{"rhivos/parking-operator-adaptor/src/main.rs", "parking-operator-adaptor"},
	}

	for _, b := range bins {
		t.Run(b.name, func(t *testing.T) {
			fullPath := filepath.Join(root, b.path)
			data, err := os.ReadFile(fullPath)
			if err != nil {
				t.Fatalf("cannot read %s: %v", b.path, err)
			}
			content := string(data)
			// Source must contain flag rejection (starts_with('-') or similar pattern)
			// and eprintln! (stderr) with process::exit.
			hasFlagCheck := strings.Contains(content, "starts_with('-')") ||
				strings.Contains(content, "starts_with(\"-\")")
			if !hasFlagCheck {
				t.Errorf("%s: expected flag-rejection logic (starts_with('-')) in source", b.path)
			}
			if !strings.Contains(content, "eprintln!") {
				t.Errorf("%s: expected eprintln! (stderr usage message) in source", b.path)
			}
			if !strings.Contains(content, "process::exit") {
				t.Errorf("%s: expected process::exit with non-zero code in source", b.path)
			}
		})
	}
}
