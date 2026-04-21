package setup

import (
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"strings"
	"testing"
)

// TestProtoFilesHaveRequiredMetadata verifies each .proto file contains
// syntax = "proto3", a package declaration, and a go_package option.
// Test Spec: TS-01-16, TS-01-P5
// Requirements: 01-REQ-5.2, 01-REQ-5.3
func TestProtoFilesHaveRequiredMetadata(t *testing.T) {
	root := findRepoRoot(t)
	protoDir := filepath.Join(root, "proto")

	var protoFiles []string
	err := filepath.WalkDir(protoDir, func(path string, d os.DirEntry, err error) error {
		if err != nil {
			return err
		}
		if !d.IsDir() && filepath.Ext(path) == ".proto" {
			protoFiles = append(protoFiles, path)
		}
		return nil
	})
	if err != nil {
		t.Fatalf("failed to walk proto directory: %v", err)
	}
	if len(protoFiles) == 0 {
		t.Fatal("no .proto files found in proto/ directory")
	}

	packageRE := regexp.MustCompile(`(?m)^package\s+\w+`)
	goPackageRE := regexp.MustCompile(`(?m)option\s+go_package\s*=`)

	for _, file := range protoFiles {
		relPath, _ := filepath.Rel(root, file)
		t.Run(relPath, func(t *testing.T) {
			data, err := os.ReadFile(file)
			if err != nil {
				t.Fatalf("failed to read %s: %v", file, err)
			}
			content := string(data)

			if !strings.Contains(content, `syntax = "proto3"`) {
				t.Errorf("file %s missing syntax = \"proto3\"", relPath)
			}
			if !packageRE.MatchString(content) {
				t.Errorf("file %s missing package declaration", relPath)
			}
			if !goPackageRE.MatchString(content) {
				t.Errorf("file %s missing go_package option", relPath)
			}
		})
	}
}

// TestProtocFailsOnMissingImport verifies protoc reports a clear error when
// a proto file references a non-existent import.
// Test Spec: TS-01-E5
// Requirement: 01-REQ-5.E1
func TestProtocFailsOnMissingImport(t *testing.T) {
	if _, err := exec.LookPath("protoc"); err != nil {
		t.Skip("protoc not found on PATH — skipping missing import test")
	}

	root := findRepoRoot(t)
	protoDir := filepath.Join(root, "proto")

	// Create a temporary proto file with a missing import.
	tmpFile := filepath.Join(protoDir, "temp_test_missing_import.proto")
	protoContent := `syntax = "proto3";
import "nonexistent_file.proto";
package test;
`
	if err := os.WriteFile(tmpFile, []byte(protoContent), 0o644); err != nil {
		t.Fatalf("failed to create temp proto file: %v", err)
	}
	defer os.Remove(tmpFile)

	cmd := exec.Command("protoc",
		"--proto_path="+protoDir,
		tmpFile,
		"--descriptor_set_out=/dev/null",
	)
	out, err := cmd.CombinedOutput()
	if err == nil {
		t.Fatal("expected protoc to fail on missing import, but it succeeded")
	}
	if !strings.Contains(string(out), "nonexistent_file.proto") {
		t.Fatalf("expected error to mention 'nonexistent_file.proto', got:\n%s", out)
	}
}
