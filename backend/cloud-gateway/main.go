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
	// Load configuration.
	configPath := os.Getenv("CONFIG_PATH")
	if configPath == "" {
		configPath = "config.json"
	}

	cfg, err := config.LoadConfig(configPath)
	if err != nil {
		slog.Error("failed to load configuration", "path", configPath, "error", err)
		os.Exit(1)
	}

	// Connect to NATS with retry.
	nc, err := natsclient.Connect(cfg.NatsURL, 5)
	if err != nil {
		slog.Error("failed to connect to NATS", "url", cfg.NatsURL, "error", err)
		os.Exit(1)
	}

	// Initialize store and NATS subscriptions.
	s := store.NewStore()

	if err := nc.SubscribeResponses(s); err != nil {
		slog.Error("failed to subscribe to command responses", "error", err)
		os.Exit(1)
	}
	if err := nc.SubscribeTelemetry(); err != nil {
		slog.Error("failed to subscribe to telemetry", "error", err)
		os.Exit(1)
	}

	// Set up HTTP routes.
	timeout := time.Duration(cfg.CommandTimeoutSeconds) * time.Second
	authMw := auth.Middleware(cfg)
	mux := http.NewServeMux()

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

	// Startup logging.
	slog.Info("cloud-gateway starting",
		"version", version,
		"port", cfg.Port,
		"nats_url", cfg.NatsURL,
		"tokens", len(cfg.Tokens),
	)
	slog.Info("cloud-gateway ready")

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

	// Drain NATS connection.
	if err := nc.Drain(); err != nil {
		slog.Error("NATS drain error", "error", err)
	}

	// Gracefully shut down HTTP server.
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	if err := srv.Shutdown(ctx); err != nil {
		slog.Error("HTTP shutdown error", "error", err)
		os.Exit(1)
	}

	slog.Info("cloud-gateway stopped")
}
