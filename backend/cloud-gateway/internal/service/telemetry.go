package service

import (
	"context"
	"log/slog"
	"time"

	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/audit"
	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/model"
	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/otel"
)

// TelemetryService handles vehicle telemetry.
// Note: Telemetry is exported to OpenTelemetry collector, NOT stored for REST API.
type TelemetryService struct {
	exporter      *otel.Exporter
	auditLogger   audit.AuditLogger
	logger        *slog.Logger
	configuredVIN string
	lastTelemetry *model.Telemetry
}

// NewTelemetryService creates a new TelemetryService.
func NewTelemetryService(
	exporter *otel.Exporter,
	auditLogger audit.AuditLogger,
	logger *slog.Logger,
	configuredVIN string,
) *TelemetryService {
	return &TelemetryService{
		exporter:      exporter,
		auditLogger:   auditLogger,
		logger:        logger,
		configuredVIN: configuredVIN,
	}
}

// HandleTelemetryMessage processes a telemetry message from the vehicle.
// It exports the telemetry to OpenTelemetry and logs an audit event.
func (s *TelemetryService) HandleTelemetryMessage(ctx context.Context, msg *model.MQTTTelemetryMessage) {
	// Parse timestamp
	timestamp, err := time.Parse(time.RFC3339, msg.Timestamp)
	if err != nil {
		s.logger.Warn("failed to parse telemetry timestamp",
			slog.String("timestamp", msg.Timestamp),
			slog.String("error", err.Error()),
		)
		timestamp = time.Now()
	}

	// Check for door state changes
	doorStateChanged := false
	if s.lastTelemetry != nil {
		doorStateChanged = s.lastTelemetry.DoorLocked != msg.DoorLocked ||
			s.lastTelemetry.DoorOpen != msg.DoorOpen
	}

	// Update last telemetry
	s.lastTelemetry = &model.Telemetry{
		Timestamp:            timestamp,
		Latitude:             msg.Latitude,
		Longitude:            msg.Longitude,
		DoorLocked:           msg.DoorLocked,
		DoorOpen:             msg.DoorOpen,
		ParkingSessionActive: msg.ParkingSessionActive,
		ReceivedAt:           time.Now(),
	}

	// Export to OpenTelemetry
	if s.exporter != nil && s.exporter.IsEnabled() {
		s.exporter.ExportTelemetry(ctx, msg)
	}

	// Log audit event
	if s.auditLogger != nil {
		event := &model.TelemetryUpdateEvent{
			AuditEventBase:   model.NewAuditEventBase(""),
			VIN:              s.configuredVIN,
			LocationPresent:  msg.Latitude != 0 || msg.Longitude != 0,
			DoorStateChanged: doorStateChanged,
		}
		s.auditLogger.LogTelemetryUpdate(ctx, event)
	}

	s.logger.Debug("telemetry processed",
		slog.String("vin", s.configuredVIN),
		slog.Float64("latitude", msg.Latitude),
		slog.Float64("longitude", msg.Longitude),
		slog.Bool("door_locked", msg.DoorLocked),
		slog.Bool("parking_session_active", msg.ParkingSessionActive),
	)
}

// GetLastTelemetry returns the most recent telemetry data.
// Note: This is for internal use only, not exposed via REST API.
func (s *TelemetryService) GetLastTelemetry() *model.Telemetry {
	return s.lastTelemetry
}

// HandleTelemetry implements the mqtt.TelemetryHandler interface.
func (s *TelemetryService) HandleTelemetry(msg *model.MQTTTelemetryMessage) {
	s.HandleTelemetryMessage(context.Background(), msg)
}
