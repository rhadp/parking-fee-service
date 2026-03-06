package main

import (
	"fmt"
	"os"

	"github.com/parking-fee-service/mock/parking-app-cli/cmd"
)

func main() {
	if len(os.Args) < 2 {
		printUsage()
		os.Exit(0)
	}

	subcmd := os.Args[1]
	if err := cmd.Run(subcmd, os.Args[2:]); err != nil {
		fmt.Fprintf(os.Stderr, "Error: %v\n", err)
		os.Exit(1)
	}
}

func printUsage() {
	fmt.Fprintln(os.Stderr, "Usage: parking-app-cli <subcommand> [flags]")
	fmt.Fprintln(os.Stderr, "")
	fmt.Fprintln(os.Stderr, "Subcommands:")
	for _, name := range cmd.SubcommandNames() {
		fmt.Fprintf(os.Stderr, "  %s\n", name)
	}
}
