// Package handler implements the HTTP handlers for the PARKING_FEE_SERVICE.
package handler

import (
	"encoding/json"
	"net/http"
	"strconv"

	"github.com/sdv-demo/parking-fee-service/backend/parking-fee-service/geo"
	"github.com/sdv-demo/parking-fee-service/backend/parking-fee-service/model"
	"github.com/sdv-demo/parking-fee-service/backend/parking-fee-service/store"
)

// writeJSON sets Content-Type: application/json, writes the given status code,
// and encodes v as JSON into the response body.
func writeJSON(w http.ResponseWriter, status int, v any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(v)
}

// writeError writes a JSON error response with the given status code.
func writeError(w http.ResponseWriter, status int, message string) {
	writeJSON(w, status, map[string]string{"error": message})
}

// NewOperatorHandler returns an http.HandlerFunc for GET /operators?lat=&lon=.
// It parses and validates the lat/lon query parameters, finds matching zones
// via geo.FindMatchingZones, and returns a JSON array of OperatorResponse
// objects (adapter field excluded).
//
// Requirements: 05-REQ-1.1, 05-REQ-1.E1, 05-REQ-1.E2, 05-REQ-1.E3,
//
//	05-REQ-5.1, 05-REQ-5.2, 05-REQ-5.3
func NewOperatorHandler(s *store.Store, zones []model.Zone, threshold float64) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		q := r.URL.Query()
		latStr := q.Get("lat")
		lonStr := q.Get("lon")

		// 05-REQ-1.E1: missing parameters (empty string means absent).
		if latStr == "" || lonStr == "" {
			writeError(w, http.StatusBadRequest, "lat and lon query parameters are required")
			return
		}

		// 05-REQ-1.E3: non-numeric values.
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

		// 05-REQ-1.E2: out-of-range values.
		if lat < -90 || lat > 90 || lon < -180 || lon > 180 {
			writeError(w, http.StatusBadRequest, "invalid coordinates")
			return
		}

		point := model.Coordinate{Lat: lat, Lon: lon}
		matchedZoneIDs := geo.FindMatchingZones(point, zones, threshold)
		operators := s.GetOperatorsByZoneIDs(matchedZoneIDs)

		// Convert to OperatorResponse (adapter field excluded per 05-REQ-5.2).
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

// NewAdapterHandler returns an http.HandlerFunc for GET /operators/{id}/adapter.
// It extracts the operator ID from the URL path and returns the adapter
// metadata as JSON.
//
// Requirements: 05-REQ-2.1, 05-REQ-2.2, 05-REQ-2.E1, 05-REQ-5.1, 05-REQ-5.3
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

// HealthHandler returns an http.HandlerFunc for GET /health.
// It always responds with HTTP 200 and {"status":"ok"}.
//
// Requirement: 05-REQ-3.1
func HealthHandler() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		writeJSON(w, http.StatusOK, map[string]string{"status": "ok"})
	}
}
