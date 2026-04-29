package setup_test

import (
	"os/exec"
	"path/filepath"
	"regexp"
	"strings"
	"testing"
)

// TS-01-16: Proto files are valid proto3
// Requirements: 01-REQ-5.1, 01-REQ-5.2, 01-REQ-5.3
func TestProtoFilesValidProto3(t *testing.T) {
	root := repoRoot(t)
	protoDir := filepath.Join(root, "proto")

	if _, err := exec.LookPath("protoc"); err != nil {
		t.Skip("protoc not found on PATH; skipping proto validation")
	}

	// Find all proto files
	protoFiles := findProtoFiles(t, protoDir)
	if len(protoFiles) == 0 {
		t.Fatal("no .proto files found in proto/ directory")
	}

	packageRe := regexp.MustCompile(`(?m)^package\s+\w+`)
	goPackageRe := regexp.MustCompile(`(?m)option\s+go_package\s*=`)

	for _, protoFile := range protoFiles {
		relPath, _ := filepath.Rel(root, protoFile)
		t.Run(relPath, func(t *testing.T) {
			content := readFileContent(t, protoFile)

			// Check syntax = "proto3"
			if !strings.Contains(content, `syntax = "proto3"`) {
				t.Errorf("%s does not contain syntax = \"proto3\"", relPath)
			}

			// Check package declaration
			if !packageRe.MatchString(content) {
				t.Errorf("%s does not contain a package declaration", relPath)
			}

			// Check go_package option
			if !goPackageRe.MatchString(content) {
				t.Errorf("%s does not contain a go_package option", relPath)
			}

			// Verify protoc can parse the file
			cmd := exec.Command("protoc",
				"--proto_path="+protoDir,
				"--descriptor_set_out=/dev/null",
				protoFile,
			)
			output, err := cmd.CombinedOutput()
			if err != nil {
				t.Errorf("protoc failed to parse %s: %v\noutput:\n%s", relPath, err, string(output))
			}
		})
	}
}

// TS-01-17: Protoc parses all proto files without errors (cross-import resolution)
// Requirement: 01-REQ-5.4
func TestProtocParsesAllProtoFiles(t *testing.T) {
	root := repoRoot(t)
	protoDir := filepath.Join(root, "proto")

	if _, err := exec.LookPath("protoc"); err != nil {
		t.Skip("protoc not found on PATH; skipping proto parse test")
	}

	protoFiles := findProtoFiles(t, protoDir)
	if len(protoFiles) == 0 {
		t.Fatal("no .proto files found in proto/ directory")
	}

	// Build protoc arguments: parse all files simultaneously
	args := []string{
		"--proto_path=" + protoDir,
		"--descriptor_set_out=/dev/null",
	}
	args = append(args, protoFiles...)

	cmd := exec.Command("protoc", args...)
	output, err := cmd.CombinedOutput()
	if err != nil {
		t.Errorf("protoc failed to parse all proto files simultaneously: %v\noutput:\n%s", err, string(output))
	}
}
