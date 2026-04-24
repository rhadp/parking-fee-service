// Package handler provides HTTP request handlers for the parking-fee-service
// REST API endpoints.
package handler

import (
	"encoding/json"
	"net/http"
	"strconv"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/geo"
	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/model"
	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/store"
)

// writeJSON encodes v as JSON and writes it to w with the given status code.
// Content-Type is set to application/json for all responses.
func writeJSON(w http.ResponseWriter, status int, v any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	json.NewEncoder(w).Encode(v)
}

// writeError writes a JSON error response with the format {"error":"<message>"}.
func writeError(w http.ResponseWriter, status int, message string) {
	writeJSON(w, status, map[string]string{"error": message})
}

// NewOperatorHandler returns an http.HandlerFunc that handles operator
// lookup by location (GET /operators?lat=&lon=).
//
// It parses and validates lat/lon query parameters, finds matching zones
// using geofence logic, retrieves operators for those zones, and returns
// a JSON array of OperatorResponse (which excludes adapter metadata).
func NewOperatorHandler(s *store.Store, zones []model.Zone, threshold float64) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		q := r.URL.Query()
		latStr := q.Get("lat")
		lonStr := q.Get("lon")

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

		if lat < -90 || lat > 90 || lon < -180 || lon > 180 {
			writeError(w, http.StatusBadRequest, "invalid coordinates")
			return
		}

		point := model.Coordinate{Lat: lat, Lon: lon}
		zoneIDs := geo.FindMatchingZones(point, zones, threshold)
		operators := s.GetOperatorsByZoneIDs(zoneIDs)

		// Convert to response format (excludes adapter field).
		response := make([]model.OperatorResponse, 0, len(operators))
		for _, op := range operators {
			response = append(response, model.OperatorResponse{
				ID:     op.ID,
				Name:   op.Name,
				ZoneID: op.ZoneID,
				Rate:   op.Rate,
			})
		}

		writeJSON(w, http.StatusOK, response)
	}
}

// NewAdapterHandler returns an http.HandlerFunc that handles adapter
// metadata retrieval (GET /operators/{id}/adapter).
//
// It extracts the operator ID from the URL path, looks up the operator,
// and returns the adapter metadata as JSON.
func NewAdapterHandler(s *store.Store) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		id := r.PathValue("id")

		op, ok := s.GetOperator(id)
		if !ok {
			writeError(w, http.StatusNotFound, "operator not found")
			return
		}

		writeJSON(w, http.StatusOK, op.Adapter)
	}
}

// HealthHandler returns an http.HandlerFunc that handles health checks
// (GET /health).
func HealthHandler() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		writeJSON(w, http.StatusOK, map[string]string{"status": "ok"})
	}
}
