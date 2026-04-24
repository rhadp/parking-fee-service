package main

import (
	"fmt"
	"os"
	"strings"
)

func main() {
	for _, arg := range os.Args[1:] {
		if strings.HasPrefix(arg, "-") {
			fmt.Fprintf(os.Stderr, "Usage: companion-app-cli\n")
			os.Exit(1)
		}
	}
	fmt.Println("companion-app-cli v0.1.0")
}
