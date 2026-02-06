// Package handler provides HTTP handlers for the parking-fee-service.
package handler

import (
	"encoding/json"
	"log/slog"
	"net/http"

	"github.com/gorilla/mux"

	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/middleware"
	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/model"
	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/service"
)

// ParkingHandler handles mock parking operator requests.
type ParkingHandler struct {
	parkingService *service.ParkingService
	logger         *slog.Logger
}

// NewParkingHandler creates a new ParkingHandler.
func NewParkingHandler(parkingService *service.ParkingService, logger *slog.Logger) *ParkingHandler {
	return &ParkingHandler{
		parkingService: parkingService,
		logger:         logger,
	}
}

// HandleStartSession handles POST /api/v1/parking/start
func (h *ParkingHandler) HandleStartSession(w http.ResponseWriter, r *http.Request) {
	var req model.StartSessionRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		middleware.WriteValidationError(w, r, "invalid request body")
		return
	}

	// Validate request
	if err := middleware.ValidateStartSessionRequest(&req); err != nil {
		middleware.WriteValidationError(w, r, err.Error())
		return
	}

	// Start session
	session, _, err := h.parkingService.StartSession(&req)
	if err != nil {
		middleware.WriteDatabaseError(w, r, "failed to start session")
		return
	}

	response := model.StartSessionResponse{
		SessionID:  session.SessionID,
		ZoneID:     session.ZoneID,
		HourlyRate: session.HourlyRate,
		StartTime:  session.StartTime.Format("2006-01-02T15:04:05Z07:00"),
	}

	middleware.WriteJSON(w, http.StatusOK, response)
}

// HandleStopSession handles POST /api/v1/parking/stop
func (h *ParkingHandler) HandleStopSession(w http.ResponseWriter, r *http.Request) {
	var req model.StopSessionRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		middleware.WriteValidationError(w, r, "invalid request body")
		return
	}

	// Validate request
	if err := middleware.ValidateStopSessionRequest(&req); err != nil {
		middleware.WriteValidationError(w, r, err.Error())
		return
	}

	// Stop session
	session, err := h.parkingService.StopSession(&req)
	if err != nil {
		middleware.WriteDatabaseError(w, r, "failed to stop session")
		return
	}

	if session == nil {
		middleware.WriteSessionNotFound(w, r, req.SessionID)
		return
	}

	var endTime string
	if session.EndTime != nil {
		endTime = session.EndTime.Format("2006-01-02T15:04:05Z07:00")
	}

	var durationSeconds int64
	if session.EndTime != nil {
		durationSeconds = int64(session.EndTime.Sub(session.StartTime).Seconds())
	}

	var totalCost float64
	if session.TotalCost != nil {
		totalCost = *session.TotalCost
	}

	var paymentStatus string
	if session.PaymentStatus != nil {
		paymentStatus = *session.PaymentStatus
	}

	response := model.StopSessionResponse{
		SessionID:       session.SessionID,
		StartTime:       session.StartTime.Format("2006-01-02T15:04:05Z07:00"),
		EndTime:         endTime,
		DurationSeconds: durationSeconds,
		TotalCost:       totalCost,
		PaymentStatus:   paymentStatus,
	}

	middleware.WriteJSON(w, http.StatusOK, response)
}

// HandleGetStatus handles GET /api/v1/parking/status/{session_id}
func (h *ParkingHandler) HandleGetStatus(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	sessionID := vars["session_id"]

	status := h.parkingService.GetSessionStatus(sessionID)
	if status == nil {
		middleware.WriteSessionNotFound(w, r, sessionID)
		return
	}

	middleware.WriteJSON(w, http.StatusOK, status)
}
