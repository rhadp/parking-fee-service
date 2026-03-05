package main

import "testing"

func TestValidCommands(t *testing.T) {
	if len(validCommands) == 0 {
		t.Fatal("expected at least one valid command")
	}
	t.Logf("parking-operator has %d valid commands", len(validCommands))
}
