// Package main provides the parking-operator mock server CLI.
package main

import (
	"context"
	"flag"
	"fmt"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"
)

func main() {
	if len(os.Args) < 2 {
		// No arguments → print version and exit 0 (01-REQ-4.2, 01-REQ-4.4).
		fmt.Println("parking-operator v0.1.0")
		return
	}

	switch os.Args[1] {
	case "serve":
		if err := runServe(os.Args[2:]); err != nil {
			fmt.Fprintln(os.Stderr, err)
			os.Exit(1)
		}
	default:
		fmt.Fprintf(os.Stderr, "unknown subcommand %q\nusage: parking-operator serve [--port=PORT]\n", os.Args[1])
		os.Exit(1)
	}
}

// runServe parses serve flags, starts the HTTP server, and handles graceful shutdown.
func runServe(args []string) error {
	fs := flag.NewFlagSet("serve", flag.ContinueOnError)
	port := fs.String("port", "", "TCP port to listen on (default: 8080, or PORT env var)")
	if err := fs.Parse(args); err != nil {
		return err
	}

	// Resolve port: flag > env > default
	listenPort := *port
	if listenPort == "" {
		listenPort = os.Getenv("PORT")
	}
	if listenPort == "" {
		listenPort = "8080"
	}

	addr := ":" + listenPort
	srv := &http.Server{
		Addr:    addr,
		Handler: NewServer(),
	}

	// Channel to receive OS signals
	quit := make(chan os.Signal, 1)
	signal.Notify(quit, syscall.SIGTERM, syscall.SIGINT)

	// Start the server in a goroutine
	serverErr := make(chan error, 1)
	go func() {
		fmt.Fprintf(os.Stderr, "parking-operator listening on %s\n", addr)
		if err := srv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			serverErr <- err
		}
	}()

	// Wait for signal or server error
	select {
	case err := <-serverErr:
		return fmt.Errorf("server error: %w", err)
	case <-quit:
		// Graceful shutdown with 5-second timeout
		ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
		defer cancel()
		if err := srv.Shutdown(ctx); err != nil {
			return fmt.Errorf("graceful shutdown failed: %w", err)
		}
	}

	return nil
}
