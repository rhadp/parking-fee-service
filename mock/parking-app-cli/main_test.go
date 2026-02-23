package main

import (
	"testing"

	"github.com/spf13/cobra"
)

func TestRootCmdExists(t *testing.T) {
	if rootCmd == nil {
		t.Fatal("expected rootCmd to be defined")
	}
	if rootCmd.Use != "parking-app-cli" {
		t.Errorf("expected rootCmd.Use to be 'parking-app-cli', got %q", rootCmd.Use)
	}
}

// TestCLI_CommandsRegistered verifies all expected commands are registered on
// the root command.
func TestCLI_CommandsRegistered(t *testing.T) {
	expected := []string{
		"lookup", "install", "watch", "list", "status",
		"start-session", "stop-session", "get-status", "get-rate",
	}

	commands := rootCmd.Commands()
	names := make(map[string]bool)
	for _, cmd := range commands {
		names[cmd.Use] = true
	}

	for _, name := range expected {
		if !names[name] {
			t.Errorf("expected command %q to be registered", name)
		}
	}
}

// TestCLI_InstallFlags verifies the install command has the expected flags.
func TestCLI_InstallFlags(t *testing.T) {
	f := installCmd.Flags()
	if f.Lookup("image-ref") == nil {
		t.Error("expected install command to have --image-ref flag")
	}
	if f.Lookup("checksum") == nil {
		t.Error("expected install command to have --checksum flag")
	}
}

// TestCLI_StartSessionFlags verifies the start-session command has the expected flags.
func TestCLI_StartSessionFlags(t *testing.T) {
	f := startSessionCmd.Flags()
	if f.Lookup("vehicle-id") == nil {
		t.Error("expected start-session command to have --vehicle-id flag")
	}
	if f.Lookup("zone-id") == nil {
		t.Error("expected start-session command to have --zone-id flag")
	}
}

// TestCLI_StopSessionFlags verifies the stop-session command has the expected flags.
func TestCLI_StopSessionFlags(t *testing.T) {
	f := stopSessionCmd.Flags()
	if f.Lookup("session-id") == nil {
		t.Error("expected stop-session command to have --session-id flag")
	}
}

// TestCLI_SilenceSettings verifies that commands silence usage and errors
// to allow main() to handle error printing.
func TestCLI_SilenceSettings(t *testing.T) {
	cmds := []*cobra.Command{
		installCmd, watchCmd, listCmd, statusCmd,
		startSessionCmd, stopSessionCmd, getStatusCmd, getRateCmd,
	}
	for _, cmd := range cmds {
		if !cmd.SilenceUsage {
			t.Errorf("expected %q to have SilenceUsage=true", cmd.Use)
		}
		if !cmd.SilenceErrors {
			t.Errorf("expected %q to have SilenceErrors=true", cmd.Use)
		}
	}
}
