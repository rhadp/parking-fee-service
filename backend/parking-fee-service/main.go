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
	configPath := os.Getenv("CONFIG_PATH")
	if configPath == "" {
		configPath = "config.json"
	}

	cfg, err := config.LoadConfig(configPath)
	if err != nil {
		slog.Error("failed to load configuration", "path", configPath, "error", err)
		os.Exit(1)
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

	// Graceful shutdown on SIGTERM/SIGINT.
	sigCh := make(chan os.Signal, 1)
	signal.Notify(sigCh, syscall.SIGTERM, syscall.SIGINT)

	go func() {
		if err := srv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			slog.Error("server error", "error", err)
			os.Exit(1)
		}
	}()

	sig := <-sigCh
	slog.Info("received signal, shutting down", "signal", sig)

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	if err := srv.Shutdown(ctx); err != nil {
		slog.Error("shutdown error", "error", err)
		os.Exit(1)
	}

	slog.Info("parking-fee-service stopped")
}
