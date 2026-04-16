package setup_test

import (
	"os"
	"path/filepath"
	"regexp"
	"testing"
)

// TestMakefileHasRequiredTargets verifies that the root Makefile declares all
// required build and test targets.
// Test Spec: TS-01-18
// Requirements: 01-REQ-6.1
func TestMakefileHasRequiredTargets(t *testing.T) {
	root := repoRoot(t)
	makefilePath := filepath.Join(root, "Makefile")

	data, err := os.ReadFile(makefilePath)
	if err != nil {
		t.Fatalf("cannot read Makefile: %v", err)
	}
	content := string(data)

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
			// Match "target:" at the start of a line (Makefile target definition)
			pattern := `(?m)^` + regexp.QuoteMeta(target) + `:`
			matched, err := regexp.MatchString(pattern, content)
			if err != nil {
				t.Fatalf("regexp error: %v", err)
			}
			if !matched {
				t.Errorf("Makefile does not define target %q (expected pattern %q)", target, pattern)
			}
		})
	}
}
