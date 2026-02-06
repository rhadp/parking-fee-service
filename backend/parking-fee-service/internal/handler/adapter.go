// Package handler provides HTTP handlers for the parking-fee-service.
package handler

import (
	"log/slog"
	"net/http"

	"github.com/gorilla/mux"

	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/middleware"
	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/model"
	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/service"
)

// AdapterHandler handles adapter registry requests.
type AdapterHandler struct {
	adapterService *service.AdapterService
	logger         *slog.Logger
}

// NewAdapterHandler creates a new AdapterHandler.
func NewAdapterHandler(adapterService *service.AdapterService, logger *slog.Logger) *AdapterHandler {
	return &AdapterHandler{
		adapterService: adapterService,
		logger:         logger,
	}
}

// HandleListAdapters handles GET /api/v1/adapters
func (h *AdapterHandler) HandleListAdapters(w http.ResponseWriter, r *http.Request) {
	adapters := h.adapterService.ListAdapters()

	response := model.AdapterListResponse{
		Adapters: adapters,
	}

	middleware.WriteJSON(w, http.StatusOK, response)
}

// HandleGetAdapter handles GET /api/v1/adapters/{adapter_id}
func (h *AdapterHandler) HandleGetAdapter(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	adapterID := vars["adapter_id"]

	adapter := h.adapterService.GetAdapter(adapterID)
	if adapter == nil {
		middleware.WriteAdapterNotFound(w, r, adapterID)
		return
	}

	middleware.WriteJSON(w, http.StatusOK, adapter)
}
