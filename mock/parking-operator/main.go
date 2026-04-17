// parking-operator is a mock REST server simulating a parking operator backend.
// Usage: parking-operator serve [--port=<port>]
package main

import (
	"context"
	"flag"
	"fmt"
	"net/http"
	"os"
	"os/signal"
	"syscall"
)

func main() {
	if len(os.Args) < 2 || os.Args[1] != "serve" {
		fmt.Fprintln(os.Stderr, "usage: parking-operator serve [--port=<port>]")
		os.Exit(1)
	}

	// Parse flags from arguments after the "serve" subcommand.
	fs := flag.NewFlagSet("serve", flag.ExitOnError)
	fs.SetOutput(os.Stderr)
	port := fs.String("port", "", "port to listen on (default 8080, env PORT)")
	fs.Parse(os.Args[2:]) //nolint:errcheck

	if *port == "" {
		*port = os.Getenv("PORT")
	}
	if *port == "" {
		*port = "8080"
	}

	s := newServer()
	mux := http.NewServeMux()
	mux.HandleFunc("POST /parking/start", s.handleStart)
	mux.HandleFunc("POST /parking/stop", s.handleStop)
	mux.HandleFunc("GET /parking/status/{session_id}", s.handleStatus)

	addr := ":" + *port
	srv := &http.Server{Addr: addr, Handler: mux}

	// Listen for SIGTERM/SIGINT and shut down gracefully.
	stop := make(chan os.Signal, 1)
	signal.Notify(stop, syscall.SIGTERM, syscall.SIGINT)

	go func() {
		fmt.Fprintf(os.Stderr, "parking-operator listening on %s\n", addr)
		if err := srv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			fmt.Fprintf(os.Stderr, "server error: %v\n", err)
			os.Exit(1)
		}
	}()

	<-stop
	fmt.Fprintln(os.Stderr, "parking-operator shutting down")
	if err := srv.Shutdown(context.Background()); err != nil {
		fmt.Fprintf(os.Stderr, "shutdown error: %v\n", err)
		os.Exit(1)
	}
}
