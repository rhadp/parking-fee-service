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
	data, err := os.ReadFile(makefilePath)
	if err != nil {
		t.Fatalf("expected %s to exist: %v", makefilePath, err)
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
			// Match target definition at start of line: "target:" or "target: deps"
			pattern := `(?m)^` + regexp.QuoteMeta(target) + `\s*:`
			matched, err := regexp.MatchString(pattern, content)
			if err != nil {
				t.Fatalf("regex error: %v", err)
			}
			if !matched {
				t.Errorf("expected Makefile to define target %q", target)
			}
		})
	}
}
