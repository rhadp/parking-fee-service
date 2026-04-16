package main

import (
	"context"
	"fmt"
	"log/slog"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"parking-fee-service/backend/parking-fee-service/config"
	"parking-fee-service/backend/parking-fee-service/handler"
	"parking-fee-service/backend/parking-fee-service/store"
)

const version = "0.1.0"

func main() {
	// Load config from CONFIG_PATH env var, defaulting to "config.json".
	configPath := os.Getenv("CONFIG_PATH")
	if configPath == "" {
		configPath = "config.json"
	}

	cfg, err := config.LoadConfig(configPath)
	if err != nil {
		slog.Error("failed to load config", "error", err, "path", configPath)
		os.Exit(1)
	}

	// Build the in-memory store.
	s := store.NewStore(cfg.Zones, cfg.Operators)

	// Register routes using Go 1.22 ServeMux patterns.
	mux := http.NewServeMux()
	mux.Handle("GET /operators", handler.NewOperatorHandler(s, cfg.Zones, cfg.ProximityThreshold))
	mux.Handle("GET /operators/{id}/adapter", handler.NewAdapterHandler(s))
	mux.Handle("GET /health", handler.HealthHandler())

	addr := fmt.Sprintf(":%d", cfg.Port)
	srv := &http.Server{
		Addr:    addr,
		Handler: mux,
	}

	// Log startup info.
	slog.Info("parking-fee-service starting",
		"version", version,
		"port", cfg.Port,
		"zones", len(cfg.Zones),
		"operators", len(cfg.Operators),
	)

	// Listen for shutdown signals in a goroutine.
	stop := make(chan os.Signal, 1)
	signal.Notify(stop, syscall.SIGTERM, syscall.SIGINT)

	serverErr := make(chan error, 1)
	go func() {
		slog.Info("parking-fee-service ready", "addr", addr)
		if err := srv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			serverErr <- err
		}
	}()

	// Block until signal or server error.
	select {
	case sig := <-stop:
		slog.Info("shutting down", "signal", sig.String())
	case err := <-serverErr:
		slog.Error("server error", "error", err)
		os.Exit(1)
	}

	// Graceful shutdown with a 10-second timeout.
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	if err := srv.Shutdown(ctx); err != nil {
		slog.Error("shutdown error", "error", err)
		os.Exit(1)
	}

	slog.Info("parking-fee-service stopped")
}
