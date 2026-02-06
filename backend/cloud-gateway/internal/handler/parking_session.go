package handler

import (
	"log/slog"
	"net/http"

	"github.com/gorilla/mux"

	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/middleware"
	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/model"
	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/service"
)

// ParkingSessionHandler handles parking session HTTP requests.
type ParkingSessionHandler struct {
	parkingSessionService *service.ParkingSessionService
	logger                *slog.Logger
	configuredVIN         string
}

// NewParkingSessionHandler creates a new ParkingSessionHandler.
func NewParkingSessionHandler(
	parkingSessionService *service.ParkingSessionService,
	logger *slog.Logger,
	configuredVIN string,
) *ParkingSessionHandler {
	return &ParkingSessionHandler{
		parkingSessionService: parkingSessionService,
		logger:                logger,
		configuredVIN:         configuredVIN,
	}
}

// HandleGetParkingSession handles GET /api/v1/vehicles/{vin}/parking-session.
func (h *ParkingSessionHandler) HandleGetParkingSession(w http.ResponseWriter, r *http.Request) {
	ctx := r.Context()
	requestID := middleware.GetRequestID(ctx)

	// Extract VIN from path
	vars := mux.Vars(r)
	vin := vars["vin"]

	// Get parking session
	result := h.parkingSessionService.GetParkingSession(ctx, vin)

	// Handle errors
	if result.Error != nil {
		switch result.Code {
		case model.ErrVehicleNotFound:
			WriteNotFound(w, r, model.ErrVehicleNotFound, "Vehicle not found")
		case model.ErrNoActiveSession:
			WriteNotFound(w, r, model.ErrNoActiveSession, "No active parking session for this vehicle")
		default:
			h.logger.Error("failed to get parking session",
				slog.String("error", result.Error.Error()),
				slog.String("request_id", requestID),
			)
			WriteInternalError(w, r, "Failed to get parking session")
		}
		return
	}

	// Build response
	response := model.ParkingSessionResponse{
		SessionID:       result.Session.SessionID,
		ZoneName:        result.Session.ZoneName,
		HourlyRate:      result.Session.HourlyRate,
		Currency:        result.Session.Currency,
		DurationSeconds: result.Session.DurationSeconds,
		CurrentCost:     result.Session.CurrentCost,
		Timestamp:       result.Session.Timestamp,
		RequestID:       requestID,
	}

	WriteJSON(w, http.StatusOK, response)
}
