package cmd

import (
	"strings"
	"testing"
)

// TS-09-E4: Unknown subcommand produces an error.
func TestSubcommandDispatch_UnknownCommand(t *testing.T) {
	err := Dispatch("foobar", nil)
	if err == nil {
		t.Fatal("expected error for unknown subcommand")
	}
	if !strings.Contains(err.Error(), "unknown") {
		t.Errorf("error should mention 'unknown', got: %v", err)
	}
}

// TS-09-6: Each known subcommand is routed (doesn't produce unknown error).
func TestSubcommandDispatch_KnownCommands(t *testing.T) {
	for _, subcmd := range ValidSubcommands {
		t.Run(subcmd, func(t *testing.T) {
			err := Dispatch(subcmd, nil)
			if err != nil && strings.Contains(err.Error(), "unknown") {
				t.Errorf("subcommand %q should not produce unknown error", subcmd)
			}
		})
	}
}

// TS-09-6: IsValidSubcommand recognizes all 3 subcommands.
func TestIsValidSubcommand(t *testing.T) {
	expected := []string{"lock", "unlock", "status"}
	for _, subcmd := range expected {
		if !IsValidSubcommand(subcmd) {
			t.Errorf("expected %q to be a valid subcommand", subcmd)
		}
	}
	if IsValidSubcommand("foobar") {
		t.Error("foobar should not be a valid subcommand")
	}
}
