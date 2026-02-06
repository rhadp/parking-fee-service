// Package main provides the entry point for the parking-fee-service.
//
// The parking-fee-service is a Go backend service that handles parking
// operations including fee calculation, session management, and payment
// processing for the SDV Parking Demo System.
//
// Communication:
// - HTTPS/REST for PARKING_OPERATOR_ADAPTOR and UPDATE_SERVICE
package main

import (
	"database/sql"
	"fmt"
	"log/slog"
	"net/http"
	"os"
	"time"

	"github.com/gorilla/mux"
	_ "modernc.org/sqlite"

	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/config"
	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/handler"
	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/middleware"
	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/model"
	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/service"
	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/store"
)

func main() {
	// Load configuration
	cfg := config.LoadConfig()

	// Initialize structured logger
	logger := slog.New(slog.NewJSONHandler(os.Stdout, &slog.HandlerOptions{
		Level: cfg.SlogLevel(),
	}))
	slog.SetDefault(logger)

	logger.Info("parking-fee-service: starting",
		"port", cfg.Port,
		"database_path", cfg.DatabasePath,
		"log_level", cfg.LogLevel,
	)

	// Initialize SQLite database
	db, err := sql.Open("sqlite", cfg.DatabasePath)
	if err != nil {
		logger.Error("failed to open SQLite database", "error", err, "path", cfg.DatabasePath)
		os.Exit(1)
	}
	defer db.Close()

	if err := db.Ping(); err != nil {
		logger.Error("failed to ping SQLite database", "error", err)
		os.Exit(1)
	}
	logger.Info("SQLite database connected", "path", cfg.DatabasePath)

	// Initialize stores
	sessionStore := store.NewSessionStore(db)
	if err := sessionStore.InitSchema(); err != nil {
		logger.Error("failed to initialize session schema", "error", err)
		os.Exit(1)
	}
	logger.Info("session store initialized")

	// Create demo zone from config
	demoZone := model.Zone{
		ZoneID:       cfg.DemoZoneID,
		OperatorName: cfg.DemoOperatorName,
		HourlyRate:   cfg.DemoHourlyRate,
		Currency:     cfg.DemoCurrency,
		AdapterImageRef: cfg.DemoAdapterImageRef,
		AdapterChecksum: cfg.DemoAdapterChecksum,
		Bounds: model.Bounds{
			MinLat: cfg.DemoZoneMinLat,
			MaxLat: cfg.DemoZoneMaxLat,
			MinLng: cfg.DemoZoneMinLng,
			MaxLng: cfg.DemoZoneMaxLng,
		},
	}
	zoneStore := store.NewZoneStore([]model.Zone{demoZone})
	logger.Info("zone store initialized", "zones", 1)

	// Create demo adapter from config
	demoAdapter := model.Adapter{
		AdapterID:    cfg.DemoAdapterID,
		OperatorName: cfg.DemoOperatorName,
		Version:      cfg.DemoAdapterVersion,
		ImageRef:     cfg.DemoAdapterImageRef,
		Checksum:     cfg.DemoAdapterChecksum,
		CreatedAt:    time.Now(),
	}
	adapterStore := store.NewAdapterStore([]model.Adapter{demoAdapter})
	logger.Info("adapter store initialized", "adapters", 1)

	// Initialize services
	zoneService := service.NewZoneService(zoneStore)
	adapterService := service.NewAdapterService(adapterStore)
	parkingService := service.NewParkingService(sessionStore, zoneStore, cfg.DemoHourlyRate)
	logger.Info("services initialized")

	// Initialize handlers
	zoneHandler := handler.NewZoneHandler(zoneService, logger)
	adapterHandler := handler.NewAdapterHandler(adapterService, logger)
	parkingHandler := handler.NewParkingHandler(parkingService, logger)
	healthHandler := handler.NewHealthHandler(sessionStore, logger)

	// Set up router
	router := mux.NewRouter()

	// Apply middleware
	router.Use(middleware.RequestIDMiddleware)
	router.Use(middleware.LoggingMiddleware(logger))

	// Register API routes
	api := router.PathPrefix("/api/v1").Subrouter()

	// Zone routes
	api.HandleFunc("/zones", zoneHandler.HandleGetZone).Methods("GET")

	// Adapter routes
	api.HandleFunc("/adapters", adapterHandler.HandleListAdapters).Methods("GET")
	api.HandleFunc("/adapters/{adapter_id}", adapterHandler.HandleGetAdapter).Methods("GET")

	// Parking routes
	api.HandleFunc("/parking/start", parkingHandler.HandleStartSession).Methods("POST")
	api.HandleFunc("/parking/stop", parkingHandler.HandleStopSession).Methods("POST")
	api.HandleFunc("/parking/status/{session_id}", parkingHandler.HandleGetStatus).Methods("GET")

	// Health routes
	router.HandleFunc("/health", healthHandler.HandleHealth).Methods("GET")
	router.HandleFunc("/ready", healthHandler.HandleReady).Methods("GET")

	logger.Info("routes registered",
		"api_routes", []string{
			"GET /api/v1/zones",
			"GET /api/v1/adapters",
			"GET /api/v1/adapters/{adapter_id}",
			"POST /api/v1/parking/start",
			"POST /api/v1/parking/stop",
			"GET /api/v1/parking/status/{session_id}",
		},
		"health_routes", []string{"GET /health", "GET /ready"},
	)

	// Start HTTP server
	addr := fmt.Sprintf(":%d", cfg.Port)
	logger.Info("starting HTTP server", "address", addr)

	if err := http.ListenAndServe(addr, router); err != nil {
		logger.Error("server failed", "error", err)
		os.Exit(1)
	}
}
