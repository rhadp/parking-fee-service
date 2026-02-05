// Copyright 2024 SDV Parking Demo System
// Property-based test for Container Image Git Tagging.
//
// **Feature: project-foundation, Property 3: Container Image Git Tagging**
//
// This test verifies that:
// *For any* container image built by the Build_System, the image tag SHALL contain
// valid git metadata (commit hash or version tag) that can be traced back to a
// specific commit in the repository.
//
// **Validates: Requirements 4.8**

package property

import (
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"strings"
	"testing"

	"pgregory.net/rapid"
)

// GitState represents different git repository states for testing
type GitState struct {
	HasTag     bool
	IsDirty    bool
	CommitHash string
	TagName    string
}

// ManifestOutput represents the JSON output from generate-manifest.sh
type ManifestOutput struct {
	ImageRef string `json:"image_ref"`
	Digest   string `json:"digest"`
	Version  string `json:"version"`
	Git      struct {
		Commit      string `json:"commit"`
		CommitShort string `json:"commit_short"`
		Branch      string `json:"branch"`
		Tag         string `json:"tag"`
		Dirty       bool   `json:"dirty"`
	} `json:"git"`
	Build struct {
		Timestamp string `json:"timestamp"`
	} `json:"build"`
	Labels map[string]string `json:"labels"`
}

// validGitCommitHashRegex matches valid git commit hashes (short or full)
var validGitCommitHashRegex = regexp.MustCompile(`^[0-9a-f]{7,40}$`)

// validGitTagRegex matches valid semantic version tags
var validGitTagRegex = regexp.MustCompile(`^v?\d+\.\d+\.\d+(-[a-zA-Z0-9.]+)?$`)

// Generator for git commit hashes (simulated)
func genGitCommitHash() *rapid.Generator[string] {
	return rapid.Custom(func(t *rapid.T) string {
		hexChars := "0123456789abcdef"
		length := rapid.IntRange(7, 40).Draw(t, "hash_length")
		var sb strings.Builder
		for i := 0; i < length; i++ {
			idx := rapid.IntRange(0, len(hexChars)-1).Draw(t, "hex_char")
			sb.WriteByte(hexChars[idx])
		}
		return sb.String()
	})
}

// Generator for semantic version tags
func genSemanticVersionTag() *rapid.Generator[string] {
	return rapid.Custom(func(t *rapid.T) string {
		major := rapid.IntRange(0, 99).Draw(t, "major")
		minor := rapid.IntRange(0, 99).Draw(t, "minor")
		patch := rapid.IntRange(0, 99).Draw(t, "patch")
		hasPrefix := rapid.Bool().Draw(t, "has_v_prefix")
		
		var sb strings.Builder
		if hasPrefix {
			sb.WriteString("v")
		}
		sb.WriteString(fmt.Sprintf("%d.%d.%d", major, minor, patch))
		return sb.String()
	})
}

// Generator for git states
func genGitState() *rapid.Generator[GitState] {
	return rapid.Custom(func(t *rapid.T) GitState {
		hasTag := rapid.Bool().Draw(t, "has_tag")
		isDirty := rapid.Bool().Draw(t, "is_dirty")
		commitHash := genGitCommitHash().Draw(t, "commit_hash")
		
		var tagName string
		if hasTag {
			tagName = genSemanticVersionTag().Draw(t, "tag_name")
		}
		
		return GitState{
			HasTag:     hasTag,
			IsDirty:    isDirty,
			CommitHash: commitHash,
			TagName:    tagName,
		}
	})
}

// TestContainerImageGitTagging is the main property-based test
// **Feature: project-foundation, Property 3: Container Image Git Tagging**
// **Validates: Requirements 4.8**
func TestContainerImageGitTagging(t *testing.T) {
	projectRoot, err := findProjectRoot()
	if err != nil {
		t.Fatalf("Failed to find project root: %v", err)
	}

	manifestScript := filepath.Join(projectRoot, "scripts", "generate-manifest.sh")
	if _, err := os.Stat(manifestScript); os.IsNotExist(err) {
		t.Fatalf("generate-manifest.sh not found at %s", manifestScript)
	}

	// Verify we're in a git repository
	if _, err := exec.Command("git", "rev-parse", "--git-dir").Output(); err != nil {
		t.Skip("Not in a git repository, skipping test")
	}

	rapid.Check(t, func(rt *rapid.T) {
		// Generate a random image name for testing
		imageName := rapid.StringMatching(`[a-z][a-z0-9-]{2,20}:[a-z0-9.-]{1,10}`).Draw(rt, "image_name")
		
		// Create temp file for manifest output
		tempFile, err := os.CreateTemp("", "manifest_*.json")
		if err != nil {
			rt.Fatalf("Failed to create temp file: %v", err)
		}
		tempPath := tempFile.Name()
		tempFile.Close()
		defer os.Remove(tempPath)

		// Run the manifest generation script
		cmd := exec.Command(manifestScript, imageName, tempPath)
		cmd.Dir = projectRoot
		output, err := cmd.CombinedOutput()
		if err != nil {
			rt.Fatalf("generate-manifest.sh failed: %v\nOutput: %s", err, output)
		}

		// Read and parse the manifest
		manifestData, err := os.ReadFile(tempPath)
		if err != nil {
			rt.Fatalf("Failed to read manifest: %v", err)
		}

		var manifest ManifestOutput
		if err := json.Unmarshal(manifestData, &manifest); err != nil {
			rt.Fatalf("Failed to parse manifest JSON: %v\nContent: %s", err, manifestData)
		}

		// Property: Version must contain valid git metadata
		if manifest.Version == "" {
			rt.Fatalf("Version is empty - must contain git metadata")
		}

		// Property: Git commit must be present and valid
		if manifest.Git.Commit == "" || manifest.Git.Commit == "unknown" {
			rt.Fatalf("Git commit is missing or unknown")
		}

		if !validGitCommitHashRegex.MatchString(manifest.Git.Commit) {
			rt.Fatalf("Git commit hash is not valid: %s", manifest.Git.Commit)
		}

		// Property: Short commit must be valid
		if manifest.Git.CommitShort == "" || manifest.Git.CommitShort == "unknown" {
			rt.Fatalf("Git short commit is missing or unknown")
		}

		if len(manifest.Git.CommitShort) < 7 {
			rt.Fatalf("Git short commit is too short: %s", manifest.Git.CommitShort)
		}

		// Property: Version must be traceable to git
		// Either it's a tag, a commit hash, or commit-dirty
		isValidVersion := false
		
		// Check if version is a valid tag
		if validGitTagRegex.MatchString(manifest.Version) {
			isValidVersion = true
		}
		
		// Check if version contains commit hash
		if strings.Contains(manifest.Version, manifest.Git.CommitShort) {
			isValidVersion = true
		}
		
		// Check if version is the short commit (possibly with -dirty suffix)
		if strings.HasPrefix(manifest.Version, manifest.Git.CommitShort) {
			isValidVersion = true
		}

		if !isValidVersion {
			rt.Fatalf("Version '%s' is not traceable to git (commit: %s, tag: %s)",
				manifest.Version, manifest.Git.CommitShort, manifest.Git.Tag)
		}

		// Property: If dirty, version should indicate it
		if manifest.Git.Dirty && !strings.Contains(manifest.Version, "-dirty") && manifest.Git.Tag == "" {
			rt.Fatalf("Dirty repository but version doesn't indicate it: %s", manifest.Version)
		}

		// Property: Build timestamp must be present
		if manifest.Build.Timestamp == "" {
			rt.Fatalf("Build timestamp is missing")
		}
	})
}

// TestManifestGitMetadataConsistency tests that git metadata is internally consistent
func TestManifestGitMetadataConsistency(t *testing.T) {
	projectRoot, err := findProjectRoot()
	if err != nil {
		t.Fatalf("Failed to find project root: %v", err)
	}

	manifestScript := filepath.Join(projectRoot, "scripts", "generate-manifest.sh")
	if _, err := os.Stat(manifestScript); os.IsNotExist(err) {
		t.Fatalf("generate-manifest.sh not found at %s", manifestScript)
	}

	// Create temp file for manifest output
	tempFile, err := os.CreateTemp("", "manifest_*.json")
	if err != nil {
		t.Fatalf("Failed to create temp file: %v", err)
	}
	tempPath := tempFile.Name()
	tempFile.Close()
	defer os.Remove(tempPath)

	// Run the manifest generation script
	cmd := exec.Command(manifestScript, "test-image:latest", tempPath)
	cmd.Dir = projectRoot
	if output, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("generate-manifest.sh failed: %v\nOutput: %s", err, output)
	}

	// Read and parse the manifest
	manifestData, err := os.ReadFile(tempPath)
	if err != nil {
		t.Fatalf("Failed to read manifest: %v", err)
	}

	var manifest ManifestOutput
	if err := json.Unmarshal(manifestData, &manifest); err != nil {
		t.Fatalf("Failed to parse manifest JSON: %v", err)
	}

	// Verify short commit is prefix of full commit
	if !strings.HasPrefix(manifest.Git.Commit, manifest.Git.CommitShort) {
		t.Errorf("Short commit '%s' is not a prefix of full commit '%s'",
			manifest.Git.CommitShort, manifest.Git.Commit)
	}

	// Verify actual git state matches manifest
	actualCommit, err := exec.Command("git", "rev-parse", "HEAD").Output()
	if err != nil {
		t.Fatalf("Failed to get actual git commit: %v", err)
	}
	actualCommitStr := strings.TrimSpace(string(actualCommit))

	if manifest.Git.Commit != actualCommitStr {
		t.Errorf("Manifest commit '%s' doesn't match actual commit '%s'",
			manifest.Git.Commit, actualCommitStr)
	}

	// Verify dirty state
	dirtyCheck := exec.Command("git", "diff", "--quiet")
	actualDirty := dirtyCheck.Run() != nil

	if manifest.Git.Dirty != actualDirty {
		t.Errorf("Manifest dirty state '%v' doesn't match actual state '%v'",
			manifest.Git.Dirty, actualDirty)
	}
}

// TestVersionTraceability tests that versions can be traced back to git
func TestVersionTraceability(t *testing.T) {
	projectRoot, err := findProjectRoot()
	if err != nil {
		t.Fatalf("Failed to find project root: %v", err)
	}

	manifestScript := filepath.Join(projectRoot, "scripts", "generate-manifest.sh")

	testCases := []string{
		"locking-service:latest",
		"update-service:v1.0.0",
		"parking-operator-adaptor:dev",
		"cloud-gateway-client:test",
	}

	for _, imageName := range testCases {
		t.Run(imageName, func(t *testing.T) {
			tempFile, err := os.CreateTemp("", "manifest_*.json")
			if err != nil {
				t.Fatalf("Failed to create temp file: %v", err)
			}
			tempPath := tempFile.Name()
			tempFile.Close()
			defer os.Remove(tempPath)

			cmd := exec.Command(manifestScript, imageName, tempPath)
			cmd.Dir = projectRoot
			if output, err := cmd.CombinedOutput(); err != nil {
				t.Fatalf("generate-manifest.sh failed: %v\nOutput: %s", err, output)
			}

			manifestData, err := os.ReadFile(tempPath)
			if err != nil {
				t.Fatalf("Failed to read manifest: %v", err)
			}

			var manifest ManifestOutput
			if err := json.Unmarshal(manifestData, &manifest); err != nil {
				t.Fatalf("Failed to parse manifest JSON: %v", err)
			}

			// Verify version is traceable
			if manifest.Version == "" || manifest.Version == "unknown" {
				t.Errorf("Version is not traceable for image %s", imageName)
			}

			// Verify git commit is valid
			if !validGitCommitHashRegex.MatchString(manifest.Git.Commit) {
				t.Errorf("Invalid git commit hash for image %s: %s", imageName, manifest.Git.Commit)
			}
		})
	}
}
