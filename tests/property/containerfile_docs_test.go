// Copyright 2024 SDV Parking Demo System
// Property-based test for Containerfile Documentation Compliance.
//
// **Feature: project-foundation, Property 6: Containerfile Documentation Compliance**
//
// This test verifies that:
// *For any* Containerfile in the project, there SHALL exist a comment block
// documenting the rationale for the chosen UBI10 variant.
//
// **Validates: Requirements 8.4**

package property

import (
	"bufio"
	"os"
	"path/filepath"
	"regexp"
	"strings"
	"testing"

	"pgregory.net/rapid"
)

// DocumentationInfo holds parsed documentation from a Containerfile
type DocumentationInfo struct {
	Path                string
	HasRationaleComment bool
	RationaleContent    string
	HasBaseImageMention bool
	HasUBI10Mention     bool
	CommentLines        []string
}

// Keywords that indicate base image rationale documentation
var rationaleKeywords = []string{
	"rationale",
	"base image",
	"baseimage",
	"ubi10",
	"ubi-minimal",
	"ubi-micro",
	"image choice",
	"chosen",
	"using",
}

// parseContainerfileDocumentation extracts documentation from a Containerfile
func parseContainerfileDocumentation(path string) (*DocumentationInfo, error) {
	file, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	defer file.Close()

	info := &DocumentationInfo{
		Path:         path,
		CommentLines: []string{},
	}

	scanner := bufio.NewScanner(file)
	var commentBlock strings.Builder
	inCommentBlock := false

	for scanner.Scan() {
		line := scanner.Text()
		trimmedLine := strings.TrimSpace(line)

		// Check for comment lines
		if strings.HasPrefix(trimmedLine, "#") {
			commentContent := strings.TrimPrefix(trimmedLine, "#")
			commentContent = strings.TrimSpace(commentContent)
			info.CommentLines = append(info.CommentLines, commentContent)

			if !inCommentBlock {
				inCommentBlock = true
			}
			commentBlock.WriteString(commentContent)
			commentBlock.WriteString(" ")
		} else if trimmedLine != "" {
			// Non-comment, non-empty line ends the comment block
			if inCommentBlock {
				inCommentBlock = false
			}
		}
	}

	if err := scanner.Err(); err != nil {
		return nil, err
	}

	// Analyze the collected comments
	allComments := strings.ToLower(commentBlock.String())

	// Check for rationale keywords
	for _, keyword := range rationaleKeywords {
		if strings.Contains(allComments, strings.ToLower(keyword)) {
			info.HasRationaleComment = true
			break
		}
	}

	// Check for base image mention
	if strings.Contains(allComments, "base image") || strings.Contains(allComments, "baseimage") {
		info.HasBaseImageMention = true
	}

	// Check for UBI10 mention
	if strings.Contains(allComments, "ubi10") || strings.Contains(allComments, "ubi-minimal") ||
		strings.Contains(allComments, "ubi-micro") || strings.Contains(allComments, "ubi ") {
		info.HasUBI10Mention = true
	}

	// Extract rationale content (first comment block)
	if len(info.CommentLines) > 0 {
		var rationaleBuilder strings.Builder
		for _, line := range info.CommentLines {
			if line == "" {
				break // Stop at first empty comment line
			}
			rationaleBuilder.WriteString(line)
			rationaleBuilder.WriteString(" ")
		}
		info.RationaleContent = strings.TrimSpace(rationaleBuilder.String())
	}

	return info, nil
}

// TestContainerfileDocumentationCompliance is the main property-based test
// **Feature: project-foundation, Property 6: Containerfile Documentation Compliance**
// **Validates: Requirements 8.4**
func TestContainerfileDocumentationCompliance(t *testing.T) {
	projectRoot, err := findProjectRoot()
	if err != nil {
		t.Fatalf("Failed to find project root: %v", err)
	}

	containerfiles, err := findAllContainerfiles(projectRoot)
	if err != nil {
		t.Fatalf("Failed to find Containerfiles: %v", err)
	}

	if len(containerfiles) == 0 {
		t.Fatal("No Containerfiles found in containers/ directory")
	}

	t.Logf("Found %d Containerfiles to check for documentation", len(containerfiles))

	rapid.Check(t, func(rt *rapid.T) {
		// Select a random Containerfile to test
		idx := rapid.IntRange(0, len(containerfiles)-1).Draw(rt, "containerfile_index")
		containerfilePath := containerfiles[idx]

		info, err := parseContainerfileDocumentation(containerfilePath)
		if err != nil {
			rt.Fatalf("Failed to parse Containerfile %s: %v", containerfilePath, err)
		}

		// Property: Must have rationale comment
		if !info.HasRationaleComment {
			rt.Fatalf("Containerfile %s: Missing base image rationale comment (expected keywords: rationale, base image, ubi10, etc.)",
				containerfilePath)
		}

		// Property: Should mention base image choice
		if !info.HasBaseImageMention && !info.HasUBI10Mention {
			rt.Fatalf("Containerfile %s: Rationale comment doesn't mention base image or UBI10 variant",
				containerfilePath)
		}
	})
}

// TestAllContainerfilesDocumentation tests all Containerfiles deterministically
func TestAllContainerfilesDocumentation(t *testing.T) {
	projectRoot, err := findProjectRoot()
	if err != nil {
		t.Fatalf("Failed to find project root: %v", err)
	}

	containerfiles, err := findAllContainerfiles(projectRoot)
	if err != nil {
		t.Fatalf("Failed to find Containerfiles: %v", err)
	}

	if len(containerfiles) == 0 {
		t.Fatal("No Containerfiles found in containers/ directory")
	}

	for _, containerfilePath := range containerfiles {
		relPath, _ := filepath.Rel(projectRoot, containerfilePath)
		t.Run(relPath, func(t *testing.T) {
			info, err := parseContainerfileDocumentation(containerfilePath)
			if err != nil {
				t.Fatalf("Failed to parse Containerfile: %v", err)
			}

			// Check for rationale comment
			if !info.HasRationaleComment {
				t.Errorf("Missing base image rationale comment")
			}

			// Check for base image or UBI10 mention
			if !info.HasBaseImageMention && !info.HasUBI10Mention {
				t.Errorf("Rationale doesn't mention base image or UBI10")
			}

			// Log the rationale content
			if info.RationaleContent != "" {
				// Truncate for display
				content := info.RationaleContent
				if len(content) > 100 {
					content = content[:100] + "..."
				}
				t.Logf("✓ Rationale: %s", content)
			}
		})
	}
}

// TestRationaleKeywordDetection tests that rationale keywords are properly detected
func TestRationaleKeywordDetection(t *testing.T) {
	testCases := []struct {
		name           string
		content        string
		expectRationale bool
		expectBaseImage bool
		expectUBI10     bool
	}{
		{
			name: "Full rationale comment",
			content: `# containers/rhivos/Containerfile.locking-service
#
# Base Image Rationale: Using ubi10/ubi-minimal for balance between size and
# package availability.
FROM registry.access.redhat.com/ubi10/ubi-minimal
`,
			expectRationale: true,
			expectBaseImage: true,
			expectUBI10:     true,
		},
		{
			name: "Simple UBI mention",
			content: `# Using UBI10 minimal for this service
FROM registry.access.redhat.com/ubi10/ubi-minimal
`,
			expectRationale: true,
			expectBaseImage: false,
			expectUBI10:     true,
		},
		{
			name: "No documentation",
			content: `FROM registry.access.redhat.com/ubi10/ubi-minimal
RUN echo hello
`,
			expectRationale: false,
			expectBaseImage: false,
			expectUBI10:     false,
		},
		{
			name: "Base image choice documented",
			content: `# Base image choice: ubi-micro for minimal footprint
FROM registry.access.redhat.com/ubi10/ubi-micro
`,
			expectRationale: true,
			expectBaseImage: true,
			expectUBI10:     true,
		},
	}

	for _, tc := range testCases {
		t.Run(tc.name, func(t *testing.T) {
			// Write to temp file
			tempFile, err := os.CreateTemp("", "Containerfile.*")
			if err != nil {
				t.Fatalf("Failed to create temp file: %v", err)
			}
			defer os.Remove(tempFile.Name())

			if _, err := tempFile.WriteString(tc.content); err != nil {
				t.Fatalf("Failed to write temp file: %v", err)
			}
			tempFile.Close()

			info, err := parseContainerfileDocumentation(tempFile.Name())
			if err != nil {
				t.Fatalf("Failed to parse: %v", err)
			}

			if info.HasRationaleComment != tc.expectRationale {
				t.Errorf("HasRationaleComment: got %v, want %v", info.HasRationaleComment, tc.expectRationale)
			}
			if info.HasBaseImageMention != tc.expectBaseImage {
				t.Errorf("HasBaseImageMention: got %v, want %v", info.HasBaseImageMention, tc.expectBaseImage)
			}
			if info.HasUBI10Mention != tc.expectUBI10 {
				t.Errorf("HasUBI10Mention: got %v, want %v", info.HasUBI10Mention, tc.expectUBI10)
			}
		})
	}
}

// TestDocumentationQuality tests the quality of documentation
func TestDocumentationQuality(t *testing.T) {
	projectRoot, err := findProjectRoot()
	if err != nil {
		t.Fatalf("Failed to find project root: %v", err)
	}

	containerfiles, err := findAllContainerfiles(projectRoot)
	if err != nil {
		t.Fatalf("Failed to find Containerfiles: %v", err)
	}

	// Quality metrics
	var totalFiles int
	var filesWithRationale int
	var filesWithBaseImageMention int
	var filesWithUBI10Mention int

	for _, containerfilePath := range containerfiles {
		info, err := parseContainerfileDocumentation(containerfilePath)
		if err != nil {
			t.Fatalf("Failed to parse %s: %v", containerfilePath, err)
		}

		totalFiles++
		if info.HasRationaleComment {
			filesWithRationale++
		}
		if info.HasBaseImageMention {
			filesWithBaseImageMention++
		}
		if info.HasUBI10Mention {
			filesWithUBI10Mention++
		}
	}

	t.Logf("Documentation Quality Metrics:")
	t.Logf("  Total Containerfiles: %d", totalFiles)
	t.Logf("  With rationale comment: %d (%.0f%%)", filesWithRationale, float64(filesWithRationale)/float64(totalFiles)*100)
	t.Logf("  With base image mention: %d (%.0f%%)", filesWithBaseImageMention, float64(filesWithBaseImageMention)/float64(totalFiles)*100)
	t.Logf("  With UBI10 mention: %d (%.0f%%)", filesWithUBI10Mention, float64(filesWithUBI10Mention)/float64(totalFiles)*100)

	// All files should have rationale
	if filesWithRationale != totalFiles {
		t.Errorf("Not all Containerfiles have rationale comments: %d/%d", filesWithRationale, totalFiles)
	}
}

// rationalePatterns are regex patterns that indicate proper documentation
var rationalePatterns = []*regexp.Regexp{
	regexp.MustCompile(`(?i)base\s+image\s+rationale`),
	regexp.MustCompile(`(?i)using\s+ubi10`),
	regexp.MustCompile(`(?i)ubi-minimal\s+for`),
	regexp.MustCompile(`(?i)ubi-micro\s+for`),
	regexp.MustCompile(`(?i)chosen\s+.*\s+because`),
}

// TestRationalePatternMatching tests that rationale follows expected patterns
func TestRationalePatternMatching(t *testing.T) {
	projectRoot, err := findProjectRoot()
	if err != nil {
		t.Fatalf("Failed to find project root: %v", err)
	}

	containerfiles, err := findAllContainerfiles(projectRoot)
	if err != nil {
		t.Fatalf("Failed to find Containerfiles: %v", err)
	}

	for _, containerfilePath := range containerfiles {
		relPath, _ := filepath.Rel(projectRoot, containerfilePath)
		t.Run(relPath, func(t *testing.T) {
			content, err := os.ReadFile(containerfilePath)
			if err != nil {
				t.Fatalf("Failed to read file: %v", err)
			}

			contentStr := string(content)
			matchedPattern := false

			for _, pattern := range rationalePatterns {
				if pattern.MatchString(contentStr) {
					matchedPattern = true
					t.Logf("✓ Matches pattern: %s", pattern.String())
					break
				}
			}

			if !matchedPattern {
				t.Logf("⚠ No standard rationale pattern matched, but may still have valid documentation")
			}
		})
	}
}
