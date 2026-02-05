// Copyright 2024 SDV Parking Demo System
// Property-based test for UBI10 Base Image Compliance.
//
// **Feature: project-foundation, Property 5: UBI10 Base Image Compliance**
//
// This test verifies that:
// *For any* Containerfile in the project, the final stage base image SHALL be a
// Red Hat Universal Base Image 10 (UBI10) variant from `registry.access.redhat.com/ubi10/*`.
//
// **Validates: Requirements 8.1, 8.2, 8.5**

package property

import (
	"bufio"
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"strings"
	"testing"

	"pgregory.net/rapid"
)

// ContainerfileInfo holds parsed information about a Containerfile
type ContainerfileInfo struct {
	Path           string
	FinalBaseImage string
	AllFromLines   []string
	IsMultiStage   bool
}

// Prohibited base images that should never appear in final stages
var prohibitedBaseImages = []string{
	"alpine",
	"ubuntu",
	"debian",
	"centos",
	"fedora",
	"busybox",
	"scratch",
}

// UBI10 image pattern
var ubi10Pattern = regexp.MustCompile(`^registry\.access\.redhat\.com/ubi10/`)

// FROM instruction pattern
var fromPattern = regexp.MustCompile(`(?i)^FROM\s+(\S+)`)

// AS pattern for multi-stage builds
var asPattern = regexp.MustCompile(`(?i)\s+AS\s+\S+`)

// parseContainerfile extracts FROM instructions from a Containerfile
func parseContainerfile(path string) (*ContainerfileInfo, error) {
	file, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	defer file.Close()

	info := &ContainerfileInfo{
		Path:         path,
		AllFromLines: []string{},
	}

	scanner := bufio.NewScanner(file)
	for scanner.Scan() {
		line := strings.TrimSpace(scanner.Text())
		
		// Skip comments and empty lines
		if strings.HasPrefix(line, "#") || line == "" {
			continue
		}

		// Check for FROM instruction
		if matches := fromPattern.FindStringSubmatch(line); len(matches) > 1 {
			baseImage := matches[1]
			info.AllFromLines = append(info.AllFromLines, baseImage)
			
			// Check if this is a named stage (multi-stage build)
			if asPattern.MatchString(line) {
				info.IsMultiStage = true
			}
		}
	}

	if err := scanner.Err(); err != nil {
		return nil, err
	}

	// The final base image is the last FROM instruction
	if len(info.AllFromLines) > 0 {
		info.FinalBaseImage = info.AllFromLines[len(info.AllFromLines)-1]
	}

	// If there are multiple FROM instructions, it's a multi-stage build
	if len(info.AllFromLines) > 1 {
		info.IsMultiStage = true
	}

	return info, nil
}

// isUBI10Image checks if an image reference is a UBI10 image
func isUBI10Image(imageRef string) bool {
	return ubi10Pattern.MatchString(imageRef)
}

// isProhibitedImage checks if an image is from a prohibited base
func isProhibitedImage(imageRef string) (bool, string) {
	lowerRef := strings.ToLower(imageRef)
	for _, prohibited := range prohibitedBaseImages {
		if strings.Contains(lowerRef, prohibited) {
			return true, prohibited
		}
	}
	return false, ""
}

// findAllContainerfiles finds all Containerfiles in the containers directory
func findAllContainerfiles(projectRoot string) ([]string, error) {
	containersDir := filepath.Join(projectRoot, "containers")
	var containerfiles []string

	err := filepath.Walk(containersDir, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return err
		}
		if !info.IsDir() && strings.HasPrefix(info.Name(), "Containerfile") {
			containerfiles = append(containerfiles, path)
		}
		return nil
	})

	return containerfiles, err
}

// TestUBI10BaseImageCompliance is the main property-based test
// **Feature: project-foundation, Property 5: UBI10 Base Image Compliance**
// **Validates: Requirements 8.1, 8.2, 8.5**
func TestUBI10BaseImageCompliance(t *testing.T) {
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

	t.Logf("Found %d Containerfiles to check", len(containerfiles))

	rapid.Check(t, func(rt *rapid.T) {
		// Select a random Containerfile to test
		idx := rapid.IntRange(0, len(containerfiles)-1).Draw(rt, "containerfile_index")
		containerfilePath := containerfiles[idx]

		info, err := parseContainerfile(containerfilePath)
		if err != nil {
			rt.Fatalf("Failed to parse Containerfile %s: %v", containerfilePath, err)
		}

		// Property: Final base image must be UBI10
		if !isUBI10Image(info.FinalBaseImage) {
			rt.Fatalf("Containerfile %s: Final base image '%s' is not a UBI10 image (expected registry.access.redhat.com/ubi10/*)",
				containerfilePath, info.FinalBaseImage)
		}

		// Property: Final base image must not be a prohibited image
		if prohibited, which := isProhibitedImage(info.FinalBaseImage); prohibited {
			rt.Fatalf("Containerfile %s: Final base image '%s' uses prohibited base '%s'",
				containerfilePath, info.FinalBaseImage, which)
		}
	})
}

// TestAllContainerfilesUBI10Compliance tests all Containerfiles deterministically
func TestAllContainerfilesUBI10Compliance(t *testing.T) {
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
			info, err := parseContainerfile(containerfilePath)
			if err != nil {
				t.Fatalf("Failed to parse Containerfile: %v", err)
			}

			// Check final base image is UBI10
			if !isUBI10Image(info.FinalBaseImage) {
				t.Errorf("Final base image '%s' is not a UBI10 image", info.FinalBaseImage)
			}

			// Check no prohibited images in final stage
			if prohibited, which := isProhibitedImage(info.FinalBaseImage); prohibited {
				t.Errorf("Final base image '%s' uses prohibited base '%s'", info.FinalBaseImage, which)
			}

			t.Logf("✓ Final base image: %s (multi-stage: %v)", info.FinalBaseImage, info.IsMultiStage)
		})
	}
}

// TestProhibitedImagesRejected tests that prohibited images are properly rejected
func TestProhibitedImagesRejected(t *testing.T) {
	testCases := []struct {
		imageRef   string
		shouldFail bool
		reason     string
	}{
		{"registry.access.redhat.com/ubi10/ubi-minimal", false, "UBI10 minimal is allowed"},
		{"registry.access.redhat.com/ubi10/ubi-micro", false, "UBI10 micro is allowed"},
		{"registry.access.redhat.com/ubi10/ubi", false, "UBI10 standard is allowed"},
		{"alpine:latest", true, "Alpine is prohibited"},
		{"alpine:3.18", true, "Alpine with version is prohibited"},
		{"ubuntu:22.04", true, "Ubuntu is prohibited"},
		{"debian:bookworm", true, "Debian is prohibited"},
		{"debian:bookworm-slim", true, "Debian slim is prohibited"},
		{"centos:7", true, "CentOS is prohibited"},
		{"fedora:39", true, "Fedora is prohibited"},
		{"docker.io/library/alpine", true, "Alpine from docker.io is prohibited"},
		{"docker.io/library/ubuntu:latest", true, "Ubuntu from docker.io is prohibited"},
	}

	for _, tc := range testCases {
		t.Run(tc.imageRef, func(t *testing.T) {
			isProhibited, _ := isProhibitedImage(tc.imageRef)
			isUBI := isUBI10Image(tc.imageRef)

			if tc.shouldFail {
				if !isProhibited {
					t.Errorf("Expected '%s' to be prohibited: %s", tc.imageRef, tc.reason)
				}
			} else {
				if !isUBI {
					t.Errorf("Expected '%s' to be recognized as UBI10: %s", tc.imageRef, tc.reason)
				}
			}
		})
	}
}

// TestMultiStageBuildCompliance tests that multi-stage builds have UBI10 final stage
func TestMultiStageBuildCompliance(t *testing.T) {
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
			info, err := parseContainerfile(containerfilePath)
			if err != nil {
				t.Fatalf("Failed to parse Containerfile: %v", err)
			}

			if info.IsMultiStage {
				t.Logf("Multi-stage build detected with %d stages", len(info.AllFromLines))
				
				// Log all stages
				for i, fromLine := range info.AllFromLines {
					stage := "intermediate"
					if i == len(info.AllFromLines)-1 {
						stage = "FINAL"
					}
					t.Logf("  Stage %d (%s): %s", i+1, stage, fromLine)
				}

				// Only the final stage needs to be UBI10
				// Intermediate stages can use any image (e.g., rust:1.75 for building)
				if !isUBI10Image(info.FinalBaseImage) {
					t.Errorf("Final stage must use UBI10, got: %s", info.FinalBaseImage)
				}
			}
		})
	}
}

// Generator for random Containerfile content (for fuzz testing)
func genContainerfileContent() *rapid.Generator[string] {
	return rapid.Custom(func(t *rapid.T) string {
		numStages := rapid.IntRange(1, 3).Draw(t, "num_stages")
		var sb strings.Builder

		sb.WriteString("# Auto-generated Containerfile for testing\n\n")

		for i := 0; i < numStages; i++ {
			if i < numStages-1 {
				// Intermediate stages can use any image
				builderImages := []string{
					"docker.io/library/rust:1.75",
					"docker.io/library/golang:1.22",
					"docker.io/library/node:20",
				}
				idx := rapid.IntRange(0, len(builderImages)-1).Draw(t, fmt.Sprintf("builder_image_%d", i))
				sb.WriteString(fmt.Sprintf("FROM %s AS builder%d\n", builderImages[idx], i))
				sb.WriteString("WORKDIR /build\n")
				sb.WriteString("RUN echo 'building...'\n\n")
			} else {
				// Final stage must use UBI10
				ubi10Images := []string{
					"registry.access.redhat.com/ubi10/ubi-minimal",
					"registry.access.redhat.com/ubi10/ubi-micro",
					"registry.access.redhat.com/ubi10/ubi",
				}
				idx := rapid.IntRange(0, len(ubi10Images)-1).Draw(t, "final_image")
				sb.WriteString(fmt.Sprintf("FROM %s\n", ubi10Images[idx]))
				sb.WriteString("COPY --from=builder0 /build/app /usr/local/bin/\n")
				sb.WriteString("ENTRYPOINT [\"/usr/local/bin/app\"]\n")
			}
		}

		return sb.String()
	})
}

// TestGeneratedContainerfilesCompliance tests that generated Containerfiles pass compliance
func TestGeneratedContainerfilesCompliance(t *testing.T) {
	rapid.Check(t, func(rt *rapid.T) {
		content := genContainerfileContent().Draw(rt, "containerfile_content")

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
		info, err := parseContainerfile(tempFile.Name())
		if err != nil {
			rt.Fatalf("Failed to parse generated Containerfile: %v", err)
		}

		// Property: Generated Containerfiles should always have UBI10 final stage
		if !isUBI10Image(info.FinalBaseImage) {
			rt.Fatalf("Generated Containerfile has non-UBI10 final stage: %s\nContent:\n%s",
				info.FinalBaseImage, content)
		}
	})
}
