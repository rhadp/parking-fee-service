package setup_test

import (
	"os"
	"path/filepath"
	"regexp"
	"testing"
)

// TS-01-18: Makefile has all required targets
// Requirement: 01-REQ-6.1
func TestMakefileTargets(t *testing.T) {
	root := repoRoot(t)
	makefilePath := filepath.Join(root, "Makefile")

	data, err := os.ReadFile(makefilePath)
	if err != nil {
		t.Fatalf("failed to read %s: %v", makefilePath, err)
	}
	content := string(data)

	targets := []string{
		"build",
		"test",
		"clean",
		"proto",
		"infra-up",
		"infra-down",
		"check",
	}

	for _, target := range targets {
		t.Run(target, func(t *testing.T) {
			// Match target definition at the start of a line (e.g., "build:" or "build: deps")
			pattern := `(?m)^` + regexp.QuoteMeta(target) + `\s*:`
			matched, err := regexp.MatchString(pattern, content)
			if err != nil {
				t.Fatalf("regex error: %v", err)
			}
			if !matched {
				t.Errorf("Makefile does not define target %q", target)
			}
		})
	}
}
