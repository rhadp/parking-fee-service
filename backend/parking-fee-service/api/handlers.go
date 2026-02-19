// Package api provides REST endpoint handlers for the PARKING_FEE_SERVICE.
//
// It implements handlers for health checks, zone lookup by location, zone
// details, and adapter metadata retrieval. All handlers use the zones.Store
// for data access and return JSON responses.
package api

import (
	"encoding/json"
	"log/slog"
	"net/http"
	"strconv"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/zones"
)

// ErrorResponse represents a JSON error response body.
type ErrorResponse struct {
	Error string `json:"error"`
	Code  string `json:"code"`
}

// ZoneDetailResponse represents the JSON response for GET /api/v1/zones/{zone_id}.
// It includes the full zone data with the polygon but excludes adapter metadata.
type ZoneDetailResponse struct {
	ZoneID       string          `json:"zone_id"`
	Name         string          `json:"name"`
	OperatorName string          `json:"operator_name"`
	Polygon      json.RawMessage `json:"polygon"`
	RateType     string          `json:"rate_type"`
	RateAmount   float64         `json:"rate_amount"`
	Currency     string          `json:"currency"`
}

// Handler holds the dependencies for the REST API handlers.
type Handler struct {
	Store *zones.Store
}

// NewHandler creates a new Handler with the given store.
func NewHandler(store *zones.Store) *Handler {
	return &Handler{Store: store}
}

// RegisterRoutes registers all REST API routes on the given mux.
func (h *Handler) RegisterRoutes(mux *http.ServeMux) {
	mux.HandleFunc("GET /healthz", h.HandleHealthz)
	mux.HandleFunc("GET /api/v1/zones", h.HandleZoneLookup)
	mux.HandleFunc("GET /api/v1/zones/{zone_id}", h.HandleZoneDetails)
	mux.HandleFunc("GET /api/v1/zones/{zone_id}/adapter", h.HandleAdapterMetadata)
}

// HandleHealthz returns HTTP 200 with an empty JSON object for health checks.
func (h *Handler) HandleHealthz(w http.ResponseWriter, r *http.Request) {
	slog.Info("request", "method", r.Method, "path", r.URL.Path)
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(map[string]any{})
}

// HandleZoneLookup handles GET /api/v1/zones?lat=X&lon=Y.
// It validates the lat/lon query parameters, performs a location lookup via
// the store, and returns the matching zones as a JSON array.
func (h *Handler) HandleZoneLookup(w http.ResponseWriter, r *http.Request) {
	slog.Info("request", "method", r.Method, "path", r.URL.Path, "query", r.URL.RawQuery)

	w.Header().Set("Content-Type", "application/json")

	// Parse and validate lat parameter.
	latStr := r.URL.Query().Get("lat")
	if latStr == "" {
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(ErrorResponse{
			Error: "missing required query parameter: lat",
			Code:  "BAD_REQUEST",
		})
		return
	}

	lat, err := strconv.ParseFloat(latStr, 64)
	if err != nil {
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(ErrorResponse{
			Error: "invalid query parameter: lat must be numeric",
			Code:  "BAD_REQUEST",
		})
		return
	}

	// Parse and validate lon parameter.
	lonStr := r.URL.Query().Get("lon")
	if lonStr == "" {
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(ErrorResponse{
			Error: "missing required query parameter: lon",
			Code:  "BAD_REQUEST",
		})
		return
	}

	lon, err := strconv.ParseFloat(lonStr, 64)
	if err != nil {
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(ErrorResponse{
			Error: "invalid query parameter: lon must be numeric",
			Code:  "BAD_REQUEST",
		})
		return
	}

	// Perform location lookup.
	matches := h.Store.FindByLocation(lat, lon)

	slog.Info("zone lookup", "lat", lat, "lon", lon, "matches", len(matches))

	// Return empty array (not null) when no matches.
	if matches == nil {
		matches = []zones.ZoneMatch{}
	}

	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(matches)
}

// HandleZoneDetails handles GET /api/v1/zones/{zone_id}.
// It returns the full zone details including polygon data but excluding
// adapter metadata.
func (h *Handler) HandleZoneDetails(w http.ResponseWriter, r *http.Request) {
	slog.Info("request", "method", r.Method, "path", r.URL.Path)

	w.Header().Set("Content-Type", "application/json")

	zoneID := r.PathValue("zone_id")

	zone, ok := h.Store.GetByID(zoneID)
	if !ok {
		w.WriteHeader(http.StatusNotFound)
		json.NewEncoder(w).Encode(ErrorResponse{
			Error: "zone not found",
			Code:  "NOT_FOUND",
		})
		return
	}

	// Marshal polygon separately to embed as raw JSON.
	polygonJSON, err := json.Marshal(zone.Polygon)
	if err != nil {
		slog.Error("failed to marshal polygon", "zone_id", zoneID, "error", err)
		w.WriteHeader(http.StatusInternalServerError)
		json.NewEncoder(w).Encode(ErrorResponse{
			Error: "internal server error",
			Code:  "INTERNAL_ERROR",
		})
		return
	}

	resp := ZoneDetailResponse{
		ZoneID:       zone.ZoneID,
		Name:         zone.Name,
		OperatorName: zone.OperatorName,
		Polygon:      polygonJSON,
		RateType:     zone.RateType,
		RateAmount:   zone.RateAmount,
		Currency:     zone.Currency,
	}

	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(resp)
}

// HandleAdapterMetadata handles GET /api/v1/zones/{zone_id}/adapter.
// It returns the adapter container image reference and checksum for a zone.
func (h *Handler) HandleAdapterMetadata(w http.ResponseWriter, r *http.Request) {
	slog.Info("request", "method", r.Method, "path", r.URL.Path)

	w.Header().Set("Content-Type", "application/json")

	zoneID := r.PathValue("zone_id")

	zone, ok := h.Store.GetByID(zoneID)
	if !ok {
		w.WriteHeader(http.StatusNotFound)
		json.NewEncoder(w).Encode(ErrorResponse{
			Error: "zone not found",
			Code:  "NOT_FOUND",
		})
		return
	}

	resp := zones.AdapterMetadata{
		ZoneID:   zone.ZoneID,
		ImageRef: zone.AdapterImageRef,
		Checksum: zone.AdapterChecksum,
	}

	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(resp)
}

// LoggingMiddleware wraps an http.Handler to log all incoming requests.
func LoggingMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		slog.Info("incoming request",
			"method", r.Method,
			"path", r.URL.Path,
			"query", r.URL.RawQuery,
			"remote_addr", r.RemoteAddr,
		)
		next.ServeHTTP(w, r)
	})
}
