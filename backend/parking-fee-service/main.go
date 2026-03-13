package main

import (
	"context"
	"fmt"
	"log/slog"
	"net/http"
	"os"
	"os/signal"
	"syscall"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/config"
	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/handler"
	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/store"
)

const version = "0.1.0"

func main() {
	// Load configuration.
	configPath := os.Getenv("CONFIG_PATH")
	if configPath == "" {
		configPath = "config.json"
	}

	cfg, err := config.LoadConfig(configPath)
	if err != nil {
		slog.Error("failed to load config", "err", err)
		os.Exit(1)
	}

	// Allow PORT env var to override configured port (useful for tests).
	portStr := os.Getenv("PORT")
	port := cfg.Port
	if portStr != "" {
		var p int
		if _, err := fmt.Sscanf(portStr, "%d", &p); err == nil && p > 0 {
			port = p
		}
	}

	// Build the in-memory store.
	s := store.NewStore(cfg.Zones, cfg.Operators)

	// Register routes using Go 1.22 ServeMux patterns.
	mux := http.NewServeMux()
	mux.Handle("GET /health", handler.HealthHandler())
	mux.Handle("GET /operators", handler.NewOperatorHandler(s, cfg.Zones, cfg.ProximityThreshold))
	mux.Handle("GET /operators/{id}/adapter", handler.NewAdapterHandler(s))

	srv := &http.Server{
		Addr:    fmt.Sprintf(":%d", port),
		Handler: mux,
	}

	// Log startup information.
	slog.Info("parking-fee-service starting",
		"version", version,
		"port", port,
		"zones", len(cfg.Zones),
		"operators", len(cfg.Operators),
	)

	// Set up graceful shutdown on SIGTERM/SIGINT.
	sigCh := make(chan os.Signal, 1)
	signal.Notify(sigCh, syscall.SIGTERM, syscall.SIGINT)

	// Start the server in a goroutine.
	errCh := make(chan error, 1)
	go func() {
		slog.Info("parking-fee-service ready", "addr", srv.Addr)
		if err := srv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			errCh <- err
		}
	}()

	// Wait for shutdown signal or server error.
	select {
	case sig := <-sigCh:
		slog.Info("received signal, shutting down", "signal", sig)
	case err := <-errCh:
		slog.Error("server error", "err", err)
		os.Exit(1)
	}

	// Graceful shutdown.
	ctx, cancel := context.WithTimeout(context.Background(), 0)
	defer cancel()
	if err := srv.Shutdown(ctx); err != nil {
		slog.Error("shutdown error", "err", err)
		os.Exit(1)
	}

	slog.Info("parking-fee-service stopped")
	os.Exit(0)
}
