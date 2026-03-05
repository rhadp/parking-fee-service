package main

import (
	"fmt"
	"os"
)

var validCommands = []string{"start-session", "stop-session", "status"}

func main() {
	if len(os.Args) < 2 {
		printUsage()
		os.Exit(0)
	}

	cmd := os.Args[1]
	for _, valid := range validCommands {
		if cmd == valid {
			fmt.Printf("parking-app-cli: executing '%s'...\n", cmd)
			os.Exit(0)
		}
	}

	fmt.Fprintf(os.Stderr, "Error: unknown command '%s'\n", cmd)
	fmt.Fprintf(os.Stderr, "Valid commands: %v\n", validCommands)
	os.Exit(1)
}

func printUsage() {
	fmt.Println("Usage: parking-app-cli <command>")
	fmt.Println()
	fmt.Println("A mock CLI app simulating the parking app for integration testing.")
	fmt.Println()
	fmt.Println("Commands:")
	for _, cmd := range validCommands {
		fmt.Printf("  %s\n", cmd)
	}
}
