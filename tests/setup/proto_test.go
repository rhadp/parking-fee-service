package setup_test

import (
	"testing"
)

// TS-01-7: UPDATE_SERVICE proto definition (01-REQ-2.1)
func TestProto_UpdateService(t *testing.T) {
	root := repoRoot(t)
	assertFileExists(t, root, "proto/update_service.proto")

	content := readFile(t, root, "proto/update_service.proto")
	for _, expected := range []string{
		"service UpdateService",
		"rpc InstallAdapter",
		"rpc WatchAdapterStates",
		"rpc ListAdapters",
		"rpc RemoveAdapter",
		"rpc GetAdapterStatus",
	} {
		assertFileContains(t, root, "proto/update_service.proto", expected)
		_ = content // ensure file was readable
	}
}

// TS-01-8: PARKING_OPERATOR_ADAPTOR proto definition (01-REQ-2.2)
func TestProto_ParkingAdaptor(t *testing.T) {
	root := repoRoot(t)
	assertFileExists(t, root, "proto/parking_adaptor.proto")

	for _, expected := range []string{
		"service ParkingAdaptor",
		"rpc StartSession",
		"rpc StopSession",
		"rpc GetStatus",
		"rpc GetRate",
	} {
		assertFileContains(t, root, "proto/parking_adaptor.proto", expected)
	}
}

// TS-01-9: Common proto types (01-REQ-2.3)
func TestProto_CommonTypes(t *testing.T) {
	root := repoRoot(t)
	assertFileExists(t, root, "proto/common.proto")

	for _, expected := range []string{
		"enum AdapterState",
		"message ErrorDetails",
		"message AdapterInfo",
	} {
		assertFileContains(t, root, "proto/common.proto", expected)
	}
}

// TS-01-10: Proto files compile with protoc (01-REQ-2.4)
func TestProto_Compile(t *testing.T) {
	root := repoRoot(t)

	// Check protoc is available
	if _, err := lookPath("protoc"); err != nil {
		t.Skip("protoc not installed, skipping proto compilation test")
	}

	result := execCommand(t, root, ".",
		"protoc",
		"--proto_path=proto/",
		"--descriptor_set_out=/dev/null",
		"proto/common.proto",
		"proto/update_service.proto",
		"proto/parking_adaptor.proto",
	)
	if result.ExitCode != 0 {
		t.Errorf("protoc failed with exit code %d: %s", result.ExitCode, result.Combined)
	}
}

// TS-01-11: Proto files use proto3 syntax (01-REQ-2.5)
func TestProto_Syntax(t *testing.T) {
	root := repoRoot(t)

	protoFiles := globFiles(t, root, "proto/*.proto")
	if len(protoFiles) == 0 {
		t.Fatal("no proto files found in proto/")
	}

	for _, f := range protoFiles {
		// Read relative to root by computing relative path
		rel, _ := relPath(root, f)
		assertFileContains(t, root, rel, `syntax = "proto3"`)
	}
}
