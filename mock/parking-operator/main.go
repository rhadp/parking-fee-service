package main

import (
	"context"
	"errors"
	"fmt"
	"log"
	"net/http"
	"os"
	"os/signal"
	"strconv"
	"strings"
	"syscall"
	"time"

	"github.com/rhadp/parking-fee-service/mock/parking-operator/server"
)

func main() {
	if err := run(os.Args[1:]); err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		os.Exit(1)
	}
}

// run is the main entry point, parsing the subcommand and flags.
func run(args []string) error {
	if len(args) == 0 {
		return errors.New("usage: parking-operator serve [--port=PORT]")
	}

	switch args[0] {
	case "serve":
		return runServe(args[1:])
	default:
		return fmt.Errorf("unknown subcommand: %s\nusage: parking-operator serve [--port=PORT]", args[0])
	}
}

// runServe starts the HTTP server and blocks until SIGTERM/SIGINT.
func runServe(args []string) error {
	port := resolvePort(args)

	handler := server.New()
	addr := fmt.Sprintf(":%d", port)

	srv := &http.Server{
		Addr:    addr,
		Handler: handler,
	}

	// Listen for shutdown signals.
	sigCh := make(chan os.Signal, 1)
	signal.Notify(sigCh, syscall.SIGTERM, syscall.SIGINT)

	// Start server in a goroutine.
	errCh := make(chan error, 1)
	go func() {
		log.Printf("parking-operator listening on %s", addr)
		if err := srv.ListenAndServe(); err != nil && !errors.Is(err, http.ErrServerClosed) {
			errCh <- err
		}
		close(errCh)
	}()

	// Wait for signal or server error.
	select {
	case sig := <-sigCh:
		log.Printf("received signal %v, shutting down", sig)
	case err := <-errCh:
		if err != nil {
			return fmt.Errorf("server error: %w", err)
		}
	}

	// Graceful shutdown with a 5-second timeout.
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	if err := srv.Shutdown(ctx); err != nil {
		return fmt.Errorf("shutdown error: %w", err)
	}

	log.Println("server stopped")
	return nil
}

// resolvePort determines the port from flags, env, or default (9090).
func resolvePort(args []string) int {
	// Check --port flag.
	for _, arg := range args {
		if val, ok := strings.CutPrefix(arg, "--port="); ok {
			if p, err := strconv.Atoi(val); err == nil {
				return p
			}
		}
	}

	// Check PORT environment variable.
	if envPort := os.Getenv("PORT"); envPort != "" {
		if p, err := strconv.Atoi(envPort); err == nil {
			return p
		}
	}

	// Default.
	return 9090
}
