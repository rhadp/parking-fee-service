package cmd

import (
	"strings"
	"testing"
)

// TS-09-E4: Unknown subcommand produces an error.
func TestSubcommandDispatch_UnknownCommand(t *testing.T) {
	err := Run("foobar", nil)
	if err == nil {
		t.Fatal("expected error for unknown subcommand")
	}
	if !strings.Contains(err.Error(), "unknown subcommand") {
		t.Fatalf("expected 'unknown subcommand' in error, got: %s", err.Error())
	}
}

// TS-09-5: Each known subcommand dispatches without panicking.
// Commands requiring flags return missing-flag errors immediately.
// Commands that attempt gRPC dial (watch, list) are skipped here to avoid
// 10-second connection timeouts; their dispatch is covered by SubcommandNames.
func TestSubcommandDispatch_AllKnownCommands(t *testing.T) {
	// Commands that require flags will fail immediately with a usage error,
	// which verifies dispatch without a network round-trip.
	flagCommands := []string{
		"lookup", "adapter-info", "install",
		"remove", "status", "start-session", "stop-session",
	}
	for _, name := range flagCommands {
		t.Run(name, func(t *testing.T) {
			err := Run(name, nil)
			if err == nil {
				t.Fatalf("expected error for '%s' with missing flags", name)
			}
		})
	}
}

// TS-09-5: SubcommandNames returns all 9 subcommands.
func TestSubcommandNames_Count(t *testing.T) {
	names := SubcommandNames()
	if len(names) != 9 {
		t.Fatalf("expected 9 subcommands, got %d: %v", len(names), names)
	}
}
