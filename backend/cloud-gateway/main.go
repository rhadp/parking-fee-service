package main

import (
	"fmt"
	"os"
	"strings"
)

func main() {
	for _, arg := range os.Args[1:] {
		if strings.HasPrefix(arg, "-") {
			fmt.Fprintf(os.Stderr, "Usage: cloud-gateway\n")
			os.Exit(1)
		}
	}
	fmt.Println("cloud-gateway v0.1.0")
}
