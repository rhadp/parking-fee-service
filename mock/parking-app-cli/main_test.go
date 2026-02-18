package main

import (
	"os"
	"testing"
)

func TestParseGlobalFlagsDefaults(t *testing.T) {
	// Clear env to avoid interference.
	os.Unsetenv("UPDATE_SERVICE_ADDR")
	os.Unsetenv("ADAPTER_ADDR")

	remaining, err := parseGlobalFlags([]string{"list-adapters"})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if updateServiceAddr != "localhost:50053" {
		t.Errorf("expected default update-service-addr 'localhost:50053', got %q", updateServiceAddr)
	}
	if adapterAddr != "localhost:50054" {
		t.Errorf("expected default adapter-addr 'localhost:50054', got %q", adapterAddr)
	}
	if len(remaining) != 1 || remaining[0] != "list-adapters" {
		t.Errorf("expected remaining [list-adapters], got %v", remaining)
	}
}

func TestParseGlobalFlagsCustom(t *testing.T) {
	os.Unsetenv("UPDATE_SERVICE_ADDR")
	os.Unsetenv("ADAPTER_ADDR")

	remaining, err := parseGlobalFlags([]string{
		"--update-service-addr", "10.0.0.1:50053",
		"--adapter-addr", "10.0.0.2:50054",
		"install-adapter",
	})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if updateServiceAddr != "10.0.0.1:50053" {
		t.Errorf("expected update-service-addr '10.0.0.1:50053', got %q", updateServiceAddr)
	}
	if adapterAddr != "10.0.0.2:50054" {
		t.Errorf("expected adapter-addr '10.0.0.2:50054', got %q", adapterAddr)
	}
	if len(remaining) != 1 || remaining[0] != "install-adapter" {
		t.Errorf("expected remaining [install-adapter], got %v", remaining)
	}
}

func TestParseGlobalFlagsFromEnv(t *testing.T) {
	t.Setenv("UPDATE_SERVICE_ADDR", "envhost:50053")
	t.Setenv("ADAPTER_ADDR", "envhost:50054")

	remaining, err := parseGlobalFlags([]string{"get-rate"})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if updateServiceAddr != "envhost:50053" {
		t.Errorf("expected update-service-addr from env 'envhost:50053', got %q", updateServiceAddr)
	}
	if adapterAddr != "envhost:50054" {
		t.Errorf("expected adapter-addr from env 'envhost:50054', got %q", adapterAddr)
	}
	if len(remaining) != 1 || remaining[0] != "get-rate" {
		t.Errorf("expected remaining [get-rate], got %v", remaining)
	}
}

func TestRunNoArgs(t *testing.T) {
	os.Unsetenv("UPDATE_SERVICE_ADDR")
	os.Unsetenv("ADAPTER_ADDR")

	err := run(nil)
	if err == nil {
		t.Fatal("expected error for no args")
	}
	if err.Error() != "no command specified" {
		t.Errorf("expected 'no command specified', got %q", err.Error())
	}
}

func TestRunUnknownCommand(t *testing.T) {
	os.Unsetenv("UPDATE_SERVICE_ADDR")
	os.Unsetenv("ADAPTER_ADDR")

	err := run([]string{"nonexistent"})
	if err == nil {
		t.Fatal("expected error for unknown command")
	}
	expected := "unknown command: nonexistent"
	if err.Error() != expected {
		t.Errorf("expected %q, got %q", expected, err.Error())
	}
}

func TestRunHelp(t *testing.T) {
	os.Unsetenv("UPDATE_SERVICE_ADDR")
	os.Unsetenv("ADAPTER_ADDR")

	err := run([]string{"help"})
	if err != nil {
		t.Fatalf("unexpected error for help command: %v", err)
	}
}

func TestFlagValue(t *testing.T) {
	tests := []struct {
		name     string
		args     []string
		flag     string
		def      string
		expected string
	}{
		{"present", []string{"--zone-id", "zone-a"}, "--zone-id", "default", "zone-a"},
		{"absent", []string{}, "--zone-id", "default", "default"},
		{"last arg no value", []string{"--zone-id"}, "--zone-id", "default", "default"},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			result := flagValue(tc.args, tc.flag, tc.def)
			if result != tc.expected {
				t.Errorf("expected %q, got %q", tc.expected, result)
			}
		})
	}
}

func TestEnvOrDefault(t *testing.T) {
	t.Setenv("TEST_CLI_VAR", "custom-value")
	if v := envOrDefault("TEST_CLI_VAR", "default"); v != "custom-value" {
		t.Errorf("expected 'custom-value', got %q", v)
	}

	os.Unsetenv("TEST_CLI_VAR_UNSET")
	if v := envOrDefault("TEST_CLI_VAR_UNSET", "fallback"); v != "fallback" {
		t.Errorf("expected 'fallback', got %q", v)
	}
}

// TestAllSubcommandsRecognized verifies that the run function recognizes all
// required subcommands without actually making gRPC calls (they will fail
// at connection time, but the command parsing should succeed).
func TestAllSubcommandsRecognized(t *testing.T) {
	subcommands := []string{
		"install-adapter",
		"list-adapters",
		"remove-adapter",
		"adapter-status",
		"watch-adapters",
		"start-session",
		"stop-session",
		"get-status",
		"get-rate",
	}

	for _, cmd := range subcommands {
		t.Run(cmd, func(t *testing.T) {
			os.Unsetenv("UPDATE_SERVICE_ADDR")
			os.Unsetenv("ADAPTER_ADDR")

			// Use an address that will fail quickly.
			err := run([]string{
				"--update-service-addr", "localhost:1",
				"--adapter-addr", "localhost:1",
				cmd,
			})
			// We expect a connection error, not an "unknown command" error.
			if err == nil {
				return // Some commands might need required flags
			}
			if err.Error() == "unknown command: "+cmd {
				t.Errorf("command %q was not recognized", cmd)
			}
		})
	}
}
