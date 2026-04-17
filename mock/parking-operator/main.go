// parking-operator mock REST server.
// Task group 3 will add the real serve subcommand with graceful shutdown.
package main

import (
	"fmt"
	"os"
)

func main() {
	if len(os.Args) < 2 || os.Args[1] != "serve" {
		fmt.Fprintln(os.Stderr, "parking-operator: usage: parking-operator serve [--port=PORT]")
		os.Exit(1)
	}
	fmt.Fprintln(os.Stderr, "parking-operator: serve not implemented")
	os.Exit(1)
}
