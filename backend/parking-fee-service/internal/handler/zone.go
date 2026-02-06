// Package handler provides HTTP handlers for the parking-fee-service.
package handler

import (
	"log/slog"
	"net/http"
	"strconv"

	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/middleware"
	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/model"
	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/service"
)

// ZoneHandler handles zone lookup requests.
type ZoneHandler struct {
	zoneService *service.ZoneService
	logger      *slog.Logger
}

// NewZoneHandler creates a new ZoneHandler.
func NewZoneHandler(zoneService *service.ZoneService, logger *slog.Logger) *ZoneHandler {
	return &ZoneHandler{
		zoneService: zoneService,
		logger:      logger,
	}
}

// HandleGetZone handles GET /api/v1/zones?lat=X&lng=Y
func (h *ZoneHandler) HandleGetZone(w http.ResponseWriter, r *http.Request) {
	// Parse lat parameter
	latStr := r.URL.Query().Get("lat")
	if latStr == "" {
		middleware.WriteInvalidParameters(w, r, "lat query parameter is required")
		return
	}

	lat, err := strconv.ParseFloat(latStr, 64)
	if err != nil {
		middleware.WriteInvalidParameters(w, r, "lat must be a valid number")
		return
	}

	// Parse lng parameter
	lngStr := r.URL.Query().Get("lng")
	if lngStr == "" {
		middleware.WriteInvalidParameters(w, r, "lng query parameter is required")
		return
	}

	lng, err := strconv.ParseFloat(lngStr, 64)
	if err != nil {
		middleware.WriteInvalidParameters(w, r, "lng must be a valid number")
		return
	}

	// Validate coordinates
	if err := middleware.ValidateCoordinates(lat, lng); err != nil {
		middleware.WriteInvalidParameters(w, r, err.Error())
		return
	}

	// Find zone
	zone := h.zoneService.FindZoneByLocation(lat, lng)
	if zone == nil {
		middleware.WriteZoneNotFound(w, r)
		return
	}

	// Return zone response
	response := model.ZoneResponse{
		ZoneID:          zone.ZoneID,
		OperatorName:    zone.OperatorName,
		HourlyRate:      zone.HourlyRate,
		Currency:        zone.Currency,
		AdapterImageRef: zone.AdapterImageRef,
		AdapterChecksum: zone.AdapterChecksum,
	}

	middleware.WriteJSON(w, http.StatusOK, response)
}
