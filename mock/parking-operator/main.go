// Package main provides the parking-operator mock server CLI.
// Implementation pending: task group 3.
package main

import (
	"fmt"
	"os"
)

func main() {
	// Unknown flags → print usage and exit 1.
	for _, arg := range os.Args[1:] {
		if len(arg) > 0 && arg[0] == '-' {
			fmt.Fprintln(os.Stderr, "usage: parking-operator serve [--port=PORT]")
			os.Exit(1)
		}
	}
	if len(os.Args) >= 2 && os.Args[1] == "serve" {
		// stub: serve not yet implemented
		fmt.Fprintln(os.Stderr, "parking-operator serve: not yet implemented")
		os.Exit(1)
	}
	fmt.Println("parking-operator v0.1.0")
}
