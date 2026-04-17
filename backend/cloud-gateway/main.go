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

	"parking-fee-service/backend/cloud-gateway/auth"
	"parking-fee-service/backend/cloud-gateway/config"
	"parking-fee-service/backend/cloud-gateway/handler"
	"parking-fee-service/backend/cloud-gateway/natsclient"
	"parking-fee-service/backend/cloud-gateway/store"
)

const version = "0.1.0"

func main() {
	// Load configuration from CONFIG_PATH env var, defaulting to "config.json".
	configPath := os.Getenv("CONFIG_PATH")
	if configPath == "" {
		configPath = "config.json"
	}

	cfg, err := config.LoadConfig(configPath)
	if err != nil {
		slog.Error("failed to load configuration", "path", configPath, "error", err)
		os.Exit(1)
	}

	// Log startup information immediately after config load (before NATS connect).
	// Requirements: 06-REQ-8.1
	slog.Info("cloud-gateway starting",
		"version", version,
		"port", cfg.Port,
		"nats_url", cfg.NatsURL,
		"token_count", len(cfg.Tokens),
	)

	// Connect to NATS server with exponential backoff retry (up to 5 attempts).
	// Requirements: 06-REQ-5.E1
	nc, err := natsclient.Connect(cfg.NatsURL, 5)
	if err != nil {
		slog.Error("failed to connect to NATS", "nats_url", cfg.NatsURL, "error", err)
		os.Exit(1)
	}

	// Create in-memory command response store.
	s := store.NewStore()

	// Subscribe to NATS subjects for command responses and telemetry.
	// Requirements: 06-REQ-5.1
	if err := nc.SubscribeResponses(s); err != nil {
		slog.Error("failed to subscribe to command responses", "error", err)
		os.Exit(1)
	}
	if err := nc.SubscribeTelemetry(); err != nil {
		slog.Error("failed to subscribe to telemetry", "error", err)
		os.Exit(1)
	}

	// Configure command timeout from config (default 30s).
	// Requirements: 06-REQ-6.3
	timeout := time.Duration(cfg.CommandTimeoutSeconds) * time.Second
	if timeout <= 0 {
		timeout = 30 * time.Second
	}

	// Register HTTP routes using Go 1.22 ServeMux pattern matching.
	mux := http.NewServeMux()
	authMiddleware := auth.Middleware(cfg)

	// Health endpoint — no auth required.
	// Requirements: 06-REQ-4.1
	mux.HandleFunc("GET /health", handler.HealthHandler())

	// Vehicle command endpoints — auth required.
	// Requirements: 06-REQ-1.1, 06-REQ-2.1
	mux.Handle("POST /vehicles/{vin}/commands",
		authMiddleware(handler.NewSubmitCommandHandler(nc, s, timeout)))
	mux.Handle("GET /vehicles/{vin}/commands/{command_id}",
		authMiddleware(handler.NewGetCommandStatusHandler(s)))

	srv := &http.Server{
		Addr:    fmt.Sprintf(":%d", cfg.Port),
		Handler: mux,
	}

	// Handle SIGTERM and SIGINT for graceful shutdown.
	// Requirements: 06-REQ-8.2
	quit := make(chan os.Signal, 1)
	signal.Notify(quit, syscall.SIGTERM, syscall.SIGINT)

	go func() {
		sig := <-quit
		slog.Info("received shutdown signal", "signal", sig)

		// Drain NATS connection to allow in-flight messages to complete.
		slog.Info("draining NATS connection...")
		if err := nc.Drain(); err != nil {
			slog.Error("error draining NATS connection", "error", err)
		}

		// Gracefully shut down the HTTP server.
		slog.Info("shutting down HTTP server...")
		ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
		defer cancel()
		if err := srv.Shutdown(ctx); err != nil {
			slog.Error("HTTP server shutdown error", "error", err)
		}
	}()

	slog.Info("cloud-gateway ready", "addr", srv.Addr)
	if err := srv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
		slog.Error("HTTP server error", "error", err)
		os.Exit(1)
	}

	slog.Info("cloud-gateway stopped")
}
