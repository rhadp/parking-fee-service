package main

import (
	"context"
	"fmt"
	"log"
	"net/http"
	"os"
	"os/signal"
	"strconv"
	"strings"
	"syscall"
	"time"
)

func main() {
	if len(os.Args) < 2 || os.Args[1] != "serve" {
		fmt.Fprintln(os.Stderr, "Usage: parking-operator serve [--port=PORT]")
		os.Exit(1)
	}

	port := resolvePort()

	srv := NewServer()

	httpServer := &http.Server{
		Addr:    fmt.Sprintf(":%d", port),
		Handler: srv.Handler(),
	}

	// Channel to receive shutdown signals.
	sigCh := make(chan os.Signal, 1)
	signal.Notify(sigCh, syscall.SIGTERM, syscall.SIGINT)

	// Start server in background.
	errCh := make(chan error, 1)
	go func() {
		log.Printf("parking-operator listening on :%d", port)
		if err := httpServer.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			errCh <- err
		}
		close(errCh)
	}()

	// Wait for shutdown signal or server error.
	select {
	case sig := <-sigCh:
		log.Printf("received signal %v, shutting down", sig)
	case err := <-errCh:
		if err != nil {
			fmt.Fprintf(os.Stderr, "server error: %v\n", err)
			os.Exit(1)
		}
	}

	// Graceful shutdown with 5-second timeout.
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	if err := httpServer.Shutdown(ctx); err != nil {
		fmt.Fprintf(os.Stderr, "shutdown error: %v\n", err)
		os.Exit(1)
	}
}

// resolvePort determines the server port from CLI flags, environment variable,
// or default (9090). Priority: --port flag > PORT env var > default.
func resolvePort() int {
	// Check CLI args for --port flag.
	for _, arg := range os.Args[2:] {
		if val, ok := strings.CutPrefix(arg, "--port="); ok {
			p, err := strconv.Atoi(val)
			if err != nil {
				fmt.Fprintf(os.Stderr, "invalid port: %s\n", val)
				os.Exit(1)
			}
			return p
		}
	}

	// Check PORT environment variable.
	if envPort := os.Getenv("PORT"); envPort != "" {
		p, err := strconv.Atoi(envPort)
		if err != nil {
			fmt.Fprintf(os.Stderr, "invalid PORT env var: %s\n", envPort)
			os.Exit(1)
		}
		return p
	}

	return 9090
}
