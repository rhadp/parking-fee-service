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

	"github.com/sdv-demo/parking-fee-service/backend/parking-fee-service/config"
	"github.com/sdv-demo/parking-fee-service/backend/parking-fee-service/handler"
	"github.com/sdv-demo/parking-fee-service/backend/parking-fee-service/store"
)

const version = "0.1.0"

func main() {
	// 05-REQ-4.1: Read CONFIG_PATH env var, defaulting to "config.json".
	configPath := os.Getenv("CONFIG_PATH")
	if configPath == "" {
		configPath = "config.json"
	}

	// 05-REQ-4.E2: Exit non-zero on invalid JSON.
	cfg, err := config.LoadConfig(configPath)
	if err != nil {
		slog.Error("failed to load configuration", "path", configPath, "error", err)
		os.Exit(1)
	}

	s := store.NewStore(cfg.Zones, cfg.Operators)

	mux := http.NewServeMux()
	mux.Handle("GET /operators", handler.NewOperatorHandler(s, cfg.Zones, cfg.ProximityThreshold))
	mux.Handle("GET /operators/{id}/adapter", handler.NewAdapterHandler(s))
	mux.Handle("GET /health", handler.HealthHandler())

	addr := fmt.Sprintf(":%d", cfg.Port)
	srv := &http.Server{
		Addr:    addr,
		Handler: mux,
	}

	// 05-REQ-6.1: Log version, port, zone count, operator count, and ready message.
	slog.Info("parking-fee-service starting",
		"version", version,
		"port", cfg.Port,
		"zones", len(cfg.Zones),
		"operators", len(cfg.Operators),
	)

	// 05-REQ-6.2: Handle SIGTERM and SIGINT for graceful shutdown.
	quit := make(chan os.Signal, 1)
	signal.Notify(quit, syscall.SIGTERM, syscall.SIGINT)

	go func() {
		slog.Info("service ready", "addr", addr)
		if err := srv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			slog.Error("server error", "error", err)
			os.Exit(1)
		}
	}()

	<-quit
	slog.Info("shutting down gracefully")

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	if err := srv.Shutdown(ctx); err != nil {
		slog.Error("graceful shutdown failed", "error", err)
		os.Exit(1)
	}

	slog.Info("shutdown complete")
}
