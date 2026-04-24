package setup_test

import (
	"os"
	"path/filepath"
	"regexp"
	"testing"
)

// TS-01-18: Makefile has all required targets
// Requirement: 01-REQ-6.1
func TestMakefileHasRequiredTargets(t *testing.T) {
	root := repoRoot(t)

	makefilePath := filepath.Join(root, "Makefile")

	if !pathExists(makefilePath) {
		t.Fatalf("expected %s to exist", makefilePath)
	}

	data, err := os.ReadFile(makefilePath)
	if err != nil {
		t.Fatalf("failed to read %s: %v", makefilePath, err)
	}
	content := string(data)

	// Required targets from 01-REQ-6.1
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
		t.Run(target, func(t *testing.T) {
			// Match target definition: "target:" at the beginning of a line
			pattern := `(?m)^` + regexp.QuoteMeta(target) + `\s*:`
			matched, err := regexp.MatchString(pattern, content)
			if err != nil {
				t.Fatalf("regex error: %v", err)
			}
			if !matched {
				t.Errorf("Makefile should define target %q", target)
			}
		})
	}
}

// TS-01-18 (extended per 01-REQ-9.3): test-setup target exists
// While 01-REQ-6.1 does not list test-setup, 01-REQ-9.3 requires
// "make test-setup" to be runnable from the repository root.
func TestMakefileHasTestSetupTarget(t *testing.T) {
	root := repoRoot(t)

	makefilePath := filepath.Join(root, "Makefile")

	if !pathExists(makefilePath) {
		t.Fatalf("expected %s to exist", makefilePath)
	}

	data, err := os.ReadFile(makefilePath)
	if err != nil {
		t.Fatalf("failed to read %s: %v", makefilePath, err)
	}
	content := string(data)

	pattern := `(?m)^test-setup\s*:`
	matched, err := regexp.MatchString(pattern, content)
	if err != nil {
		t.Fatalf("regex error: %v", err)
	}
	if !matched {
		t.Errorf("Makefile should define target %q (required by 01-REQ-9.3)", "test-setup")
	}
}
