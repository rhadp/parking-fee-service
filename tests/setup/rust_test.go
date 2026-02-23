package setup_test

import (
	"testing"
)

// TS-01-12: Rust Cargo workspace members (01-REQ-3.1)
func TestRust_WorkspaceMembers(t *testing.T) {
	root := repoRoot(t)
	assertFileExists(t, root, "rhivos/Cargo.toml")

	for _, member := range []string{
		"locking-service",
		"cloud-gateway-client",
		"update-service",
		"parking-operator-adaptor",
	} {
		assertFileContains(t, root, "rhivos/Cargo.toml", member)
	}
}

// TS-01-15: Rust skeletons include generated proto code (01-REQ-3.4)
func TestRust_ProtoGeneration(t *testing.T) {
	root := repoRoot(t)

	for _, crate := range []string{"update-service", "parking-operator-adaptor"} {
		buildRS := "rhivos/" + crate + "/build.rs"
		assertFileExists(t, root, buildRS)

		content := readFile(t, root, buildRS)
		if content == "" {
			t.Errorf("build.rs for %s is empty", crate)
			continue
		}
		assertFileContains(t, root, buildRS, ".proto")
		assertFileContains(t, root, buildRS, "tonic_build")
	}
}

// TS-01-16: Rust stubs return unimplemented (01-REQ-3.5)
func TestRust_StubsUnimplemented(t *testing.T) {
	root := repoRoot(t)

	for _, crate := range []string{"update-service", "parking-operator-adaptor"} {
		libRS := "rhivos/" + crate + "/src/lib.rs"
		assertFileExists(t, root, libRS)
		assertFileContains(t, root, libRS, "unimplemented")
	}
}
