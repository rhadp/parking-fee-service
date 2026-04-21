// Package main is the entry point for the cloud-gateway service.
// It wires configuration, NATS connectivity, HTTP routing, and graceful shutdown.
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

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/auth"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/config"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/handler"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/natsclient"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/store"
)

const version = "0.1.0"

func main() {
	// Load configuration (06-REQ-6.1).
	cfgPath := os.Getenv("CONFIG_PATH")
	if cfgPath == "" {
		cfgPath = "config.json"
	}
	cfg, err := config.LoadConfig(cfgPath)
	if err != nil {
		slog.Error("failed to load configuration", "path", cfgPath, "error", err)
		os.Exit(1)
	}

	// Log startup information (06-REQ-8.1).
	slog.Info("cloud-gateway starting",
		"version", version,
		"port", cfg.Port,
		"nats_url", cfg.NatsURL,
		"token_count", len(cfg.Tokens),
	)

	// Connect to NATS with exponential backoff (06-REQ-5.E1).
	nc, err := natsclient.Connect(cfg.NatsURL, 5)
	if err != nil {
		slog.Error("failed to connect to NATS", "url", cfg.NatsURL, "error", err)
		os.Exit(1)
	}

	// Initialize response store.
	s := store.NewStore()

	// Subscribe to command responses and telemetry (06-REQ-5.1).
	if err := nc.SubscribeResponses(s); err != nil {
		slog.Error("failed to subscribe to command responses", "error", err)
		os.Exit(1)
	}
	if err := nc.SubscribeTelemetry(); err != nil {
		slog.Error("failed to subscribe to telemetry", "error", err)
		os.Exit(1)
	}

	// Compute command timeout duration (06-REQ-6.3).
	timeout := time.Duration(cfg.CommandTimeoutSeconds) * time.Second
	if timeout == 0 {
		timeout = 30 * time.Second
	}

	// Build HTTP routes using Go 1.22 ServeMux patterns.
	mux := http.NewServeMux()
	authMW := auth.Middleware(cfg)

	// POST /vehicles/{vin}/commands -> auth middleware + submit handler (06-REQ-1.1).
	mux.Handle("POST /vehicles/{vin}/commands",
		authMW(handler.NewSubmitCommandHandler(nc, s, timeout)))

	// GET /vehicles/{vin}/commands/{command_id} -> auth middleware + status handler (06-REQ-2.1).
	mux.Handle("GET /vehicles/{vin}/commands/{command_id}",
		authMW(handler.NewGetCommandStatusHandler(s)))

	// GET /health -> health handler (06-REQ-4.1).
	mux.Handle("GET /health", handler.HealthHandler())

	// Create HTTP server.
	srv := &http.Server{
		Addr:    fmt.Sprintf(":%d", cfg.Port),
		Handler: mux,
	}

	// Start HTTP server in background.
	serverErr := make(chan error, 1)
	go func() {
		slog.Info("cloud-gateway ready", "addr", srv.Addr)
		if err := srv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			serverErr <- err
		}
		close(serverErr)
	}()

	// Wait for shutdown signal (06-REQ-8.2).
	sigCh := make(chan os.Signal, 1)
	signal.Notify(sigCh, syscall.SIGTERM, syscall.SIGINT)

	select {
	case sig := <-sigCh:
		slog.Info("shutdown signal received", "signal", sig)
	case err := <-serverErr:
		if err != nil {
			slog.Error("HTTP server error", "error", err)
			os.Exit(1)
		}
	}

	// Graceful shutdown: drain NATS, then stop HTTP server.
	slog.Info("draining NATS connection")
	if err := nc.Drain(); err != nil {
		slog.Warn("NATS drain error", "error", err)
	}

	slog.Info("shutting down HTTP server")
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()
	if err := srv.Shutdown(ctx); err != nil {
		slog.Warn("HTTP server shutdown error", "error", err)
	}

	slog.Info("cloud-gateway stopped")
}
