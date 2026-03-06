package main

import (
	"testing"

	"github.com/parking-fee-service/mock/parking-app-cli/cmd"
)

// TS-09-E4: No arguments should produce a non-empty subcommand list.
func TestSubcommandNames_NonEmpty(t *testing.T) {
	names := cmd.SubcommandNames()
	if len(names) == 0 {
		t.Fatal("expected at least one subcommand")
	}
}
