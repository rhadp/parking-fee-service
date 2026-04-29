package main

import (
	"fmt"
	"os"
)

func main() {
	if len(os.Args) > 1 {
		fmt.Fprintf(os.Stderr, "Error: unknown argument '%s'\n", os.Args[1])
		fmt.Fprintf(os.Stderr, "Usage: %s\n", os.Args[0])
		os.Exit(1)
	}
	fmt.Println("cloud-gateway v0.1.0")
}
