package setup

import (
	"os"
	"regexp"
	"testing"
)

// TestMakefileHasRequiredTargets verifies the root Makefile declares all required targets.
// Test Spec: TS-01-18
// Requirement: 01-REQ-6.1
func TestMakefileHasRequiredTargets(t *testing.T) {
	root := findRepoRoot(t)

	makefilePath := repoPath(root, "Makefile")
	assertFileExists(t, makefilePath)

	data, err := os.ReadFile(makefilePath)
	if err != nil {
		t.Fatalf("failed to read Makefile: %v", err)
	}
	content := string(data)

	// Each target must appear as a Make target definition (target:)
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
			// Match target at start of line followed by colon (Make target syntax).
			pattern := `(?m)^` + regexp.QuoteMeta(target) + `\s*:`
			matched, err := regexp.MatchString(pattern, content)
			if err != nil {
				t.Fatalf("regex error for target %q: %v", target, err)
			}
			if !matched {
				t.Fatalf("Makefile does not define target %q (expected pattern: %q)", target, target+":")
			}
		})
	}
}

// TestMakefileHasTestSetupTarget verifies the Makefile defines a test-setup target.
// Test Spec: TS-01-30 (partial — target existence)
// Requirement: 01-REQ-9.3
func TestMakefileHasTestSetupTarget(t *testing.T) {
	root := findRepoRoot(t)

	makefilePath := repoPath(root, "Makefile")
	assertFileExists(t, makefilePath)

	data, err := os.ReadFile(makefilePath)
	if err != nil {
		t.Fatalf("failed to read Makefile: %v", err)
	}
	content := string(data)

	pattern := `(?m)^test-setup\s*:`
	matched, err := regexp.MatchString(pattern, content)
	if err != nil {
		t.Fatalf("regex error: %v", err)
	}
	if !matched {
		t.Fatalf("Makefile does not define target %q", "test-setup")
	}
}
