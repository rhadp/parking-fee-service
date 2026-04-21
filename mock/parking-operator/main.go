// Package main provides the parking-operator mock server CLI.
// Implementation pending: task group 3.
package main

import (
	"fmt"
	"os"
)

func main() {
	if len(os.Args) < 2 || os.Args[1] != "serve" {
		fmt.Fprintln(os.Stderr, "usage: parking-operator serve [--port=PORT]")
		os.Exit(1)
	}
	// stub: serve not yet implemented
	fmt.Fprintln(os.Stderr, "parking-operator serve: not yet implemented")
	os.Exit(1)
}
