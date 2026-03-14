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

	natsgo "github.com/nats-io/nats.go"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/auth"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/config"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/handler"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/natsclient"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/store"
)

const version = "0.1.0"

// natsPublisher adapts the natsclient package functions to the handler.NATSPublisher interface.
type natsPublisher struct {
	nc *natsgo.Conn
}

func (p *natsPublisher) PublishCommand(vin string, cmd model.Command, bearerToken string) error {
	return natsclient.PublishCommand(p.nc, vin, cmd, bearerToken)
}

func main() {
	// Load configuration from CONFIG_PATH or default "config.json".
	cfgPath := os.Getenv("CONFIG_PATH")
	if cfgPath == "" {
		cfgPath = "config.json"
	}

	cfg, err := config.LoadConfig(cfgPath)
	if err != nil {
		slog.Error("failed to load configuration", "path", cfgPath, "error", err)
		os.Exit(1)
	}

	// Log startup information.
	slog.Info("cloud-gateway starting",
		"version", version,
		"port", cfg.Port,
		"nats_url", cfg.NatsURL,
		"token_count", len(cfg.Tokens),
	)

	// Connect to NATS with exponential backoff.
	nc, err := natsclient.Connect(cfg.NatsURL, 5)
	if err != nil {
		slog.Error("failed to connect to NATS", "error", err)
		os.Exit(1)
	}
	// Create in-memory command store and authenticator.
	s := store.NewStore()
	a := auth.NewAuthenticator(cfg.Tokens)

	// Subscribe to command responses and telemetry.
	if _, err := natsclient.SubscribeResponses(nc, s); err != nil {
		slog.Error("failed to subscribe to command responses", "error", err)
		os.Exit(1)
	}
	if _, err := natsclient.SubscribeTelemetry(nc); err != nil {
		slog.Error("failed to subscribe to telemetry", "error", err)
		os.Exit(1)
	}

	// Start periodic timeout expiry goroutine.
	timeout := time.Duration(cfg.CommandTimeout) * time.Second
	stopExpiry := make(chan struct{})
	go func() {
		ticker := time.NewTicker(5 * time.Second)
		defer ticker.Stop()
		for {
			select {
			case <-ticker.C:
				s.ExpireTimedOut(timeout)
			case <-stopExpiry:
				return
			}
		}
	}()

	// Build HTTP routes using Go 1.22 ServeMux pattern matching.
	pub := &natsPublisher{nc: nc}
	mux := http.NewServeMux()
	mux.Handle("POST /vehicles/{vin}/commands", handler.NewCommandHandler(s, pub, a))
	mux.Handle("GET /vehicles/{vin}/commands/{command_id}", handler.NewStatusHandler(s, a))
	mux.Handle("GET /health", handler.HealthHandler())

	srv := &http.Server{
		Addr:    fmt.Sprintf(":%d", cfg.Port),
		Handler: mux,
	}

	// Start HTTP server in background.
	serverErr := make(chan error, 1)
	go func() {
		slog.Info("HTTP server listening", "addr", srv.Addr)
		if err := srv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			serverErr <- err
		}
	}()

	// Wait for shutdown signal or server error.
	quit := make(chan os.Signal, 1)
	signal.Notify(quit, syscall.SIGTERM, syscall.SIGINT)

	select {
	case sig := <-quit:
		slog.Info("received shutdown signal", "signal", sig)
	case err := <-serverErr:
		slog.Error("HTTP server error", "error", err)
		os.Exit(1)
	}

	// Graceful shutdown.
	close(stopExpiry)

	if err := nc.Drain(); err != nil {
		slog.Warn("NATS drain error", "error", err)
	}

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()
	if err := srv.Shutdown(ctx); err != nil {
		slog.Error("HTTP server shutdown error", "error", err)
		os.Exit(1)
	}

	slog.Info("cloud-gateway stopped")
}
