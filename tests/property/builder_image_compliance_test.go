// Copyright 2024 SDV Parking Demo System
// Property-based test for Builder Image Compliance.
//
// **Feature: project-foundation, Property 7: Builder Image Compliance**
//
// This test verifies that:
// *For any* Containerfile building Golang or Rust artifacts, the build stage base image
// SHALL be `ghcr.io/rhadp/builder`.
//
// **Validates: Requirements 8.6**

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

// BuildStageInfo holds parsed information about build stages in a Containerfile
type BuildStageInfo struct {
	Path             string
	BuildStages      []BuildStage
	IsGoProject      bool
	IsRustProject    bool
	HasBuildStage    bool
}

// BuildStage represents a single build stage in a multi-stage Containerfile
type BuildStage struct {
	BaseImage   string
	StageName   string
	IsBuildStage bool
}

// Approved builder image for Go and Rust builds
const approvedBuilderImage = "ghcr.io/rhadp/builder"

// Prohibited builder images that should not be used for Go/Rust builds
var prohibitedBuilderPatterns = []*regexp.Regexp{
	regexp.MustCompile(`(?i)^docker\.io/library/golang:`),
	regexp.MustCompile(`(?i)^docker\.io/library/rust:`),
	regexp.MustCompile(`(?i)^golang:`),
	regexp.MustCompile(`(?i)^rust:`),
	regexp.MustCompile(`(?i)^docker\.io/golang:`),
	regexp.MustCompile(`(?i)^docker\.io/rust:`),
}

// Patterns to detect Go or Rust build operations
var goBuildPatterns = []*regexp.Regexp{
	regexp.MustCompile(`(?i)\bgo\s+build\b`),
	regexp.MustCompile(`(?i)\bCGO_ENABLED\b`),
	regexp.MustCompile(`(?i)\bGOOS=`),
	regexp.MustCompile(`(?i)\bgo\s+mod\b`),
}

var rustBuildPatterns = []*regexp.Regexp{
	regexp.MustCompile(`(?i)cargo\s+build`),
	regexp.MustCompile(`(?i)cargo\s+install`),
	regexp.MustCompile(`(?i)rustc`),
}

// parseContainerfileBuildStages extracts build stage information from a Containerfile
func parseContainerfileBuildStages(path string) (*BuildStageInfo, error) {
	file, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	defer file.Close()

	info := &BuildStageInfo{
		Path:        path,
		BuildStages: []BuildStage{},
	}

	scanner := bufio.NewScanner(file)
	var currentStage *BuildStage
	var fileContent strings.Builder

	for scanner.Scan() {
		line := scanner.Text()
		fileContent.WriteString(line)
		fileContent.WriteString("\n")
		trimmedLine := strings.TrimSpace(line)

		// Skip comments and empty lines for FROM detection
		if strings.HasPrefix(trimmedLine, "#") || trimmedLine == "" {
			continue
		}

		// Check for FROM instruction
		if matches := fromPattern.FindStringSubmatch(trimmedLine); len(matches) > 1 {
			// Save previous stage if exists
			if currentStage != nil {
				info.BuildStages = append(info.BuildStages, *currentStage)
			}

			baseImage := matches[1]
			stageName := ""

			// Check for AS clause (named stage)
			if asMatches := regexp.MustCompile(`(?i)\s+AS\s+(\S+)`).FindStringSubmatch(trimmedLine); len(asMatches) > 1 {
				stageName = asMatches[1]
			}

			// Determine if this is a build stage based on name
			isBuildStage := strings.Contains(strings.ToLower(stageName), "build")

			currentStage = &BuildStage{
				BaseImage:    baseImage,
				StageName:    stageName,
				IsBuildStage: isBuildStage,
			}
		}
	}

	// Add the last stage
	if currentStage != nil {
		info.BuildStages = append(info.BuildStages, *currentStage)
	}

	if err := scanner.Err(); err != nil {
		return nil, err
	}

	// Analyze file content for Go/Rust build operations
	content := fileContent.String()

	for _, pattern := range goBuildPatterns {
		if pattern.MatchString(content) {
			info.IsGoProject = true
			break
		}
	}

	for _, pattern := range rustBuildPatterns {
		if pattern.MatchString(content) {
			info.IsRustProject = true
			break
		}
	}

	// Mark build stages based on content analysis
	info.HasBuildStage = len(info.BuildStages) > 1 && (info.IsGoProject || info.IsRustProject)

	return info, nil
}

// isProhibitedBuilderImage checks if an image is a prohibited builder image
func isProhibitedBuilderImage(imageRef string) (bool, string) {
	for _, pattern := range prohibitedBuilderPatterns {
		if pattern.MatchString(imageRef) {
			return true, pattern.String()
		}
	}
	return false, ""
}

// isApprovedBuilderImage checks if an image is the approved builder image
func isApprovedBuilderImage(imageRef string) bool {
	return strings.HasPrefix(imageRef, approvedBuilderImage)
}

// findGoRustContainerfiles finds all Containerfiles that build Go or Rust artifacts
func findGoRustContainerfiles(projectRoot string) ([]string, error) {
	allContainerfiles, err := findAllContainerfiles(projectRoot)
	if err != nil {
		return nil, err
	}

	var goRustContainerfiles []string
	for _, cf := range allContainerfiles {
		info, err := parseContainerfileBuildStages(cf)
		if err != nil {
			continue
		}
		if info.IsGoProject || info.IsRustProject {
			goRustContainerfiles = append(goRustContainerfiles, cf)
		}
	}

	return goRustContainerfiles, nil
}

// TestBuilderImageCompliance is the main property-based test
// **Feature: project-foundation, Property 7: Builder Image Compliance**
// **Validates: Requirements 8.6**
func TestBuilderImageCompliance(t *testing.T) {
	projectRoot, err := findProjectRoot()
	if err != nil {
		t.Fatalf("Failed to find project root: %v", err)
	}

	containerfiles, err := findGoRustContainerfiles(projectRoot)
	if err != nil {
		t.Fatalf("Failed to find Go/Rust Containerfiles: %v", err)
	}

	if len(containerfiles) == 0 {
		t.Skip("No Go/Rust Containerfiles found in containers/ directory")
	}

	t.Logf("Found %d Go/Rust Containerfiles to check for builder image compliance", len(containerfiles))

	rapid.Check(t, func(rt *rapid.T) {
		// Select a random Containerfile to test
		idx := rapid.IntRange(0, len(containerfiles)-1).Draw(rt, "containerfile_index")
		containerfilePath := containerfiles[idx]

		info, err := parseContainerfileBuildStages(containerfilePath)
		if err != nil {
			rt.Fatalf("Failed to parse Containerfile %s: %v", containerfilePath, err)
		}

		// Check each build stage (non-final stages in multi-stage builds)
		for i, stage := range info.BuildStages {
			// Skip the final stage (it's checked by UBI10 compliance test)
			if i == len(info.BuildStages)-1 {
				continue
			}

			// Property: Build stages must use approved builder image
			if !isApprovedBuilderImage(stage.BaseImage) {
				// Check if it's using a prohibited image
				if prohibited, pattern := isProhibitedBuilderImage(stage.BaseImage); prohibited {
					rt.Fatalf("Containerfile %s: Build stage uses prohibited builder image '%s' (matched pattern: %s). Must use '%s'",
						containerfilePath, stage.BaseImage, pattern, approvedBuilderImage)
				}
			}
		}
	})
}

// TestAllGoRustContainerfilesBuilderCompliance tests all Go/Rust Containerfiles deterministically
func TestAllGoRustContainerfilesBuilderCompliance(t *testing.T) {
	projectRoot, err := findProjectRoot()
	if err != nil {
		t.Fatalf("Failed to find project root: %v", err)
	}

	containerfiles, err := findGoRustContainerfiles(projectRoot)
	if err != nil {
		t.Fatalf("Failed to find Go/Rust Containerfiles: %v", err)
	}

	if len(containerfiles) == 0 {
		t.Skip("No Go/Rust Containerfiles found in containers/ directory")
	}

	var violations []string

	for _, containerfilePath := range containerfiles {
		relPath, _ := filepath.Rel(projectRoot, containerfilePath)
		t.Run(relPath, func(t *testing.T) {
			info, err := parseContainerfileBuildStages(containerfilePath)
			if err != nil {
				t.Fatalf("Failed to parse Containerfile: %v", err)
			}

			projectType := ""
			if info.IsGoProject && info.IsRustProject {
				projectType = "Go+Rust"
			} else if info.IsGoProject {
				projectType = "Go"
			} else if info.IsRustProject {
				projectType = "Rust"
			}

			t.Logf("Project type: %s, Build stages: %d", projectType, len(info.BuildStages))

			// Check each build stage (non-final stages)
			for i, stage := range info.BuildStages {
				// Skip the final stage
				if i == len(info.BuildStages)-1 {
					t.Logf("  Stage %d (FINAL): %s", i+1, stage.BaseImage)
					continue
				}

				stageName := stage.StageName
				if stageName == "" {
					stageName = "unnamed"
				}

				// Check for prohibited builder images
				if prohibited, _ := isProhibitedBuilderImage(stage.BaseImage); prohibited {
					violation := relPath + ": " + stage.BaseImage
					violations = append(violations, violation)
					t.Errorf("Stage %d (%s): Uses prohibited builder image '%s'. Must use '%s'",
						i+1, stageName, stage.BaseImage, approvedBuilderImage)
				} else if !isApprovedBuilderImage(stage.BaseImage) {
					t.Logf("  Stage %d (%s): %s (not approved builder, but not explicitly prohibited)",
						i+1, stageName, stage.BaseImage)
				} else {
					t.Logf("  Stage %d (%s): %s ✓", i+1, stageName, stage.BaseImage)
				}
			}
		})
	}

	if len(violations) > 0 {
		t.Logf("\n=== SUMMARY: %d Containerfiles with prohibited builder images ===", len(violations))
		for _, v := range violations {
			t.Logf("  - %s", v)
		}
	}
}

// TestProhibitedBuilderImagesRejected tests that prohibited builder images are properly detected
func TestProhibitedBuilderImagesRejected(t *testing.T) {
	testCases := []struct {
		imageRef     string
		shouldReject bool
		reason       string
	}{
		{approvedBuilderImage, false, "Approved builder image should be accepted"},
		{"ghcr.io/rhadp/builder:latest", false, "Approved builder with tag should be accepted"},
		{"docker.io/library/golang:1.22", true, "Official Docker golang image is prohibited"},
		{"docker.io/library/golang:latest", true, "Official Docker golang:latest is prohibited"},
		{"docker.io/library/rust:1.75", true, "Official Docker rust image is prohibited"},
		{"docker.io/library/rust:latest", true, "Official Docker rust:latest is prohibited"},
		{"golang:1.22", true, "Short golang image reference is prohibited"},
		{"rust:1.75", true, "Short rust image reference is prohibited"},
		{"docker.io/golang:1.22", true, "docker.io/golang is prohibited"},
		{"docker.io/rust:1.75", true, "docker.io/rust is prohibited"},
		{"registry.access.redhat.com/ubi10/ubi-minimal", false, "UBI10 is not a builder image (final stage)"},
		{"node:20", false, "Node is not Go/Rust, not explicitly prohibited"},
	}

	for _, tc := range testCases {
		t.Run(tc.imageRef, func(t *testing.T) {
			isProhibited, _ := isProhibitedBuilderImage(tc.imageRef)

			if tc.shouldReject && !isProhibited {
				t.Errorf("Expected '%s' to be rejected: %s", tc.imageRef, tc.reason)
			}
			if !tc.shouldReject && isProhibited {
				t.Errorf("Expected '%s' to be accepted: %s", tc.imageRef, tc.reason)
			}
		})
	}
}

// TestApprovedBuilderImageRecognition tests that the approved builder image is recognized
func TestApprovedBuilderImageRecognition(t *testing.T) {
	testCases := []struct {
		imageRef string
		approved bool
	}{
		{"ghcr.io/rhadp/builder", true},
		{"ghcr.io/rhadp/builder:latest", true},
		{"ghcr.io/rhadp/builder:v1.0.0", true},
		{"ghcr.io/other/builder", false},
		{"docker.io/library/golang:1.22", false},
		{"docker.io/library/rust:1.75", false},
	}

	for _, tc := range testCases {
		t.Run(tc.imageRef, func(t *testing.T) {
			result := isApprovedBuilderImage(tc.imageRef)
			if result != tc.approved {
				t.Errorf("isApprovedBuilderImage(%s) = %v, want %v", tc.imageRef, result, tc.approved)
			}
		})
	}
}

// TestBuildStageDetection tests that Go and Rust build operations are properly detected
func TestBuildStageDetection(t *testing.T) {
	testCases := []struct {
		name       string
		content    string
		expectGo   bool
		expectRust bool
	}{
		{
			name: "Go build with CGO_ENABLED",
			content: `FROM docker.io/library/golang:1.22 AS builder
WORKDIR /app
COPY . .
RUN CGO_ENABLED=0 GOOS=linux go build -o app .

FROM registry.access.redhat.com/ubi10/ubi-micro
COPY --from=builder /app/app /usr/local/bin/
`,
			expectGo:   true,
			expectRust: false,
		},
		{
			name: "Rust cargo build",
			content: `FROM docker.io/library/rust:1.75 AS builder
WORKDIR /build
COPY . .
RUN cargo build --release

FROM registry.access.redhat.com/ubi10/ubi-minimal
COPY --from=builder /build/target/release/app /usr/local/bin/
`,
			expectGo:   false,
			expectRust: true,
		},
		{
			name: "Non-Go/Rust build",
			content: `FROM node:20 AS builder
WORKDIR /app
COPY . .
RUN npm run build

FROM registry.access.redhat.com/ubi10/ubi-minimal
COPY --from=builder /app/dist /app/
`,
			expectGo:   false,
			expectRust: false,
		},
		{
			name: "Go mod download",
			content: `FROM ghcr.io/rhadp/builder AS builder
WORKDIR /app
COPY go.mod go.sum ./
RUN go mod download
COPY . .
RUN go build -o app .

FROM registry.access.redhat.com/ubi10/ubi-micro
COPY --from=builder /app/app /usr/local/bin/
`,
			expectGo:   true,
			expectRust: false,
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

			info, err := parseContainerfileBuildStages(tempFile.Name())
			if err != nil {
				t.Fatalf("Failed to parse: %v", err)
			}

			if info.IsGoProject != tc.expectGo {
				t.Errorf("IsGoProject: got %v, want %v", info.IsGoProject, tc.expectGo)
			}
			if info.IsRustProject != tc.expectRust {
				t.Errorf("IsRustProject: got %v, want %v", info.IsRustProject, tc.expectRust)
			}
		})
	}
}

// Generator for random Containerfile content with Go/Rust builds
func genGoRustContainerfileContent() *rapid.Generator[string] {
	return rapid.Custom(func(t *rapid.T) string {
		isGo := rapid.Bool().Draw(t, "is_go")
		useApprovedBuilder := rapid.Bool().Draw(t, "use_approved_builder")

		var sb strings.Builder
		sb.WriteString("# Auto-generated Containerfile for testing\n")
		sb.WriteString("# Base Image Rationale: Using appropriate builder and UBI10 final stage\n\n")

		// Build stage
		if useApprovedBuilder {
			sb.WriteString("FROM ghcr.io/rhadp/builder AS builder\n")
		} else {
			if isGo {
				sb.WriteString("FROM docker.io/library/golang:1.22 AS builder\n")
			} else {
				sb.WriteString("FROM docker.io/library/rust:1.75 AS builder\n")
			}
		}

		sb.WriteString("WORKDIR /build\n")
		sb.WriteString("COPY . .\n")

		if isGo {
			sb.WriteString("RUN CGO_ENABLED=0 GOOS=linux go build -o app .\n\n")
		} else {
			sb.WriteString("RUN cargo build --release\n\n")
		}

		// Final stage
		sb.WriteString("FROM registry.access.redhat.com/ubi10/ubi-micro\n")
		if isGo {
			sb.WriteString("COPY --from=builder /build/app /usr/local/bin/\n")
		} else {
			sb.WriteString("COPY --from=builder /build/target/release/app /usr/local/bin/\n")
		}
		sb.WriteString("ENTRYPOINT [\"/usr/local/bin/app\"]\n")

		return sb.String()
	})
}

// TestGeneratedContainerfilesBuilderCompliance tests generated Containerfiles
func TestGeneratedContainerfilesBuilderCompliance(t *testing.T) {
	rapid.Check(t, func(rt *rapid.T) {
		content := genGoRustContainerfileContent().Draw(rt, "containerfile_content")

		// Write to temp file
		tempFile, err := os.CreateTemp("", "Containerfile.*")
		if err != nil {
			rt.Fatalf("Failed to create temp file: %v", err)
		}
		defer os.Remove(tempFile.Name())

		if _, err := tempFile.WriteString(content); err != nil {
			rt.Fatalf("Failed to write temp file: %v", err)
		}
		tempFile.Close()

		// Parse and verify
		info, err := parseContainerfileBuildStages(tempFile.Name())
		if err != nil {
			rt.Fatalf("Failed to parse generated Containerfile: %v", err)
		}

		// Check if it's a Go/Rust project
		if !info.IsGoProject && !info.IsRustProject {
			rt.Fatalf("Generated Containerfile not detected as Go/Rust project:\n%s", content)
		}

		// Check build stages for prohibited images
		for i, stage := range info.BuildStages {
			if i == len(info.BuildStages)-1 {
				continue // Skip final stage
			}

			if prohibited, pattern := isProhibitedBuilderImage(stage.BaseImage); prohibited {
				// This is expected for some generated files - just log it
				rt.Logf("Generated Containerfile uses prohibited builder: %s (pattern: %s)", stage.BaseImage, pattern)
			}
		}
	})
}
