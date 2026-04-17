// Package main is the entry point for the CLOUD_GATEWAY service.
//
// The CLOUD_GATEWAY bridges REST (COMPANION_APP) and NATS (CLOUD_GATEWAY_CLIENT)
// protocols for vehicle command routing. It authenticates bearer tokens mapped to
// VINs via a JSON config file and stores command responses in memory.
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

	"github.com/sdv-demo/parking-fee-service/backend/cloud-gateway/auth"
	"github.com/sdv-demo/parking-fee-service/backend/cloud-gateway/config"
	"github.com/sdv-demo/parking-fee-service/backend/cloud-gateway/handler"
	"github.com/sdv-demo/parking-fee-service/backend/cloud-gateway/natsclient"
	"github.com/sdv-demo/parking-fee-service/backend/cloud-gateway/store"
)

const version = "0.1.0"

func main() {
	// Read config path from environment, defaulting to "config.json".
	configPath := os.Getenv("CONFIG_PATH")
	if configPath == "" {
		configPath = "config.json"
	}

	// Load configuration.
	cfg, err := config.LoadConfig(configPath)
	if err != nil {
		slog.Error("failed to load config", "path", configPath, "error", err)
		os.Exit(1)
	}

	// Log startup info immediately after config load (06-REQ-8.1).
	slog.Info("cloud-gateway starting",
		"version", version,
		"port", cfg.Port,
		"nats_url", cfg.NatsURL,
		"token_count", len(cfg.Tokens))

	// Connect to NATS with exponential backoff, up to 5 attempts.
	nc, err := natsclient.Connect(cfg.NatsURL, 5)
	if err != nil {
		slog.Error("failed to connect to NATS", "error", err)
		os.Exit(1)
	}

	// Initialize the in-memory response store.
	s := store.NewStore()

	// Subscribe to command responses from vehicles.
	if err := nc.SubscribeResponses(s); err != nil {
		slog.Error("failed to subscribe to command responses", "error", err)
		os.Exit(1)
	}

	// Subscribe to telemetry from vehicles (logged only, not stored).
	if err := nc.SubscribeTelemetry(); err != nil {
		slog.Error("failed to subscribe to telemetry", "error", err)
		os.Exit(1)
	}

	// Configure the command timeout duration.
	timeout := time.Duration(cfg.CommandTimeoutSeconds) * time.Second

	// Register HTTP routes using Go 1.22 ServeMux pattern matching.
	mux := http.NewServeMux()
	mux.Handle("POST /vehicles/{vin}/commands",
		auth.Middleware(cfg)(handler.NewSubmitCommandHandler(nc, s, timeout)))
	mux.Handle("GET /vehicles/{vin}/commands/{command_id}",
		auth.Middleware(cfg)(handler.NewGetCommandStatusHandler(s)))
	mux.Handle("GET /health", handler.HealthHandler())

	// Create the HTTP server.
	addr := fmt.Sprintf(":%d", cfg.Port)
	srv := &http.Server{
		Addr:    addr,
		Handler: mux,
	}

	// Start the HTTP server in a background goroutine.
	serverErr := make(chan error, 1)
	go func() {
		slog.Info("cloud-gateway ready", "addr", addr)
		if err := srv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			serverErr <- err
		}
	}()

	// Wait for a shutdown signal (SIGTERM or SIGINT) or a server error.
	quit := make(chan os.Signal, 1)
	signal.Notify(quit, syscall.SIGTERM, syscall.SIGINT)

	select {
	case sig := <-quit:
		slog.Info("shutdown signal received", "signal", sig)
	case err := <-serverErr:
		slog.Error("HTTP server error", "error", err)
		os.Exit(1)
	}

	// Graceful shutdown: drain NATS first, then shut down the HTTP server.
	slog.Info("draining NATS connection")
	if err := nc.Drain(); err != nil {
		slog.Warn("failed to drain NATS connection", "error", err)
	}

	slog.Info("shutting down HTTP server")
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()
	if err := srv.Shutdown(ctx); err != nil {
		slog.Warn("HTTP server shutdown error", "error", err)
	}

	slog.Info("cloud-gateway stopped")
}
