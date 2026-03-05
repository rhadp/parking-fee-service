package main

import (
	"fmt"
	"os"
)

var validCommands = []string{"add-zone", "list-zones", "check-fee"}

func main() {
	if len(os.Args) < 2 {
		printUsage()
		os.Exit(0)
	}

	cmd := os.Args[1]
	for _, valid := range validCommands {
		if cmd == valid {
			fmt.Printf("parking-operator: executing '%s'...\n", cmd)
			os.Exit(0)
		}
	}

	fmt.Fprintf(os.Stderr, "Error: unknown command '%s'\n", cmd)
	fmt.Fprintf(os.Stderr, "Valid commands: %v\n", validCommands)
	os.Exit(1)
}

func printUsage() {
	fmt.Println("Usage: parking-operator <command>")
	fmt.Println()
	fmt.Println("A mock CLI app simulating the parking operator for integration testing.")
	fmt.Println()
	fmt.Println("Commands:")
	for _, cmd := range validCommands {
		fmt.Printf("  %s\n", cmd)
	}
}
