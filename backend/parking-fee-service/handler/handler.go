// Package handler provides HTTP handlers for the parking-fee-service REST API.
package handler

import (
	"encoding/json"
	"net/http"
	"strconv"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/geo"
	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/model"
	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/store"
)

// writeJSON sets Content-Type: application/json, writes the given status code,
// and encodes v as JSON into the response body.
func writeJSON(w http.ResponseWriter, status int, v any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(v)
}

// writeError writes a JSON error response {"error":"<msg>"} with the given status.
func writeError(w http.ResponseWriter, status int, msg string) {
	writeJSON(w, status, map[string]string{"error": msg})
}

// NewOperatorHandler returns an HTTP handler for GET /operators?lat=&lon=.
// It parses and validates lat/lon, finds matching zones, and returns
// a JSON array of OperatorResponse objects (adapter field excluded).
func NewOperatorHandler(s *store.Store, zones []model.Zone, threshold float64) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		latStr := r.URL.Query().Get("lat")
		lonStr := r.URL.Query().Get("lon")

		// Check for missing parameters.
		if latStr == "" || lonStr == "" {
			writeError(w, http.StatusBadRequest, "lat and lon query parameters are required")
			return
		}

		// Parse lat.
		lat, err := strconv.ParseFloat(latStr, 64)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid coordinates")
			return
		}

		// Parse lon.
		lon, err := strconv.ParseFloat(lonStr, 64)
		if err != nil {
			writeError(w, http.StatusBadRequest, "invalid coordinates")
			return
		}

		// Validate coordinate ranges.
		if lat < -90 || lat > 90 || lon < -180 || lon > 180 {
			writeError(w, http.StatusBadRequest, "invalid coordinates")
			return
		}

		point := model.Coordinate{Lat: lat, Lon: lon}
		matchedZoneIDs := geo.FindMatchingZones(point, zones, threshold)
		operators := s.GetOperatorsByZoneIDs(matchedZoneIDs)

		// Convert to OperatorResponse (no adapter field).
		responses := make([]model.OperatorResponse, 0, len(operators))
		for _, op := range operators {
			responses = append(responses, model.OperatorResponse{
				ID:     op.ID,
				Name:   op.Name,
				ZoneID: op.ZoneID,
				Rate:   op.Rate,
			})
		}

		writeJSON(w, http.StatusOK, responses)
	}
}

// NewAdapterHandler returns an HTTP handler for GET /operators/{id}/adapter.
// It extracts the operator ID from the path, looks it up, and returns adapter metadata.
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

// HealthHandler returns an HTTP handler for GET /health.
// It always returns HTTP 200 with {"status":"ok"}.
func HealthHandler() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		writeJSON(w, http.StatusOK, map[string]string{"status": "ok"})
	}
}
