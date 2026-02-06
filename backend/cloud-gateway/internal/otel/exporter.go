// Package otel provides OpenTelemetry integration for the cloud-gateway service.
package otel

import (
	"context"
	"log/slog"
	"sync"
	"time"

	"go.opentelemetry.io/otel"
	"go.opentelemetry.io/otel/attribute"
	"go.opentelemetry.io/otel/exporters/otlp/otlpmetric/otlpmetricgrpc"
	"go.opentelemetry.io/otel/metric"
	sdkmetric "go.opentelemetry.io/otel/sdk/metric"

	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/config"
	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/model"
)

// Exporter exports telemetry data to an OpenTelemetry collector.
type Exporter struct {
	logger         *slog.Logger
	meterProvider  *sdkmetric.MeterProvider
	meter          metric.Meter
	enabled        bool
	vin            string
	mu             sync.Mutex
	lastExportTime time.Time

	// Gauges for vehicle telemetry
	latitudeGauge             metric.Float64Gauge
	longitudeGauge            metric.Float64Gauge
	doorLockedGauge           metric.Int64Gauge
	doorOpenGauge             metric.Int64Gauge
	parkingSessionActiveGauge metric.Int64Gauge
}

// NewExporter creates a new OpenTelemetry exporter.
// If the OTLP endpoint is empty, the exporter will be disabled.
func NewExporter(ctx context.Context, cfg *config.OTelConfig, vin string, logger *slog.Logger) (*Exporter, error) {
	e := &Exporter{
		logger:  logger,
		enabled: cfg.Enabled,
		vin:     vin,
	}

	if !cfg.Enabled {
		logger.Info("OpenTelemetry export disabled (no OTLP_ENDPOINT configured)")
		return e, nil
	}

	// Create OTLP exporter
	exporter, err := otlpmetricgrpc.New(ctx,
		otlpmetricgrpc.WithEndpoint(cfg.Endpoint),
		otlpmetricgrpc.WithInsecure(), // Use insecure for local development
	)
	if err != nil {
		logger.Warn("failed to create OTLP exporter, telemetry export disabled",
			slog.String("error", err.Error()),
		)
		e.enabled = false
		return e, nil
	}

	// Create meter provider
	e.meterProvider = sdkmetric.NewMeterProvider(
		sdkmetric.WithReader(sdkmetric.NewPeriodicReader(exporter,
			sdkmetric.WithInterval(10*time.Second),
		)),
	)

	// Set as global provider
	otel.SetMeterProvider(e.meterProvider)

	// Create meter
	e.meter = e.meterProvider.Meter("cloud-gateway")

	// Create gauges for vehicle telemetry
	if err := e.createGauges(); err != nil {
		logger.Warn("failed to create telemetry gauges",
			slog.String("error", err.Error()),
		)
		e.enabled = false
		return e, nil
	}

	logger.Info("OpenTelemetry exporter initialized",
		slog.String("endpoint", cfg.Endpoint),
	)

	return e, nil
}

// createGauges creates the OpenTelemetry gauges for vehicle telemetry.
func (e *Exporter) createGauges() error {
	var err error

	e.latitudeGauge, err = e.meter.Float64Gauge("vehicle.location.latitude",
		metric.WithDescription("Vehicle latitude"),
		metric.WithUnit("degrees"),
	)
	if err != nil {
		return err
	}

	e.longitudeGauge, err = e.meter.Float64Gauge("vehicle.location.longitude",
		metric.WithDescription("Vehicle longitude"),
		metric.WithUnit("degrees"),
	)
	if err != nil {
		return err
	}

	e.doorLockedGauge, err = e.meter.Int64Gauge("vehicle.door.locked",
		metric.WithDescription("Door locked state (1=locked, 0=unlocked)"),
	)
	if err != nil {
		return err
	}

	e.doorOpenGauge, err = e.meter.Int64Gauge("vehicle.door.open",
		metric.WithDescription("Door open state (1=open, 0=closed)"),
	)
	if err != nil {
		return err
	}

	e.parkingSessionActiveGauge, err = e.meter.Int64Gauge("vehicle.parking.session_active",
		metric.WithDescription("Parking session active state (1=active, 0=inactive)"),
	)
	if err != nil {
		return err
	}

	return nil
}

// ExportTelemetry exports vehicle telemetry to the OpenTelemetry collector.
func (e *Exporter) ExportTelemetry(ctx context.Context, telemetry *model.MQTTTelemetryMessage) {
	if !e.enabled {
		return
	}

	e.mu.Lock()
	defer e.mu.Unlock()

	// Create attributes with VIN
	attrs := metric.WithAttributes(
		attribute.String("vin", e.vin),
	)

	// Record telemetry values
	e.latitudeGauge.Record(ctx, telemetry.Latitude, attrs)
	e.longitudeGauge.Record(ctx, telemetry.Longitude, attrs)
	e.doorLockedGauge.Record(ctx, boolToInt64(telemetry.DoorLocked), attrs)
	e.doorOpenGauge.Record(ctx, boolToInt64(telemetry.DoorOpen), attrs)
	e.parkingSessionActiveGauge.Record(ctx, boolToInt64(telemetry.ParkingSessionActive), attrs)

	e.lastExportTime = time.Now()

	e.logger.Debug("exported telemetry to OpenTelemetry",
		slog.String("vin", e.vin),
		slog.Float64("latitude", telemetry.Latitude),
		slog.Float64("longitude", telemetry.Longitude),
	)
}

// IsEnabled returns true if the exporter is enabled.
func (e *Exporter) IsEnabled() bool {
	return e.enabled
}

// Shutdown gracefully shuts down the exporter.
func (e *Exporter) Shutdown(ctx context.Context) error {
	if e.meterProvider == nil {
		return nil
	}

	e.logger.Info("shutting down OpenTelemetry exporter")
	return e.meterProvider.Shutdown(ctx)
}

// boolToInt64 converts a boolean to int64 (1 or 0).
func boolToInt64(b bool) int64 {
	if b {
		return 1
	}
	return 0
}
