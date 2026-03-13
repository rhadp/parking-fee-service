package setup

import (
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
)

// TS-01-18: Proto Files Exist
// Requirement: 01-REQ-5.1
func TestProtoFilesExist(t *testing.T) {
	root := repoRoot(t)

	files := []string{"common.proto", "update_service.proto", "parking_adaptor.proto"}
	for _, file := range files {
		path := filepath.Join(root, "proto", file)
		if !fileExists(path) {
			t.Errorf("expected proto file proto/%s to exist", file)
		}
	}
}

// TS-01-19: Common Proto Types
// Requirement: 01-REQ-5.2
func TestCommonProtoTypes(t *testing.T) {
	root := repoRoot(t)

	path := filepath.Join(root, "proto", "common.proto")
	content, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("failed to read proto/common.proto: %v", err)
	}

	text := string(content)
	expected := []string{"enum AdapterState", "message AdapterInfo", "message ErrorDetails"}
	for _, exp := range expected {
		if !strings.Contains(text, exp) {
			t.Errorf("common.proto missing %q", exp)
		}
	}
}

// TS-01-20: UpdateService Proto RPCs
// Requirement: 01-REQ-5.3
func TestUpdateServiceProtoRPCs(t *testing.T) {
	root := repoRoot(t)

	path := filepath.Join(root, "proto", "update_service.proto")
	content, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("failed to read proto/update_service.proto: %v", err)
	}

	text := string(content)
	if !strings.Contains(text, "service UpdateService") {
		t.Error("update_service.proto missing 'service UpdateService'")
	}

	rpcs := []string{"InstallAdapter", "WatchAdapterStates", "ListAdapters", "RemoveAdapter", "GetAdapterStatus"}
	for _, rpc := range rpcs {
		if !strings.Contains(text, "rpc "+rpc) {
			t.Errorf("update_service.proto missing RPC %q", rpc)
		}
	}
}

// TS-01-21: ParkingAdaptor Proto RPCs
// Requirement: 01-REQ-5.4
func TestParkingAdaptorProtoRPCs(t *testing.T) {
	root := repoRoot(t)

	path := filepath.Join(root, "proto", "parking_adaptor.proto")
	content, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("failed to read proto/parking_adaptor.proto: %v", err)
	}

	text := string(content)
	if !strings.Contains(text, "service ParkingAdaptor") {
		t.Error("parking_adaptor.proto missing 'service ParkingAdaptor'")
	}

	rpcs := []string{"StartSession", "StopSession", "GetStatus", "GetRate"}
	for _, rpc := range rpcs {
		if !strings.Contains(text, "rpc "+rpc) {
			t.Errorf("parking_adaptor.proto missing RPC %q", rpc)
		}
	}
}

// TS-01-22: Proto Generation Produces Go Code
// Requirement: 01-REQ-5.5
func TestProtoGeneration(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping proto generation test in short mode")
	}

	root := repoRoot(t)

	// Check if protoc is available
	if _, err := exec.LookPath("protoc"); err != nil {
		t.Skip("skipping: protoc not installed")
	}

	// Run make proto
	cmd := exec.Command("make", "proto")
	cmd.Dir = root
	output, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("make proto failed: %v\n%s", err, string(output))
	}

	// Check generated packages exist
	packages := []string{"commonpb", "updateservicepb", "parkingadaptorpb"}
	for _, pkg := range packages {
		dir := filepath.Join(root, "gen", "go", pkg)
		if !isDir(dir) {
			t.Errorf("expected generated Go package directory gen/go/%s to exist", pkg)
			continue
		}

		// Check for at least one .go file
		entries, err := os.ReadDir(dir)
		if err != nil {
			t.Errorf("failed to read gen/go/%s: %v", pkg, err)
			continue
		}
		hasGo := false
		for _, entry := range entries {
			if strings.HasSuffix(entry.Name(), ".go") {
				hasGo = true
				break
			}
		}
		if !hasGo {
			t.Errorf("gen/go/%s contains no .go files", pkg)
		}
	}
}

// TS-01-23: Generated Go Code Compiles
// Requirement: 01-REQ-5.6
func TestGeneratedGoCompiles(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping generated code compilation test in short mode")
	}

	root := repoRoot(t)

	genDir := filepath.Join(root, "gen", "go")
	if !isDir(genDir) {
		t.Skip("gen/go/ does not exist; run make proto first")
	}

	cmd := exec.Command("go", "build", "./gen/go/...")
	cmd.Dir = root
	output, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("go build ./gen/go/... failed: %v\n%s", err, string(output))
	}
}
