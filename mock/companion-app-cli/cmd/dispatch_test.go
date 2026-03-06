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

// TS-09-6: Each known subcommand dispatches without panicking.
func TestSubcommandDispatch_AllKnownCommands(t *testing.T) {
	for _, name := range SubcommandNames() {
		t.Run(name, func(t *testing.T) {
			err := Run(name, nil)
			if err == nil {
				t.Fatalf("expected stub error for '%s' (not yet implemented)", name)
			}
		})
	}
}

// TS-09-6: SubcommandNames returns all 3 subcommands.
func TestSubcommandNames_Count(t *testing.T) {
	names := SubcommandNames()
	if len(names) != 3 {
		t.Fatalf("expected 3 subcommands, got %d: %v", len(names), names)
	}
}
