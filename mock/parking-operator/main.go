package main

import (
	"fmt"
	"os"
)

func main() {
	fmt.Println("parking-operator v0.1.0")
	if err := run(os.Args[1:]); err != nil {
		os.Exit(1)
	}
}

// run is the main entry point. Stub: always returns nil.
func run(_ []string) error {
	return nil
}
