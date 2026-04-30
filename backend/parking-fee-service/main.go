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

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/config"
	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/handler"
	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/store"
)

const version = "0.1.0"

func main() {
	// Determine config file path.
	cfgPath := os.Getenv("CONFIG_PATH")
	if cfgPath == "" {
		cfgPath = "config.json"
	}

	// Load configuration.
	cfg, err := config.LoadConfig(cfgPath)
	if err != nil {
		slog.Error("failed to load config", "path", cfgPath, "error", err)
		os.Exit(1)
	}

	// Create in-memory store.
	s := store.NewStore(cfg.Zones, cfg.Operators)

	// Log startup information.
	slog.Info("parking-fee-service starting",
		"version", version,
		"port", cfg.Port,
		"zones", len(cfg.Zones),
		"operators", len(cfg.Operators),
	)

	// Set up HTTP routes using Go 1.22 ServeMux patterns.
	mux := http.NewServeMux()
	mux.HandleFunc("GET /operators", handler.NewOperatorHandler(s, cfg.Zones, cfg.ProximityThreshold))
	mux.HandleFunc("GET /operators/{id}/adapter", handler.NewAdapterHandler(s))
	mux.HandleFunc("GET /health", handler.HealthHandler())

	// Create HTTP server.
	srv := &http.Server{
		Addr:    fmt.Sprintf(":%d", cfg.Port),
		Handler: mux,
	}

	// Set up signal handling for graceful shutdown.
	sigCh := make(chan os.Signal, 1)
	signal.Notify(sigCh, syscall.SIGTERM, syscall.SIGINT)

	// Start server in a goroutine.
	go func() {
		slog.Info("service ready", "address", srv.Addr)
		if err := srv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			slog.Error("server error", "error", err)
			os.Exit(1)
		}
	}()

	// Wait for shutdown signal.
	sig := <-sigCh
	slog.Info("received signal, shutting down", "signal", sig)

	// Gracefully shut down with a timeout.
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	if err := srv.Shutdown(ctx); err != nil {
		slog.Error("shutdown error", "error", err)
		os.Exit(1)
	}

	slog.Info("service stopped")
}
