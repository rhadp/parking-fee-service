// Package handler provides HTTP request handlers for the parking-fee-service.
package handler

import (
	"encoding/json"
	"net/http"
	"strconv"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/geo"
	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/model"
	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/store"
)

// writeJSON encodes v as JSON and writes it with the given HTTP status code.
// Content-Type is explicitly set to application/json before writing headers.
func writeJSON(w http.ResponseWriter, status int, v any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(v)
}

// writeError writes an error response in {"error":"<message>"} format.
func writeError(w http.ResponseWriter, status int, msg string) {
	writeJSON(w, status, map[string]string{"error": msg})
}

// NewOperatorHandler returns a handler for GET /operators?lat=&lon=.
// It validates coordinates, finds matching zones via geofence logic, and returns
// a JSON array of OperatorResponse objects (adapter field excluded).
func NewOperatorHandler(st *store.Store, zones []model.Zone, threshold float64) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		latStr := r.URL.Query().Get("lat")
		lonStr := r.URL.Query().Get("lon")

		// Both parameters are required.
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

		// Find zones that contain or are near the given point.
		point := model.Coordinate{Lat: lat, Lon: lon}
		zoneIDs := geo.FindMatchingZones(point, zones, threshold)

		// Retrieve operators for matching zones.
		operators := st.GetOperatorsByZoneIDs(zoneIDs)

		// Build response list, stripping adapter metadata.
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

// NewAdapterHandler returns a handler for GET /operators/{id}/adapter.
// It looks up the operator by ID and returns adapter metadata JSON.
func NewAdapterHandler(st *store.Store) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		id := r.PathValue("id")
		op, found := st.GetOperator(id)
		if !found {
			writeError(w, http.StatusNotFound, "operator not found")
			return
		}
		writeJSON(w, http.StatusOK, op.Adapter)
	}
}

// HealthHandler returns a handler for GET /health that returns {"status":"ok"}.
func HealthHandler() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		writeJSON(w, http.StatusOK, map[string]string{"status": "ok"})
	}
}
