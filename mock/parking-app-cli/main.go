// parking-app-cli stub.
// Task group 4 will replace this with a real implementation.
package main

import (
	"fmt"
	"os"
)

func main() {
	if len(os.Args) < 2 {
		fmt.Fprintln(os.Stderr, "parking-app-cli: subcommand required")
		os.Exit(1)
	}
	fmt.Fprintln(os.Stderr, "parking-app-cli: not implemented")
	os.Exit(1)
}
