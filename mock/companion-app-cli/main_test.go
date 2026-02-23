package main

import (
	"testing"
)

func TestRootCmdExists(t *testing.T) {
	if rootCmd == nil {
		t.Fatal("expected rootCmd to be defined")
	}
	if rootCmd.Use != "companion-app-cli" {
		t.Errorf("expected rootCmd.Use to be 'companion-app-cli', got %q", rootCmd.Use)
	}
}
