package main

import (
	"context"
	"fmt"
	"log"
	"net/http"
	"os"
	"os/signal"
	"strconv"
	"syscall"
	"time"

	"parking-fee-service/mock/parking-operator/server"
)

func main() {
	if err := run(os.Args[1:]); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}

func run(args []string) error {
	if len(args) == 0 || args[0] != "serve" {
		return fmt.Errorf("usage: parking-operator serve [--port=PORT]")
	}

	port := resolvePort(args[1:])

	srv := server.New()
	httpSrv := &http.Server{
		Addr:    ":" + strconv.Itoa(port),
		Handler: srv,
	}

	// Channel to capture server errors.
	errCh := make(chan error, 1)
	go func() {
		log.Printf("parking-operator listening on :%d", port)
		if err := httpSrv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			errCh <- err
		}
		close(errCh)
	}()

	// Wait for shutdown signal.
	sigCh := make(chan os.Signal, 1)
	signal.Notify(sigCh, syscall.SIGTERM, syscall.SIGINT)

	select {
	case sig := <-sigCh:
		log.Printf("received %s, shutting down", sig)
	case err := <-errCh:
		if err != nil {
			return fmt.Errorf("server error: %w", err)
		}
	}

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	if err := httpSrv.Shutdown(ctx); err != nil {
		return fmt.Errorf("shutdown error: %w", err)
	}

	log.Println("parking-operator stopped")
	return nil
}

// resolvePort determines the port from flags, env, or default (8080).
func resolvePort(args []string) int {
	// Check --port=N flag.
	for _, arg := range args {
		if len(arg) > 7 && arg[:7] == "--port=" {
			if p, err := strconv.Atoi(arg[7:]); err == nil && p > 0 {
				return p
			}
		}
	}

	// Check PORT env var.
	if envPort := os.Getenv("PORT"); envPort != "" {
		if p, err := strconv.Atoi(envPort); err == nil && p > 0 {
			return p
		}
	}

	return 8080
}
