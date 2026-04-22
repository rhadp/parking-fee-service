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
	os.Exit(run())
}

func run() int {
	// Determine config file path.
	configPath := os.Getenv("CONFIG_PATH")
	if configPath == "" {
		configPath = "config.json"
	}

	// Load configuration.
	cfg, err := config.LoadConfig(configPath)
	if err != nil {
		slog.Error("failed to load configuration", "path", configPath, "error", err)
		return 1
	}

	// Connect to NATS with retry.
	nc, err := natsclient.Connect(cfg.NatsURL, 5)
	if err != nil {
		slog.Error("failed to connect to NATS", "url", cfg.NatsURL, "error", err)
		return 1
	}

	// Create response store.
	s := store.NewStore()

	// Subscribe to NATS subjects.
	if err := nc.SubscribeResponses(s); err != nil {
		slog.Error("failed to subscribe to command responses", "error", err)
		return 1
	}
	if err := nc.SubscribeTelemetry(); err != nil {
		slog.Error("failed to subscribe to telemetry", "error", err)
		return 1
	}

	// Calculate command timeout.
	timeout := time.Duration(cfg.CommandTimeoutSeconds) * time.Second
	if timeout == 0 {
		timeout = 30 * time.Second
	}

	// Set up HTTP routes.
	mux := http.NewServeMux()
	authMw := auth.Middleware(cfg)

	mux.Handle("POST /vehicles/{vin}/commands",
		authMw(handler.NewSubmitCommandHandler(nc, s, timeout)))
	mux.Handle("GET /vehicles/{vin}/commands/{command_id}",
		authMw(handler.NewGetCommandStatusHandler(s)))
	mux.Handle("GET /health", handler.HealthHandler())

	addr := fmt.Sprintf(":%d", cfg.Port)
	srv := &http.Server{
		Addr:    addr,
		Handler: mux,
	}

	// Log startup information.
	slog.Info("cloud-gateway starting",
		"version", version,
		"port", cfg.Port,
		"nats_url", cfg.NatsURL,
		"tokens", len(cfg.Tokens),
	)
	slog.Info("cloud-gateway ready")

	// Handle shutdown signals.
	sigCh := make(chan os.Signal, 1)
	signal.Notify(sigCh, syscall.SIGTERM, syscall.SIGINT)

	// Start HTTP server in a goroutine.
	errCh := make(chan error, 1)
	go func() {
		if err := srv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			errCh <- err
		}
	}()

	// Wait for shutdown signal or server error.
	select {
	case sig := <-sigCh:
		slog.Info("received signal, shutting down", "signal", sig)
	case err := <-errCh:
		slog.Error("server error", "error", err)
		return 1
	}

	// Drain NATS connection.
	if err := nc.Drain(); err != nil {
		slog.Error("failed to drain NATS connection", "error", err)
	}

	// Gracefully shut down HTTP server.
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	if err := srv.Shutdown(ctx); err != nil {
		slog.Error("failed to shut down HTTP server", "error", err)
		return 1
	}

	slog.Info("cloud-gateway stopped")
	return 0
}
