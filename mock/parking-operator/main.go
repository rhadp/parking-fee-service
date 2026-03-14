package main

import (
	"fmt"
	"os"
	"strconv"
)

// getPort returns the port to listen on. It reads from the PORT environment
// variable first, then the --port flag, then falls back to 8080.
// Satisfies: 09-REQ-5.4
func getPort(args []string) int {
	// Check --port=N flag in args.
	for _, arg := range args {
		if len(arg) > 7 && arg[:7] == "--port=" {
			if n, err := strconv.Atoi(arg[7:]); err == nil {
				return n
			}
		}
	}
	// Check PORT environment variable.
	if v := os.Getenv("PORT"); v != "" {
		if n, err := strconv.Atoi(v); err == nil {
			return n
		}
	}
	return 8080
}

func main() {
	args := os.Args[1:]

	if len(args) == 0 || args[0] == "--help" || args[0] == "-h" {
		fmt.Fprintln(os.Stderr, "Usage: parking-operator <subcommand> [options]")
		fmt.Fprintln(os.Stderr, "")
		fmt.Fprintln(os.Stderr, "Subcommands:")
		fmt.Fprintln(os.Stderr, "  serve [--port=N]   Start the HTTP server")
		fmt.Fprintln(os.Stderr, "")
		fmt.Fprintln(os.Stderr, "Environment variables:")
		fmt.Fprintln(os.Stderr, "  PORT   Listen port (default 8080)")
		if len(args) > 0 && (args[0] == "--help" || args[0] == "-h") {
			os.Exit(0)
		}
		os.Exit(1)
	}

	switch args[0] {
	case "serve":
		port := getPort(args[1:])
		fmt.Printf("parking-operator: starting server on port %d (stub — not yet implemented)\n", port)
		os.Exit(1)
	default:
		fmt.Fprintf(os.Stderr, "unknown subcommand: %q\n", args[0])
		fmt.Fprintln(os.Stderr, "Run 'parking-operator --help' for usage.")
		os.Exit(1)
	}
}
