package property

import (
	"context"
	"log/slog"
	"os"
	"testing"
	"time"

	"github.com/leanovate/gopter"
	"github.com/leanovate/gopter/gen"
	"github.com/leanovate/gopter/prop"

	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/model"
	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/mqtt"
	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/service"
	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/store"
)

// mockMQTTClient implements mqtt.Client for testing.
type mockMQTTClient struct {
	published []mockPublishedMessage
	connected bool
}

type mockPublishedMessage struct {
	topic   string
	payload []byte
}

func (m *mockMQTTClient) Connect() error                                            { m.connected = true; return nil }
func (m *mockMQTTClient) Disconnect()                                               { m.connected = false }
func (m *mockMQTTClient) IsConnected() bool                                         { return m.connected }
func (m *mockMQTTClient) Subscribe(topic string, handler mqtt.MessageHandler) error { return nil }
func (m *mockMQTTClient) Publish(topic string, payload []byte) error {
	m.published = append(m.published, mockPublishedMessage{topic: topic, payload: payload})
	return nil
}

// mockAuditLogger implements audit.AuditLogger for testing.
type mockAuditLogger struct {
	events []interface{}
}

func (m *mockAuditLogger) LogCommandSubmission(ctx context.Context, event *model.CommandSubmissionEvent) {
	m.events = append(m.events, event)
}
func (m *mockAuditLogger) LogCommandStatusChange(ctx context.Context, event *model.CommandStatusChangeEvent) {
	m.events = append(m.events, event)
}
func (m *mockAuditLogger) LogAuthAttempt(ctx context.Context, event *model.AuthAttemptEvent) {
	m.events = append(m.events, event)
}
func (m *mockAuditLogger) LogTelemetryUpdate(ctx context.Context, event *model.TelemetryUpdateEvent) {
	m.events = append(m.events, event)
}
func (m *mockAuditLogger) LogMQTTConnectionEvent(ctx context.Context, event *model.MQTTConnectionEvent) {
	m.events = append(m.events, event)
}
func (m *mockAuditLogger) LogValidationFailure(ctx context.Context, event *model.ValidationFailureEvent) {
	m.events = append(m.events, event)
}

// TestCommandCreationWithUniqueID tests Property 2: Command Creation with Unique ID.
func TestCommandCreationWithUniqueID(t *testing.T) {
	// Feature: cloud-gateway, Property 2: Command Creation with Unique ID
	// Validates: Requirements 2.1, 2.3, 2.4

	parameters := gopter.DefaultTestParameters()
	parameters.MinSuccessfulTests = 100

	properties := gopter.NewProperties(parameters)

	logger := slog.New(slog.NewJSONHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))

	properties.Property("each command gets a unique ID", prop.ForAll(
		func(numCommands int) bool {
			cmdStore := store.NewCommandStore(100)
			mqttClient := &mockMQTTClient{connected: true}
			auditLogger := &mockAuditLogger{}

			svc := service.NewCommandService(
				cmdStore,
				mqttClient,
				auditLogger,
				logger,
				30*time.Second,
				"TEST_VIN",
			)

			ids := make(map[string]bool)
			for i := 0; i < numCommands; i++ {
				req := &model.SubmitCommandRequest{
					CommandType: model.CommandTypeLock,
					Doors:       []string{model.DoorAll},
					AuthToken:   "token",
				}

				cmd, err := svc.SubmitCommand(context.Background(), req, "127.0.0.1")
				if err != nil {
					t.Logf("failed to submit command: %v", err)
					return false
				}

				if ids[cmd.CommandID] {
					t.Logf("duplicate command ID: %s", cmd.CommandID)
					return false
				}
				ids[cmd.CommandID] = true
			}

			return len(ids) == numCommands
		},
		gen.IntRange(1, 50),
	))

	properties.Property("command is immediately retrievable after submission", prop.ForAll(
		func(cmdType string) bool {
			cmdStore := store.NewCommandStore(100)
			mqttClient := &mockMQTTClient{connected: true}
			auditLogger := &mockAuditLogger{}

			svc := service.NewCommandService(
				cmdStore,
				mqttClient,
				auditLogger,
				logger,
				30*time.Second,
				"TEST_VIN",
			)

			req := &model.SubmitCommandRequest{
				CommandType: cmdType,
				Doors:       []string{model.DoorAll},
				AuthToken:   "token",
			}

			cmd, err := svc.SubmitCommand(context.Background(), req, "127.0.0.1")
			if err != nil {
				return false
			}

			retrieved := svc.GetCommandStatus(cmd.CommandID)
			return retrieved != nil && retrieved.CommandID == cmd.CommandID
		},
		gen.OneConstOf(model.CommandTypeLock, model.CommandTypeUnlock),
	))

	properties.Property("command is published to MQTT on submission", prop.ForAll(
		func(cmdType string) bool {
			cmdStore := store.NewCommandStore(100)
			mqttClient := &mockMQTTClient{connected: true}
			auditLogger := &mockAuditLogger{}

			svc := service.NewCommandService(
				cmdStore,
				mqttClient,
				auditLogger,
				logger,
				30*time.Second,
				"TEST_VIN",
			)

			req := &model.SubmitCommandRequest{
				CommandType: cmdType,
				Doors:       []string{model.DoorAll},
				AuthToken:   "token",
			}

			svc.SubmitCommand(context.Background(), req, "127.0.0.1")

			// Check that message was published
			return len(mqttClient.published) == 1
		},
		gen.OneConstOf(model.CommandTypeLock, model.CommandTypeUnlock),
	))

	properties.TestingRun(t)
}

// TestCommandResponseProcessing tests Property 7: Command Response Processing.
func TestCommandResponseProcessing(t *testing.T) {
	// Feature: cloud-gateway, Property 7: Command Response Processing
	// Validates: Requirements 4.2, 4.3, 4.4

	parameters := gopter.DefaultTestParameters()
	parameters.MinSuccessfulTests = 100

	properties := gopter.NewProperties(parameters)

	logger := slog.New(slog.NewJSONHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))

	properties.Property("command status updated on response", prop.ForAll(
		func(responseStatus string) bool {
			cmdStore := store.NewCommandStore(100)
			mqttClient := &mockMQTTClient{connected: true}
			auditLogger := &mockAuditLogger{}

			svc := service.NewCommandService(
				cmdStore,
				mqttClient,
				auditLogger,
				logger,
				30*time.Second,
				"TEST_VIN",
			)

			// Submit a command
			req := &model.SubmitCommandRequest{
				CommandType: model.CommandTypeLock,
				Doors:       []string{model.DoorAll},
				AuthToken:   "token",
			}
			cmd, _ := svc.SubmitCommand(context.Background(), req, "127.0.0.1")

			// Simulate response
			response := &model.MQTTCommandResponse{
				CommandID: cmd.CommandID,
				Status:    responseStatus,
				Timestamp: time.Now().UTC().Format(time.RFC3339),
			}
			svc.HandleCommandResponse(context.Background(), response)

			// Verify status updated
			updated := svc.GetCommandStatus(cmd.CommandID)
			return updated != nil && updated.Status == responseStatus
		},
		gen.OneConstOf(model.CommandStatusSuccess, model.CommandStatusFailed),
	))

	properties.Property("completed_at set on response", prop.ForAll(
		func() bool {
			cmdStore := store.NewCommandStore(100)
			mqttClient := &mockMQTTClient{connected: true}
			auditLogger := &mockAuditLogger{}

			svc := service.NewCommandService(
				cmdStore,
				mqttClient,
				auditLogger,
				logger,
				30*time.Second,
				"TEST_VIN",
			)

			req := &model.SubmitCommandRequest{
				CommandType: model.CommandTypeLock,
				Doors:       []string{model.DoorAll},
				AuthToken:   "token",
			}
			cmd, _ := svc.SubmitCommand(context.Background(), req, "127.0.0.1")

			// Initially completed_at should be nil
			initial := svc.GetCommandStatus(cmd.CommandID)
			if initial.CompletedAt != nil {
				return false
			}

			// Simulate response
			response := &model.MQTTCommandResponse{
				CommandID: cmd.CommandID,
				Status:    model.CommandStatusSuccess,
				Timestamp: time.Now().UTC().Format(time.RFC3339),
			}
			svc.HandleCommandResponse(context.Background(), response)

			// completed_at should now be set
			updated := svc.GetCommandStatus(cmd.CommandID)
			return updated.CompletedAt != nil
		},
	))

	properties.TestingRun(t)
}

// TestCommandTimeoutStatus tests Property 8: Command Timeout Status.
func TestCommandTimeoutStatus(t *testing.T) {
	// Feature: cloud-gateway, Property 8: Command Timeout Status
	// Validates: Requirements 5.2, 5.3

	parameters := gopter.DefaultTestParameters()
	parameters.MinSuccessfulTests = 50

	properties := gopter.NewProperties(parameters)

	logger := slog.New(slog.NewJSONHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))

	properties.Property("commands exceeding timeout get timeout status", prop.ForAll(
		func() bool {
			cmdStore := store.NewCommandStore(100)
			mqttClient := &mockMQTTClient{connected: true}
			auditLogger := &mockAuditLogger{}

			// Use very short timeout for testing
			svc := service.NewCommandService(
				cmdStore,
				mqttClient,
				auditLogger,
				logger,
				1*time.Millisecond, // Very short timeout
				"TEST_VIN",
			)

			req := &model.SubmitCommandRequest{
				CommandType: model.CommandTypeLock,
				Doors:       []string{model.DoorAll},
				AuthToken:   "token",
			}
			cmd, _ := svc.SubmitCommand(context.Background(), req, "127.0.0.1")

			// Wait for timeout
			time.Sleep(10 * time.Millisecond)

			// Manually trigger timeout check
			svc.StartTimeoutChecker(1 * time.Millisecond)
			time.Sleep(10 * time.Millisecond)
			svc.StopTimeoutChecker()

			// Check status
			updated := svc.GetCommandStatus(cmd.CommandID)
			return updated != nil &&
				updated.Status == model.CommandStatusTimeout &&
				updated.ErrorCode == model.ErrTimeout
		},
	))

	properties.TestingRun(t)
}
