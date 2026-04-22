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
	os.Exit(run())
}

func run() int {
	configPath := os.Getenv("CONFIG_PATH")
	if configPath == "" {
		configPath = "config.json"
	}

	cfg, err := config.LoadConfig(configPath)
	if err != nil {
		slog.Error("failed to load configuration", "error", err)
		return 1
	}

	s := store.NewStore(cfg.Zones, cfg.Operators)

	mux := http.NewServeMux()
	mux.HandleFunc("GET /operators", handler.NewOperatorHandler(s, cfg.Zones, cfg.ProximityThreshold))
	mux.HandleFunc("GET /operators/{id}/adapter", handler.NewAdapterHandler(s))
	mux.HandleFunc("GET /health", handler.HealthHandler())

	addr := fmt.Sprintf(":%d", cfg.Port)
	srv := &http.Server{
		Addr:    addr,
		Handler: mux,
	}

	slog.Info("parking-fee-service starting",
		"version", version,
		"port", cfg.Port,
		"zones", len(cfg.Zones),
		"operators", len(cfg.Operators),
	)
	slog.Info("parking-fee-service ready")

	// Handle graceful shutdown on SIGTERM/SIGINT.
	sigCh := make(chan os.Signal, 1)
	signal.Notify(sigCh, syscall.SIGTERM, syscall.SIGINT)

	errCh := make(chan error, 1)
	go func() {
		errCh <- srv.ListenAndServe()
	}()

	select {
	case sig := <-sigCh:
		slog.Info("received signal, shutting down", "signal", sig)
	case err := <-errCh:
		if err != nil && err != http.ErrServerClosed {
			slog.Error("server error", "error", err)
			return 1
		}
	}

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	if err := srv.Shutdown(ctx); err != nil {
		slog.Error("shutdown error", "error", err)
		return 1
	}

	slog.Info("parking-fee-service stopped")
	return 0
}
