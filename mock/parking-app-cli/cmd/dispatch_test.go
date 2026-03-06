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
func TestSubcommandDispatch_AllKnownCommands(t *testing.T) {
	for _, name := range SubcommandNames() {
		t.Run(name, func(t *testing.T) {
			// All stubs return errors, but they should not panic.
			err := Run(name, nil)
			if err == nil {
				t.Fatalf("expected stub error for '%s' (not yet implemented)", name)
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
