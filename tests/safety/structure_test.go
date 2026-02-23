package safety_test

import (
	"testing"
)

// TS-02-1: VSS overlay defines custom command signals (02-REQ-1.1)
//
// Verify the VSS overlay file defines Vehicle.Command.Door.Lock and
// Vehicle.Command.Door.Response as string-typed signals.
func TestStructure_VssOverlay(t *testing.T) {
	root := repoRoot(t)

	// Overlay file must exist
	assertFileExists(t, root, "infra/kuksa/vss-overlay.json")

	// Parse and verify custom signals
	parsed := parseJSONFile(t, root, "infra/kuksa/vss-overlay.json")

	// Navigate: Vehicle.children.Command.children.Door.children.Lock.datatype
	lockDatatype := navigateJSON(parsed, "Vehicle.children.Command.children.Door.children.Lock.datatype")
	if lockDatatype == nil {
		t.Error("Vehicle.Command.Door.Lock signal not found in VSS overlay")
	} else if lockDatatype != "string" {
		t.Errorf("Vehicle.Command.Door.Lock datatype: got %v, want \"string\"", lockDatatype)
	}

	// Navigate: Vehicle.children.Command.children.Door.children.Response.datatype
	responseDatatype := navigateJSON(parsed, "Vehicle.children.Command.children.Door.children.Response.datatype")
	if responseDatatype == nil {
		t.Error("Vehicle.Command.Door.Response signal not found in VSS overlay")
	} else if responseDatatype != "string" {
		t.Errorf("Vehicle.Command.Door.Response datatype: got %v, want \"string\"", responseDatatype)
	}
}

// TS-02-26: LOCKING_SERVICE uses UDS for DATA_BROKER (02-REQ-7.1)
//
// Verify LOCKING_SERVICE configuration uses UDS endpoint for DATA_BROKER
// connection. The default endpoint should be a UDS path, not a TCP address.
func TestConfig_LockingServiceUDS(t *testing.T) {
	root := repoRoot(t)

	// Locking service source must reference UDS
	assertDirExists(t, root, "rhivos/locking-service/src")
	content := readAllRustFiles(t, root, "rhivos/locking-service/src")
	if content == "" {
		t.Fatal("no Rust source files found in rhivos/locking-service/src/")
	}

	// Must reference Unix domain sockets or .sock path
	found := false
	for _, term := range []string{"unix", "uds", ".sock", "UDS", "Unix"} {
		if containsString(content, term) {
			found = true
			break
		}
	}
	if !found {
		t.Error("locking-service source does not reference UDS/unix/.sock — expected UDS endpoint for DATA_BROKER")
	}
}

// TS-02-27: CLOUD_GATEWAY_CLIENT uses UDS for DATA_BROKER (02-REQ-7.2)
//
// Verify CLOUD_GATEWAY_CLIENT configuration uses UDS endpoint for DATA_BROKER
// connection. The default endpoint should be a UDS path, not a TCP address.
func TestConfig_CloudGatewayClientUDS(t *testing.T) {
	root := repoRoot(t)

	// Cloud gateway client source must reference UDS
	assertDirExists(t, root, "rhivos/cloud-gateway-client/src")
	content := readAllRustFiles(t, root, "rhivos/cloud-gateway-client/src")
	if content == "" {
		t.Fatal("no Rust source files found in rhivos/cloud-gateway-client/src/")
	}

	// Must reference Unix domain sockets or .sock path
	found := false
	for _, term := range []string{"unix", "uds", ".sock", "UDS", "Unix"} {
		if containsString(content, term) {
			found = true
			break
		}
	}
	if !found {
		t.Error("cloud-gateway-client source does not reference UDS/unix/.sock — expected UDS endpoint for DATA_BROKER")
	}
}

// TS-02-28: Mock sensors support configurable endpoint (02-REQ-7.3)
//
// Verify mock sensor CLIs accept a configurable DATA_BROKER endpoint.
func TestConfig_SensorConfigurableEndpoint(t *testing.T) {
	root := repoRoot(t)

	// Check for mock-sensors source
	assertDirExists(t, root, "rhivos/mock-sensors/src")
	content := readAllRustFiles(t, root, "rhivos/mock-sensors/src")
	if content == "" {
		t.Fatal("no Rust source files found in rhivos/mock-sensors/src/")
	}

	// Must reference endpoint configuration (CLI flag or env var)
	found := false
	for _, term := range []string{"endpoint", "databroker", "addr", "DATABROKER_ADDR"} {
		if containsString(content, term) {
			found = true
			break
		}
	}
	if !found {
		t.Error("mock-sensors source does not reference endpoint/databroker/addr — expected configurable endpoint")
	}
}

// TS-02-29: UDS socket path configurable via environment (02-REQ-7.4)
//
// Verify the UDS socket path can be configured via environment variable.
func TestConfig_UdsSocketPathEnv(t *testing.T) {
	root := repoRoot(t)

	for _, svc := range []string{"locking-service", "cloud-gateway-client"} {
		t.Run(svc, func(t *testing.T) {
			srcDir := "rhivos/" + svc + "/src"
			content := readAllRustFiles(t, root, srcDir)
			if content == "" {
				t.Fatalf("no Rust source files found in %s", srcDir)
			}

			// Must reference DATABROKER_UDS_PATH env var or equivalent
			found := false
			for _, term := range []string{"DATABROKER_UDS_PATH", "databroker_uds", "UDS_PATH"} {
				if containsString(content, term) {
					found = true
					break
				}
			}
			if !found {
				t.Errorf("%s source does not reference DATABROKER_UDS_PATH env var", svc)
			}
		})
	}
}

// TS-02-31: Integration tests runnable with single command (02-REQ-8.2)
//
// Verify integration test files exist at expected paths.
func TestStructure_IntegrationTestExists(t *testing.T) {
	root := repoRoot(t)

	// Check for integration test files in the rhivos workspace
	patterns := []string{
		"rhivos/tests/integration.rs",
		"rhivos/tests/integration/*.rs",
		"rhivos/locking-service/tests/integration.rs",
		"rhivos/locking-service/tests/integration/*.rs",
	}

	found := false
	for _, pattern := range patterns {
		matches := globFiles(t, root, pattern)
		if len(matches) > 0 {
			found = true
			break
		}
	}

	if !found {
		t.Error("no integration test files found; expected files matching rhivos/**/tests/integration*.rs")
	}
}

// TS-02-P7: UDS Exclusivity — Property 7 (02-REQ-7.1, 02-REQ-7.2)
//
// For LOCKING_SERVICE and CLOUD_GATEWAY_CLIENT, the DATA_BROKER connection
// transport is UDS. The default config should NOT contain a TCP endpoint as
// the DATA_BROKER address.
func TestConfig_UdsExclusivity(t *testing.T) {
	root := repoRoot(t)

	for _, svc := range []string{"locking-service", "cloud-gateway-client"} {
		t.Run(svc, func(t *testing.T) {
			srcDir := "rhivos/" + svc + "/src"
			content := readAllRustFiles(t, root, srcDir)
			if content == "" {
				t.Fatalf("no Rust source files found in %s", srcDir)
			}

			// Must reference UDS/unix/.sock
			foundUDS := false
			for _, term := range []string{"unix", "uds", ".sock", "UDS", "Unix"} {
				if containsString(content, term) {
					foundUDS = true
					break
				}
			}
			if !foundUDS {
				t.Errorf("%s source does not reference UDS — expected Unix Domain Socket endpoint", svc)
			}

			// Default config should NOT use localhost:55556 as the DATA_BROKER endpoint
			// (TCP is only for cross-partition access, not for same-partition services)
			if containsString(content, "localhost:55556") {
				// Only flag this if it's used as a DEFAULT, not just referenced
				// Look for it set as a default endpoint
				if containsString(content, "default") && containsString(content, "55556") {
					t.Errorf("%s appears to use TCP endpoint (localhost:55556) as default DATA_BROKER address — should use UDS", svc)
				}
			}
		})
	}
}

// containsString is a simple case-sensitive substring check helper.
func containsString(haystack, needle string) bool {
	return len(needle) > 0 && len(haystack) > 0 && contains(haystack, needle)
}

func contains(s, substr string) bool {
	return len(s) >= len(substr) && searchString(s, substr)
}

func searchString(s, substr string) bool {
	for i := 0; i <= len(s)-len(substr); i++ {
		if s[i:i+len(substr)] == substr {
			return true
		}
	}
	return false
}
