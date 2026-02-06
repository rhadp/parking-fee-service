package property

import (
	"bytes"
	"context"
	"log/slog"
	"strings"
	"testing"
	"time"

	"github.com/leanovate/gopter"
	"github.com/leanovate/gopter/gen"
	"github.com/leanovate/gopter/prop"

	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/audit"
	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/model"
)

// TestAuditLogEventCompleteness tests Property 14: Audit Log Event Completeness.
// For any audit event type, the audit log entry SHALL contain all required fields.
func TestAuditLogEventCompleteness(t *testing.T) {
	// Feature: cloud-gateway, Property 14: Audit Log Event Completeness
	// Validates: Requirements 14.1, 14.2, 14.3, 14.4, 14.6, 14.7

	parameters := gopter.DefaultTestParameters()
	parameters.MinSuccessfulTests = 100

	properties := gopter.NewProperties(parameters)

	properties.Property("command submission audit logs contain all required fields", prop.ForAll(
		func(vin, cmdType, sourceIP, requestID, commandID string) bool {
			var buf bytes.Buffer
			logger := slog.New(slog.NewJSONHandler(&buf, nil))
			auditLogger := audit.NewAuditLogger(logger)

			event := &model.CommandSubmissionEvent{
				AuditEventBase: model.NewAuditEventBase(requestID),
				VIN:            vin,
				CommandType:    cmdType,
				Doors:          []string{model.DoorAll},
				SourceIP:       sourceIP,
				RequestID:      requestID,
				CommandID:      commandID,
			}

			auditLogger.LogCommandSubmission(context.Background(), event)

			logOutput := buf.String()
			return strings.Contains(logOutput, vin) &&
				strings.Contains(logOutput, cmdType) &&
				strings.Contains(logOutput, sourceIP) &&
				strings.Contains(logOutput, requestID) &&
				strings.Contains(logOutput, commandID) &&
				strings.Contains(logOutput, "command_submission")
		},
		gen.AlphaString().SuchThat(func(s string) bool { return len(s) > 0 }),
		gen.OneConstOf("lock", "unlock"),
		gen.AlphaString().SuchThat(func(s string) bool { return len(s) > 0 }),
		gen.AlphaString().SuchThat(func(s string) bool { return len(s) > 0 }),
		gen.AlphaString().SuchThat(func(s string) bool { return len(s) > 0 }),
	))

	properties.Property("command status change audit logs contain all required fields", prop.ForAll(
		func(commandID, prevStatus, newStatus string) bool {
			var buf bytes.Buffer
			logger := slog.New(slog.NewJSONHandler(&buf, nil))
			auditLogger := audit.NewAuditLogger(logger)

			event := &model.CommandStatusChangeEvent{
				AuditEventBase: model.NewAuditEventBase("corr-123"),
				CommandID:      commandID,
				PreviousStatus: prevStatus,
				NewStatus:      newStatus,
			}

			auditLogger.LogCommandStatusChange(context.Background(), event)

			logOutput := buf.String()
			return strings.Contains(logOutput, commandID) &&
				strings.Contains(logOutput, prevStatus) &&
				strings.Contains(logOutput, newStatus) &&
				strings.Contains(logOutput, "command_status_change")
		},
		gen.AlphaString().SuchThat(func(s string) bool { return len(s) > 0 }),
		gen.OneConstOf(model.CommandStatusPending, model.CommandStatusSuccess),
		gen.OneConstOf(model.CommandStatusSuccess, model.CommandStatusFailed, model.CommandStatusTimeout),
	))

	properties.Property("MQTT connection audit logs contain all required fields", prop.ForAll(
		func(eventType, brokerAddress string) bool {
			var buf bytes.Buffer
			logger := slog.New(slog.NewJSONHandler(&buf, nil))
			auditLogger := audit.NewAuditLogger(logger)

			event := &model.MQTTConnectionEvent{
				AuditEventBase: model.NewAuditEventBase("corr-123"),
				EventType:      eventType,
				BrokerAddress:  brokerAddress,
			}

			auditLogger.LogMQTTConnectionEvent(context.Background(), event)

			logOutput := buf.String()
			return strings.Contains(logOutput, eventType) &&
				strings.Contains(logOutput, brokerAddress)
		},
		gen.OneConstOf(model.AuditEventMQTTConnect, model.AuditEventMQTTDisconnect, model.AuditEventMQTTReconnect),
		gen.AlphaString().SuchThat(func(s string) bool { return len(s) > 0 }),
	))

	properties.Property("validation failure audit logs contain all required fields", prop.ForAll(
		func(vin, endpoint, validationError, sourceIP string) bool {
			var buf bytes.Buffer
			logger := slog.New(slog.NewJSONHandler(&buf, nil))
			auditLogger := audit.NewAuditLogger(logger)

			event := &model.ValidationFailureEvent{
				AuditEventBase:  model.NewAuditEventBase("corr-123"),
				VIN:             vin,
				Endpoint:        endpoint,
				ValidationError: validationError,
				SourceIP:        sourceIP,
			}

			auditLogger.LogValidationFailure(context.Background(), event)

			logOutput := buf.String()
			return strings.Contains(logOutput, vin) &&
				strings.Contains(logOutput, endpoint) &&
				strings.Contains(logOutput, validationError) &&
				strings.Contains(logOutput, sourceIP) &&
				strings.Contains(logOutput, "validation_failure")
		},
		gen.AlphaString().SuchThat(func(s string) bool { return len(s) > 0 }),
		gen.AlphaString().SuchThat(func(s string) bool { return len(s) > 0 }),
		gen.AlphaString().SuchThat(func(s string) bool { return len(s) > 0 }),
		gen.AlphaString().SuchThat(func(s string) bool { return len(s) > 0 }),
	))

	properties.TestingRun(t)
}

// TestAuditLogStructureConsistency tests Property 15: Audit Log Structure Consistency.
// For any audit log entry, it SHALL contain a log_type field set to "audit" and
// a correlation_id field for request tracing.
func TestAuditLogStructureConsistency(t *testing.T) {
	// Feature: cloud-gateway, Property 15: Audit Log Structure Consistency
	// Validates: Requirements 14.5, 14.8

	parameters := gopter.DefaultTestParameters()
	parameters.MinSuccessfulTests = 100

	properties := gopter.NewProperties(parameters)

	properties.Property("all audit logs have log_type=audit and correlation_id", prop.ForAll(
		func(correlationID string) bool {
			var buf bytes.Buffer
			logger := slog.New(slog.NewJSONHandler(&buf, nil))
			auditLogger := audit.NewAuditLogger(logger)

			event := &model.CommandSubmissionEvent{
				AuditEventBase: model.AuditEventBase{
					LogType:       "audit",
					CorrelationID: correlationID,
					Timestamp:     time.Now(),
				},
				VIN:         "TEST_VIN",
				CommandType: "lock",
				Doors:       []string{"all"},
				SourceIP:    "127.0.0.1",
				RequestID:   "req-123",
				CommandID:   "cmd-123",
			}

			auditLogger.LogCommandSubmission(context.Background(), event)

			logOutput := buf.String()
			return strings.Contains(logOutput, `"log_type":"audit"`) &&
				strings.Contains(logOutput, correlationID)
		},
		gen.AlphaString().SuchThat(func(s string) bool { return len(s) > 0 }),
	))

	properties.Property("ensureBaseFields sets log_type to audit when empty", prop.ForAll(
		func(correlationID string) bool {
			var buf bytes.Buffer
			logger := slog.New(slog.NewJSONHandler(&buf, nil))
			auditLogger := audit.NewAuditLogger(logger)

			// Create event without setting LogType
			event := &model.CommandSubmissionEvent{
				AuditEventBase: model.AuditEventBase{
					CorrelationID: correlationID,
					Timestamp:     time.Now(),
				},
				VIN:         "TEST_VIN",
				CommandType: "lock",
			}

			auditLogger.LogCommandSubmission(context.Background(), event)

			logOutput := buf.String()
			// Should have log_type=audit even though we didn't set it
			return strings.Contains(logOutput, `"log_type":"audit"`)
		},
		gen.AlphaString().SuchThat(func(s string) bool { return len(s) > 0 }),
	))

	properties.TestingRun(t)
}

// TestSensitiveDataExclusion tests Property 16: Sensitive Data Exclusion.
// For any audit log entry, it SHALL NOT contain the full auth_token or user credentials.
func TestSensitiveDataExclusion(t *testing.T) {
	// Feature: cloud-gateway, Property 16: Sensitive Data Exclusion
	// Validates: Requirements 14.9

	parameters := gopter.DefaultTestParameters()
	parameters.MinSuccessfulTests = 100

	properties := gopter.NewProperties(parameters)

	properties.Property("auth tokens are hashed and truncated in audit logs", prop.ForAll(
		func(authToken string) bool {
			if len(authToken) < 10 {
				return true // skip short tokens
			}

			var buf bytes.Buffer
			logger := slog.New(slog.NewJSONHandler(&buf, nil))
			auditLogger := audit.NewAuditLogger(logger)

			tokenHash := audit.HashToken(authToken)

			event := &model.AuthAttemptEvent{
				AuditEventBase: model.NewAuditEventBase("corr-123"),
				VIN:            "TEST_VIN",
				AuthTokenHash:  tokenHash,
				Success:        true,
				SourceIP:       "127.0.0.1",
			}

			auditLogger.LogAuthAttempt(context.Background(), event)

			logOutput := buf.String()
			// Full token should NOT appear, only truncated hash
			return !strings.Contains(logOutput, authToken) &&
				len(tokenHash) == 8
		},
		gen.AlphaString().SuchThat(func(s string) bool { return len(s) >= 10 }),
	))

	properties.Property("HashToken returns exactly 8 characters", prop.ForAll(
		func(token string) bool {
			if token == "" {
				return audit.HashToken(token) == ""
			}
			hash := audit.HashToken(token)
			return len(hash) == 8
		},
		gen.AlphaString(),
	))

	properties.Property("HashToken is deterministic", prop.ForAll(
		func(token string) bool {
			hash1 := audit.HashToken(token)
			hash2 := audit.HashToken(token)
			return hash1 == hash2
		},
		gen.AlphaString(),
	))

	properties.TestingRun(t)
}

// TestCommandLifecycleTraceability tests Property 17: Command Lifecycle Traceability.
// For any complete command lifecycle, the audit logs SHALL contain sufficient detail
// to reconstruct the sequence of events.
func TestCommandLifecycleTraceability(t *testing.T) {
	// Feature: cloud-gateway, Property 17: Command Lifecycle Traceability
	// Validates: Requirements 14.10

	parameters := gopter.DefaultTestParameters()
	parameters.MinSuccessfulTests = 100

	properties := gopter.NewProperties(parameters)

	properties.Property("command lifecycle can be traced via correlation_id and command_id", prop.ForAll(
		func(correlationID, commandID string) bool {
			var buf bytes.Buffer
			logger := slog.New(slog.NewJSONHandler(&buf, nil))
			auditLogger := audit.NewAuditLogger(logger)

			// Log submission
			auditLogger.LogCommandSubmission(context.Background(), &model.CommandSubmissionEvent{
				AuditEventBase: model.NewAuditEventBase(correlationID),
				VIN:            "TEST_VIN",
				CommandType:    "lock",
				Doors:          []string{"all"},
				SourceIP:       "127.0.0.1",
				RequestID:      correlationID,
				CommandID:      commandID,
			})

			// Log status change
			auditLogger.LogCommandStatusChange(context.Background(), &model.CommandStatusChangeEvent{
				AuditEventBase: model.NewAuditEventBase(correlationID),
				CommandID:      commandID,
				PreviousStatus: model.CommandStatusPending,
				NewStatus:      model.CommandStatusSuccess,
			})

			logOutput := buf.String()

			// Both events should contain the command_id
			submissionFound := strings.Contains(logOutput, "command_submission")
			statusChangeFound := strings.Contains(logOutput, "command_status_change")
			commandIDFound := strings.Count(logOutput, commandID) >= 2
			correlationIDFound := strings.Count(logOutput, correlationID) >= 2

			return submissionFound && statusChangeFound && commandIDFound && correlationIDFound
		},
		gen.AlphaString().SuchThat(func(s string) bool { return len(s) > 0 }),
		gen.AlphaString().SuchThat(func(s string) bool { return len(s) > 0 }),
	))

	properties.TestingRun(t)
}
