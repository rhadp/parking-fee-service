// Package handler implements HTTP request handlers for the parking-fee-service.
package handler

import (
	"encoding/json"
	"net/http"
	"strconv"

	"parking-fee-service/backend/parking-fee-service/geo"
	"parking-fee-service/backend/parking-fee-service/model"
	"parking-fee-service/backend/parking-fee-service/store"
)

// writeJSON writes v as JSON to w with the given status code and sets
// Content-Type: application/json.
func writeJSON(w http.ResponseWriter, status int, v any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(v)
}

// writeError writes a JSON error response: {"error":"<msg>"}.
func writeError(w http.ResponseWriter, status int, msg string) {
	writeJSON(w, status, map[string]string{"error": msg})
}

// NewOperatorHandler returns an HTTP handler for GET /operators.
// It parses lat/lon query params, finds matching zones, and returns operators.
func NewOperatorHandler(s *store.Store, zones []model.Zone, threshold float64) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		q := r.URL.Query()
		latStr := q.Get("lat")
		lonStr := q.Get("lon")

		// Require both parameters.
		if latStr == "" || lonStr == "" {
			writeError(w, http.StatusBadRequest, "lat and lon query parameters are required")
			return
		}

		lat, err := strconv.ParseFloat(latStr, 64)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid coordinates")
			return
		}
		lon, err := strconv.ParseFloat(lonStr, 64)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid coordinates")
			return
		}

		// Validate ranges.
		if lat < -90 || lat > 90 || lon < -180 || lon > 180 {
			writeError(w, http.StatusBadRequest, "invalid coordinates")
			return
		}

		point := model.Coordinate{Lat: lat, Lon: lon}
		zoneIDs := geo.FindMatchingZones(point, zones, threshold)
		operators := s.GetOperatorsByZoneIDs(zoneIDs)

		// Convert to OperatorResponse (strips Adapter field).
		responses := make([]model.OperatorResponse, len(operators))
		for i, op := range operators {
			responses[i] = model.OperatorResponse{
				ID:     op.ID,
				Name:   op.Name,
				ZoneID: op.ZoneID,
				Rate:   op.Rate,
			}
		}

		writeJSON(w, http.StatusOK, responses)
	}
}

// NewAdapterHandler returns an HTTP handler for GET /operators/{id}/adapter.
func NewAdapterHandler(s *store.Store) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		id := r.PathValue("id")
		op, found := s.GetOperator(id)
		if !found {
			writeError(w, http.StatusNotFound, "operator not found")
			return
		}
		writeJSON(w, http.StatusOK, op.Adapter)
	}
}

// HealthHandler returns an HTTP handler for GET /health.
func HealthHandler() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		writeJSON(w, http.StatusOK, map[string]string{"status": "ok"})
	}
}
