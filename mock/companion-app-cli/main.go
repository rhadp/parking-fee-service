// Package main is the companion-app-cli stub.
// Implementation pending: task group 4.
package main

import (
	"fmt"
	"os"
)

func main() {
	// Unknown flags → print usage and exit 1.
	for _, arg := range os.Args[1:] {
		if len(arg) > 0 && arg[0] == '-' {
			fmt.Fprintln(os.Stderr, "usage: companion-app-cli <command>")
			os.Exit(1)
		}
	}
	fmt.Println("companion-app-cli v0.1.0")
}
