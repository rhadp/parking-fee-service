package mqtt

import (
	"encoding/json"
	"log/slog"

	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/model"
)

// CommandResponseHandler handles command response messages from vehicles.
type CommandResponseHandler interface {
	HandleCommandResponse(response *model.MQTTCommandResponse)
}

// TelemetryHandler handles telemetry messages from vehicles.
type TelemetryHandler interface {
	HandleTelemetry(telemetry *model.MQTTTelemetryMessage)
}

// MessageHandlers creates MQTT message handlers for the cloud-gateway.
type MessageHandlers struct {
	logger           *slog.Logger
	commandHandler   CommandResponseHandler
	telemetryHandler TelemetryHandler
}

// NewMessageHandlers creates new message handlers.
func NewMessageHandlers(
	logger *slog.Logger,
	commandHandler CommandResponseHandler,
	telemetryHandler TelemetryHandler,
) *MessageHandlers {
	return &MessageHandlers{
		logger:           logger,
		commandHandler:   commandHandler,
		telemetryHandler: telemetryHandler,
	}
}

// HandleCommandResponse handles command response messages from the vehicle.
func (h *MessageHandlers) HandleCommandResponse(topic string, payload []byte) {
	var response model.MQTTCommandResponse
	if err := json.Unmarshal(payload, &response); err != nil {
		h.logger.Error("failed to parse command response",
			slog.String("topic", topic),
			slog.String("error", err.Error()),
		)
		return
	}

	h.logger.Debug("received command response",
		slog.String("command_id", response.CommandID),
		slog.String("status", response.Status),
	)

	if h.commandHandler != nil {
		h.commandHandler.HandleCommandResponse(&response)
	}
}

// HandleTelemetry handles telemetry messages from the vehicle.
func (h *MessageHandlers) HandleTelemetry(topic string, payload []byte) {
	var telemetry model.MQTTTelemetryMessage
	if err := json.Unmarshal(payload, &telemetry); err != nil {
		h.logger.Error("failed to parse telemetry message",
			slog.String("topic", topic),
			slog.String("error", err.Error()),
		)
		return
	}

	h.logger.Debug("received telemetry",
		slog.String("timestamp", telemetry.Timestamp),
		slog.Float64("latitude", telemetry.Latitude),
		slog.Float64("longitude", telemetry.Longitude),
	)

	if h.telemetryHandler != nil {
		h.telemetryHandler.HandleTelemetry(&telemetry)
	}
}
