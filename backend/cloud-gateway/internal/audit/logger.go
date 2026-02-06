// Package audit provides audit logging for security-relevant operations.
package audit

import (
	"context"
	"crypto/sha256"
	"encoding/hex"
	"log/slog"

	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/middleware"
	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/model"
)

// AuditLogger defines the interface for audit logging.
type AuditLogger interface {
	// LogCommandSubmission logs a command submission event.
	LogCommandSubmission(ctx context.Context, event *model.CommandSubmissionEvent)

	// LogCommandStatusChange logs a command status change event.
	LogCommandStatusChange(ctx context.Context, event *model.CommandStatusChangeEvent)

	// LogAuthAttempt logs an authentication attempt event.
	LogAuthAttempt(ctx context.Context, event *model.AuthAttemptEvent)

	// LogTelemetryUpdate logs a telemetry update event.
	LogTelemetryUpdate(ctx context.Context, event *model.TelemetryUpdateEvent)

	// LogMQTTConnectionEvent logs MQTT connection events.
	LogMQTTConnectionEvent(ctx context.Context, event *model.MQTTConnectionEvent)

	// LogValidationFailure logs a validation failure event.
	LogValidationFailure(ctx context.Context, event *model.ValidationFailureEvent)
}

// AuditLoggerImpl implements AuditLogger using structured JSON logging.
type AuditLoggerImpl struct {
	logger *slog.Logger
}

// NewAuditLogger creates a new AuditLogger.
func NewAuditLogger(logger *slog.Logger) *AuditLoggerImpl {
	return &AuditLoggerImpl{
		logger: logger,
	}
}

// LogCommandSubmission logs a command submission event.
func (a *AuditLoggerImpl) LogCommandSubmission(ctx context.Context, event *model.CommandSubmissionEvent) {
	// Ensure base fields are set
	a.ensureBaseFields(ctx, &event.AuditEventBase)
	event.EventType = model.AuditEventCommandSubmission

	a.logger.Info("audit event",
		slog.String("log_type", event.LogType),
		slog.String("correlation_id", event.CorrelationID),
		slog.Time("timestamp", event.Timestamp),
		slog.String("event_type", event.EventType),
		slog.String("vin", event.VIN),
		slog.String("command_type", event.CommandType),
		slog.Any("doors", event.Doors),
		slog.String("source_ip", event.SourceIP),
		slog.String("request_id", event.RequestID),
		slog.String("command_id", event.CommandID),
	)
}

// LogCommandStatusChange logs a command status change event.
func (a *AuditLoggerImpl) LogCommandStatusChange(ctx context.Context, event *model.CommandStatusChangeEvent) {
	a.ensureBaseFields(ctx, &event.AuditEventBase)
	event.EventType = model.AuditEventCommandStatusChange

	a.logger.Info("audit event",
		slog.String("log_type", event.LogType),
		slog.String("correlation_id", event.CorrelationID),
		slog.Time("timestamp", event.Timestamp),
		slog.String("event_type", event.EventType),
		slog.String("command_id", event.CommandID),
		slog.String("previous_status", event.PreviousStatus),
		slog.String("new_status", event.NewStatus),
	)
}

// LogAuthAttempt logs an authentication attempt event.
func (a *AuditLoggerImpl) LogAuthAttempt(ctx context.Context, event *model.AuthAttemptEvent) {
	a.ensureBaseFields(ctx, &event.AuditEventBase)
	event.EventType = model.AuditEventAuthAttempt

	a.logger.Info("audit event",
		slog.String("log_type", event.LogType),
		slog.String("correlation_id", event.CorrelationID),
		slog.Time("timestamp", event.Timestamp),
		slog.String("event_type", event.EventType),
		slog.String("vin", event.VIN),
		slog.String("auth_token_hash", event.AuthTokenHash),
		slog.Bool("success", event.Success),
		slog.String("source_ip", event.SourceIP),
	)
}

// LogTelemetryUpdate logs a telemetry update event.
func (a *AuditLoggerImpl) LogTelemetryUpdate(ctx context.Context, event *model.TelemetryUpdateEvent) {
	a.ensureBaseFields(ctx, &event.AuditEventBase)
	event.EventType = model.AuditEventTelemetryUpdate

	a.logger.Info("audit event",
		slog.String("log_type", event.LogType),
		slog.String("correlation_id", event.CorrelationID),
		slog.Time("timestamp", event.Timestamp),
		slog.String("event_type", event.EventType),
		slog.String("vin", event.VIN),
		slog.Bool("location_present", event.LocationPresent),
		slog.Bool("door_state_changed", event.DoorStateChanged),
	)
}

// LogMQTTConnectionEvent logs MQTT connection events.
func (a *AuditLoggerImpl) LogMQTTConnectionEvent(ctx context.Context, event *model.MQTTConnectionEvent) {
	a.ensureBaseFields(ctx, &event.AuditEventBase)

	a.logger.Info("audit event",
		slog.String("log_type", event.LogType),
		slog.String("correlation_id", event.CorrelationID),
		slog.Time("timestamp", event.Timestamp),
		slog.String("event_type", event.EventType),
		slog.String("broker_address", event.BrokerAddress),
	)
}

// LogValidationFailure logs a validation failure event.
func (a *AuditLoggerImpl) LogValidationFailure(ctx context.Context, event *model.ValidationFailureEvent) {
	a.ensureBaseFields(ctx, &event.AuditEventBase)
	event.EventType = model.AuditEventValidationFailure

	a.logger.Info("audit event",
		slog.String("log_type", event.LogType),
		slog.String("correlation_id", event.CorrelationID),
		slog.Time("timestamp", event.Timestamp),
		slog.String("event_type", event.EventType),
		slog.String("vin", event.VIN),
		slog.String("endpoint", event.Endpoint),
		slog.String("validation_error", event.ValidationError),
		slog.String("source_ip", event.SourceIP),
	)
}

// ensureBaseFields ensures that the base audit fields are populated.
func (a *AuditLoggerImpl) ensureBaseFields(ctx context.Context, base *model.AuditEventBase) {
	if base.LogType == "" {
		base.LogType = "audit"
	}
	if base.CorrelationID == "" {
		base.CorrelationID = middleware.GetRequestID(ctx)
	}
	if base.Timestamp.IsZero() {
		base.Timestamp = model.NewAuditEventBase("").Timestamp
	}
}

// HashToken returns first 8 characters of SHA256 hash of token.
// Used to log auth tokens without exposing sensitive data.
func HashToken(token string) string {
	if token == "" {
		return ""
	}
	hash := sha256.Sum256([]byte(token))
	hexHash := hex.EncodeToString(hash[:])
	if len(hexHash) >= 8 {
		return hexHash[:8]
	}
	return hexHash
}
