package main

import (
	"fmt"
	"net/http"
	"os"
)

func main() {
	if len(os.Args) < 2 {
		printUsage()
		os.Exit(0)
	}

	subcmd := os.Args[1]
	switch subcmd {
	case "serve":
		runServe(os.Args[2:])
	default:
		fmt.Fprintf(os.Stderr, "unknown subcommand '%s'\n", subcmd)
		printUsage()
		os.Exit(1)
	}
}

func printUsage() {
	fmt.Fprintln(os.Stderr, "Usage: parking-operator <subcommand> [flags]")
	fmt.Fprintln(os.Stderr, "")
	fmt.Fprintln(os.Stderr, "Subcommands:")
	fmt.Fprintln(os.Stderr, "  serve    Start the parking operator REST server")
}

func runServe(args []string) {
	port := ""
	for i := 0; i < len(args)-1; i++ {
		if args[i] == "-port" || args[i] == "--port" {
			port = args[i+1]
			break
		}
	}
	if port == "" {
		port = os.Getenv("PORT")
	}
	if port == "" {
		port = "9090"
	}

	store := NewSessionStore()
	h := NewHandler(store)

	mux := http.NewServeMux()
	mux.HandleFunc("POST /parking/start", h.HandleStartParking)
	mux.HandleFunc("POST /parking/stop", h.HandleStopParking)
	mux.HandleFunc("GET /parking/status", h.HandleParkingStatus)

	addr := ":" + port
	fmt.Printf("parking-operator listening on %s\n", addr)
	if err := http.ListenAndServe(addr, mux); err != nil {
		fmt.Fprintf(os.Stderr, "Error: %v\n", err)
		os.Exit(1)
	}
}
