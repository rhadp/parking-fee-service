package handler

import (
	"context"
	"encoding/json"
	"log/slog"
	"net/http"
	"time"

	"github.com/gorilla/mux"

	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/audit"
	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/middleware"
	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/model"
	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/service"
)

// CommandHandler handles command-related HTTP requests.
type CommandHandler struct {
	commandService *service.CommandService
	auditLogger    audit.AuditLogger
	logger         *slog.Logger
	configuredVIN  string
}

// NewCommandHandler creates a new CommandHandler.
func NewCommandHandler(
	commandService *service.CommandService,
	auditLogger audit.AuditLogger,
	logger *slog.Logger,
	configuredVIN string,
) *CommandHandler {
	return &CommandHandler{
		commandService: commandService,
		auditLogger:    auditLogger,
		logger:         logger,
		configuredVIN:  configuredVIN,
	}
}

// HandleSubmitCommand handles POST /api/v1/vehicles/{vin}/commands.
func (h *CommandHandler) HandleSubmitCommand(w http.ResponseWriter, r *http.Request) {
	ctx := r.Context()
	requestID := middleware.GetRequestID(ctx)
	sourceIP := middleware.GetSourceIP(r)

	// Extract VIN from path
	vars := mux.Vars(r)
	vin := vars["vin"]

	// Validate VIN
	if vin != h.configuredVIN {
		h.logValidationFailure(ctx, vin, "/api/v1/vehicles/{vin}/commands", "VIN not found", sourceIP)
		WriteNotFound(w, r, model.ErrVehicleNotFound, "Vehicle not found")
		return
	}

	// Parse request body
	var req model.SubmitCommandRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		h.logValidationFailure(ctx, vin, "/api/v1/vehicles/{vin}/commands", "Invalid JSON body", sourceIP)
		WriteValidationError(w, r, model.ErrInternalError, "Invalid request body")
		return
	}

	// Validate command type
	if !isValidCommandType(req.CommandType) {
		h.logValidationFailure(ctx, vin, "/api/v1/vehicles/{vin}/commands", "Invalid command type: "+req.CommandType, sourceIP)
		WriteValidationError(w, r, model.ErrInvalidCommandType, "Invalid command type. Must be 'lock' or 'unlock'")
		return
	}

	// Validate auth token
	if req.AuthToken == "" {
		h.logAuthAttempt(ctx, vin, "", false, sourceIP)
		WriteValidationError(w, r, model.ErrMissingAuthToken, "Auth token is required")
		return
	}

	// Validate doors
	if !isValidDoors(req.Doors) {
		h.logValidationFailure(ctx, vin, "/api/v1/vehicles/{vin}/commands", "Invalid doors value", sourceIP)
		WriteValidationError(w, r, model.ErrInvalidDoor, "Invalid doors value. Must be 'driver' or 'all'")
		return
	}

	// Log successful auth attempt
	h.logAuthAttempt(ctx, vin, req.AuthToken, true, sourceIP)

	// Submit command
	cmd, err := h.commandService.SubmitCommand(ctx, &req, sourceIP)
	if err != nil {
		h.logger.Error("failed to submit command",
			slog.String("error", err.Error()),
			slog.String("request_id", requestID),
		)
		WriteInternalError(w, r, "Failed to submit command")
		return
	}

	// Build response
	response := model.SubmitCommandResponse{
		CommandID: cmd.CommandID,
		Status:    cmd.Status,
		RequestID: requestID,
	}

	WriteJSON(w, http.StatusAccepted, response)
}

// HandleGetCommandStatus handles GET /api/v1/vehicles/{vin}/commands/{command_id}.
func (h *CommandHandler) HandleGetCommandStatus(w http.ResponseWriter, r *http.Request) {
	ctx := r.Context()
	requestID := middleware.GetRequestID(ctx)
	sourceIP := middleware.GetSourceIP(r)

	// Extract path parameters
	vars := mux.Vars(r)
	vin := vars["vin"]
	commandID := vars["command_id"]

	// Validate VIN
	if vin != h.configuredVIN {
		h.logValidationFailure(ctx, vin, "/api/v1/vehicles/{vin}/commands/{command_id}", "VIN not found", sourceIP)
		WriteNotFound(w, r, model.ErrVehicleNotFound, "Vehicle not found")
		return
	}

	// Get command status
	cmd := h.commandService.GetCommandStatus(commandID)
	if cmd == nil {
		WriteNotFound(w, r, model.ErrCommandNotFound, "Command not found")
		return
	}

	// Build response
	response := model.CommandStatusResponse{
		CommandID:    cmd.CommandID,
		CommandType:  cmd.CommandType,
		Status:       cmd.Status,
		CreatedAt:    cmd.CreatedAt.Format(time.RFC3339),
		ErrorCode:    cmd.ErrorCode,
		ErrorMessage: cmd.ErrorMessage,
		RequestID:    requestID,
	}

	if cmd.CompletedAt != nil {
		completedAt := cmd.CompletedAt.Format(time.RFC3339)
		response.CompletedAt = &completedAt
	}

	WriteJSON(w, http.StatusOK, response)
}

// isValidCommandType validates the command type.
func isValidCommandType(commandType string) bool {
	return commandType == model.CommandTypeLock || commandType == model.CommandTypeUnlock
}

// isValidDoors validates the doors value.
func isValidDoors(doors []string) bool {
	if len(doors) == 0 {
		return false
	}
	for _, door := range doors {
		if door != model.DoorDriver && door != model.DoorAll {
			return false
		}
	}
	return true
}

// logValidationFailure logs a validation failure audit event.
func (h *CommandHandler) logValidationFailure(ctx context.Context, vin, endpoint, validationError, sourceIP string) {
	if h.auditLogger == nil {
		return
	}
	event := &model.ValidationFailureEvent{
		AuditEventBase:  model.NewAuditEventBase(middleware.GetRequestID(ctx)),
		VIN:             vin,
		Endpoint:        endpoint,
		ValidationError: validationError,
		SourceIP:        sourceIP,
	}
	h.auditLogger.LogValidationFailure(ctx, event)
}

// logAuthAttempt logs an authentication attempt audit event.
func (h *CommandHandler) logAuthAttempt(ctx context.Context, vin, authToken string, success bool, sourceIP string) {
	if h.auditLogger == nil {
		return
	}
	event := &model.AuthAttemptEvent{
		AuditEventBase: model.NewAuditEventBase(middleware.GetRequestID(ctx)),
		VIN:            vin,
		AuthTokenHash:  audit.HashToken(authToken),
		Success:        success,
		SourceIP:       sourceIP,
	}
	h.auditLogger.LogAuthAttempt(ctx, event)
}
