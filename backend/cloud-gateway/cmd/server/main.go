// Package main provides the entry point for the cloud-gateway service.
//
// The cloud-gateway is a Go backend service deployed on OpenShift that acts as
// a bridge for vehicle-to-cloud communication. It provides two distinct interfaces:
//
//  1. REST API Interface (Northbound): Serves the COMPANION_APP for remote vehicle
//     control and command status queries via HTTPS/REST
//  2. MQTT Interface (Southbound): Communicates with vehicles via the CLOUD_GATEWAY_CLIENT
//     through an Eclipse Mosquitto MQTT broker
//
// The service translates REST API requests from the COMPANION_APP into MQTT messages
// for vehicles, and routes MQTT command responses from vehicles back to REST API
// consumers. Vehicle telemetry received via MQTT is exported to an OpenTelemetry
// collector for observability.
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

	"github.com/gorilla/mux"

	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/audit"
	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/config"
	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/handler"
	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/middleware"
	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/mqtt"
	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/otel"
	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/service"
	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/store"
)

const (
	serviceName    = "cloud-gateway"
	serviceVersion = "0.1.0"
)

func main() {
	// Initialize structured logger
	logger := slog.New(slog.NewJSONHandler(os.Stdout, &slog.HandlerOptions{
		Level: slog.LevelInfo,
	}))
	slog.SetDefault(logger)

	logger.Info("starting cloud-gateway service",
		slog.String("service", serviceName),
		slog.String("version", serviceVersion),
	)

	// Load configuration
	cfg, err := config.LoadConfig()
	if err != nil {
		logger.Error("failed to load configuration", slog.String("error", err.Error()))
		os.Exit(1)
	}

	// Log configuration values (except secrets)
	cfg.LogConfigValues(logger)

	// Initialize audit logger
	auditLogger := audit.NewAuditLogger(logger)

	// Initialize command store
	cmdStore := store.NewCommandStore(100)

	// Initialize OpenTelemetry exporter
	ctx := context.Background()
	otelExporter, err := otel.NewExporter(ctx, cfg.GetOTelConfig(), cfg.ConfiguredVIN, logger)
	if err != nil {
		logger.Warn("failed to initialize OpenTelemetry exporter, continuing without telemetry export",
			slog.String("error", err.Error()),
		)
	}

	// Initialize MQTT client
	mqttClient := mqtt.NewClient(cfg.GetMQTTConfig(), logger, auditLogger)

	// Initialize services
	commandService := service.NewCommandService(
		cmdStore,
		mqttClient,
		auditLogger,
		logger,
		cfg.CommandTimeout,
		cfg.ConfiguredVIN,
	)

	telemetryService := service.NewTelemetryService(
		otelExporter,
		auditLogger,
		logger,
		cfg.ConfiguredVIN,
	)

	parkingSessionService := service.NewParkingSessionService(
		cfg.ParkingFeeServiceURL,
		logger,
		cfg.ConfiguredVIN,
	)

	// Initialize MQTT handlers
	mqttHandlers := mqtt.NewMessageHandlers(logger, commandService, telemetryService)

	// Initialize HTTP handlers
	commandHandler := handler.NewCommandHandler(commandService, auditLogger, logger, cfg.ConfiguredVIN)
	healthHandler := handler.NewHealthHandler(mqttClient, serviceName)
	parkingSessionHandler := handler.NewParkingSessionHandler(parkingSessionService, logger, cfg.ConfiguredVIN)

	// Set up router
	router := mux.NewRouter()

	// Apply middleware
	router.Use(middleware.RequestIDMiddleware)
	router.Use(middleware.LoggingMiddleware(logger))

	// Register health endpoints
	router.HandleFunc("/health", healthHandler.HandleHealth).Methods("GET")
	router.HandleFunc("/ready", healthHandler.HandleReady).Methods("GET")

	// Register API endpoints
	api := router.PathPrefix("/api/v1").Subrouter()
	api.HandleFunc("/vehicles/{vin}/commands", commandHandler.HandleSubmitCommand).Methods("POST")
	api.HandleFunc("/vehicles/{vin}/commands/{command_id}", commandHandler.HandleGetCommandStatus).Methods("GET")
	api.HandleFunc("/vehicles/{vin}/parking-session", parkingSessionHandler.HandleGetParkingSession).Methods("GET")

	// Connect to MQTT broker
	if err := mqttClient.Connect(); err != nil {
		logger.Error("failed to connect to MQTT broker", slog.String("error", err.Error()))
		os.Exit(1)
	}

	// Subscribe to MQTT topics
	responsesTopic := fmt.Sprintf("vehicles/%s/command_responses", cfg.ConfiguredVIN)
	if err := mqttClient.Subscribe(responsesTopic, mqttHandlers.HandleCommandResponse); err != nil {
		logger.Error("failed to subscribe to command responses topic", slog.String("error", err.Error()))
		os.Exit(1)
	}

	telemetryTopic := fmt.Sprintf("vehicles/%s/telemetry", cfg.ConfiguredVIN)
	if err := mqttClient.Subscribe(telemetryTopic, mqttHandlers.HandleTelemetry); err != nil {
		logger.Error("failed to subscribe to telemetry topic", slog.String("error", err.Error()))
		os.Exit(1)
	}

	logger.Info("subscribed to MQTT topics",
		slog.String("responses_topic", responsesTopic),
		slog.String("telemetry_topic", telemetryTopic),
	)

	// Start command timeout checker
	commandService.StartTimeoutChecker(5 * time.Second)

	// Create HTTP server
	server := &http.Server{
		Addr:         fmt.Sprintf(":%d", cfg.Port),
		Handler:      router,
		ReadTimeout:  15 * time.Second,
		WriteTimeout: 15 * time.Second,
		IdleTimeout:  60 * time.Second,
	}

	// Start HTTP server in goroutine
	go func() {
		logger.Info("starting HTTP server", slog.Int("port", cfg.Port))
		if err := server.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			logger.Error("HTTP server error", slog.String("error", err.Error()))
			os.Exit(1)
		}
	}()

	// Wait for termination signal
	sigCtx, stop := signal.NotifyContext(context.Background(), syscall.SIGTERM, syscall.SIGINT)
	defer stop()

	<-sigCtx.Done()

	logger.Info("shutting down cloud-gateway service")

	// Create shutdown context with timeout
	shutdownCtx, cancel := context.WithTimeout(context.Background(), 15*time.Second)
	defer cancel()

	// Stop command timeout checker
	commandService.StopTimeoutChecker()

	// Shutdown HTTP server gracefully (10s timeout for in-flight requests)
	serverShutdownCtx, serverCancel := context.WithTimeout(shutdownCtx, 10*time.Second)
	defer serverCancel()

	if err := server.Shutdown(serverShutdownCtx); err != nil {
		logger.Error("HTTP server shutdown error", slog.String("error", err.Error()))
	}

	// Disconnect MQTT client
	mqttClient.Disconnect()

	// Shutdown OpenTelemetry exporter
	if otelExporter != nil {
		if err := otelExporter.Shutdown(shutdownCtx); err != nil {
			logger.Error("OpenTelemetry exporter shutdown error", slog.String("error", err.Error()))
		}
	}

	logger.Info("cloud-gateway service stopped")
}
