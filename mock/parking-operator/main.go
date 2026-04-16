// parking-operator is a mock REST server simulating a parking operator backend.
// Stub — full implementation in task group 3.
package main

import (
	"flag"
	"fmt"
	"net/http"
	"os"
)

func main() {
	port := flag.String("port", "", "port to listen on (default 8080, env PORT)")
	flag.Parse()

	if *port == "" {
		*port = os.Getenv("PORT")
	}
	if *port == "" {
		*port = "8080"
	}

	if flag.Arg(0) != "serve" {
		fmt.Fprintln(os.Stderr, "usage: parking-operator serve [--port=<port>]")
		os.Exit(1)
	}

	s := newServer()
	mux := http.NewServeMux()
	mux.HandleFunc("POST /parking/start", s.handleStart)
	mux.HandleFunc("POST /parking/stop", s.handleStop)
	mux.HandleFunc("GET /parking/status/{session_id}", s.handleStatus)

	addr := ":" + *port
	fmt.Fprintf(os.Stderr, "parking-operator listening on %s\n", addr)
	if err := http.ListenAndServe(addr, mux); err != nil {
		fmt.Fprintf(os.Stderr, "server error: %v\n", err)
		os.Exit(1)
	}
}
