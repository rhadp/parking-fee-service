package setup_test

import (
	"os"
	"path/filepath"
	"regexp"
	"testing"
)

// TestMakefileHasRequiredTargets verifies the root Makefile declares all
// required targets (TS-01-18, 01-REQ-6.1).
func TestMakefileHasRequiredTargets(t *testing.T) {
	root := repoRoot(t)
	makefile := filepath.Join(root, "Makefile")
	assertPathExists(t, makefile)

	data, err := os.ReadFile(makefile)
	if err != nil {
		t.Fatalf("cannot read Makefile: %v", err)
	}
	content := string(data)

	// Required targets per 01-REQ-6.1 plus test-setup per 01-REQ-9.3
	requiredTargets := []string{
		"build",
		"test",
		"clean",
		"proto",
		"infra-up",
		"infra-down",
		"check",
		"test-setup",
	}
	for _, target := range requiredTargets {
		pattern := `(?m)^` + regexp.QuoteMeta(target) + `:`
		matched, err := regexp.MatchString(pattern, content)
		if err != nil {
			t.Errorf("regex error for target %q: %v", target, err)
			continue
		}
		if !matched {
			t.Errorf("Makefile is missing required target %q", target)
		}
	}
}
