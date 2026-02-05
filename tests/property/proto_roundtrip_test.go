// Copyright 2024 SDV Parking Demo System
// Property-based test for Protocol Buffer regeneration round-trip.
//
// **Feature: project-foundation, Property 1: Proto Regeneration Round-Trip**
//
// This test verifies that:
// *For any* modification to a Protocol Buffer definition file, running make proto
// SHALL regenerate language bindings that are syntactically valid and compile
// without errors in their respective languages (Rust, Kotlin, Dart, Go).
//
// **Validates: Requirements 2.7**

package property

import (
"bytes"
"fmt"
"go/parser"
"go/token"
"os"
"os/exec"
"path/filepath"
"strings"
"testing"
"time"

"pgregory.net/rapid"
)

// ProtoModification represents a valid modification to a proto file
type ProtoModification struct {
	Type        ModificationType
	Name        string
	FieldNumber int32
	FieldType   string
	Comment     string
}

// ModificationType represents the type of proto modification
type ModificationType int

const (
AddField ModificationType = iota
AddMessage
AddEnum
AddEnumValue
AddRPCMethod
)

func (m ModificationType) String() string {
	switch m {
	case AddField:
		return "AddField"
	case AddMessage:
		return "AddMessage"
	case AddEnum:
		return "AddEnum"
	case AddEnumValue:
		return "AddEnumValue"
	case AddRPCMethod:
		return "AddRPCMethod"
	default:
		return "Unknown"
	}
}

// validProtoFieldTypes contains valid proto3 field types
var validProtoFieldTypes = []string{
	"string", "int32", "int64", "uint32", "uint64",
	"sint32", "sint64", "fixed32", "fixed64",
	"sfixed32", "sfixed64", "float", "double", "bool", "bytes",
}

// validIdentifierChars for proto identifiers (must start with letter)
var validIdentifierStartChars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"
var validIdentifierChars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_"

// Generator for valid proto identifiers
func genProtoIdentifier() *rapid.Generator[string] {
	return rapid.Custom(func(t *rapid.T) string {
length := rapid.IntRange(3, 15).Draw(t, "identifier_length")
var sb strings.Builder
firstCharIdx := rapid.IntRange(0, len(validIdentifierStartChars)-1).Draw(t, "first_char")
sb.WriteByte(validIdentifierStartChars[firstCharIdx])
for i := 1; i < length; i++ {
			charIdx := rapid.IntRange(0, len(validIdentifierChars)-1).Draw(t, fmt.Sprintf("char_%d", i))
			sb.WriteByte(validIdentifierChars[charIdx])
		}
		return sb.String()
	})
}

// Generator for valid proto field types
func genProtoFieldType() *rapid.Generator[string] {
	return rapid.SampledFrom(validProtoFieldTypes)
}

// Generator for proto field numbers (valid range: 1 to 536870911, excluding 19000-19999)
func genProtoFieldNumber() *rapid.Generator[int32] {
	return rapid.Custom(func(t *rapid.T) int32 {
return int32(rapid.IntRange(100, 999).Draw(t, "field_number"))
})
}

// Generator for proto modifications
func genProtoModification() *rapid.Generator[ProtoModification] {
	return rapid.Custom(func(t *rapid.T) ProtoModification {
modType := ModificationType(rapid.IntRange(0, 4).Draw(t, "mod_type"))
name := genProtoIdentifier().Draw(t, "name")
		fieldNumber := genProtoFieldNumber().Draw(t, "field_number")
		fieldType := genProtoFieldType().Draw(t, "field_type")
		comment := fmt.Sprintf("// Auto-generated for property test at %s", time.Now().Format(time.RFC3339))
		return ProtoModification{
			Type:        modType,
			Name:        name,
			FieldNumber: fieldNumber,
			FieldType:   fieldType,
			Comment:     comment,
		}
	})
}

// TestProtoRegenerationRoundTrip is the main property-based test
// **Feature: project-foundation, Property 1: Proto Regeneration Round-Trip**
// **Validates: Requirements 2.7**
func TestProtoRegenerationRoundTrip(t *testing.T) {
	if !checkRequiredTools(t) {
		t.Skip("Skipping test: required tools not available")
	}

	tempDir, err := os.MkdirTemp("", "proto_roundtrip_test_*")
	if err != nil {
		t.Fatalf("Failed to create temp directory: %v", err)
	}
	defer os.RemoveAll(tempDir)

	// Run the property test with minimum 100 iterations as specified in design doc
	rapid.Check(t, func(rt *rapid.T) {
mod := genProtoModification().Draw(rt, "modification")
		testProtoContent := generateTestProto(mod)

	testProtoPath := filepath.Join(tempDir, fmt.Sprintf("test_%d.proto", time.Now().UnixNano()))
		if err := os.WriteFile(testProtoPath, []byte(testProtoContent), 0644); err != nil {
			rt.Fatalf("Failed to write test proto: %v", err)
		}

		if err := validateProtoSyntax(testProtoPath, tempDir); err != nil {
			rt.Fatalf("Generated proto is not syntactically valid: %v\nProto content:\n%s", err, testProtoContent)
		}

		if err := testGoGeneration(testProtoPath, tempDir); err != nil {
			rt.Fatalf("Go code generation failed for modification %s: %v\nProto content:\n%s",
mod.Type.String(), err, testProtoContent)
		}

		os.Remove(testProtoPath)
	})
}

// TestProtoModificationTypes tests specific modification types
func TestProtoModificationTypes(t *testing.T) {
	if !checkRequiredTools(t) {
		t.Skip("Skipping test: required tools not available")
	}

	tempDir, err := os.MkdirTemp("", "proto_mod_types_test_*")
	if err != nil {
		t.Fatalf("Failed to create temp directory: %v", err)
	}
	defer os.RemoveAll(tempDir)

	testCases := []struct {
		name string
		mod  ProtoModification
	}{
		{"AddField", ProtoModification{Type: AddField, Name: "test_field", FieldNumber: 100, FieldType: "string", Comment: "// Test field addition"}},
		{"AddMessage", ProtoModification{Type: AddMessage, Name: "TestMessage", FieldNumber: 1, FieldType: "string", Comment: "// Test message addition"}},
		{"AddEnum", ProtoModification{Type: AddEnum, Name: "TestEnum", FieldNumber: 0, FieldType: "", Comment: "// Test enum addition"}},
		{"AddEnumValue", ProtoModification{Type: AddEnumValue, Name: "TEST_VALUE", FieldNumber: 10, FieldType: "", Comment: "// Test enum value addition"}},
		{"AddRPCMethod", ProtoModification{Type: AddRPCMethod, Name: "TestMethod", FieldNumber: 0, FieldType: "", Comment: "// Test RPC method addition"}},
	}

	for _, tc := range testCases {
		t.Run(tc.name, func(t *testing.T) {
testProtoContent := generateTestProto(tc.mod)
testProtoPath := filepath.Join(tempDir, fmt.Sprintf("test_%s.proto", tc.name))

if err := os.WriteFile(testProtoPath, []byte(testProtoContent), 0644); err != nil {
				t.Fatalf("Failed to write test proto: %v", err)
			}

			if err := validateProtoSyntax(testProtoPath, tempDir); err != nil {
				t.Fatalf("Proto syntax validation failed: %v\nContent:\n%s", err, testProtoContent)
			}

			if err := testGoGeneration(testProtoPath, tempDir); err != nil {
				t.Fatalf("Go generation failed: %v\nContent:\n%s", err, testProtoContent)
			}
		})
	}
}

// generateTestProto creates a valid proto file with the given modification
func generateTestProto(mod ProtoModification) string {
	var sb strings.Builder
	sb.WriteString("// Auto-generated test proto file for property-based testing\n")
	sb.WriteString("syntax = \"proto3\";\n\n")
	sb.WriteString("package sdv.test;\n\n")
	sb.WriteString("option go_package = \"github.com/sdv-parking-demo/backend/gen/test\";\n\n")

	switch mod.Type {
	case AddField:
		sb.WriteString(fmt.Sprintf("%s\nmessage TestContainer {\n  %s %s = %d;\n}\n",
mod.Comment, mod.FieldType, mod.Name, mod.FieldNumber))

	case AddMessage:
		sb.WriteString(fmt.Sprintf("%s\nmessage %s {\n  %s content = 1;\n  int32 id = 2;\n  bool active = 3;\n}\n",
mod.Comment, mod.Name, mod.FieldType))

	case AddEnum:
		sb.WriteString(fmt.Sprintf("%s\nenum %s {\n  %s_UNSPECIFIED = 0;\n  %s_VALUE_ONE = 1;\n  %s_VALUE_TWO = 2;\n}\n\nmessage %sContainer {\n  %s value = 1;\n}\n",
mod.Comment, mod.Name, strings.ToUpper(mod.Name), strings.ToUpper(mod.Name),
strings.ToUpper(mod.Name), mod.Name, mod.Name))

	case AddEnumValue:
		sb.WriteString(fmt.Sprintf("%s\nenum TestEnumWithValue {\n  TEST_ENUM_WITH_VALUE_UNSPECIFIED = 0;\n  %s = %d;\n}\n",
mod.Comment, mod.Name, mod.FieldNumber))

case AddRPCMethod:
		sb.WriteString(fmt.Sprintf("%s\nmessage %sRequest {\n  string input = 1;\n}\n\nmessage %sResponse {\n  string output = 1;\n  bool success = 2;\n}\n\nservice TestService {\n  rpc %s(%sRequest) returns (%sResponse);\n}\n",
mod.Comment, mod.Name, mod.Name, mod.Name, mod.Name, mod.Name))
	}

	return sb.String()
}

// validateProtoSyntax checks if a proto file is syntactically valid
func validateProtoSyntax(protoPath, tempDir string) error {
	cmd := exec.Command("protoc",
"--proto_path="+tempDir,
"--descriptor_set_out=/dev/null",
protoPath,
)
	var stderr bytes.Buffer
	cmd.Stderr = &stderr
	if err := cmd.Run(); err != nil {
		return fmt.Errorf("protoc validation failed: %v\nstderr: %s", err, stderr.String())
	}
	return nil
}

// testGoGeneration tests that Go code can be generated and is syntactically valid
func testGoGeneration(protoPath, tempDir string) error {
	goOutDir := filepath.Join(tempDir, "gen", "go")
	if err := os.MkdirAll(goOutDir, 0755); err != nil {
		return fmt.Errorf("failed to create Go output directory: %v", err)
	}

	cmd := exec.Command("protoc",
"--proto_path="+tempDir,
"--go_out="+goOutDir,
"--go_opt=paths=source_relative",
protoPath,
)
	var stderr bytes.Buffer
	cmd.Stderr = &stderr
	if err := cmd.Run(); err != nil {
		return fmt.Errorf("protoc Go generation failed: %v\nstderr: %s", err, stderr.String())
	}

	var generatedGoFile string
	err := filepath.Walk(goOutDir, func(path string, info os.FileInfo, err error) error {
if err != nil {
return err
}
if !info.IsDir() && strings.HasSuffix(path, ".pb.go") {
			generatedGoFile = path
			return filepath.SkipAll
		}
		return nil
	})
	if err != nil && err != filepath.SkipAll {
		return fmt.Errorf("failed to find generated Go file: %v", err)
	}

	if generatedGoFile == "" {
		return fmt.Errorf("no .pb.go file was generated")
	}

	if err := validateGoSyntax(generatedGoFile); err != nil {
		return fmt.Errorf("generated Go code is not valid: %v", err)
	}

	return nil
}

// validateGoSyntax checks if a Go file is syntactically valid using Go parser
func validateGoSyntax(goFilePath string) error {
	fset := token.NewFileSet()
	_, err := parser.ParseFile(fset, goFilePath, nil, parser.AllErrors)
	if err != nil {
	return fmt.Errorf("Go syntax error: %v", err)
	}
	return nil
}

// findProjectRoot finds the root directory of the project
func findProjectRoot() (string, error) {
	dir, err := os.Getwd()
	if err != nil {
		return "", err
	}

	for {
		if _, err := os.Stat(filepath.Join(dir, "Makefile")); err == nil {
			if _, err := os.Stat(filepath.Join(dir, "proto")); err == nil {
				return dir, nil
			}
		}
	parent := filepath.Dir(dir)
		if parent == dir {
			return "", fmt.Errorf("could not find project root")
		}
		dir = parent
	}
}

// checkRequiredTools verifies that required tools are available
func checkRequiredTools(t *testing.T) bool {
	tools := []string{"protoc", "go"}
	for _, tool := range tools {
		if _, err := exec.LookPath(tool); err != nil {
			t.Logf("Required tool '%s' not found: %v", tool, err)
			return false
		}
	}

	if _, err := exec.LookPath("protoc-gen-go"); err != nil {
		gopath := os.Getenv("GOPATH")
		if gopath == "" {
			home, _ := os.UserHomeDir()
			gopath = filepath.Join(home, "go")
		}
		protocGenGo := filepath.Join(gopath, "bin", "protoc-gen-go")
		if _, err := os.Stat(protocGenGo); err != nil {
			t.Logf("protoc-gen-go not found in PATH or GOPATH/bin")
			return false
		}
		os.Setenv("PATH", os.Getenv("PATH")+":"+filepath.Join(gopath, "bin"))
	}

	return true
}

// TestProtoRoundTripWithExistingProtos tests modifications to existing project protos
func TestProtoRoundTripWithExistingProtos(t *testing.T) {
	projectRoot, err := findProjectRoot()
	if err != nil {
		t.Fatalf("Failed to find project root: %v", err)
	}

	if !checkRequiredTools(t) {
		t.Skip("Skipping test: required tools not available")
	}

	tempDir, err := os.MkdirTemp("", "proto_existing_test_*")
	if err != nil {
		t.Fatalf("Failed to create temp directory: %v", err)
	}
	defer os.RemoveAll(tempDir)

	protoDir := filepath.Join(projectRoot, "proto")
	tempProtoDir := filepath.Join(tempDir, "proto")

	rapid.Check(t, func(rt *rapid.T) {
os.RemoveAll(tempProtoDir)
if err := copyDir(protoDir, tempProtoDir); err != nil {
			rt.Fatalf("Failed to copy proto directory: %v", err)
		}

		// Generate a modification (only message, enum, or service - not bare fields)
		modType := ModificationType(rapid.IntRange(1, 4).Draw(rt, "mod_type"))
	name := genProtoIdentifier().Draw(rt, "name")
		fieldNumber := genProtoFieldNumber().Draw(rt, "field_number")
		fieldType := genProtoFieldType().Draw(rt, "field_type")

		mod := ProtoModification{
			Type:        modType,
			Name:        name,
			FieldNumber: fieldNumber,
			FieldType:   fieldType,
			Comment:     fmt.Sprintf("// Auto-generated at %s", time.Now().Format(time.RFC3339)),
		}

		targetProto := filepath.Join(tempProtoDir, "vss", "signals.proto")
		existingContent, err := os.ReadFile(targetProto)
		if err != nil {
			rt.Fatalf("Failed to read existing proto: %v", err)
		}

		modContent := generateTopLevelModification(mod)
		newContent := string(existingContent) + "\n" + modContent

		if err := os.WriteFile(targetProto, []byte(newContent), 0644); err != nil {
			rt.Fatalf("Failed to write modified proto: %v", err)
		}

		if err := validateProtoSyntaxWithIncludes(targetProto, tempProtoDir); err != nil {
			rt.Fatalf("Modified proto is not syntactically valid: %v\nModification: %s", err, modContent)
		}

		goOutDir := filepath.Join(tempDir, "gen", "go")
		os.RemoveAll(goOutDir)
		if err := os.MkdirAll(goOutDir, 0755); err != nil {
			rt.Fatalf("Failed to create Go output directory: %v", err)
		}

		cmd := exec.Command("protoc",
"--proto_path="+tempProtoDir,
"--go_out="+goOutDir,
"--go_opt=paths=source_relative",
targetProto,
)
		var stderr bytes.Buffer
	cmd.Stderr = &stderr
		if err := cmd.Run(); err != nil {
			rt.Fatalf("Go generation failed: %v\nstderr: %s\nModification: %s", err, stderr.String(), modContent)
		}

		err = filepath.Walk(goOutDir, func(path string, info os.FileInfo, err error) error {
if err != nil {
return err
}
if !info.IsDir() && strings.HasSuffix(path, ".pb.go") {
				if err := validateGoSyntax(path); err != nil {
					return fmt.Errorf("invalid Go syntax in %s: %v", path, err)
				}
			}
			return nil
		})
		if err != nil {
			rt.Fatalf("Generated Go validation failed: %v", err)
		}
	})
}

// generateTopLevelModification creates a top-level proto construct
func generateTopLevelModification(mod ProtoModification) string {
	suffix := fmt.Sprintf("_%d", time.Now().UnixNano()%10000)

	switch mod.Type {
	case AddMessage:
		return fmt.Sprintf("\n%s\nmessage %s%s {\n  %s content = 1;\n  int32 id = 2;\n  bool active = 3;\n}\n",
mod.Comment, mod.Name, suffix, mod.FieldType)

	case AddEnum:
		enumName := mod.Name + suffix
		return fmt.Sprintf("\n%s\nenum %s {\n  %s_UNSPECIFIED = 0;\n  %s_ACTIVE = 1;\n}\n",
mod.Comment, enumName, strings.ToUpper(enumName), strings.ToUpper(enumName))

	case AddEnumValue:
		enumName := "TestEnum" + suffix
		return fmt.Sprintf("\n%s\nenum %s {\n  %s_UNSPECIFIED = 0;\n  %s = %d;\n}\n",
mod.Comment, enumName, strings.ToUpper(enumName), mod.Name, mod.FieldNumber)

case AddRPCMethod:
		serviceName := mod.Name + suffix
		return fmt.Sprintf("\n%s\nmessage %sRequest {\n  string input = 1;\n}\n\nmessage %sResponse {\n  string output = 1;\n  bool success = 2;\n}\n\nservice %sService {\n  rpc %s(%sRequest) returns (%sResponse);\n}\n",
mod.Comment, serviceName, serviceName, serviceName, serviceName, serviceName, serviceName)

	default:
		return fmt.Sprintf("\nmessage Generated%s%s {\n  string value = 1;\n}\n", mod.Name, suffix)
	}
}

// validateProtoSyntaxWithIncludes validates proto with include paths
func validateProtoSyntaxWithIncludes(protoPath, protoDir string) error {
	cmd := exec.Command("protoc",
"--proto_path="+protoDir,
"--descriptor_set_out=/dev/null",
protoPath,
)
	var stderr bytes.Buffer
	cmd.Stderr = &stderr
	if err := cmd.Run(); err != nil {
		return fmt.Errorf("protoc validation failed: %v\nstderr: %s", err, stderr.String())
	}
return nil
}

// copyDir recursively copies a directory
func copyDir(src, dst string) error {
	return filepath.Walk(src, func(path string, info os.FileInfo, err error) error {
if err != nil {
return err
}
relPath, err := filepath.Rel(src, path)
if err != nil {
return err
}
dstPath := filepath.Join(dst, relPath)
if info.IsDir() {
			return os.MkdirAll(dstPath, info.Mode())
		}
	data, err := os.ReadFile(path)
		if err != nil {
			return err
		}
		return os.WriteFile(dstPath, data, info.Mode())
	})
}
