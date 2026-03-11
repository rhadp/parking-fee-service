package main

import (
	"fmt"
	"os"
)

func main() {
	if len(os.Args) < 2 {
		printUsage()
		os.Exit(0)
	}

	switch os.Args[1] {
	case "help":
		printUsage()
	default:
		fmt.Fprintf(os.Stderr, "Error: unknown subcommand %q\n", os.Args[1])
		fmt.Fprintln(os.Stderr, "Valid subcommands: help")
		os.Exit(1)
	}
}

func printUsage() {
	fmt.Println("Usage: companion-app-cli <subcommand>")
	fmt.Println()
	fmt.Println("Subcommands:")
	fmt.Println("  help    Show this help message")
}
