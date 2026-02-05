// Copyright 2024 SDV Parking Demo System
// Property-based test for Documentation Directory Coverage.
//
// **Feature: project-foundation, Property 4: Documentation Directory Coverage**
//
// This test verifies that:
// *For any* major directory in the project structure (rhivos/, android/, backend/,
// proto/, infra/, containers/), there SHALL exist a README.md file that describes
// the directory's purpose and contents.
//
// **Validates: Requirements 6.2**

package property

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	"pgregory.net/rapid"
)

// MajorDirectories defines the directories that must have README.md files
var MajorDirectories = []string{
	"rhivos",
	"android",
	"backend",
	"proto",
	"infra",
	"containers",
}

// DirectoryInfo holds information about a directory's documentation status
type DirectoryInfo struct {
	Path          string
	HasReadme     bool
	ReadmeContent string
	ContentLength int
}

// checkDirectoryReadme checks if a directory has a README.md file with content
func checkDirectoryReadme(projectRoot, dirName string) (*DirectoryInfo, error) {
	dirPath := filepath.Join(projectRoot, dirName)
	readmePath := filepath.Join(dirPath, "README.md")

	info := &DirectoryInfo{
		Path:      dirPath,
		HasReadme: false,
	}

	// Check if directory exists
	if _, err := os.Stat(dirPath); os.IsNotExist(err) {
		return info, nil
	}

	// Check if README.md exists
	content, err := os.ReadFile(readmePath)
	if err != nil {
		if os.IsNotExist(err) {
			return info, nil
		}
		return nil, err
	}

	info.HasReadme = true
	info.ReadmeContent = string(content)
	info.ContentLength = len(strings.TrimSpace(string(content)))

	return info, nil
}

// isReadmeContentMeaningful checks if README content is meaningful (not just a placeholder)
func isReadmeContentMeaningful(content string) bool {
	trimmed := strings.TrimSpace(content)
	
	// Must have at least 50 characters of content
	if len(trimmed) < 50 {
		return false
	}

	// Must have a heading (starts with #)
	if !strings.HasPrefix(trimmed, "#") {
		return false
	}

	// Must have multiple lines
	lines := strings.Split(trimmed, "\n")
	nonEmptyLines := 0
	for _, line := range lines {
		if strings.TrimSpace(line) != "" {
			nonEmptyLines++
		}
	}
	
	return nonEmptyLines >= 5
}

// TestDocumentationDirectoryCoverage is the main property-based test
// **Feature: project-foundation, Property 4: Documentation Directory Coverage**
// **Validates: Requirements 6.2**
func TestDocumentationDirectoryCoverage(t *testing.T) {
	projectRoot, err := findProjectRoot()
	if err != nil {
		t.Fatalf("Failed to find project root: %v", err)
	}

	t.Logf("Checking %d major directories for README.md files", len(MajorDirectories))

	rapid.Check(t, func(rt *rapid.T) {
		// Select a random major directory to test
		idx := rapid.IntRange(0, len(MajorDirectories)-1).Draw(rt, "directory_index")
		dirName := MajorDirectories[idx]

		info, err := checkDirectoryReadme(projectRoot, dirName)
		if err != nil {
			rt.Fatalf("Failed to check directory %s: %v", dirName, err)
		}

		// Property: Directory must have a README.md file
		if !info.HasReadme {
			rt.Fatalf("Directory '%s' does not have a README.md file", dirName)
		}

		// Property: README.md must have meaningful content
		if !isReadmeContentMeaningful(info.ReadmeContent) {
			rt.Fatalf("Directory '%s' has README.md but content is not meaningful (length: %d)",
				dirName, info.ContentLength)
		}
	})
}

// TestAllMajorDirectoriesHaveReadme tests all major directories deterministically
func TestAllMajorDirectoriesHaveReadme(t *testing.T) {
	projectRoot, err := findProjectRoot()
	if err != nil {
		t.Fatalf("Failed to find project root: %v", err)
	}

	for _, dirName := range MajorDirectories {
		t.Run(dirName, func(t *testing.T) {
			info, err := checkDirectoryReadme(projectRoot, dirName)
			if err != nil {
				t.Fatalf("Failed to check directory: %v", err)
			}

			// Check README exists
			if !info.HasReadme {
				t.Errorf("Directory '%s' does not have a README.md file", dirName)
				return
			}

			// Check README has meaningful content
			if !isReadmeContentMeaningful(info.ReadmeContent) {
				t.Errorf("Directory '%s' has README.md but content is not meaningful (length: %d)",
					dirName, info.ContentLength)
				return
			}

			t.Logf("✓ %s/README.md exists with %d characters", dirName, info.ContentLength)
		})
	}
}

// TestReadmeContentQuality tests the quality of README content
func TestReadmeContentQuality(t *testing.T) {
	projectRoot, err := findProjectRoot()
	if err != nil {
		t.Fatalf("Failed to find project root: %v", err)
	}

	for _, dirName := range MajorDirectories {
		t.Run(dirName, func(t *testing.T) {
			info, err := checkDirectoryReadme(projectRoot, dirName)
			if err != nil {
				t.Fatalf("Failed to check directory: %v", err)
			}

			if !info.HasReadme {
				t.Skip("README.md does not exist")
				return
			}

			content := info.ReadmeContent

			// Check for heading
			if !strings.HasPrefix(strings.TrimSpace(content), "#") {
				t.Errorf("README.md should start with a heading")
			}

			// Check for directory structure section (recommended)
			hasStructure := strings.Contains(content, "Structure") ||
				strings.Contains(content, "structure") ||
				strings.Contains(content, "```")
			if !hasStructure {
				t.Logf("Note: README.md could benefit from a directory structure section")
			}

			// Check for description
			lines := strings.Split(content, "\n")
			hasDescription := false
			for i, line := range lines {
				// Skip heading line, look for description in first few lines
				if i > 0 && i < 5 && len(strings.TrimSpace(line)) > 20 {
					hasDescription = true
					break
				}
			}
			if !hasDescription {
				t.Logf("Note: README.md could benefit from a description after the heading")
			}

			t.Logf("✓ Content quality check passed for %s/README.md", dirName)
		})
	}
}

// TestRootReadmeExists tests that the root README.md exists
func TestRootReadmeExists(t *testing.T) {
	projectRoot, err := findProjectRoot()
	if err != nil {
		t.Fatalf("Failed to find project root: %v", err)
	}

	readmePath := filepath.Join(projectRoot, "README.md")
	content, err := os.ReadFile(readmePath)
	if err != nil {
		if os.IsNotExist(err) {
			t.Fatal("Root README.md does not exist")
		}
		t.Fatalf("Failed to read root README.md: %v", err)
	}

	if !isReadmeContentMeaningful(string(content)) {
		t.Error("Root README.md exists but content is not meaningful")
	}

	t.Logf("✓ Root README.md exists with %d characters", len(content))
}

// TestDocsDirectoryExists tests that the docs directory has setup guides
func TestDocsDirectoryExists(t *testing.T) {
	projectRoot, err := findProjectRoot()
	if err != nil {
		t.Fatalf("Failed to find project root: %v", err)
	}

	expectedDocs := []string{
		"docs/setup-rust.md",
		"docs/setup-android.md",
		"docs/setup-flutter.md",
		"docs/setup-go.md",
		"docs/local-infrastructure.md",
	}

	for _, docPath := range expectedDocs {
		t.Run(docPath, func(t *testing.T) {
			fullPath := filepath.Join(projectRoot, docPath)
			content, err := os.ReadFile(fullPath)
			if err != nil {
				if os.IsNotExist(err) {
					t.Errorf("Documentation file '%s' does not exist", docPath)
					return
				}
				t.Fatalf("Failed to read '%s': %v", docPath, err)
			}

			if len(strings.TrimSpace(string(content))) < 100 {
				t.Errorf("Documentation file '%s' has insufficient content", docPath)
				return
			}

			t.Logf("✓ %s exists with %d characters", docPath, len(content))
		})
	}
}

// Generator for random directory names (for fuzz testing)
func genDirectoryName() *rapid.Generator[string] {
	return rapid.SampledFrom(MajorDirectories)
}

// TestRandomDirectoryDocumentation tests random directories have documentation
func TestRandomDirectoryDocumentation(t *testing.T) {
	projectRoot, err := findProjectRoot()
	if err != nil {
		t.Fatalf("Failed to find project root: %v", err)
	}

	rapid.Check(t, func(rt *rapid.T) {
		dirName := genDirectoryName().Draw(rt, "directory")

		info, err := checkDirectoryReadme(projectRoot, dirName)
		if err != nil {
			rt.Fatalf("Failed to check directory %s: %v", dirName, err)
		}

		// Property: Every major directory must have documentation
		if !info.HasReadme {
			rt.Fatalf("Property violation: Directory '%s' lacks README.md documentation", dirName)
		}

		// Property: Documentation must be meaningful
		if info.ContentLength < 50 {
			rt.Fatalf("Property violation: Directory '%s' README.md is too short (%d chars)",
				dirName, info.ContentLength)
		}
	})
}
